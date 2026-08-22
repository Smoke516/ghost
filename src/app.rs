use crate::config::{AppSettings, ConfigManager};
use crate::forms::ServerForm;
use crate::health::{HealthMonitor, HealthUpdate};
use crate::models::{
    process, AppMode, AppState, HealthStatus, LoadingContext, ServerConnection, SessionInfo,
};
use crate::ssh::{detect_available_terminal, ConnectionMode};
use crate::ssh_config;
use crate::tui;
use crate::ui::ui;
use anyhow::Result;
use chrono::Utc;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::time::{Duration, Instant};

/// How often to reap sessions whose process has exited. Previously this ran on
/// every 50 ms tick, forking a process per tracked session — roughly 20 process
/// spawns per second, per session, forever.
const SESSION_REAP_INTERVAL: Duration = Duration::from_secs(2);

/// How long a status popup stays up before dismissing itself.
const POPUP_TTL_SECS: i64 = 4;

pub struct App {
    pub state: AppState,
    pub last_tick: Instant,
    pub tick_rate: Duration,
    pub config_manager: ConfigManager,
    pub app_settings: AppSettings,
    pub health_monitor: HealthMonitor,
    pub health_task: Option<tokio::task::JoinHandle<()>>,
    pub connection_mode: ConnectionMode,
    last_session_reap: Instant,
}

impl App {
    pub fn new(tick_rate: Duration, connection_mode: ConnectionMode) -> Result<Self> {
        let config_manager = ConfigManager::new()?;
        let config = config_manager.load_config()?;
        let connections = config_manager.config_to_connections(&config);

        let mut state = AppState::default();
        state.server_manager.connections = connections;
        state.server_manager.show_only_online = config.settings.show_only_online;
        state.theme_manager.set_theme(config.settings.theme);
        state.show_tooltips = config.settings.show_tooltips;
        state.config_path = config_manager.config_path().display().to_string();
        state.layout = crate::models::PanelLayout::from_name(&config.settings.panel_layout);

        // Read before `config.settings` is moved into `app_settings`.
        // Clamp the probe interval so a typo can't turn Ghost into a port
        // scanner against the user's own fleet.
        let refresh_interval = config.settings.refresh_interval.clamp(5, 3600);

        Ok(Self {
            state,
            last_tick: Instant::now(),
            tick_rate,
            config_manager,
            app_settings: config.settings,
            // Honour the configured probe interval instead of a hardcoded 30s.
            health_monitor: HealthMonitor::new(refresh_interval),
            health_task: None,
            connection_mode,
            last_session_reap: Instant::now(),
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut terminal = tui::init()?;

        // Seed the monitor and start it unconditionally: it re-reads the server
        // list each cycle, so it picks up servers added later in the session
        // even when the config started out empty.
        self.sync_health_servers().await;
        self.health_task = Some(self.health_monitor.start().await);

        let result = self.run_app(&mut terminal).await;

        self.health_monitor.stop().await;
        if let Some(task) = self.health_task.take() {
            task.abort();
        }

        tui::restore();
        result
    }

    /// Push the current connection list to the health monitor. Must be called
    /// after any add / edit / delete / import.
    async fn sync_health_servers(&self) {
        let servers: Vec<ServerConnection> = self
            .state
            .server_manager
            .connections
            .values()
            .cloned()
            .collect();
        self.health_monitor.set_servers(servers).await;
    }

    async fn run_app(&mut self, terminal: &mut tui::Tui) -> Result<()> {
        loop {
            // A direct SSH session tore the screen down behind ratatui's back;
            // discard the diff baseline before drawing again.
            if self.state.force_redraw {
                terminal.clear()?;
                self.state.force_redraw = false;
            }

            let ui_start = Instant::now();
            terminal.draw(|f| ui(f, &mut self.state))?;
            self.state.performance.ui_render_time = Some(ui_start.elapsed());
            self.state.update_frame_rate();

            let timeout = self.tick_rate.saturating_sub(self.last_tick.elapsed());

            if event::poll(timeout)? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        self.handle_key_event(key.code, key.modifiers).await?;
                    }
                    // A resize invalidates ratatui's cached buffer.
                    Event::Resize(_, _) => self.state.force_redraw = true,
                    _ => {}
                }
            }

            if self.last_tick.elapsed() >= self.tick_rate {
                self.on_tick().await;
                self.last_tick = Instant::now();
            }

            if self.state.should_quit {
                break;
            }
        }
        Ok(())
    }

    async fn handle_key_event(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        match self.state.mode {
            AppMode::Normal => self.handle_normal_mode(key, modifiers).await?,
            AppMode::Search => self.handle_search_mode(key).await?,
            AppMode::ThemeSelector => self.handle_theme_selector_mode(key)?,
            AppMode::AddServer | AppMode::EditServer(_) => self.handle_form_input(key).await?,
            AppMode::ConfirmDelete(_) => self.handle_confirm_delete_mode(key).await?,
            AppMode::ConfirmDiscard => self.handle_confirm_discard_mode(key).await?,
            AppMode::Help => self.handle_help_mode(key).await?,
            AppMode::Connecting(_) => self.handle_connecting_mode(key).await?,
            AppMode::Loading(_) => self.handle_loading_mode(key).await?,
            AppMode::History => self.handle_history_mode(key).await?,
            AppMode::Analytics => self.handle_analytics_mode(key).await?,
            AppMode::Sessions => self.handle_sessions_mode(key).await?,
        }
        Ok(())
    }

    /// Dismiss an active popup. Returns true if there was one to dismiss, so
    /// callers can treat the keypress as consumed.
    fn dismiss_popup(&mut self) -> bool {
        if self.state.show_popup {
            self.state.show_popup = false;
            self.state.popup_message.clear();
            self.state.popup_shown_at = None;
            true
        } else {
            false
        }
    }

    fn notify(&mut self, message: impl Into<String>) {
        self.state.show_popup = true;
        self.state.popup_message = message.into();
        self.state.popup_shown_at = Some(Utc::now());
    }

    async fn handle_normal_mode(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        match key {
            KeyCode::Char('q') => self.state.should_quit = true,
            KeyCode::Esc => {
                if !self.dismiss_popup() {
                    // Esc clears an active search before it quits, so a stray
                    // press can't drop you out of the app with a filter set.
                    if self.state.server_manager.filter.is_empty() {
                        self.state.should_quit = true;
                    } else {
                        self.clear_search();
                    }
                }
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.state.should_quit = true
            }
            KeyCode::Char('x') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.kill_all_sessions().await
            }
            KeyCode::Char('j') | KeyCode::Down => self.move_selection_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_selection_up(),
            KeyCode::Char('g') | KeyCode::Home => {
                self.state.server_manager.selected_index = 0;
            }
            KeyCode::Char('G') | KeyCode::End => {
                let len = self.state.server_manager.filtered_connections().len();
                self.state.server_manager.selected_index = len.saturating_sub(1);
            }
            KeyCode::PageDown => self.move_selection_by(10),
            KeyCode::PageUp => self.move_selection_by(-10),
            KeyCode::Char('/') => {
                self.state.mode = AppMode::Search;
                self.state
                    .search_input
                    .set_value(self.state.server_manager.filter.clone());
            }
            KeyCode::Char('a') => {
                self.state.server_form = Some(ServerForm::new_add_form());
                self.state.mode = AppMode::AddServer;
            }
            KeyCode::Char('d') => {
                if let Some(connection) = self.get_selected_connection() {
                    self.state.mode = AppMode::ConfirmDelete(connection.id.clone());
                }
            }
            KeyCode::Char('e') => {
                if let Some(connection) = self.get_selected_connection() {
                    let connection_id = connection.id.clone();
                    self.state.server_form = Some(ServerForm::new_edit_form(connection));
                    self.state.mode = AppMode::EditServer(connection_id);
                }
            }
            KeyCode::Char('i') => self.import_ssh_config().await,
            KeyCode::Char('r') => self.refresh_connections().await,
            KeyCode::Char('f') => {
                self.state.server_manager.show_only_online =
                    !self.state.server_manager.show_only_online;
                self.state.server_manager.clamp_selection();
                self.app_settings.show_only_online = self.state.server_manager.show_only_online;
                let on = self.state.server_manager.show_only_online;
                self.persist_settings();
                self.notify(if on {
                    "Filter: online servers only"
                } else {
                    "Filter: showing all servers"
                });
            }
            KeyCode::Char('h') | KeyCode::F(1) => {
                self.state.help_scroll = 0;
                self.state.mode = AppMode::Help;
            }
            KeyCode::Char('H') => self.state.mode = AppMode::History,
            KeyCode::Char('A') => self.state.mode = AppMode::Analytics,
            KeyCode::Char('S') => self.state.mode = AppMode::Sessions,
            KeyCode::Char('t') => self.open_theme_selector(),
            KeyCode::Char('T') => {
                self.state.theme_manager.next_theme();
                self.app_settings.theme = self.state.theme_manager.current_variant();
                let name = self.state.theme_manager.current_variant().name();
                match self.save_config() {
                    Ok(()) => self.notify(format!("🎨 Theme: {}", name)),
                    Err(e) => self.notify(format!("Failed to save theme: {}", e)),
                }
            }
            KeyCode::Char('l') => {
                self.state.layout.cycle_layout();
                self.app_settings.panel_layout = self.state.layout.name().to_string();
                let mode = format!("{:?}", self.state.layout.mode);
                self.persist_settings();
                self.notify(format!("📐 Layout: {}", mode));
            }
            KeyCode::Char('[') => self.resize_panels(-5),
            KeyCode::Char(']') => self.resize_panels(5),
            KeyCode::Char('?') => self.show_contextual_tooltip(),
            KeyCode::F(2) => {
                self.state.toggle_tooltips();
                self.app_settings.show_tooltips = self.state.show_tooltips;
                self.persist_settings();
                let enabled = self.state.show_tooltips;
                self.notify(if enabled {
                    "Tooltips enabled — press ? for contextual help"
                } else {
                    "Tooltips disabled"
                });
            }
            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                let num = c.to_digit(10).unwrap() as usize;
                let id = self
                    .state
                    .server_manager
                    .filtered_connections()
                    .get(num - 1)
                    .map(|c| c.id.clone());
                if let Some(id) = id {
                    self.connect_to_server(id).await;
                }
            }
            KeyCode::Enter if !self.dismiss_popup() => {
                if let Some(connection) = self.get_selected_connection() {
                    let id = connection.id.clone();
                    self.connect_to_server(id).await;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Incremental search: every keystroke re-filters the list live.
    async fn handle_search_mode(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.clear_search();
                self.state.mode = AppMode::Normal;
            }
            // Enter keeps the filter and returns to navigation.
            KeyCode::Enter => self.state.mode = AppMode::Normal,
            KeyCode::Down => {
                self.move_selection_down();
                return Ok(());
            }
            KeyCode::Up => {
                self.move_selection_up();
                return Ok(());
            }
            KeyCode::Backspace => {
                self.state.search_input.delete_char();
                self.apply_search();
            }
            KeyCode::Delete => {
                self.state.search_input.delete_char_forward();
                self.apply_search();
            }
            KeyCode::Left => self.state.search_input.move_cursor_left(),
            KeyCode::Right => self.state.search_input.move_cursor_right(),
            KeyCode::Home => self.state.search_input.move_cursor_to_start(),
            KeyCode::End => self.state.search_input.move_cursor_to_end(),
            KeyCode::Char(c) => {
                self.state.search_input.insert_char(c);
                self.apply_search();
            }
            _ => {}
        }
        Ok(())
    }

    /// Open the theme picker, remembering the current theme so Esc can undo a
    /// preview. Pressing `t` previously toggled a flag nothing ever rendered.
    fn open_theme_selector(&mut self) {
        let current = self.state.theme_manager.current_variant();
        self.state.theme_before_preview = Some(current);
        self.state.theme_selector_index = crate::themes::ThemeVariant::all()
            .iter()
            .position(|&v| v == current)
            .unwrap_or(0);
        self.state.mode = AppMode::ThemeSelector;
    }

    /// Moving through the list previews the theme immediately; Enter keeps it,
    /// Esc restores whatever was active when the picker opened.
    fn handle_theme_selector_mode(&mut self, key: KeyCode) -> Result<()> {
        let variants = crate::themes::ThemeVariant::all();
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                self.state.theme_selector_index =
                    (self.state.theme_selector_index + 1) % variants.len();
                self.preview_selected_theme();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.state.theme_selector_index = if self.state.theme_selector_index == 0 {
                    variants.len() - 1
                } else {
                    self.state.theme_selector_index - 1
                };
                self.preview_selected_theme();
            }
            KeyCode::Enter => {
                self.app_settings.theme = self.state.theme_manager.current_variant();
                let name = self.app_settings.theme.name();
                self.state.theme_before_preview = None;
                self.state.mode = AppMode::Normal;
                match self.save_config() {
                    Ok(()) => self.notify(format!("🎨 Theme: {}", name)),
                    Err(e) => self.notify(format!("Failed to save theme: {}", e)),
                }
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('t') => {
                if let Some(previous) = self.state.theme_before_preview.take() {
                    self.state.theme_manager.set_theme(previous);
                }
                self.state.mode = AppMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }

    fn preview_selected_theme(&mut self) {
        let variants = crate::themes::ThemeVariant::all();
        if let Some(&variant) = variants.get(self.state.theme_selector_index) {
            self.state.theme_manager.set_theme(variant);
        }
    }

    fn apply_search(&mut self) {
        self.state.server_manager.filter = self.state.search_input.value.clone();
        self.state.server_manager.clamp_selection();
    }

    fn clear_search(&mut self) {
        self.state.search_input.set_value(String::new());
        self.state.server_manager.filter.clear();
        self.state.server_manager.clamp_selection();
    }

    fn resize_panels(&mut self, delta: i16) {
        self.state.layout.resize_panels(delta);
        let sizes = self.state.layout.panel_sizes;
        self.notify(format!(
            "⚖ Panels: {}% | {}% | {}%",
            sizes[0], sizes[1], sizes[2]
        ));
    }

    /// Persist settings-only changes, surfacing failures rather than dropping
    /// them silently.
    fn persist_settings(&mut self) {
        if let Err(e) = self.save_config() {
            self.notify(format!("Failed to save settings: {}", e));
        }
    }

    async fn handle_confirm_delete_mode(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let AppMode::ConfirmDelete(id) = self.state.mode.clone() {
                    let removed = self.state.server_manager.remove_connection(&id);
                    self.state.server_manager.clamp_selection();
                    self.sync_health_servers().await;
                    if let Err(e) = self.save_config() {
                        self.notify(format!("Failed to save config: {}", e));
                    } else if let Some(server) = removed {
                        self.notify(format!("Deleted {}", server.name));
                    }
                }
                self.state.mode = AppMode::Normal;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.state.mode = AppMode::Normal
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_confirm_discard_mode(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.state.server_form = None;
                self.state.mode = AppMode::Normal;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.state.mode = match &self.state.server_form {
                    Some(form) if form.is_editing => {
                        AppMode::EditServer(form.original_id.clone().unwrap_or_default())
                    }
                    Some(_) => AppMode::AddServer,
                    None => AppMode::Normal,
                };
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_help_mode(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') => {
                self.state.mode = AppMode::Normal;
                self.state.help_scroll = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.state.help_scroll = self.state.help_scroll.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.state.help_scroll = self.state.help_scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                self.state.help_scroll = self.state.help_scroll.saturating_add(10);
            }
            KeyCode::PageUp => {
                self.state.help_scroll = self.state.help_scroll.saturating_sub(10);
            }
            KeyCode::Home | KeyCode::Char('g') => self.state.help_scroll = 0,
            _ => {}
        }
        Ok(())
    }

    async fn handle_connecting_mode(&mut self, key: KeyCode) -> Result<()> {
        if key == KeyCode::Esc {
            self.state.mode = AppMode::Normal;
        }
        Ok(())
    }

    async fn handle_loading_mode(&mut self, key: KeyCode) -> Result<()> {
        if key == KeyCode::Esc {
            // Cancel only stops *waiting*; in-flight probes still deliver their
            // results through the channel and update the list.
            self.state.complete_loading();
            self.notify("Refresh running in background");
        }
        Ok(())
    }

    async fn handle_history_mode(&mut self, key: KeyCode) -> Result<()> {
        if matches!(key, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('H')) {
            self.state.mode = AppMode::Normal;
        }
        Ok(())
    }

    async fn handle_analytics_mode(&mut self, key: KeyCode) -> Result<()> {
        if matches!(key, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('A')) {
            self.state.mode = AppMode::Normal;
        }
        Ok(())
    }

    async fn handle_sessions_mode(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('S') => {
                self.state.mode = AppMode::Normal
            }
            KeyCode::Char('j') | KeyCode::Down => self.move_session_selection_down(),
            KeyCode::Char('k') | KeyCode::Up => self.move_session_selection_up(),
            KeyCode::Char('d') => {
                if let Some(pid) = self.get_selected_session().map(|s| s.pid) {
                    match self.state.kill_session(pid) {
                        Ok(()) => self.notify(format!("Terminated session PID {}", pid)),
                        Err(e) => self.notify(e),
                    }
                    let len = self.state.get_filtered_sessions().len();
                    if self.state.session_selected_index >= len {
                        self.state.session_selected_index = len.saturating_sub(1);
                    }
                }
            }
            KeyCode::Char('r') => self.refresh_all_sessions().await,
            KeyCode::Enter => {
                if let Some(session) = self.get_selected_session() {
                    let message = format!(
                        "{} — PID {}\n{}\nRunning for {}",
                        session.server_name,
                        session.pid,
                        session.window_title,
                        session.format_duration()
                    );
                    self.notify(message);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn move_selection_down(&mut self) {
        let len = self.state.server_manager.filtered_connections().len();
        if len > 0 {
            self.state.server_manager.selected_index =
                (self.state.server_manager.selected_index + 1) % len;
        }
    }

    fn move_selection_up(&mut self) {
        let len = self.state.server_manager.filtered_connections().len();
        if len > 0 {
            self.state.server_manager.selected_index =
                if self.state.server_manager.selected_index == 0 {
                    len - 1
                } else {
                    self.state.server_manager.selected_index - 1
                };
        }
    }

    /// Page-style movement; saturates at the ends rather than wrapping.
    fn move_selection_by(&mut self, delta: i32) {
        let len = self.state.server_manager.filtered_connections().len() as i32;
        if len == 0 {
            return;
        }
        let current = self.state.server_manager.selected_index as i32;
        self.state.server_manager.selected_index = (current + delta).clamp(0, len - 1) as usize;
    }

    fn get_selected_connection(&self) -> Option<&ServerConnection> {
        let connections = self.state.server_manager.filtered_connections();
        connections
            .get(self.state.server_manager.selected_index)
            .copied()
    }

    /// Kick off a health refresh. Returns immediately — probes run concurrently
    /// in the background and results stream in through `on_tick`, so the UI
    /// keeps drawing and stays responsive to Esc.
    async fn refresh_connections(&mut self) {
        let server_count = self.state.server_manager.connections.len();
        if server_count == 0 {
            self.notify("No servers to refresh. Press 'a' to add one.");
            return;
        }

        self.sync_health_servers().await;

        for connection in self.state.server_manager.connections.values_mut() {
            connection.health_status = HealthStatus::Connecting;
        }

        let total = self.health_monitor.refresh_now().await;
        self.state.start_loading(LoadingContext::RefreshingHealth {
            total,
            completed: 0,
        });
    }

    async fn connect_to_server(&mut self, server_id: String) {
        let Some(server) = self
            .state
            .server_manager
            .get_connection(&server_id)
            .cloned()
        else {
            return;
        };

        self.state.mode = AppMode::Connecting(server_id.clone());
        if let Some(connection) = self.state.server_manager.get_connection_mut(&server_id) {
            connection.health_status = HealthStatus::Connecting;
        }

        let result = self
            .health_monitor
            .connect_to_server_with_mode(&server, self.connection_mode.clone())
            .await;

        match result {
            Ok(pid) => {
                // Direct mode tears the TUI down and rebuilds it; force a full
                // repaint so no stale cells survive.
                if self.connection_mode == ConnectionMode::Direct {
                    self.state.force_redraw = true;
                }

                // Client/server terminals (GNOME Terminal, Konsole) hand off to
                // a daemon and exit, so the PID we captured is already dead.
                // Tracking it would show a phantom session that vanishes on the
                // next reap and can't be killed.
                let trackable = self.connection_mode != ConnectionMode::Direct
                    && detect_available_terminal().reports_stable_pid();

                if let Some(connection) = self.state.server_manager.get_connection_mut(&server_id) {
                    connection.health_status = HealthStatus::Online;
                    connection.stats.connection_count += 1;
                    connection.stats.last_connected = Some(Utc::now());
                    if trackable {
                        connection.add_session(pid, format!("Ghost SSH: {}", server.name));
                    }
                }

                self.state
                    .server_manager
                    .add_to_history(server_id.clone(), server.name.clone());
                self.state.server_manager.update_session_count();

                if trackable {
                    self.notify(format!("🚀 {} — session PID {}", server.name, pid));
                } else {
                    self.notify(format!("🚀 Launched session for {}", server.name));
                }
            }
            Err(error) => {
                if let Some(connection) = self.state.server_manager.get_connection_mut(&server_id) {
                    connection.health_status = HealthStatus::Offline;
                    connection.stats.failed_attempts += 1;
                    connection.last_error = Some(error.clone());
                }
                self.notify(format!("⚠ Connection failed:\n{}", error));
            }
        }

        self.state.mode = AppMode::Normal;
    }

    async fn kill_all_sessions(&mut self) {
        let pids: Vec<u32> = self
            .state
            .server_manager
            .connections
            .values()
            .flat_map(|c| c.active_sessions.iter().map(|s| s.pid))
            .collect();

        if pids.is_empty() {
            self.notify("No active sessions to terminate");
            return;
        }

        let mut killed = 0;
        let mut failed = 0;
        for pid in pids {
            match process::terminate(pid) {
                Ok(()) => killed += 1,
                Err(_) => failed += 1,
            }
        }

        for connection in self.state.server_manager.connections.values_mut() {
            connection.active_sessions.clear();
        }
        self.state.server_manager.update_session_count();

        if failed == 0 {
            self.notify(format!("Terminated {} session(s)", killed));
        } else {
            self.notify(format!(
                "Terminated {} session(s); {} could not be signalled",
                killed, failed
            ));
        }
    }

    /// Merge hosts from the user's `~/.ssh/config` into the server list.
    async fn import_ssh_config(&mut self) {
        let Some(path) = ssh_config::default_path() else {
            self.notify("Could not locate your home directory");
            return;
        };
        if !path.exists() {
            self.notify(format!("No SSH config found at {}", path.display()));
            return;
        }

        let hosts = match ssh_config::parse_file(&path) {
            Ok(hosts) => hosts,
            Err(e) => {
                self.notify(format!("Could not read SSH config: {}", e));
                return;
            }
        };

        let (imported, skipped) = self.merge_ssh_hosts(hosts);

        if imported == 0 {
            self.notify(format!(
                "Nothing new to import ({} host(s) already known)",
                skipped
            ));
            return;
        }

        self.sync_health_servers().await;
        match self.save_config() {
            Ok(()) => self.notify(format!(
                "Imported {} host(s) from ssh config ({} already known)",
                imported, skipped
            )),
            Err(e) => self.notify(format!(
                "Imported {} host(s), but saving failed: {}",
                imported, e
            )),
        }
    }

    /// Insert hosts that aren't already configured. Identity is
    /// user@host:port, so re-importing an unchanged config is a no-op.
    fn merge_ssh_hosts(&mut self, hosts: Vec<ssh_config::SshHost>) -> (usize, usize) {
        let existing: std::collections::HashSet<String> = self
            .state
            .server_manager
            .connections
            .values()
            .map(|c| c.connection_string().to_lowercase())
            .collect();

        let mut imported = 0;
        let mut skipped = 0;
        for host in hosts {
            let conn = host.to_connection();
            if existing.contains(&conn.connection_string().to_lowercase()) {
                skipped += 1;
                continue;
            }
            self.state
                .server_manager
                .connections
                .insert(conn.id.clone(), conn);
            imported += 1;
        }
        (imported, skipped)
    }

    async fn on_tick(&mut self) {
        self.state.last_update = Utc::now();
        self.state.globe_animation_frame = (self.state.globe_animation_frame + 1) % 80;

        if self.state.show_popup {
            if let Some(shown_at) = self.state.popup_shown_at {
                if Utc::now().signed_duration_since(shown_at).num_seconds() >= POPUP_TTL_SECS {
                    self.dismiss_popup();
                }
            }
        }

        if self.state.should_auto_dismiss_tooltip() {
            self.state.hide_tooltip();
        }

        // Reaping is throttled: checking process liveness every 50 ms was pure
        // overhead, and a session ending two seconds late is imperceptible.
        if self.last_session_reap.elapsed() >= SESSION_REAP_INTERVAL {
            self.cleanup_ended_sessions();
            self.last_session_reap = Instant::now();
        }

        // Drain health results. Each one may also advance a refresh's progress.
        while let Some(health_update) = self.health_monitor.try_recv_update() {
            self.handle_health_update(health_update);

            if let AppMode::Loading(LoadingContext::RefreshingHealth {
                ref mut completed,
                total,
            }) = self.state.mode
            {
                *completed += 1;
                if *completed >= total {
                    self.state.complete_loading();
                }
            }
        }
    }

    /// Save current configuration to file
    pub fn save_config(&self) -> Result<()> {
        let config = self.config_manager.connections_to_config(
            &self.state.server_manager.connections,
            self.app_settings.clone(),
        );
        self.config_manager.save_config(&config)
    }

    /// Handle form input for add/edit server modes
    async fn handle_form_input(&mut self, key: KeyCode) -> Result<()> {
        // Submission is handled first and separately to avoid holding a mutable
        // borrow of the form across the save.
        if key == KeyCode::Enter {
            let submit = self
                .state
                .server_form
                .as_ref()
                .is_some_and(|f| !f.auth_method_focused);

            if submit {
                let outcome = self.state.server_form.as_ref().map(|f| {
                    (
                        f.to_server_connection(),
                        f.is_editing,
                        f.original_id.clone(),
                    )
                });

                if let Some((result, is_editing, original_id)) = outcome {
                    match result {
                        Ok(connection) => {
                            let id = if is_editing {
                                original_id.unwrap_or_else(|| connection.id.clone())
                            } else {
                                connection.id.clone()
                            };
                            let name = connection.name.clone();
                            self.state
                                .server_manager
                                .connections
                                .insert(id.clone(), connection);
                            self.state.server_form = None;
                            self.state.mode = AppMode::Normal;

                            // Keep the cursor on the server just edited, even
                            // though the list re-sorts by name.
                            self.state.server_manager.select_by_id(&id);
                            self.sync_health_servers().await;

                            match self.save_config() {
                                Ok(()) => self.notify(if is_editing {
                                    format!("Updated {}", name)
                                } else {
                                    format!("Added {}", name)
                                }),
                                Err(e) => self.notify(format!("Failed to save config: {}", e)),
                            }
                        }
                        Err(error) => self.notify(format!("Validation error: {}", error)),
                    }
                    return Ok(());
                }
            }
        }

        if let Some(ref mut form) = self.state.server_form {
            match key {
                KeyCode::Esc => {
                    if form.has_input() {
                        self.state.mode = AppMode::ConfirmDiscard;
                        return Ok(());
                    }
                    self.state.server_form = None;
                    self.state.mode = AppMode::Normal;
                }
                KeyCode::Tab => form.next_field(),
                KeyCode::BackTab => form.previous_field(),
                KeyCode::Enter => {
                    if form.auth_method_focused {
                        form.auth_method_focused = false;
                        form.next_field();
                    }
                }
                KeyCode::Up => {
                    if form.auth_method_focused {
                        form.previous_auth_method();
                    } else {
                        form.previous_field();
                    }
                }
                KeyCode::Down => {
                    if form.auth_method_focused {
                        form.next_auth_method();
                    } else {
                        form.next_field();
                    }
                }
                KeyCode::Left => {
                    if let Some(field) = form.current_field_mut() {
                        field.move_cursor_left();
                    }
                }
                KeyCode::Right => {
                    if let Some(field) = form.current_field_mut() {
                        field.move_cursor_right();
                    }
                }
                KeyCode::Home => {
                    if let Some(field) = form.current_field_mut() {
                        field.move_cursor_to_start();
                    }
                }
                KeyCode::End => {
                    if let Some(field) = form.current_field_mut() {
                        field.move_cursor_to_end();
                    }
                }
                KeyCode::Backspace => {
                    if let Some(field) = form.current_field_mut() {
                        field.delete_char();
                    }
                }
                KeyCode::Delete => {
                    if let Some(field) = form.current_field_mut() {
                        field.delete_char_forward();
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(field) = form.current_field_mut() {
                        field.insert_char(c);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Drop sessions whose process has exited.
    fn cleanup_ended_sessions(&mut self) {
        let mut ended = false;
        for connection in self.state.server_manager.connections.values_mut() {
            let before = connection.active_sessions.len();
            connection
                .active_sessions
                .retain(|session| process::is_alive(session.pid));
            if connection.active_sessions.len() != before {
                ended = true;
            }
        }

        if ended {
            self.state.server_manager.update_session_count();
            let len = self.state.get_filtered_sessions().len();
            if self.state.session_selected_index >= len {
                self.state.session_selected_index = len.saturating_sub(1);
            }
        }
    }

    /// Apply a health result and announce genuine state transitions.
    fn handle_health_update(&mut self, update: HealthUpdate) {
        let Some(connection) = self
            .state
            .server_manager
            .get_connection_mut(&update.server_id)
        else {
            return;
        };

        // Capture the old status BEFORE applying the result. The previous code
        // wrote the new status first and then compared against it, so the
        // transition checks were always false and these notifications never
        // fired at all.
        let previous = connection.health_status.clone();
        update.result.update_server_stats(connection);
        let name = connection.name.clone();
        let current = update.result.status.clone();

        // Only announce on an actual edge, and never on the first observation
        // (Unknown -> anything), which would otherwise spam at startup.
        match (&previous, &current) {
            (HealthStatus::Unknown, _) | (HealthStatus::Connecting, _) => {}
            (before, HealthStatus::Offline) if *before != HealthStatus::Offline => {
                self.notify(format!("⚠ {} went offline", name));
            }
            (HealthStatus::Offline, HealthStatus::Online) => {
                self.notify(format!("✓ {} is back online", name));
            }
            _ => {}
        }
    }

    fn move_session_selection_down(&mut self) {
        let len = self.state.get_filtered_sessions().len();
        if len > 0 {
            self.state.session_selected_index = (self.state.session_selected_index + 1) % len;
        }
    }

    fn move_session_selection_up(&mut self) {
        let len = self.state.get_filtered_sessions().len();
        if len > 0 {
            self.state.session_selected_index = if self.state.session_selected_index == 0 {
                len - 1
            } else {
                self.state.session_selected_index - 1
            };
        }
    }

    fn get_selected_session(&self) -> Option<&SessionInfo> {
        let sessions = self.state.get_filtered_sessions();
        sessions.get(self.state.session_selected_index).copied()
    }

    async fn refresh_all_sessions(&mut self) {
        self.cleanup_ended_sessions();
        let count = self.state.get_all_sessions().len();
        self.notify(format!("{} active session(s)", count));
    }

    /// Show contextual tooltips based on current state
    fn show_contextual_tooltip(&mut self) {
        use crate::models::TooltipCategory;

        match self.state.mode {
            AppMode::Normal | AppMode::Search => {
                if self.state.server_manager.connections.is_empty() {
                    self.state.show_tooltip(
                        "Getting Started".to_string(),
                        "Press 'a' to add a server by hand, or 'i' to import every host from your ~/.ssh/config.".to_string(),
                        Some("a: Add | i: Import ssh config".to_string()),
                        TooltipCategory::Server,
                    );
                } else if let Some(connection) = self.get_selected_connection() {
                    let title = format!("Server: {}", connection.name);
                    let body = format!(
                        "{}@{}:{} | Status: {}",
                        connection.username,
                        connection.host,
                        connection.port,
                        connection.health_status.as_str()
                    );
                    self.state.show_tooltip(
                        title,
                        body,
                        Some("Enter: Connect | e: Edit | d: Delete | r: Refresh".to_string()),
                        TooltipCategory::Server,
                    );
                } else {
                    self.state.show_tooltip(
                        "Navigation".to_string(),
                        "j/k or arrows to move, 1-9 to quick-connect, / to search, l to change layout.".to_string(),
                        Some("j/k: Move | /: Search | Enter: Connect".to_string()),
                        TooltipCategory::Navigation,
                    );
                }
            }
            AppMode::Sessions => {
                let count = self.state.get_filtered_sessions().len();
                if count == 0 {
                    self.state.show_tooltip(
                        "No Active Sessions".to_string(),
                        "Sessions launched into a new terminal window are tracked here."
                            .to_string(),
                        Some("Esc: Back".to_string()),
                        TooltipCategory::Session,
                    );
                } else {
                    self.state.show_tooltip(
                        "Session Management".to_string(),
                        format!(
                            "{} active session(s). 'd' terminates the selected one.",
                            count
                        ),
                        Some("d: Kill | r: Refresh | Enter: Details".to_string()),
                        TooltipCategory::Session,
                    );
                }
            }
            AppMode::Analytics => self.state.show_tooltip(
                "Analytics".to_string(),
                "Connection counts, probe-based uptime, and latency for each server.".to_string(),
                Some("A / Esc: Back".to_string()),
                TooltipCategory::System,
            ),
            AppMode::History => {
                let count = self.state.server_manager.connection_history.len();
                self.state.show_tooltip(
                    "Connection History".to_string(),
                    format!("{} recent connection(s) recorded this session.", count),
                    Some("H / Esc: Back".to_string()),
                    TooltipCategory::System,
                )
            }
            _ => self.state.show_tooltip(
                "Context Help".to_string(),
                "Press Esc to return to the main view.".to_string(),
                Some("Esc: Back".to_string()),
                TooltipCategory::System,
            ),
        }
    }
}

/// One-shot `--import-ssh-config` entry point: parse, merge, save, report.
/// Runs entirely outside the TUI so it can be scripted.
pub fn run_ssh_config_import(path: Option<&str>, dry_run: bool) -> Result<()> {
    let path = match path {
        Some(p) => std::path::PathBuf::from(shellexpand::tilde(p).to_string()),
        None => ssh_config::default_path()
            .ok_or_else(|| anyhow::anyhow!("Could not locate your home directory"))?,
    };

    if !path.exists() {
        anyhow::bail!("No SSH config found at {}", path.display());
    }

    let hosts = ssh_config::parse_file(&path)?;
    if hosts.is_empty() {
        println!("No importable hosts found in {}", path.display());
        return Ok(());
    }

    let config_manager = ConfigManager::new()?;
    let mut config = config_manager.load_config()?;
    let mut connections = config_manager.config_to_connections(&config);

    let existing: std::collections::HashSet<String> = connections
        .values()
        .map(|c| c.connection_string().to_lowercase())
        .collect();

    let mut imported = Vec::new();
    let mut skipped = 0;
    for host in hosts {
        let conn = host.to_connection();
        if existing.contains(&conn.connection_string().to_lowercase()) {
            skipped += 1;
            continue;
        }
        imported.push(conn.clone());
        connections.insert(conn.id.clone(), conn);
    }

    for conn in &imported {
        println!("  + {}  ({})", conn.name, conn.connection_string());
    }

    if imported.is_empty() {
        println!(
            "Nothing new to import ({} host(s) already configured).",
            skipped
        );
        return Ok(());
    }

    if dry_run {
        println!(
            "\n{} host(s) would be imported, {} already configured. (dry run — nothing saved)",
            imported.len(),
            skipped
        );
        return Ok(());
    }

    config = config_manager.connections_to_config(&connections, config.settings);
    config_manager.save_config(&config)?;
    println!(
        "\nImported {} host(s), {} already configured.",
        imported.len(),
        skipped
    );
    Ok(())
}
