use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::forms::{InputField, ServerForm};
use crate::themes::ThemeManager;
use ratatui::widgets::ListState;

/// Represents the health status of a server
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum HealthStatus {
    #[default]
    Online,
    Offline,
    Connecting,
    Warning,
    Unknown,
}

impl HealthStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            HealthStatus::Online => "ONLINE",
            HealthStatus::Offline => "OFFLINE",
            HealthStatus::Connecting => "CONNECTING",
            HealthStatus::Warning => "WARNING",
            HealthStatus::Unknown => "UNKNOWN",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            // Filled vs hollow, not green vs red. These were the same glyph
            // separated only by colour, which is invisible to red-green colour
            // blindness (~8% of men), in a screenshot, or on a mono terminal.
            HealthStatus::Online => "●",
            HealthStatus::Offline => "○",
            HealthStatus::Connecting => "◐",
            HealthStatus::Warning => "▲",
            HealthStatus::Unknown => "?",
        }
    }
}

/// At-a-glance hint of how a server authenticates.
///
/// IMPORTANT: this reflects the *local connection configuration* only — it is
/// NOT a security audit of the remote host. It cannot tell you whether the
/// server itself is patched, hardened, or compromised; it only surfaces which
/// auth method you've configured, so weaker choices (password) stand out.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum AuthStrength {
    /// Public-key file (`-i <key>`)
    Key,
    /// ssh-agent
    Agent,
    /// Password authentication (weaker — phishable/brute-forceable)
    Password,
    /// Keyboard-interactive
    Interactive,
    /// Not yet assessed
    #[default]
    Unknown,
}

impl AuthStrength {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthStrength::Key => "KEY",
            AuthStrength::Agent => "AGENT",
            AuthStrength::Password => "PASSWORD",
            AuthStrength::Interactive => "INTERACTIVE",
            AuthStrength::Unknown => "UNKNOWN",
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            AuthStrength::Key => "🔑",
            AuthStrength::Agent => "🔑",
            AuthStrength::Password => "⚠",
            AuthStrength::Interactive => "💬",
            AuthStrength::Unknown => "?",
        }
    }
}

/// How many latency samples to retain per server for the sparkline.
pub const LATENCY_HISTORY_LEN: usize = 32;

/// Connection statistics for monitoring.
///
/// Reachability probes and user-initiated SSH launches are counted separately.
/// Conflating them (the old behaviour) made "Total Connections" really mean
/// "successful TCP probes", which drifted upward on its own and made the
/// analytics view meaningless.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub latency: Option<Duration>,
    /// Recent latency measurements in ms, oldest first.
    pub latency_history: Vec<u32>,
    pub uptime_percentage: f32,
    /// Last time the user actually opened a session to this host.
    pub last_connected: Option<DateTime<Utc>>,
    /// Last time a health probe found the host reachable.
    pub last_seen_online: Option<DateTime<Utc>>,
    /// Sessions the user launched.
    pub connection_count: u32,
    /// Session launches that failed.
    pub failed_attempts: u32,
    /// Background reachability probes that succeeded.
    pub probe_success: u32,
    /// Background reachability probes that failed.
    pub probe_failure: u32,
    pub total_session_duration: Duration,
    pub average_session_duration: Duration,
    pub peak_usage_hour: Option<u8>, // 0-23 hour of day
}

impl Default for ConnectionStats {
    fn default() -> Self {
        Self {
            latency: None,
            latency_history: Vec::new(),
            uptime_percentage: 0.0,
            last_connected: None,
            last_seen_online: None,
            connection_count: 0,
            failed_attempts: 0,
            probe_success: 0,
            probe_failure: 0,
            total_session_duration: Duration::from_secs(0),
            average_session_duration: Duration::from_secs(0),
            peak_usage_hour: None,
        }
    }
}

impl ConnectionStats {
    /// Record a latency sample, keeping the window bounded.
    pub fn push_latency(&mut self, latency: Duration) {
        let ms = latency.as_millis().min(u32::MAX as u128) as u32;
        self.latency_history.push(ms);
        if self.latency_history.len() > LATENCY_HISTORY_LEN {
            let excess = self.latency_history.len() - LATENCY_HISTORY_LEN;
            self.latency_history.drain(0..excess);
        }
    }

    /// Share of health probes that found the host reachable.
    pub fn recompute_uptime(&mut self) {
        let total = self.probe_success + self.probe_failure;
        self.uptime_percentage = if total == 0 {
            0.0
        } else {
            self.probe_success as f32 / total as f32 * 100.0
        };
    }
}

/// SSH server connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConnection {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: AuthMethod,
    pub description: Option<String>,
    pub tags: Vec<String>,
    /// ssh ConnectTimeout in seconds; `None` uses the built-in default.
    pub timeout: Option<u64>,
    /// Bastion this host is reached through (`ProxyJump`), if any.
    pub proxy_jump: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,

    // Status information (not persisted, computed at runtime)
    #[serde(skip)]
    pub health_status: HealthStatus,
    #[serde(skip)]
    pub auth_strength: AuthStrength,
    #[serde(skip)]
    pub stats: ConnectionStats,
    /// Last health-check error message, if the most recent check failed
    /// (cleared on a successful check). Runtime-only.
    #[serde(skip)]
    pub last_error: Option<String>,

    // Session tracking (not persisted)
    #[serde(skip)]
    pub active_sessions: Vec<SessionInfo>,
}

/// Information about an active SSH session
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub pid: u32,
    pub started_at: DateTime<Utc>,
    pub window_title: String,
    pub server_name: String,
    pub is_idle: bool,
}

impl SessionInfo {
    pub fn new(pid: u32, window_title: String, server_name: String, _server_id: String) -> Self {
        Self {
            pid,
            started_at: Utc::now(),
            window_title,
            server_name,
            is_idle: false,
        }
    }

    pub fn duration(&self) -> Duration {
        Utc::now()
            .signed_duration_since(self.started_at)
            .to_std()
            .unwrap_or_default()
    }

    pub fn format_duration(&self) -> String {
        let duration = self.duration();
        let hours = duration.as_secs() / 3600;
        let minutes = (duration.as_secs() % 3600) / 60;
        let seconds = duration.as_secs() % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }
}

/// Connection history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionHistoryEntry {
    pub server_id: String,
    pub server_name: String,
    pub connected_at: DateTime<Utc>,
    pub duration: Option<Duration>,
}

/// Analytics data for the entire application.
// Data model for the analytics feature; not yet populated at runtime.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalAnalytics {
    pub total_connections: u32,
    pub total_session_time: Duration,
    pub daily_connections: Vec<DailyUsage>,
    pub most_used_servers: Vec<ServerUsage>,
    pub connection_success_rate: f32,
    pub average_session_duration: Duration,
}

/// Daily usage statistics
#[allow(dead_code)] // part of the analytics data model; not yet populated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsage {
    pub date: DateTime<Utc>,
    pub connection_count: u32,
    pub session_duration: Duration,
}

/// Server usage statistics for ranking
#[allow(dead_code)] // part of the analytics data model; not yet populated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerUsage {
    pub server_id: String,
    pub server_name: String,
    pub connection_count: u32,
    pub total_duration: Duration,
    pub last_used: DateTime<Utc>,
}

/// Authentication methods for SSH connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    Password,
    PublicKey { key_path: String },
    Agent,
    Interactive,
}

impl ServerConnection {
    pub fn new(name: String, host: String, port: u16, username: String) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            host,
            port,
            username,
            auth_method: AuthMethod::Agent,
            description: None,
            tags: Vec::new(),
            timeout: None,
            proxy_jump: None,
            created_at: now,
            last_modified: now,
            health_status: HealthStatus::Unknown,
            auth_strength: AuthStrength::Unknown,
            stats: ConnectionStats::default(),
            last_error: None,
            active_sessions: Vec::new(),
        }
    }

    pub fn connection_string(&self) -> String {
        format!("{}@{}:{}", self.username, self.host, self.port)
    }

    pub fn is_healthy(&self) -> bool {
        matches!(
            self.health_status,
            HealthStatus::Online | HealthStatus::Warning
        )
    }

    /// Check if this server has active SSH sessions
    pub fn has_active_sessions(&self) -> bool {
        !self.active_sessions.is_empty()
    }

    /// Get the count of active sessions
    pub fn session_count(&self) -> usize {
        self.active_sessions.len()
    }

    /// Add an active session
    pub fn add_session(&mut self, pid: u32, window_title: String) {
        self.active_sessions.push(SessionInfo::new(
            pid,
            window_title,
            self.name.clone(),
            self.id.clone(),
        ));
    }
}

/// Application state and server manager
#[derive(Debug, Default)]
pub struct ServerManager {
    pub connections: HashMap<String, ServerConnection>,
    pub selected_index: usize,
    pub filter: String,
    pub show_only_online: bool,
    pub connection_history: Vec<ConnectionHistoryEntry>,
    pub active_session_count: usize,
}

impl ServerManager {
    pub fn remove_connection(&mut self, id: &str) -> Option<ServerConnection> {
        self.connections.remove(id)
    }

    pub fn get_connection(&self, id: &str) -> Option<&ServerConnection> {
        self.connections.get(id)
    }

    pub fn get_connection_mut(&mut self, id: &str) -> Option<&mut ServerConnection> {
        self.connections.get_mut(id)
    }

    pub fn filtered_connections(&self) -> Vec<&ServerConnection> {
        let mut connections: Vec<&ServerConnection> = self
            .connections
            .values()
            .filter(|conn| {
                // Filter by search term
                if !self.filter.is_empty() {
                    let needle = self.filter.to_lowercase();
                    let matches = conn.name.to_lowercase().contains(&needle)
                        || conn.host.to_lowercase().contains(&needle)
                        || conn.username.to_lowercase().contains(&needle)
                        || conn
                            .description
                            .as_deref()
                            .is_some_and(|d| d.to_lowercase().contains(&needle))
                        || conn.tags.iter().any(|t| t.to_lowercase().contains(&needle));
                    if !matches {
                        return false;
                    }
                }

                // Filter by online status
                if self.show_only_online && !conn.is_healthy() {
                    return false;
                }

                true
            })
            .collect();

        // Sort by name
        connections.sort_by(|a, b| a.name.cmp(&b.name));
        connections
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Keep `selected_index` inside the visible list.
    ///
    /// The list shrinks whenever a server is deleted or the filter narrows; an
    /// out-of-range index silently blanked the details panel until the user
    /// happened to press a movement key.
    pub fn clamp_selection(&mut self) {
        let len = self.filtered_connections().len();
        if len == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= len {
            self.selected_index = len - 1;
        }
    }

    /// Re-select a connection by id after the list has been re-sorted.
    pub fn select_by_id(&mut self, id: &str) {
        if let Some(idx) = self.filtered_connections().iter().position(|c| c.id == id) {
            self.selected_index = idx;
        } else {
            self.clamp_selection();
        }
    }

    pub fn online_count(&self) -> usize {
        self.connections
            .values()
            .filter(|conn| conn.is_healthy())
            .count()
    }

    /// Add a connection to history
    pub fn add_to_history(&mut self, server_id: String, server_name: String) {
        let entry = ConnectionHistoryEntry {
            server_id,
            server_name,
            connected_at: Utc::now(),
            duration: None,
        };

        self.connection_history.insert(0, entry);

        // Keep only last 50 entries
        if self.connection_history.len() > 50 {
            self.connection_history.truncate(50);
        }
    }

    /// Update active session count
    pub fn update_session_count(&mut self) {
        self.active_session_count = self
            .connections
            .values()
            .map(|conn| conn.session_count())
            .sum();
    }
}

/// Layout configurations for the UI
#[derive(Debug, Clone, PartialEq)]
// The `Panel` suffix reads better than bare `Two`/`Three`/`Single` at call
// sites, which always qualify with `LayoutMode::`.
#[allow(clippy::enum_variant_names)]
pub enum LayoutMode {
    /// Two panels: server list and details
    TwoPanel,
    /// Three panels: server list, details, and metrics
    ThreePanel,
    /// Single panel mode (full-width server list)
    SinglePanel,
}

/// Panel sizing configuration
#[derive(Debug, Clone)]
pub struct PanelLayout {
    pub mode: LayoutMode,
    /// Panel size percentages [left, center, right] (0-100)
    /// For TwoPanel: [server_list, details, 0]
    /// For ThreePanel: [server_list, details, metrics]
    /// For SinglePanel: [100, 0, 0]
    pub panel_sizes: [u16; 3],
}

impl Default for PanelLayout {
    fn default() -> Self {
        Self {
            mode: LayoutMode::ThreePanel,
            panel_sizes: [50, 25, 25], // Default: 50% server list, 25% details, 25% metrics
        }
    }
}

impl PanelLayout {
    /// Parse the persisted layout name. Unknown values fall back to the
    /// default rather than failing the whole config load.
    pub fn from_name(name: &str) -> Self {
        let mode = match name {
            "single" => LayoutMode::SinglePanel,
            "two" => LayoutMode::TwoPanel,
            _ => LayoutMode::ThreePanel,
        };
        let panel_sizes = match mode {
            LayoutMode::SinglePanel => [100, 0, 0],
            LayoutMode::TwoPanel => [70, 30, 0],
            LayoutMode::ThreePanel => [50, 25, 25],
        };
        Self { mode, panel_sizes }
    }

    /// Stable name for persisting the current mode.
    pub fn name(&self) -> &'static str {
        match self.mode {
            LayoutMode::SinglePanel => "single",
            LayoutMode::TwoPanel => "two",
            LayoutMode::ThreePanel => "three",
        }
    }

    /// Get constraints for ratatui layout based on current configuration
    pub fn get_constraints(&self) -> Vec<ratatui::layout::Constraint> {
        use ratatui::layout::Constraint;
        match self.mode {
            LayoutMode::SinglePanel => vec![Constraint::Percentage(100)],
            LayoutMode::TwoPanel => vec![
                Constraint::Percentage(self.panel_sizes[0]),
                Constraint::Percentage(self.panel_sizes[1]),
            ],
            LayoutMode::ThreePanel => vec![
                Constraint::Percentage(self.panel_sizes[0]),
                Constraint::Percentage(self.panel_sizes[1]),
                Constraint::Percentage(self.panel_sizes[2]),
            ],
        }
    }

    /// Toggle between layout modes
    pub fn cycle_layout(&mut self) {
        self.mode = match self.mode {
            LayoutMode::TwoPanel => LayoutMode::ThreePanel,
            LayoutMode::ThreePanel => LayoutMode::SinglePanel,
            LayoutMode::SinglePanel => LayoutMode::TwoPanel,
        };

        // Update panel sizes for the new mode
        self.panel_sizes = match self.mode {
            LayoutMode::SinglePanel => [100, 0, 0],
            LayoutMode::TwoPanel => [70, 30, 0],
            LayoutMode::ThreePanel => [50, 25, 25],
        };
    }

    /// Resize panels (increase left panel, decrease right)
    pub fn resize_panels(&mut self, delta: i16) {
        match self.mode {
            LayoutMode::TwoPanel => {
                let new_left = (self.panel_sizes[0] as i16 + delta).clamp(20, 80) as u16;
                self.panel_sizes[0] = new_left;
                self.panel_sizes[1] = 100 - new_left;
            }
            LayoutMode::ThreePanel => {
                // For three panels, resize first two and adjust third
                let new_left = (self.panel_sizes[0] as i16 + delta).clamp(20, 60) as u16;
                let remaining = 100 - new_left;
                self.panel_sizes[0] = new_left;
                self.panel_sizes[1] = remaining / 2;
                self.panel_sizes[2] = remaining - self.panel_sizes[1];
            }
            LayoutMode::SinglePanel => {} // No resizing in single panel
        }
    }
}

/// Application modes for different UI states
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    /// Incremental search over the server list; keystrokes edit the filter.
    Search,
    /// Theme picker overlay, with live preview as you move through the list.
    ThemeSelector,
    /// Hosts grouped by the bastion they are reached through.
    Topology,
    AddServer,
    EditServer(String),
    ConfirmDelete(String),
    /// Confirm discarding unsaved changes in the add/edit form.
    ConfirmDiscard,
    Help,
    Connecting(String),
    Loading(LoadingContext),
    History,
    Analytics,
    Sessions,
}

/// Context for different loading operations
#[derive(Debug, Clone, PartialEq)]
pub enum LoadingContext {
    RefreshingHealth { total: usize, completed: usize },
}

/// Tooltip information for UI elements
#[derive(Debug, Clone)]
pub struct TooltipInfo {
    pub title: String,
    pub description: String,
    pub key_hint: Option<String>,
    pub category: TooltipCategory,
}

/// Categories for organizing tooltips
#[allow(dead_code)] // Theme/Layout categories reserved for upcoming tooltips
#[derive(Debug, Clone, PartialEq)]
pub enum TooltipCategory {
    Navigation,
    Server,
    Session,
    Theme,
    Layout,
    System,
}

/// Performance metrics for the application
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    #[allow(dead_code)] // tracked for a future metrics view
    pub app_startup_time: Option<Duration>,
    pub last_refresh_duration: Option<Duration>,
    pub average_refresh_time: Duration,
    pub total_refreshes: u32,
    #[allow(dead_code)] // tracked for a future metrics view
    pub memory_usage: Option<u64>, // In bytes
    pub frame_rate: f32, // Frames per second
    pub ui_render_time: Option<Duration>,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            app_startup_time: None,
            last_refresh_duration: None,
            average_refresh_time: Duration::from_millis(0),
            total_refreshes: 0,
            memory_usage: None,
            frame_rate: 0.0,
            ui_render_time: None,
        }
    }
}

/// Global application state
#[derive(Debug)]
pub struct AppState {
    pub server_manager: ServerManager,
    pub mode: AppMode,
    pub should_quit: bool,
    pub show_popup: bool,
    pub popup_message: String,
    pub popup_shown_at: Option<DateTime<Utc>>,
    pub last_update: DateTime<Utc>,
    pub server_form: Option<ServerForm>,
    /// Scroll/selection state for the server list widget. Without this the list
    /// rendered statelessly and never scrolled — selecting past the visible
    /// rows highlighted something off-screen.
    pub server_list_state: ListState,
    pub session_list_state: ListState,
    pub topology_list_state: ListState,
    /// Cursor into the topology view's selectable rows.
    pub topology_selected: usize,
    /// The in-progress search string while in `AppMode::Search`.
    pub search_input: InputField,
    /// Path of the config file, shown in the metrics panel.
    pub config_path: String,
    /// Scroll offset for the help popup, which is taller than any terminal.
    pub help_scroll: u16,
    pub globe_animation_frame: u8,
    pub session_selected_index: usize,
    pub session_filter: String,
    pub theme_manager: ThemeManager,
    /// Index into `ThemeVariant::all()` while the theme picker is open.
    pub theme_selector_index: usize,
    /// Theme to restore if the picker is cancelled.
    pub theme_before_preview: Option<crate::themes::ThemeVariant>,
    pub layout: PanelLayout,
    pub show_tooltips: bool,
    pub current_tooltip: Option<TooltipInfo>,
    pub tooltip_shown_at: Option<DateTime<Utc>>,
    // Performance and loading state
    pub performance: PerformanceMetrics,
    /// Set after Ghost's TUI is torn down (direct SSH mode) so the next frame
    /// repaints every cell instead of diffing against a stale buffer.
    pub force_redraw: bool,
    pub loading_start_time: Option<DateTime<Utc>>,
    pub last_frame_time: Option<DateTime<Utc>>,
    pub frame_count: u64,
}

impl AppState {
    /// Get the current globe character for animation
    pub fn get_globe_char(&self) -> &'static str {
        match (self.globe_animation_frame / 20) % 4 {
            0 => "◐", // Half circle rotating
            1 => "◓", // Different half circle
            2 => "◑", // Another rotation
            3 => "◒", // Complete rotation
            _ => "◐", // Fallback
        }
    }

    /// Get all active sessions across all servers
    pub fn get_all_sessions(&self) -> Vec<&SessionInfo> {
        self.server_manager
            .connections
            .values()
            .flat_map(|conn| &conn.active_sessions)
            .collect()
    }

    /// Get filtered sessions based on current filter
    pub fn get_filtered_sessions(&self) -> Vec<&SessionInfo> {
        let mut sessions = self.get_all_sessions();

        if !self.session_filter.is_empty() {
            let filter_lower = self.session_filter.to_lowercase();
            sessions.retain(|session| {
                session.server_name.to_lowercase().contains(&filter_lower)
                    || session.window_title.to_lowercase().contains(&filter_lower)
                    || session.pid.to_string().contains(&filter_lower)
            });
        }

        // Sort by start time (newest first)
        sessions.sort_by_key(|s| std::cmp::Reverse(s.started_at));
        sessions
    }

    /// Get session by PID
    #[allow(dead_code)] // helper kept for session lookups; not yet called
    pub fn get_session_by_pid(&self, pid: u32) -> Option<(&ServerConnection, &SessionInfo)> {
        for conn in self.server_manager.connections.values() {
            for session in &conn.active_sessions {
                if session.pid == pid {
                    return Some((conn, session));
                }
            }
        }
        None
    }

    /// Show a tooltip with the given information
    pub fn show_tooltip(
        &mut self,
        title: String,
        description: String,
        key_hint: Option<String>,
        category: TooltipCategory,
    ) {
        if self.show_tooltips {
            self.current_tooltip = Some(TooltipInfo {
                title,
                description,
                key_hint,
                category,
            });
            self.tooltip_shown_at = Some(Utc::now());
        }
    }

    /// Hide the current tooltip
    pub fn hide_tooltip(&mut self) {
        self.current_tooltip = None;
        self.tooltip_shown_at = None;
    }

    /// Check if tooltip should be auto-dismissed (after 3 seconds)
    pub fn should_auto_dismiss_tooltip(&self) -> bool {
        if let Some(shown_at) = self.tooltip_shown_at {
            Utc::now().signed_duration_since(shown_at).num_seconds() >= 3
        } else {
            false
        }
    }

    /// Toggle tooltips on/off
    pub fn toggle_tooltips(&mut self) {
        self.show_tooltips = !self.show_tooltips;
        if !self.show_tooltips {
            self.hide_tooltip();
        }
    }

    /// Start a loading operation
    pub fn start_loading(&mut self, context: LoadingContext) {
        self.mode = AppMode::Loading(context);
        self.loading_start_time = Some(Utc::now());
    }

    /// Complete loading operation and return to normal mode
    pub fn complete_loading(&mut self) {
        if let Some(start_time) = self.loading_start_time {
            let duration = Utc::now()
                .signed_duration_since(start_time)
                .to_std()
                .unwrap_or_default();

            // Update performance metrics for health refresh
            if let AppMode::Loading(LoadingContext::RefreshingHealth { .. }) = self.mode {
                self.performance.last_refresh_duration = Some(duration);
                self.performance.total_refreshes += 1;
                self.update_average_refresh_time(duration);
            }
        }

        self.mode = AppMode::Normal;
        self.loading_start_time = None;
    }

    /// Update frame rate calculation
    pub fn update_frame_rate(&mut self) {
        let now = Utc::now();
        self.frame_count += 1;

        if let Some(last_frame) = self.last_frame_time {
            let frame_duration = now
                .signed_duration_since(last_frame)
                .to_std()
                .unwrap_or_default();

            if frame_duration.as_millis() > 0 {
                let current_fps = 1000.0 / frame_duration.as_millis() as f32;
                // Smooth the frame rate with exponential moving average
                self.performance.frame_rate = self.performance.frame_rate * 0.9 + current_fps * 0.1;
            }
        }

        self.last_frame_time = Some(now);
    }

    /// Get loading progress as a percentage string
    #[allow(dead_code)] // helper for loading UI; not yet called
    pub fn get_loading_progress_display(&self) -> String {
        match &self.mode {
            AppMode::Loading(LoadingContext::RefreshingHealth { completed, total }) => {
                if *total > 0 {
                    format!("{}/{}", completed, total)
                } else {
                    "0/0".to_string()
                }
            }
            _ => "".to_string(),
        }
    }

    /// Update average refresh time
    fn update_average_refresh_time(&mut self, duration: Duration) {
        if self.performance.total_refreshes > 0 {
            let total_time = self.performance.average_refresh_time.as_millis() as f64
                * (self.performance.total_refreshes - 1) as f64
                + duration.as_millis() as f64;
            self.performance.average_refresh_time = Duration::from_millis(
                (total_time / self.performance.total_refreshes as f64) as u64,
            );
        } else {
            self.performance.average_refresh_time = duration;
        }
    }

    /// Kill a session by PID, removing it from tracking on success.
    pub fn kill_session(&mut self, pid: u32) -> Result<(), String> {
        crate::models::process::terminate(pid)?;
        for conn in self.server_manager.connections.values_mut() {
            conn.active_sessions.retain(|s| s.pid != pid);
        }
        self.server_manager.update_session_count();
        Ok(())
    }
}

/// Process liveness and termination.
///
/// These used to fork `kill`/`tasklist` subprocesses — and `is_alive` was
/// called for every tracked session on every 50 ms tick, which meant dozens of
/// `fork`+`exec` pairs per second just to ask whether a PID existed.
pub mod process {
    /// PID 0 is not a process: on POSIX it addresses *every process in the
    /// caller's process group*, so passing it to `kill` would signal Ghost
    /// itself. Treat it as invalid everywhere rather than relying on callers.
    fn is_valid_pid(pid: u32) -> bool {
        pid > 0
    }

    /// Is a process with this PID still running?
    #[cfg(unix)]
    pub fn is_alive(pid: u32) -> bool {
        if !is_valid_pid(pid) {
            return false;
        }
        // Signal 0 performs error checking (does it exist, may we signal it?)
        // without actually delivering anything.
        unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
    }

    #[cfg(windows)]
    pub fn is_alive(pid: u32) -> bool {
        if !is_valid_pid(pid) {
            return false;
        }
        use std::process::Command;
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }

    /// Ask a process to terminate.
    #[cfg(unix)]
    pub fn terminate(pid: u32) -> Result<(), String> {
        if !is_valid_pid(pid) {
            return Err(
                "Refusing to signal PID 0 (would target our own process group)".to_string(),
            );
        }
        let rc = unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
        if rc == 0 {
            Ok(())
        } else {
            Err(format!(
                "Failed to signal PID {}: {}",
                pid,
                std::io::Error::last_os_error()
            ))
        }
    }

    #[cfg(windows)]
    pub fn terminate(pid: u32) -> Result<(), String> {
        if !is_valid_pid(pid) {
            return Err("Refusing to signal PID 0".to_string());
        }
        use std::process::Command;
        match Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output()
        {
            Ok(o) if o.status.success() => Ok(()),
            Ok(_) => Err(format!("Failed to kill PID {}", pid)),
            Err(e) => Err(format!("Error killing PID {}: {}", pid, e)),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            server_manager: ServerManager::default(),
            mode: AppMode::Normal,
            should_quit: false,
            show_popup: false,
            popup_message: String::new(),
            popup_shown_at: None,
            last_update: Utc::now(),
            server_form: None,
            server_list_state: ListState::default(),
            session_list_state: ListState::default(),
            topology_list_state: ListState::default(),
            topology_selected: 0,
            search_input: InputField::new("Search", "name, host, user, tag…"),
            config_path: String::new(),
            help_scroll: 0,
            globe_animation_frame: 0,
            session_selected_index: 0,
            session_filter: String::new(),
            theme_manager: ThemeManager::default(),
            theme_selector_index: 0,
            theme_before_preview: None,
            layout: PanelLayout::default(),
            show_tooltips: true, // Enable tooltips by default
            current_tooltip: None,
            tooltip_shown_at: None,
            // Performance and loading state
            performance: PerformanceMetrics::default(),
            force_redraw: false,
            loading_start_time: None,
            last_frame_time: None,
            frame_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager_with(names: &[(&str, &str, &str)]) -> ServerManager {
        let mut m = ServerManager::default();
        for (name, host, user) in names {
            let conn =
                ServerConnection::new(name.to_string(), host.to_string(), 22, user.to_string());
            m.connections.insert(conn.id.clone(), conn);
        }
        m
    }

    #[test]
    fn filter_matches_name_host_user_tag_and_description() {
        let mut m = manager_with(&[("Alpha", "alpha.example.com", "root")]);
        let id = m.connections.keys().next().unwrap().clone();
        {
            let conn = m.connections.get_mut(&id).unwrap();
            conn.tags = vec!["production".to_string()];
            conn.description = Some("the main box".to_string());
        }

        for needle in ["alph", "EXAMPLE", "root", "produc", "main box"] {
            m.filter = needle.to_string();
            assert_eq!(m.filtered_connections().len(), 1, "failed on {:?}", needle);
        }

        m.filter = "nomatch".to_string();
        assert!(m.filtered_connections().is_empty());
    }

    #[test]
    fn clamp_selection_keeps_the_index_in_range() {
        // Regression: deleting the last server or narrowing the filter left
        // selected_index past the end, blanking the details panel.
        let mut m = manager_with(&[("a", "a", "u"), ("b", "b", "u"), ("c", "c", "u")]);
        m.selected_index = 2;

        m.filter = "a".to_string();
        m.clamp_selection();
        assert_eq!(m.selected_index, 0);

        m.filter.clear();
        m.selected_index = 2;
        let id = m.filtered_connections()[2].id.clone();
        m.remove_connection(&id);
        m.clamp_selection();
        assert_eq!(m.selected_index, 1);
    }

    #[test]
    fn clamp_selection_handles_an_empty_list() {
        let mut m = ServerManager {
            selected_index: 7,
            ..Default::default()
        };
        m.clamp_selection();
        assert_eq!(m.selected_index, 0);
    }

    #[test]
    fn select_by_id_survives_a_rename_resort() {
        // The list sorts by name, so renaming can move a server; the cursor
        // should follow the server, not the index.
        let mut m = manager_with(&[("aaa", "a", "u"), ("zzz", "z", "u")]);
        let zzz = m
            .filtered_connections()
            .iter()
            .find(|c| c.name == "zzz")
            .unwrap()
            .id
            .clone();

        m.connections.get_mut(&zzz).unwrap().name = "000".to_string();
        m.select_by_id(&zzz);
        assert_eq!(m.filtered_connections()[m.selected_index].id, zzz);
    }

    #[test]
    fn filtered_connections_are_sorted_by_name() {
        let m = manager_with(&[
            ("charlie", "c", "u"),
            ("alpha", "a", "u"),
            ("bravo", "b", "u"),
        ]);
        let names: Vec<&str> = m
            .filtered_connections()
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(names, vec!["alpha", "bravo", "charlie"]);
    }

    #[test]
    fn online_filter_hides_unhealthy_servers() {
        let mut m = manager_with(&[("a", "a", "u"), ("b", "b", "u")]);
        let ids: Vec<String> = m.connections.keys().cloned().collect();
        m.connections.get_mut(&ids[0]).unwrap().health_status = HealthStatus::Online;
        m.connections.get_mut(&ids[1]).unwrap().health_status = HealthStatus::Offline;

        m.show_only_online = true;
        assert_eq!(m.filtered_connections().len(), 1);
        assert_eq!(m.online_count(), 1);
    }

    #[test]
    fn latency_history_stays_bounded() {
        let mut stats = ConnectionStats::default();
        for ms in 0..(LATENCY_HISTORY_LEN as u64 + 20) {
            stats.push_latency(Duration::from_millis(ms));
        }
        assert_eq!(stats.latency_history.len(), LATENCY_HISTORY_LEN);
        // Oldest samples are dropped, newest retained.
        assert_eq!(
            *stats.latency_history.last().unwrap(),
            (LATENCY_HISTORY_LEN as u32) + 19
        );
    }

    #[test]
    fn uptime_is_computed_from_probes_only() {
        let mut stats = ConnectionStats::default();
        stats.recompute_uptime();
        assert_eq!(stats.uptime_percentage, 0.0);

        stats.probe_success = 3;
        stats.probe_failure = 1;
        stats.recompute_uptime();
        assert_eq!(stats.uptime_percentage, 75.0);

        // Launch counters must not affect uptime.
        stats.connection_count = 100;
        stats.failed_attempts = 100;
        stats.recompute_uptime();
        assert_eq!(stats.uptime_percentage, 75.0);
    }

    #[test]
    fn history_is_capped_at_fifty_entries() {
        let mut m = ServerManager::default();
        for i in 0..60 {
            m.add_to_history(format!("id{}", i), format!("server{}", i));
        }
        assert_eq!(m.connection_history.len(), 50);
        // Newest first.
        assert_eq!(m.connection_history[0].server_name, "server59");
    }

    #[test]
    fn layout_cycles_through_every_mode() {
        let mut layout = PanelLayout::default();
        let start = layout.mode.clone();
        let mut seen = vec![start.clone()];
        for _ in 0..2 {
            layout.cycle_layout();
            seen.push(layout.mode.clone());
        }
        layout.cycle_layout();
        assert_eq!(layout.mode, start);
        assert_eq!(seen.len(), 3);
        assert_eq!(
            layout.panel_sizes.iter().map(|&s| s as u32).sum::<u32>(),
            100
        );
    }

    #[test]
    fn panel_resize_stays_within_bounds_and_sums_to_100() {
        let mut layout = PanelLayout::default();
        for _ in 0..40 {
            layout.resize_panels(-5);
        }
        assert!(layout.panel_sizes[0] >= 20);
        assert_eq!(
            layout.panel_sizes.iter().map(|&s| s as u32).sum::<u32>(),
            100
        );

        for _ in 0..40 {
            layout.resize_panels(5);
        }
        assert!(layout.panel_sizes[0] <= 60);
        assert_eq!(
            layout.panel_sizes.iter().map(|&s| s as u32).sum::<u32>(),
            100
        );
    }

    #[test]
    fn pid_zero_is_never_treated_as_a_live_process() {
        // On POSIX, kill(0, sig) signals the caller's entire process group —
        // Ghost included. PID 0 must never reach the syscall.
        assert!(!process::is_alive(0));
        assert!(process::terminate(0).is_err());
    }

    #[test]
    fn our_own_pid_is_reported_alive() {
        assert!(process::is_alive(std::process::id()));
    }
}
