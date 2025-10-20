use crate::config::{AppSettings, ConfigManager};
use crate::forms::ServerForm;
use crate::health::{HealthMonitor, HealthUpdate};
use crate::models::{AppMode, AppState, HealthStatus, ServerConnection, SessionInfo};
use crate::ssh::ConnectionMode;
use crate::ui::ui;
use anyhow::Result;
use chrono::Utc;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::{
    io::stdout,
    time::{Duration, Instant},
};

pub struct App {
    pub state: AppState,
    pub last_tick: Instant,
    pub tick_rate: Duration,
    pub config_manager: ConfigManager,
    pub app_settings: AppSettings,
    pub health_monitor: HealthMonitor,
    pub health_task: Option<tokio::task::JoinHandle<()>>,
    pub connection_mode: ConnectionMode,
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
        
        Ok(Self {
            state,
            last_tick: Instant::now(),
            tick_rate,
            config_manager,
            app_settings: config.settings,
            health_monitor: HealthMonitor::new(30), // Check every 30 seconds
            health_task: None,
            connection_mode,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        eprintln!("🔧 Setting up terminal...");
        // Setup terminal
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        eprintln!("✅ Terminal setup complete");
        
        let backend = CrosstermBackend::new(stdout());
        eprintln!("🔧 Creating terminal backend...");
        let mut terminal = Terminal::new(backend)?;
        eprintln!("✅ Terminal created successfully");

        // Start background health monitoring
        let servers: Vec<ServerConnection> = self.state.server_manager.connections.values().cloned().collect();
        eprintln!("🔧 Starting health monitoring for {} servers...", servers.len());
        if !servers.is_empty() {
            let health_task = self.health_monitor.start(servers.clone()).await;
            self.health_task = Some(health_task);
        }
        eprintln!("✅ Health monitoring started");

        eprintln!("🚀 Starting main app loop...");
        let result = self.run_app(&mut terminal).await;
        eprintln!("✅ Main app loop finished");

        // Stop health monitoring
        self.health_monitor.stop().await;
        if let Some(task) = self.health_task.take() {
            task.abort();
        }

        // Cleanup terminal
        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;

        result
    }

    async fn run_app(&mut self, terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>) -> Result<()> {
        loop {
            let ui_start = Instant::now();
            terminal.draw(|f| ui(f, &mut self.state))?;
            let ui_duration = ui_start.elapsed();
            self.state.performance.ui_render_time = Some(ui_duration);
            self.state.update_frame_rate();

            let timeout = self.tick_rate.saturating_sub(self.last_tick.elapsed());
            
            if event::poll(timeout)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key_event(key.code, key.modifiers).await?;
                    }
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
            AppMode::AddServer => self.handle_add_server_mode(key).await?,
            AppMode::EditServer(_) => self.handle_edit_server_mode(key).await?,
            AppMode::ConfirmDelete(_) => self.handle_confirm_delete_mode(key).await?,
            AppMode::Help => self.handle_help_mode(key).await?,
            AppMode::Connecting(_) => self.handle_connecting_mode(key).await?,
            AppMode::Loading(_) => self.handle_loading_mode(key).await?,
            AppMode::History => self.handle_history_mode(key).await?,
            AppMode::Analytics => self.handle_analytics_mode(key).await?,
            AppMode::Sessions => self.handle_sessions_mode(key).await?,
        }
        Ok(())
    }

    async fn handle_normal_mode(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        match key {
            KeyCode::Char('q') => {
                self.state.should_quit = true;
            }
            KeyCode::Esc => {
                // First check if there's a popup to dismiss
                if self.state.show_popup {
                    self.state.show_popup = false;
                    self.state.popup_message.clear();
                    self.state.popup_shown_at = None;
                } else {
                    self.state.should_quit = true;
                }
            }
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.state.should_quit = true;
            }
            KeyCode::Char('x') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.kill_all_sessions().await;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_selection_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_selection_up();
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
            KeyCode::Char('r') => {
                self.refresh_connections().await;
            }
            KeyCode::Char('f') => {
                self.state.server_manager.show_only_online = !self.state.server_manager.show_only_online;
            }
            KeyCode::Char('h') | KeyCode::F(1) => {
                self.state.mode = AppMode::Help;
            }
            KeyCode::Char('H') => {
                self.state.mode = AppMode::History;
            }
            KeyCode::Char('A') => {
                self.state.mode = AppMode::Analytics;
            }
            KeyCode::Char('S') => {
                self.state.mode = AppMode::Sessions;
            }
            KeyCode::Char('t') => {
                // Toggle theme selector
                self.state.show_theme_selector = !self.state.show_theme_selector;
            }
            KeyCode::Char('T') => {
                // Quick theme cycle
                self.state.theme_manager.next_theme();
                // Save theme preference
                self.app_settings.theme = self.state.theme_manager.current_variant();
                if let Err(e) = self.save_config() {
                    self.state.show_popup = true;
                    self.state.popup_message = format!("Failed to save theme: {}", e);
                    self.state.popup_shown_at = Some(Utc::now());
                } else {
                    self.state.show_popup = true;
                    self.state.popup_message = format!("🎨 Switched to {}", self.state.theme_manager.current_variant().name());
                    self.state.popup_shown_at = Some(Utc::now());
                }
            }
            KeyCode::Char('l') => {
                // Cycle layout mode
                self.state.layout.cycle_layout();
                self.state.show_popup = true;
                self.state.popup_message = format!("📐 Layout: {:?}", self.state.layout.mode);
                self.state.popup_shown_at = Some(Utc::now());
            }
            KeyCode::Char('[') => {
                // Resize panels - decrease left, increase right
                self.state.layout.resize_panels(-5);
                self.state.show_popup = true;
                self.state.popup_message = format!("⚖️  Panel sizes: {}% | {}% | {}%", 
                    self.state.layout.panel_sizes[0], 
                    self.state.layout.panel_sizes[1], 
                    self.state.layout.panel_sizes[2]);
                self.state.popup_shown_at = Some(Utc::now());
            }
            KeyCode::Char(']') => {
                // Resize panels - increase left, decrease right
                self.state.layout.resize_panels(5);
                self.state.show_popup = true;
                self.state.popup_message = format!("⚖️  Panel sizes: {}% | {}% | {}%", 
                    self.state.layout.panel_sizes[0], 
                    self.state.layout.panel_sizes[1], 
                    self.state.layout.panel_sizes[2]);
                self.state.popup_shown_at = Some(Utc::now());
            }
            KeyCode::Char('?') => {
                // Show contextual tooltip based on current mode/selection
                self.show_contextual_tooltip();
            }
            KeyCode::F(2) => {
                // Toggle tooltips on/off
                self.state.toggle_tooltips();
                self.state.show_popup = true;
                self.state.popup_message = if self.state.show_tooltips {
                    "📊 Tooltips enabled - Press ? for contextual help".to_string()
                } else {
                    "❌ Tooltips disabled".to_string()
                };
                self.state.popup_shown_at = Some(Utc::now());
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                let num = c.to_digit(10).unwrap() as usize;
                if num > 0 && num <= 9 {
                    let connections = self.state.server_manager.filtered_connections();
                    if let Some(connection) = connections.get(num - 1) {
                        self.connect_to_server(connection.id.clone()).await;
                    }
                }
            }
            KeyCode::Enter => {
                // First check if there's a popup to dismiss
                if self.state.show_popup {
                    self.state.show_popup = false;
                    self.state.popup_message.clear();
                    self.state.popup_shown_at = None;
                } else if let Some(connection) = self.get_selected_connection() {
                    self.connect_to_server(connection.id.clone()).await;
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_add_server_mode(&mut self, key: KeyCode) -> Result<()> {
        self.handle_form_input(key).await
    }

    async fn handle_edit_server_mode(&mut self, key: KeyCode) -> Result<()> {
        self.handle_form_input(key).await
    }

    async fn handle_confirm_delete_mode(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let AppMode::ConfirmDelete(id) = &self.state.mode.clone() {
                    self.state.server_manager.remove_connection(id);
                    // Auto-save configuration
                    if let Err(e) = self.save_config() {
                        self.state.show_popup = true;
                        self.state.popup_message = format!("Failed to save config: {}", e);
                        self.state.popup_shown_at = Some(Utc::now());
                    }
                }
                self.state.mode = AppMode::Normal;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.state.mode = AppMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_help_mode(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') => {
                self.state.mode = AppMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_connecting_mode(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.state.mode = AppMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }
    
    async fn handle_loading_mode(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                // Allow users to cancel loading operations
                self.state.complete_loading();
            }
            _ => {
                // Ignore other keys during loading
            }
        }
        Ok(())
    }
    
    async fn handle_history_mode(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('H') => {
                self.state.mode = AppMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }
    
    async fn handle_analytics_mode(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('A') => {
                self.state.mode = AppMode::Normal;
            }
            _ => {}
        }
        Ok(())
    }
    
    async fn handle_sessions_mode(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('S') => {
                self.state.mode = AppMode::Normal;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_session_selection_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_session_selection_up();
            }
            KeyCode::Char('d') => {
                // Kill selected session
                if let Some(session) = self.get_selected_session() {
                    let _ = self.state.kill_session(session.pid);
                }
            }
            KeyCode::Char('r') => {
                // Refresh sessions
                self.refresh_all_sessions().await;
            }
            KeyCode::Enter => {
                // Bring session to foreground (placeholder)
                if let Some(session) = self.get_selected_session() {
                    let message = format!("Session for {} is running in PID {}\nWindow: {}", 
                        session.server_name, session.pid, session.window_title);
                    self.state.show_popup = true;
                    self.state.popup_message = message;
                    self.state.popup_shown_at = Some(chrono::Utc::now());
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn move_selection_down(&mut self) {
        let connections = self.state.server_manager.filtered_connections();
        if !connections.is_empty() {
            self.state.server_manager.selected_index = 
                (self.state.server_manager.selected_index + 1) % connections.len();
        }
    }

    fn move_selection_up(&mut self) {
        let connections = self.state.server_manager.filtered_connections();
        if !connections.is_empty() {
            self.state.server_manager.selected_index = 
                if self.state.server_manager.selected_index == 0 {
                    connections.len() - 1
                } else {
                    self.state.server_manager.selected_index - 1
                };
        }
    }

    fn get_selected_connection(&self) -> Option<&ServerConnection> {
        let connections = self.state.server_manager.filtered_connections();
        connections.get(self.state.server_manager.selected_index).copied()
    }

    async fn refresh_connections(&mut self) {
        use crate::models::LoadingContext;
        
        let server_count = self.state.server_manager.connections.len();
        if server_count == 0 {
            self.state.show_popup = true;
            self.state.popup_message = "No servers to refresh. Press 'a' to add a server first.".to_string();
            self.state.popup_shown_at = Some(Utc::now());
            return;
        }
        
        // Show immediate feedback
        self.state.show_popup = true;
        self.state.popup_message = format!("🔄 Refreshing {} server(s)...", server_count);
        self.state.popup_shown_at = Some(Utc::now());
        
        // Start loading state
        self.state.start_loading(LoadingContext::RefreshingHealth {
            total: server_count,
            completed: 0,
        });
        
        // Set all connections to "checking" status
        for connection in self.state.server_manager.connections.values_mut() {
            connection.health_status = HealthStatus::Connecting;
        }
        
        // Perform real health checks with progress tracking
        let servers: Vec<ServerConnection> = self.state.server_manager.connections.values().cloned().collect();
        let mut completed_count = 0;
        
        for server in servers {
            let result = self.health_monitor.check_server_now(&server).await;
            
            if let Some(connection) = self.state.server_manager.get_connection_mut(&server.id) {
                result.update_server_stats(connection);
            }
            
            completed_count += 1;
            
            // Update progress
            if let AppMode::Loading(LoadingContext::RefreshingHealth { ref mut completed, .. }) = self.state.mode {
                *completed = completed_count;
            }
        }
        
        // Complete loading
        self.state.complete_loading();
        
        // Show completion message
        self.state.show_popup = true;
        self.state.popup_message = format!("🔄 Refreshed {} server(s) | Avg time: {}ms", 
            server_count,
            self.state.performance.average_refresh_time.as_millis());
        self.state.popup_shown_at = Some(Utc::now());
    }

    async fn connect_to_server(&mut self, server_id: String) {
        self.state.mode = AppMode::Connecting(server_id.clone());
        
        if let Some(server) = self.state.server_manager.get_connection(&server_id).cloned() {
            // Update connection status to connecting
            if let Some(connection) = self.state.server_manager.get_connection_mut(&server_id) {
                connection.health_status = HealthStatus::Connecting;
            }
            // Attempt real SSH connection with the configured mode
            match self.health_monitor.connect_to_server_with_mode(&server, self.connection_mode.clone()).await {
                Ok(pid) => {
                    self.state.show_popup = true;
                    self.state.popup_message = format!("🚀 Launched SSH session for {}!\nPID: {} | Check your terminal windows.", server.name, pid);
                    self.state.popup_shown_at = Some(Utc::now());
                    
                    // Update connection status and add session tracking
                    if let Some(connection) = self.state.server_manager.get_connection_mut(&server_id) {
                        connection.health_status = HealthStatus::Online;
                        connection.stats.connection_count += 1;
                        connection.stats.last_connected = Some(Utc::now());
                        
                        // Track the active session
                        let window_title = format!("Ghost SSH: {}", server.name);
                        connection.add_session(pid, window_title);
                    }
                    
                    // Add to connection history
                    self.state.server_manager.add_to_history(server_id.clone(), server.name.clone());
                    
                    // Update session counts
                    self.state.server_manager.update_session_count();
                }
                Err(error) => {
                    self.state.show_popup = true;
                    self.state.popup_message = format!("⚠️ Connection Error:\n{}", error);
                    self.state.popup_shown_at = Some(Utc::now());
                    
                    // Update connection status
                    if let Some(connection) = self.state.server_manager.get_connection_mut(&server_id) {
                        connection.health_status = HealthStatus::Offline;
                        connection.stats.failed_attempts += 1;
                    }
                }
            }
        }
        
        self.state.mode = AppMode::Normal;
    }
    
    async fn kill_all_sessions(&mut self) {
        let mut killed_count = 0;
        let mut failed_kills = Vec::new();
        
        // Collect all active sessions
        let mut sessions_to_kill = Vec::new();
        for connection in self.state.server_manager.connections.values() {
            for session in &connection.active_sessions {
                sessions_to_kill.push((session.pid, connection.name.clone()));
            }
        }
        
        // Kill each session
        for (pid, server_name) in sessions_to_kill {
            #[cfg(unix)]
            {
                use std::process::Command;
                match Command::new("kill").arg("-TERM").arg(pid.to_string()).output() {
                    Ok(output) => {
                        if output.status.success() {
                            killed_count += 1;
                        } else {
                            failed_kills.push((pid, server_name.clone()));
                        }
                    }
                    Err(_) => {
                        failed_kills.push((pid, server_name.clone()));
                    }
                }
            }
            
            #[cfg(windows)]
            {
                use std::process::Command;
                match Command::new("taskkill").args(["/F", "/PID", &pid.to_string()]).output() {
                    Ok(output) => {
                        if output.status.success() {
                            killed_count += 1;
                        } else {
                            failed_kills.push((pid, server_name.clone()));
                        }
                    }
                    Err(_) => {
                        failed_kills.push((pid, server_name.clone()));
                    }
                }
            }
        }
        
        // Clear all sessions from connections
        for connection in self.state.server_manager.connections.values_mut() {
            connection.active_sessions.clear();
        }
        
        // Update session count
        self.state.server_manager.update_session_count();
        
        // Show result popup
        self.state.show_popup = true;
        self.state.popup_shown_at = Some(Utc::now());
        if failed_kills.is_empty() {
            self.state.popup_message = format!("🔫 Killed {} SSH sessions", killed_count);
        } else {
            self.state.popup_message = format!("🔫 Killed {} sessions\n⚠️ {} failed to kill", killed_count, failed_kills.len());
        }
    }

    async fn on_tick(&mut self) {
        self.state.last_update = Utc::now();
        
        
        // Update globe animation frame (changes every ~2 seconds) 
        self.state.globe_animation_frame = (self.state.globe_animation_frame + 1) % 80;
        
        // Auto-dismiss popup after 4 seconds
        if self.state.show_popup {
            if let Some(shown_at) = self.state.popup_shown_at {
                if Utc::now().signed_duration_since(shown_at).num_seconds() >= 4 {
                    self.state.show_popup = false;
                    self.state.popup_message.clear();
                    self.state.popup_shown_at = None;
                }
            }
        }
        
        // Auto-dismiss tooltips after 3 seconds
        if self.state.should_auto_dismiss_tooltip() {
            self.state.hide_tooltip();
        }
        
        // Clean up ended SSH sessions
        self.cleanup_ended_sessions().await;
        
        // Check for health updates from background monitoring
        while let Some(health_update) = self.health_monitor.try_recv_update().await {
            self.handle_health_update(health_update).await;
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
        // Handle form submission separately to avoid borrowing conflicts
        if key == KeyCode::Enter {
            if let Some(ref form) = self.state.server_form {
                if !form.auth_method_focused {
                    // Try to save the form
                    match form.to_server_connection() {
                        Ok(connection) => {
                            let is_editing = form.is_editing;
                            let original_id = form.original_id.clone();
                            
                            if is_editing {
                                // Update existing server
                                if let Some(id) = original_id {
                                    self.state.server_manager.connections.insert(id, connection);
                                }
                            } else {
                                // Add new server
                                let id = connection.id.clone();
                                self.state.server_manager.connections.insert(id, connection);
                            }
                            
                            let success_message = if is_editing {
                                "Server updated successfully!".to_string()
                            } else {
                                "Server added successfully!".to_string()
                            };
                            
                            self.state.server_form = None;
                            self.state.mode = AppMode::Normal;
                            
                            // Auto-save configuration
                            if let Err(e) = self.save_config() {
                                self.state.show_popup = true;
                                self.state.popup_message = format!("Failed to save config: {}", e);
                                self.state.popup_shown_at = Some(Utc::now());
                            } else {
                                self.state.show_popup = true;
                                self.state.popup_message = success_message;
                                self.state.popup_shown_at = Some(Utc::now());
                            }
                            return Ok(());
                        }
                        Err(error) => {
                            self.state.show_popup = true;
                            self.state.popup_message = format!("Validation error: {}", error);
                            self.state.popup_shown_at = Some(Utc::now());
                            return Ok(());
                        }
                    }
                }
            }
        }

        // Handle other form input
        if let Some(ref mut form) = self.state.server_form {
            match key {
                KeyCode::Esc => {
                    // Check if form has input and warn user
                    if form.has_input() {
                        self.state.show_popup = true;
                        self.state.popup_message = "Press Esc again to discard changes or Enter to save".to_string();
                        self.state.popup_shown_at = Some(Utc::now());
                        // TODO: Add confirmation dialog state
                        return Ok(());
                    }
                    self.state.server_form = None;
                    self.state.mode = AppMode::Normal;
                }
                KeyCode::Tab => {
                    form.next_field();
                }
                KeyCode::BackTab => {
                    form.previous_field();
                }
                KeyCode::Enter => {
                    if form.auth_method_focused {
                        form.auth_method_focused = false;
                        form.next_field();
                    }
                    // Form submission is handled above
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

    /// Clean up SSH sessions that have ended
    async fn cleanup_ended_sessions(&mut self) {
        let mut sessions_ended = false;
        
        for connection in self.state.server_manager.connections.values_mut() {
            let mut sessions_to_remove = Vec::new();
            
            for (i, session) in connection.active_sessions.iter().enumerate() {
                // Check if the process is still running
                #[cfg(unix)]
                {
                    use std::process::Command;
                    match Command::new("kill").arg("-0").arg(session.pid.to_string()).output() {
                        Ok(output) => {
                            if !output.status.success() {
                                // Process is not running anymore
                                sessions_to_remove.push(i);
                                sessions_ended = true;
                            }
                        }
                        Err(_) => {
                            // If we can't check the process, assume it's dead
                            sessions_to_remove.push(i);
                            sessions_ended = true;
                        }
                    }
                }
                
                #[cfg(windows)]
                {
                    use std::process::Command;
                    match Command::new("tasklist").args(["/FI", &format!("PID eq {}", session.pid)]).output() {
                        Ok(output) => {
                            let output_str = String::from_utf8_lossy(&output.stdout);
                            if !output_str.contains(&session.pid.to_string()) {
                                sessions_to_remove.push(i);
                                sessions_ended = true;
                            }
                        }
                        Err(_) => {
                            sessions_to_remove.push(i);
                            sessions_ended = true;
                        }
                    }
                }
            }
            
            // Remove ended sessions in reverse order to maintain indices
            for &i in sessions_to_remove.iter().rev() {
                connection.active_sessions.remove(i);
            }
        }
        
        // Update session count if any sessions ended
        if sessions_ended {
            self.state.server_manager.update_session_count();
        }
    }
    
    /// Handle health updates from background monitoring
    async fn handle_health_update(&mut self, update: HealthUpdate) {
        if let Some(connection) = self.state.server_manager.get_connection_mut(&update.server_id) {
            update.result.update_server_stats(connection);
            
            // Show notification for status changes that might need attention
            match update.result.status {
                HealthStatus::Offline => {
                    if connection.health_status != HealthStatus::Offline {
                        // Status changed to offline
                        self.state.show_popup = true;
                        self.state.popup_message = format!(
                            "⚠️ {} went offline", 
                            connection.name
                        );
                        self.state.popup_shown_at = Some(Utc::now());
                    }
                }
                HealthStatus::Online => {
                    if connection.health_status == HealthStatus::Offline {
                        // Status recovered to online
                        self.state.show_popup = true;
                        self.state.popup_message = format!(
                            "✅ {} is back online", 
                            connection.name
                        );
                        self.state.popup_shown_at = Some(Utc::now());
                    }
                }
                _ => {}
            }
        }
    }
    
    // Session management helper methods
    fn move_session_selection_down(&mut self) {
        let sessions = self.state.get_filtered_sessions();
        if !sessions.is_empty() {
            self.state.session_selected_index = 
                (self.state.session_selected_index + 1) % sessions.len();
        }
    }

    fn move_session_selection_up(&mut self) {
        let sessions = self.state.get_filtered_sessions();
        if !sessions.is_empty() {
            self.state.session_selected_index = 
                if self.state.session_selected_index == 0 {
                    sessions.len() - 1
                } else {
                    self.state.session_selected_index - 1
                };
        }
    }

    fn get_selected_session(&self) -> Option<&SessionInfo> {
        let sessions = self.state.get_filtered_sessions();
        sessions.get(self.state.session_selected_index).map(|session| *session)
    }

    async fn refresh_all_sessions(&mut self) {
        // Just run cleanup to remove dead sessions and update counts
        self.cleanup_ended_sessions().await;
        
        self.state.show_popup = true;
        self.state.popup_message = "Sessions refreshed".to_string();
        self.state.popup_shown_at = Some(chrono::Utc::now());
    }
    
    /// Show contextual tooltips based on current state
    fn show_contextual_tooltip(&mut self) {
        use crate::models::{TooltipCategory, AppMode};
        
        match self.state.mode {
            AppMode::Normal => {
                if self.state.server_manager.connections.is_empty() {
                    self.state.show_tooltip(
                        "Getting Started".to_string(),
                        "Add your first server with 'a' key. You can also edit, delete, or connect to servers from this view.".to_string(),
                        Some("Press 'a' to add server".to_string()),
                        TooltipCategory::Server,
                    );
                } else if let Some(connection) = self.get_selected_connection() {
                    let key_hints = vec![
                        "Enter: Connect".to_string(),
                        "e: Edit".to_string(), 
                        "d: Delete".to_string(),
                        "r: Refresh status".to_string(),
                    ];
                    
                    self.state.show_tooltip(
                        format!("Server: {}", connection.name),
                        format!("{}@{}:{} | Status: {}", connection.username, connection.host, connection.port, connection.health_status.as_str()),
                        Some(key_hints.join(" | ")),
                        TooltipCategory::Server,
                    );
                } else {
                    self.state.show_tooltip(
                        "Navigation Help".to_string(),
                        "Use j/k or arrow keys to navigate. Press 1-9 for quick connect. 'f' to filter, 'l' for layout options.".to_string(),
                        Some("j/k: Navigate | Enter: Connect | ?: Help".to_string()),
                        TooltipCategory::Navigation,
                    );
                }
            }
            AppMode::Sessions => {
                let sessions = self.state.get_filtered_sessions();
                if sessions.is_empty() {
                    self.state.show_tooltip(
                        "No Active Sessions".to_string(),
                        "Connect to servers to see active SSH sessions here. Sessions are tracked automatically.".to_string(),
                        Some("Esc: Return to servers | Enter: Connect to server".to_string()),
                        TooltipCategory::Session,
                    );
                } else {
                    self.state.show_tooltip(
                        "Session Management".to_string(),
                        format!("{} active sessions. Use 'd' to kill sessions, 'r' to refresh.", sessions.len()),
                        Some("d: Kill | r: Refresh | Enter: Info".to_string()),
                        TooltipCategory::Session,
                    );
                }
            }
            AppMode::Analytics => {
                self.state.show_tooltip(
                    "Analytics Dashboard".to_string(),
                    "View connection statistics, most used servers, and usage patterns.".to_string(),
                    Some("A: Return to servers | Esc: Exit".to_string()),
                    TooltipCategory::System,
                );
            }
            AppMode::Help => {
                self.state.show_tooltip(
                    "Help System".to_string(),
                    "Browse all available keyboard shortcuts and features.".to_string(),
                    Some("h/Esc: Return | F1: Toggle help".to_string()),
                    TooltipCategory::System,
                );
            }
            AppMode::History => {
                self.state.show_tooltip(
                    "Connection History".to_string(),
                    format!("{} recent connections tracked. Shows your connection activity over time.", 
                        self.state.server_manager.connection_history.len()),
                    Some("H: Return to servers | Esc: Exit".to_string()),
                    TooltipCategory::System,
                );
            }
            _ => {
                self.state.show_tooltip(
                    "Context Help".to_string(),
                    "Press Esc to return to the main view, or follow the on-screen instructions.".to_string(),
                    Some("Esc: Return | ?: Context help".to_string()),
                    TooltipCategory::System,
                );
            }
        }
    }
}
