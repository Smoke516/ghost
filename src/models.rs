use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::forms::ServerForm;
use crate::themes::ThemeManager;

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
            HealthStatus::Online => "●",
            HealthStatus::Offline => "●",
            HealthStatus::Connecting => "◐",
            HealthStatus::Warning => "▲",
            HealthStatus::Unknown => "?",
        }
    }
}

/// Security status assessment for SSH connections
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum SecurityStatus {
    #[default]
    Secure,
    Vulnerable,
    Compromised,
    Unknown,
}

impl SecurityStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SecurityStatus::Secure => "SECURE",
            SecurityStatus::Vulnerable => "VULNERABLE", 
            SecurityStatus::Compromised => "COMPROMISED",
            SecurityStatus::Unknown => "UNKNOWN",
        }
    }
    
    pub fn symbol(&self) -> &'static str {
        match self {
            SecurityStatus::Secure => "🛡",
            SecurityStatus::Vulnerable => "⚠",
            SecurityStatus::Compromised => "💀",
            SecurityStatus::Unknown => "?",
        }
    }
}


/// Connection statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub latency: Option<Duration>,
    pub latency_history: Vec<u32>, // Last 10 latency measurements in ms
    pub uptime_percentage: f32,
    pub last_connected: Option<DateTime<Utc>>,
    pub connection_count: u32,
    pub failed_attempts: u32,
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
            connection_count: 0,
            failed_attempts: 0,
            total_session_duration: Duration::from_secs(0),
            average_session_duration: Duration::from_secs(0),
            peak_usage_hour: None,
        }
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
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    
    // Status information (not persisted, computed at runtime)
    #[serde(skip)]
    pub health_status: HealthStatus,
    #[serde(skip)]
    pub security_status: SecurityStatus,
    #[serde(skip)]
    pub stats: ConnectionStats,
    
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
        Utc::now().signed_duration_since(self.started_at).to_std().unwrap_or_default()
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

/// Analytics data for the entire application
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsage {
    pub date: DateTime<Utc>,
    pub connection_count: u32,
    pub session_duration: Duration,
}

/// Server usage statistics for ranking
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
            created_at: now,
            last_modified: now,
            health_status: HealthStatus::Unknown,
            security_status: SecurityStatus::Unknown,
            stats: ConnectionStats::default(),
            active_sessions: Vec::new(),
        }
    }
    
    pub fn connection_string(&self) -> String {
        format!("{}@{}:{}", self.username, self.host, self.port)
    }
    
    
    pub fn is_healthy(&self) -> bool {
        matches!(self.health_status, HealthStatus::Online | HealthStatus::Warning)
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
            self.id.clone()
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
        let mut connections: Vec<&ServerConnection> = self.connections
            .values()
            .filter(|conn| {
                // Filter by search term
                if !self.filter.is_empty() {
                    let filter_lower = self.filter.to_lowercase();
                    if !conn.name.to_lowercase().contains(&filter_lower) &&
                       !conn.host.to_lowercase().contains(&filter_lower) &&
                       !conn.username.to_lowercase().contains(&filter_lower) {
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
    
    pub fn online_count(&self) -> usize {
        self.connections.values().filter(|conn| conn.is_healthy()).count()
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
        self.active_session_count = self.connections.values()
            .map(|conn| conn.session_count())
            .sum();
    }
}

/// Layout configurations for the UI
#[derive(Debug, Clone, PartialEq)]
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
            LayoutMode::SinglePanel => {}, // No resizing in single panel
        }
    }
}

/// Application modes for different UI states
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    AddServer,
    EditServer(String),
    ConfirmDelete(String),
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
    RefreshingHealth {
        total: usize,
        completed: usize,
    },
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
    pub app_startup_time: Option<Duration>,
    pub last_refresh_duration: Option<Duration>,
    pub average_refresh_time: Duration,
    pub total_refreshes: u32,
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
    pub globe_animation_frame: u8,
    pub session_selected_index: usize,
    pub session_filter: String,
    pub theme_manager: ThemeManager,
    pub show_theme_selector: bool,
    pub layout: PanelLayout,
    pub show_tooltips: bool,
    pub current_tooltip: Option<TooltipInfo>,
    pub tooltip_shown_at: Option<DateTime<Utc>>,
    // Performance and loading state
    pub performance: PerformanceMetrics,
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
        self.server_manager.connections.values()
            .flat_map(|conn| &conn.active_sessions)
            .collect()
    }
    
    /// Get filtered sessions based on current filter
    pub fn get_filtered_sessions(&self) -> Vec<&SessionInfo> {
        let mut sessions = self.get_all_sessions();
        
        if !self.session_filter.is_empty() {
            let filter_lower = self.session_filter.to_lowercase();
            sessions.retain(|session| {
                session.server_name.to_lowercase().contains(&filter_lower) ||
                session.window_title.to_lowercase().contains(&filter_lower) ||
                session.pid.to_string().contains(&filter_lower)
            });
        }
        
        // Sort by start time (newest first)
        sessions.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        sessions
    }
    
    /// Get session by PID
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
    pub fn show_tooltip(&mut self, title: String, description: String, key_hint: Option<String>, category: TooltipCategory) {
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
            let duration = Utc::now().signed_duration_since(start_time)
                .to_std().unwrap_or_default();
            
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
            let frame_duration = now.signed_duration_since(last_frame)
                .to_std().unwrap_or_default();
            
            if frame_duration.as_millis() > 0 {
                let current_fps = 1000.0 / frame_duration.as_millis() as f32;
                // Smooth the frame rate with exponential moving average
                self.performance.frame_rate = self.performance.frame_rate * 0.9 + current_fps * 0.1;
            }
        }
        
        self.last_frame_time = Some(now);
    }
    
    /// Get loading progress as a percentage string
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
            self.performance.average_refresh_time = 
                Duration::from_millis((total_time / self.performance.total_refreshes as f64) as u64);
        } else {
            self.performance.average_refresh_time = duration;
        }
    }

    /// Kill session by PID
    pub fn kill_session(&mut self, pid: u32) -> Result<(), String> {
        #[cfg(unix)]
        {
            use std::process::Command;
            match Command::new("kill").arg("-TERM").arg(pid.to_string()).output() {
                Ok(output) => {
                    if output.status.success() {
                        // Remove session from tracking
                        for conn in self.server_manager.connections.values_mut() {
                            conn.active_sessions.retain(|s| s.pid != pid);
                        }
                        self.server_manager.update_session_count();
                        Ok(())
                    } else {
                        Err(format!("Failed to kill session PID {}", pid))
                    }
                }
                Err(e) => Err(format!("Error killing session: {}", e)),
            }
        }
        
        #[cfg(windows)]
        {
            use std::process::Command;
            match Command::new("taskkill").args(["/F", "/PID", &pid.to_string()]).output() {
                Ok(output) => {
                    if output.status.success() {
                        // Remove session from tracking
                        for conn in self.server_manager.connections.values_mut() {
                            conn.active_sessions.retain(|s| s.pid != pid);
                        }
                        self.server_manager.update_session_count();
                        Ok(())
                    } else {
                        Err(format!("Failed to kill session PID {}", pid))
                    }
                }
                Err(e) => Err(format!("Error killing session: {}", e)),
            }
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
            globe_animation_frame: 0,
            session_selected_index: 0,
            session_filter: String::new(),
            theme_manager: ThemeManager::default(),
            show_theme_selector: false,
            layout: PanelLayout::default(),
            show_tooltips: true, // Enable tooltips by default
            current_tooltip: None,
            tooltip_shown_at: None,
            // Performance and loading state
            performance: PerformanceMetrics::default(),
            loading_start_time: None,
            last_frame_time: None,
            frame_count: 0,
        }
    }
}
