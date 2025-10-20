use crate::colors::TokyoNight;
use crate::models::{AppMode, AppState, HealthStatus, SecurityStatus};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Wrap,
    },
    Frame,
};

pub fn ui(f: &mut Frame, app_state: &mut AppState) {
    let size = f.size();
    

    // Create main layout
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(0),     // Content
            Constraint::Length(3),  // Footer
        ])
        .split(size);

    // Render header
    render_header(f, main_chunks[0], app_state);

    // Render content based on mode
    match app_state.mode.clone() {
        AppMode::Normal => render_main_view(f, main_chunks[1], app_state),
        AppMode::Help => render_help_popup(f, size, app_state),
        AppMode::History => render_history_popup(f, size, app_state),
        AppMode::Analytics => render_analytics_dashboard(f, main_chunks[1], app_state),
        AppMode::Sessions => render_sessions_view(f, main_chunks[1], app_state),
        AppMode::ConfirmDelete(id) => render_confirm_delete_popup(f, size, app_state, &id),
        AppMode::Connecting(id) => render_connecting_popup(f, size, app_state, &id),
        AppMode::Loading(context) => {
            render_main_view(f, main_chunks[1], app_state);
            render_loading_popup(f, size, app_state, &context);
        }
        AppMode::AddServer | AppMode::EditServer(_) => {
            render_main_view(f, main_chunks[1], app_state);
            render_server_form_popup(f, size, app_state);
        }
    }

    // Render footer
    render_footer(f, main_chunks[2], app_state);

    // Render general popup if needed
    if app_state.show_popup {
        render_message_popup(f, size, app_state);
    }
    
    // Render tooltip if active
    if let Some(ref tooltip) = app_state.current_tooltip {
        render_tooltip(f, size, app_state, tooltip);
    }
}

fn render_header(f: &mut Frame, area: Rect, app_state: &AppState) {
    let theme = app_state.theme_manager.current_theme();
    let header_text = vec![
        Line::from(vec![
            Span::styled("👻 ", Style::default().fg(theme.theme_primary)),
            Span::styled("GHOST", Style::default()
                .fg(theme.theme_primary)
                .add_modifier(Modifier::BOLD)),
            Span::styled(" SSH Manager ", Style::default().fg(theme.fg)),
            Span::styled(app_state.get_globe_char(), Style::default().fg(theme.fg)),
            Span::styled(" ", Style::default()),
            Span::styled(
                format!("[{}]", get_status_line(app_state)),
                Style::default().fg(theme.cyan)
            ),
        ]),
    ];

    let header = Paragraph::new(header_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg)),
        )
        .alignment(Alignment::Center);

    f.render_widget(header, area);
}

fn render_main_view(f: &mut Frame, area: Rect, app_state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(app_state.layout.get_constraints())
        .split(area);

    match app_state.layout.mode {
        crate::models::LayoutMode::SinglePanel => {
            // Only render server list in full width
            render_server_list(f, chunks[0], app_state);
        }
        crate::models::LayoutMode::TwoPanel => {
            // Render server list and details
            render_server_list(f, chunks[0], app_state);
            render_details_panel(f, chunks[1], app_state);
        }
        crate::models::LayoutMode::ThreePanel => {
            // Render server list, details, and metrics
            render_server_list(f, chunks[0], app_state);
            render_details_panel(f, chunks[1], app_state);
            render_metrics_panel(f, chunks[2], app_state);
        }
    }
}

fn render_server_list(f: &mut Frame, area: Rect, app_state: &mut AppState) {
    let connections = app_state.server_manager.filtered_connections();
    
    let items: Vec<ListItem> = connections
        .iter()
        .enumerate()
        .map(|(i, conn)| {
            let style = if i == app_state.server_manager.selected_index {
                Style::default()
                    .bg(TokyoNight::BG_HIGHLIGHT)
                    .fg(TokyoNight::THEME_GREEN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TokyoNight::FG)
            };

            let health_color = match conn.health_status {
                HealthStatus::Online => TokyoNight::STATUS_ONLINE,
                HealthStatus::Offline => TokyoNight::STATUS_OFFLINE,
                HealthStatus::Connecting => TokyoNight::STATUS_CONNECTING,
                HealthStatus::Warning => TokyoNight::STATUS_WARNING,
                HealthStatus::Unknown => TokyoNight::STATUS_UNKNOWN,
            };

            let security_color = match conn.security_status {
                SecurityStatus::Secure => TokyoNight::GREEN,
                SecurityStatus::Vulnerable => TokyoNight::ORANGE,
                SecurityStatus::Compromised => TokyoNight::RED,
                SecurityStatus::Unknown => TokyoNight::COMMENT,
            };

            // Use spinning globe for connecting servers, otherwise use normal symbol
            let health_symbol = if matches!(conn.health_status, HealthStatus::Connecting) {
                app_state.get_globe_char()
            } else {
                conn.health_status.symbol()
            };

            let connection_string = conn.connection_string();
            
            // Add session indicator
            let session_indicator = if conn.has_active_sessions() {
                format!(" [{}]", conn.session_count())
            } else {
                String::new()
            };
            
            // Add quick connect number (1-9)
            let quick_num = if i < 9 {
                format!("{}:", i + 1)
            } else {
                "  ".to_string()
            };
            
            let content = vec![
                Line::from(vec![
                    Span::styled(quick_num.clone(), Style::default().fg(TokyoNight::COMMENT)),
                    Span::styled(health_symbol, Style::default().fg(health_color)),
                    Span::raw(" "),
                    Span::styled(conn.security_status.symbol(), Style::default().fg(security_color)),
                    Span::raw(" "),
                    Span::styled(&conn.name, style),
                    if conn.has_active_sessions() {
                        Span::styled(session_indicator, Style::default().fg(TokyoNight::GREEN).add_modifier(Modifier::BOLD))
                    } else {
                        Span::raw("")
                    },
                ]),
                Line::from(vec![
                    Span::raw("     "),
                    Span::styled(connection_string, 
                        Style::default().fg(TokyoNight::COMMENT)),
                ]),
            ];

            ListItem::new(content).style(style)
        })
        .collect();

    let title = format!(
        " Servers [{}/{}] ",
        connections.len(),
        app_state.server_manager.connection_count()
    );

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .title_style(Style::default().fg(TokyoNight::THEME_GREEN).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(TokyoNight::BORDER))
                .style(Style::default().bg(TokyoNight::BG)),
        )
        .style(Style::default().fg(TokyoNight::FG));

    f.render_widget(list, area);
}

fn render_metrics_panel(f: &mut Frame, area: Rect, app_state: &AppState) {
    let theme = app_state.theme_manager.current_theme();
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // System overview
    let total = app_state.server_manager.connection_count();
    let online = app_state.server_manager.online_count();
    let sessions = app_state.server_manager.active_session_count;
    
    let overview_text = vec![
        Line::from(vec![
            Span::styled("📊 Overview", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Servers: ", Style::default().fg(theme.comment)),
            Span::styled(format!("{}/{} online", online, total), 
                Style::default().fg(if online == total { theme.green } else { theme.orange })),
        ]),
        Line::from(vec![
            Span::styled("Sessions: ", Style::default().fg(theme.comment)),
            Span::styled(sessions.to_string(), 
                Style::default().fg(if sessions > 0 { theme.green } else { theme.comment })),
        ]),
        Line::from(vec![
            Span::styled("Layout: ", Style::default().fg(theme.comment)),
            Span::styled(format!("{:?}", app_state.layout.mode), Style::default().fg(theme.fg)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Panel Sizes:", Style::default().fg(theme.purple).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(format!("  [{}% | {}% | {}%]", 
                app_state.layout.panel_sizes[0], 
                app_state.layout.panel_sizes[1], 
                app_state.layout.panel_sizes[2]), 
                Style::default().fg(theme.comment)),
        ]),
    ];

    let overview = Paragraph::new(overview_text)
        .block(
            Block::default()
                .title(" System Metrics ")
                .title_style(Style::default().fg(theme.theme_primary).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(overview, chunks[0]);
    
    // Quick stats
    let stats_text = vec![
        Line::from(vec![
            Span::styled("⚡ Quick Stats", Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Theme: ", Style::default().fg(theme.comment)),
            Span::styled(format!("{:?}", app_state.theme_manager.current_variant()), Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("History: ", Style::default().fg(theme.comment)),
            Span::styled(format!("{} entries", app_state.server_manager.connection_history.len()), 
                Style::default().fg(theme.fg)),
        ]),
    ];

    let stats = Paragraph::new(stats_text)
        .block(
            Block::default()
                .title(" Stats ")
                .title_style(Style::default().fg(theme.theme_primary).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(stats, chunks[1]);
}

fn render_details_panel(f: &mut Frame, area: Rect, app_state: &AppState) {
    let connections = app_state.server_manager.filtered_connections();
    
    if let Some(connection) = connections.get(app_state.server_manager.selected_index) {
        let details = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(&connection.name, Style::default().fg(TokyoNight::FG)),
            ]),
            Line::from(vec![]),
            Line::from(vec![
                Span::styled("Host: ", Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(&connection.host, Style::default().fg(TokyoNight::FG)),
            ]),
            Line::from(vec![
                Span::styled("Port: ", Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(connection.port.to_string(), Style::default().fg(TokyoNight::FG)),
            ]),
            Line::from(vec![
                Span::styled("User: ", Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(&connection.username, Style::default().fg(TokyoNight::FG)),
            ]),
            Line::from(vec![]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(connection.health_status.symbol(), 
                    Style::default().fg(get_health_color(&connection.health_status))),
                Span::raw(" "),
                Span::styled(connection.health_status.as_str(), 
                    Style::default().fg(get_health_color(&connection.health_status))),
            ]),
            Line::from(vec![
                Span::styled("Security: ", Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(connection.security_status.symbol(), 
                    Style::default().fg(get_security_color(&connection.security_status))),
                Span::raw(" "),
                Span::styled(connection.security_status.as_str(), 
                    Style::default().fg(get_security_color(&connection.security_status))),
            ]),
            Line::from(vec![]),
            Line::from(vec![
                Span::styled("Created: ", Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(
                    connection.created_at.format("%Y-%m-%d %H:%M").to_string(),
                    Style::default().fg(TokyoNight::COMMENT)
                ),
            ]),
            Line::from(vec![]),
            Line::from(vec![
                Span::styled("Latency: ", Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD)),
                if let Some(latency) = connection.stats.latency {
                    Span::styled(format!("{}ms", latency.as_millis()), Style::default().fg(TokyoNight::GREEN))
                } else {
                    Span::styled("N/A", Style::default().fg(TokyoNight::COMMENT))
                },
                Span::raw(" "),
                Span::styled(render_latency_sparkline(&connection.stats.latency_history), 
                    Style::default().fg(TokyoNight::BLUE)),
            ]),
            Line::from(vec![
                Span::styled("Connections: ", Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(connection.stats.connection_count.to_string(), 
                    Style::default().fg(TokyoNight::FG)),
                if connection.stats.failed_attempts > 0 {
                    Span::styled(format!(" ({} failed)", connection.stats.failed_attempts), 
                        Style::default().fg(TokyoNight::RED))
                } else {
                    Span::raw("")
                },
            ]),
            Line::from(vec![]),
            Line::from(vec![
                Span::styled("Sessions: ", Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD)),
                if connection.has_active_sessions() {
                    Span::styled(format!("{} active", connection.session_count()), 
                        Style::default().fg(TokyoNight::GREEN).add_modifier(Modifier::BOLD))
                } else {
                    Span::styled("None", Style::default().fg(TokyoNight::COMMENT))
                },
            ]),
        ];
        
        // Add session details if any are active
        let mut details = details;
        if connection.has_active_sessions() {
            details.push(Line::from(vec![]));
            for (i, session) in connection.active_sessions.iter().enumerate() {
                if i < 3 { // Show max 3 sessions to avoid clutter
                    details.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(format!("PID {}: ", session.pid), 
                            Style::default().fg(TokyoNight::COMMENT)),
                        Span::styled(
                            session.started_at.format("%H:%M:%S").to_string(),
                            Style::default().fg(TokyoNight::FG)
                        ),
                    ]));
                }
            }
            if connection.active_sessions.len() > 3 {
                details.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(format!("... and {} more", connection.active_sessions.len() - 3),
                        Style::default().fg(TokyoNight::COMMENT)),
                ]));
            }
        }

        let paragraph = Paragraph::new(details)
            .block(
                Block::default()
                    .title(" Details ")
                    .title_style(Style::default().fg(TokyoNight::THEME_GREEN).add_modifier(Modifier::BOLD))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(TokyoNight::BORDER))
                    .style(Style::default().bg(TokyoNight::BG)),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    } else {
        let no_selection = Paragraph::new("No server selected")
            .block(
                Block::default()
                    .title(" Details ")
                    .title_style(Style::default().fg(TokyoNight::THEME_GREEN).add_modifier(Modifier::BOLD))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(TokyoNight::BORDER))
                    .style(Style::default().bg(TokyoNight::BG)),
            )
            .style(Style::default().fg(TokyoNight::COMMENT))
            .alignment(Alignment::Center);

        f.render_widget(no_selection, area);
    }
}

fn render_footer(f: &mut Frame, area: Rect, app_state: &AppState) {
    let keybindings = match app_state.mode {
        AppMode::Normal => "j/k: Navigate | Enter/1-9: Connect | a: Add | e: Edit | d: Delete | r: Refresh & Security Check | f: Filter | S: Sessions | A: Analytics | H: History | t/T: Themes | l: Layout | [/]: Resize | ?: Tips | h: Help | Ctrl+X: Kill All | q: Quit",
        AppMode::Help => "Press h, q, or Esc to return",
        AppMode::History => "Press H, q, or Esc to return",
        AppMode::Analytics => "Press A, q, or Esc to return",
        AppMode::Sessions => "j/k: Navigate | d: Kill | r: Refresh | Enter: Info | S/q/Esc: Return",
        AppMode::ConfirmDelete(_) => "y: Confirm | n: Cancel",
        AppMode::Connecting(_) => "Esc: Cancel connection",
        _ => "Esc: Return to main view",
    };

    let footer = Paragraph::new(keybindings)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(TokyoNight::BORDER))
                .style(Style::default().bg(TokyoNight::BG)),
        )
        .style(Style::default().fg(TokyoNight::COMMENT))
        .alignment(Alignment::Center);

    f.render_widget(footer, area);
}

fn render_help_popup(f: &mut Frame, area: Rect, _app_state: &AppState) {
    let popup_area = centered_rect(60, 70, area);
    
    let help_text = vec![
        Line::from(Span::styled("👻 GHOST SSH Manager - Help", 
            Style::default().fg(TokyoNight::THEME_GREEN).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("NAVIGATION:"),
        Line::from("  j/k or ↑/↓     Navigate server list"),
        Line::from("  Enter          Connect to selected server / Dismiss popup"),
        Line::from("  1-9            Quick connect to server 1-9"),
        Line::from(""),
        Line::from("SERVER MANAGEMENT:"),
        Line::from("  a              Add new server"),
        Line::from("  e              Edit selected server"),
        Line::from("  d              Delete selected server"),
        Line::from("  r              Refresh server status & security assessment"),
        Line::from(""),
        Line::from("CONNECTION MODES:"),
        Line::from("  CLI: --new-terminal     Force new terminal window"),
        Line::from("  CLI: --direct          Force current terminal (Warp compatible)"),
        Line::from("  CLI: --connection-mode  auto/new-terminal/direct"),
        Line::from(""),
        Line::from("FILTERING & VIEWS:"),
        Line::from("  f              Toggle online-only filter"),
        Line::from("  S              Session manager (view active SSH sessions)"),
        Line::from("  A              Analytics dashboard (usage statistics)"),
        Line::from("  H              Connection history"),
        Line::from(""),
        Line::from("SESSION MANAGEMENT:"),
        Line::from("  Ctrl+X         Kill all active SSH sessions"),
        Line::from(""),
        Line::from("THEMES & LAYOUT:"),
        Line::from("  t              Toggle theme selector"),
        Line::from("  T              Quick theme cycle"),
        Line::from("  l              Cycle layout mode (Single/Two/Three panels)"),
        Line::from("  [ / ]          Resize panels (decrease/increase left panel)"),
        Line::from(""),
        Line::from("TOOLTIPS & HELP:"),
        Line::from("  ?              Show contextual tooltip"),
        Line::from("  F2             Toggle tooltips on/off"),
        Line::from("  h or F1        Show this help"),
        Line::from(""),
        Line::from("SECURITY STATUS:"),
        Line::from("  🛡️ SECURE       SSH keys, non-standard ports"),
        Line::from("  ⚠️ VULNERABLE   Password auth on port 22"),
        Line::from("  ? UNKNOWN       Assessment pending/failed"),
        Line::from(""),
        Line::from("TERMINAL SUPPORT:"),
        Line::from("  ✅ Ghostty, Alacritty, Kitty, Wezterm, GNOME, Konsole"),
        Line::from("  ⚠️ Warp Terminal (direct mode only)"),
        Line::from(""),
        Line::from("GENERAL:"),
        Line::from("  q              Quit application"),
        Line::from("  Esc            Dismiss popup / Quit application"),
        Line::from("  Ctrl+C         Force quit"),
        Line::from(""),
        Line::from(Span::styled("Press h, q, or Esc to return", 
            Style::default().fg(TokyoNight::CYAN))),
    ];

    f.render_widget(Clear, popup_area);
    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .title_style(Style::default().fg(TokyoNight::THEME_GREEN).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(TokyoNight::BORDER_HIGHLIGHT))
                .style(Style::default().bg(TokyoNight::BG_POPUP)),
        )
        .style(Style::default().fg(TokyoNight::FG))
        .wrap(Wrap { trim: true });

    f.render_widget(help, popup_area);
}

fn render_history_popup(f: &mut Frame, area: Rect, app_state: &AppState) {
    let popup_area = centered_rect(80, 70, area);
    
    let history_items: Vec<ListItem> = app_state.server_manager.connection_history
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let time_str = entry.connected_at.format("%Y-%m-%d %H:%M:%S").to_string();
            let content = vec![
                Line::from(vec![
                    Span::styled(format!("{}. ", i + 1), Style::default().fg(TokyoNight::COMMENT)),
                    Span::styled(&entry.server_name, Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(time_str, Style::default().fg(TokyoNight::COMMENT)),
                ]),
            ];
            ListItem::new(content)
        })
        .collect();
    
    let history_text = if history_items.is_empty() {
        vec![Line::from(Span::styled(
            "No connection history yet. Connect to servers to see history here.",
            Style::default().fg(TokyoNight::COMMENT)
        ))]
    } else {
        vec![] // The list will be rendered separately
    };
    
    f.render_widget(Clear, popup_area);
    
    if history_items.is_empty() {
        let history = Paragraph::new(history_text)
            .block(
                Block::default()
                    .title(" Connection History ")
                    .title_style(Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(TokyoNight::BORDER_HIGHLIGHT))
                    .style(Style::default().bg(TokyoNight::BG_POPUP)),
            )
            .style(Style::default().fg(TokyoNight::FG))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(history, popup_area);
    } else {
        let history_list = List::new(history_items)
            .block(
                Block::default()
                    .title(format!(" Connection History ({}) ", app_state.server_manager.connection_history.len()))
                    .title_style(Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(TokyoNight::BORDER_HIGHLIGHT))
                    .style(Style::default().bg(TokyoNight::BG_POPUP)),
            )
            .style(Style::default().fg(TokyoNight::FG));
        f.render_widget(history_list, popup_area);
    }
    
    // Add instructions at the bottom
    let instruction_area = Rect {
        x: popup_area.x,
        y: popup_area.y + popup_area.height - 1,
        width: popup_area.width,
        height: 1,
    };
    
    let instructions = Paragraph::new("Press H, q, or Esc to return")
        .style(Style::default().fg(TokyoNight::COMMENT))
        .alignment(Alignment::Center);
    f.render_widget(instructions, instruction_area);
}

fn render_confirm_delete_popup(f: &mut Frame, area: Rect, app_state: &AppState, server_id: &str) {
    let popup_area = centered_rect(50, 20, area);
    
    let server_name = app_state.server_manager.get_connection(server_id)
        .map(|c| c.name.as_str())
        .unwrap_or("Unknown");
    
    let text = vec![
        Line::from(""),
        Line::from(Span::styled("⚠️  WARNING", 
            Style::default().fg(TokyoNight::RED).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::raw("Delete server \""),
            Span::styled(server_name, Style::default().fg(TokyoNight::CYAN)),
            Span::raw("\"?"),
        ]),
        Line::from(""),
        Line::from(Span::styled("y: Yes | n: No", 
            Style::default().fg(TokyoNight::COMMENT))),
    ];

    f.render_widget(Clear, popup_area);
    let confirm = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Confirm Delete ")
                .title_style(Style::default().fg(TokyoNight::RED).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(TokyoNight::RED))
                .style(Style::default().bg(TokyoNight::BG_POPUP)),
        )
        .style(Style::default().fg(TokyoNight::FG))
        .alignment(Alignment::Center);

    f.render_widget(confirm, popup_area);
}

fn render_connecting_popup(f: &mut Frame, area: Rect, app_state: &AppState, server_id: &str) {
    let popup_area = centered_rect(40, 15, area);
    
    let server_name = app_state.server_manager.get_connection(server_id)
        .map(|c| c.name.as_str())
        .unwrap_or("Unknown");
    
    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(app_state.get_globe_char(), Style::default().fg(TokyoNight::FG)),
            Span::raw(" → Connecting to "),
            Span::styled(server_name, Style::default().fg(TokyoNight::CYAN)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Press Esc to cancel", 
            Style::default().fg(TokyoNight::COMMENT))),
    ];

    f.render_widget(Clear, popup_area);
    let connecting = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Connecting... ")
                .title_style(Style::default().fg(TokyoNight::BLUE).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(TokyoNight::BLUE))
                .style(Style::default().bg(TokyoNight::BG_POPUP)),
        )
        .style(Style::default().fg(TokyoNight::FG))
        .alignment(Alignment::Center);

    f.render_widget(connecting, popup_area);
}

fn render_loading_popup(f: &mut Frame, area: Rect, app_state: &AppState, context: &crate::models::LoadingContext) {
    use crate::models::LoadingContext;
    let theme = app_state.theme_manager.current_theme();
    
    let popup_area = centered_rect(50, 18, area);
    
    let (title, status_text, progress_info) = match context {
        LoadingContext::RefreshingHealth { completed, total } => {
            let progress = if *total > 0 { *completed as f32 / *total as f32 } else { 0.0 };
            let progress_bar = create_progress_bar(progress, 30);
            
            (
                "🔄 Refreshing Health",
                "Checking server status...".to_string(),
                format!("{}\n{}/{} servers checked", progress_bar, completed, total)
            )
        }
    };
    
    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(app_state.get_globe_char(), Style::default().fg(theme.theme_primary)),
            Span::raw(" "),
            Span::styled(status_text, Style::default().fg(theme.fg)),
        ]),
        Line::from(""),
        Line::from(Span::styled(progress_info, Style::default().fg(theme.comment))),
        Line::from(""),
        Line::from(Span::styled("Press Esc to cancel", 
            Style::default().fg(theme.comment).add_modifier(Modifier::ITALIC))),
    ];

    f.render_widget(Clear, popup_area);
    let loading_popup = Paragraph::new(text)
        .block(
            Block::default()
                .title(format!(" {} ", title))
                .title_style(Style::default().fg(theme.theme_primary).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.theme_primary))
                .style(Style::default().bg(theme.bg)),
        )
        .style(Style::default().fg(theme.fg))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(loading_popup, popup_area);
}

/// Create a visual progress bar
fn create_progress_bar(progress: f32, width: usize) -> String {
    let filled = (progress * width as f32) as usize;
    let empty = width.saturating_sub(filled);
    
    let filled_str = "█".repeat(filled);
    let empty_str = "░".repeat(empty);
    
    format!("{}{}", filled_str, empty_str)
}

fn render_server_form_popup(f: &mut Frame, area: Rect, app_state: &AppState) {
    if let Some(ref form) = app_state.server_form {
        let popup_area = centered_rect(80, 90, area);
        
        let title = if form.is_editing {
            " Edit Server "
        } else {
            " Add Server "
        };

        f.render_widget(Clear, popup_area);
        
        // Split the popup into sections
        let form_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(2),  // Instructions
                Constraint::Min(0),     // Form fields
                Constraint::Length(3),  // Action buttons
            ])
            .split(popup_area);

        // Render instructions
        let instructions = Paragraph::new("Tab/Shift+Tab: Navigate | Enter: Save | Esc: Cancel")
            .style(Style::default().fg(TokyoNight::COMMENT))
            .alignment(Alignment::Center);
        f.render_widget(instructions, form_chunks[0]);

        // Render form fields
        render_form_fields(f, form_chunks[1], form);

        // Render action buttons
        let actions = vec![
            Line::from(vec![
                Span::styled("[Enter]", Style::default().fg(TokyoNight::GREEN).add_modifier(Modifier::BOLD)),
                Span::raw(" Save   "),
                Span::styled("[Esc]", Style::default().fg(TokyoNight::RED).add_modifier(Modifier::BOLD)),
                Span::raw(" Cancel"),
            ])
        ];
        let action_bar = Paragraph::new(actions)
            .style(Style::default().fg(TokyoNight::FG))
            .alignment(Alignment::Center);
        f.render_widget(action_bar, form_chunks[2]);

        // Render the main popup block
        let popup_block = Block::default()
            .title(title)
            .title_style(Style::default().fg(TokyoNight::THEME_GREEN).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(TokyoNight::BORDER_HIGHLIGHT))
            .style(Style::default().bg(TokyoNight::BG_POPUP));
        f.render_widget(popup_block, popup_area);
    }
}

fn render_form_fields(f: &mut Frame, area: Rect, form: &crate::forms::ServerForm) {
    let field_height = 3; // Input field with border
    let auth_height = 4;  // Auth method dropdown
    let _total_fields = form.fields.len() + 1 + 1; // fields + auth + tags
    
    let field_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            std::iter::repeat(Constraint::Length(field_height))
                .take(form.fields.len())
                .chain(std::iter::once(Constraint::Length(auth_height))) // Auth method
                .chain(std::iter::once(Constraint::Length(field_height))) // Tags
                .collect::<Vec<_>>()
        )
        .split(area);

    // Render regular input fields
    for (i, field) in form.fields.iter().enumerate() {
        if let Some(field_area) = field_areas.get(i) {
            render_input_field(f, *field_area, field, i == form.current_field && !form.auth_method_focused);
        }
    }

    // Render auth method dropdown
    if let Some(auth_area) = field_areas.get(form.fields.len()) {
        render_auth_method_field(f, *auth_area, form);
    }

    // Render tags field
    if let Some(tags_area) = field_areas.get(form.fields.len() + 1) {
        render_input_field(f, *tags_area, &form.tags_input, form.current_field == form.fields.len() && !form.auth_method_focused);
    }
}

fn render_input_field(f: &mut Frame, area: Rect, field: &crate::forms::InputField, is_focused: bool) {

    // Render input field
    let display_value = if field.value.is_empty() {
        if is_focused {
            String::new() // Show empty string for focused empty fields
        } else {
            field.placeholder.clone()
        }
    } else {
        field.display_value()
    };
    
    let input_style = if is_focused {
        Style::default().bg(TokyoNight::BG_HIGHLIGHT).fg(TokyoNight::THEME_GREEN)
    } else {
        Style::default().bg(TokyoNight::BG).fg(
            if field.value.is_empty() {
                TokyoNight::COMMENT
            } else {
                TokyoNight::FG
            }
        )
    };
    
    let border_style = if is_focused {
        Style::default().fg(TokyoNight::THEME_GREEN)
    } else {
        Style::default().fg(TokyoNight::BORDER)
    };
    
    let title = if is_focused {
        format!(" {} [EDITING] ", field.label)
    } else {
        format!(" {} ", field.label)
    };

    let input = Paragraph::new(display_value)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style)
        )
        .style(input_style);
    f.render_widget(input, area);

    // Render cursor if focused
    if is_focused {
        let cursor_x = area.x + 1 + field.cursor_position as u16;
        let cursor_y = area.y + 1;
        if cursor_x < area.x + area.width - 1 {
            f.set_cursor(cursor_x.min(area.x + area.width - 2), cursor_y);
        }
    }
}


fn render_auth_method_field(f: &mut Frame, area: Rect, form: &crate::forms::ServerForm) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(2)])
        .split(area);

    // Render label
    let label_style = if form.auth_method_focused {
        Style::default().fg(TokyoNight::THEME_GREEN).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(TokyoNight::CYAN)
    };
    let label = Paragraph::new("Authentication:")
        .style(label_style);
    f.render_widget(label, chunks[0]);

    // Render dropdown
    let dropdown_style = if form.auth_method_focused {
        Style::default().bg(TokyoNight::BG_HIGHLIGHT).fg(TokyoNight::FG)
    } else {
        Style::default().bg(TokyoNight::BG).fg(TokyoNight::FG)
    };
    
    let border_style = if form.auth_method_focused {
        Style::default().fg(TokyoNight::THEME_GREEN)
    } else {
        Style::default().fg(TokyoNight::BORDER)
    };

    let auth_text = vec![
        Line::from(vec![
            Span::styled("▼ ", Style::default().fg(TokyoNight::THEME_GREEN)),
            Span::styled(form.auth_method.display_name(), dropdown_style),
        ]),
        Line::from(Span::styled(
            form.auth_method.description(),
            Style::default().fg(TokyoNight::COMMENT)
        )),
    ];

    let dropdown = Paragraph::new(auth_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
        )
        .style(dropdown_style);
    f.render_widget(dropdown, chunks[1]);
}

fn render_tooltip(f: &mut Frame, area: Rect, app_state: &AppState, tooltip: &crate::models::TooltipInfo) {
    use chrono::Utc;
    use crate::models::TooltipCategory;
    let theme = app_state.theme_manager.current_theme();
    
    // Position tooltip in bottom-right corner
    let tooltip_width = 50;
    let tooltip_height = if tooltip.key_hint.is_some() { 8 } else { 6 };
    
    let tooltip_area = Rect {
        x: area.width.saturating_sub(tooltip_width + 2),
        y: area.height.saturating_sub(tooltip_height + 2),
        width: tooltip_width,
        height: tooltip_height,
    };
    
    // Calculate remaining time for auto-dismiss
    let remaining_time = if let Some(shown_at) = app_state.tooltip_shown_at {
        let elapsed = Utc::now().signed_duration_since(shown_at).num_seconds();
        (3 - elapsed).max(0)
    } else {
        3
    };
    
    // Choose colors based on category
    let (title_color, border_color, category_icon) = match tooltip.category {
        TooltipCategory::Navigation => (theme.blue, theme.blue, "🧭"),
        TooltipCategory::Server => (theme.green, theme.green, "🖥️"),
        TooltipCategory::Session => (theme.orange, theme.orange, "⚡"),
        TooltipCategory::Theme => (theme.purple, theme.purple, "🎨"),
        TooltipCategory::Layout => (theme.cyan, theme.cyan, "📐"),
        TooltipCategory::System => (theme.theme_primary, theme.theme_primary, "⚙️"),
    };
    
    let mut content = vec![
        Line::from(vec![
            Span::styled(category_icon, Style::default().fg(title_color)),
            Span::raw(" "),
            Span::styled(&tooltip.title, Style::default().fg(title_color).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(Span::styled(&tooltip.description, Style::default().fg(theme.fg))),
    ];
    
    if let Some(ref key_hint) = tooltip.key_hint {
        content.push(Line::from(""));
        content.push(Line::from(vec![
            Span::styled("💡 ", Style::default().fg(theme.yellow)),
            Span::styled(key_hint, Style::default().fg(theme.comment).add_modifier(Modifier::ITALIC)),
        ]));
    }
    
    content.push(Line::from(""));
    content.push(Line::from(vec![
        Span::styled(format!("Auto-dismiss in {}s | Esc: Close", remaining_time), 
            Style::default().fg(theme.comment).add_modifier(Modifier::ITALIC)),
    ]));
    
    f.render_widget(Clear, tooltip_area);
    let tooltip_widget = Paragraph::new(content)
        .block(
            Block::default()
                .title(" 💬 Tooltip ")
                .title_style(Style::default().fg(title_color).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(theme.bg)),
        )
        .style(Style::default().fg(theme.fg))
        .wrap(Wrap { trim: true });

    f.render_widget(tooltip_widget, tooltip_area);
}

fn render_message_popup(f: &mut Frame, area: Rect, app_state: &AppState) {
    use chrono::Utc;
    
    let popup_area = centered_rect(40, 10, area);
    
    // Calculate remaining time for auto-dismiss
    let remaining_time = if let Some(shown_at) = app_state.popup_shown_at {
        let elapsed = Utc::now().signed_duration_since(shown_at).num_seconds();
        (4 - elapsed).max(0)
    } else {
        4
    };
    
    // Create message with countdown
    let message_with_time = if remaining_time > 0 {
        format!("{}\n\n[Auto-dismiss in {}s | Press Enter/Esc to close]", app_state.popup_message, remaining_time)
    } else {
        app_state.popup_message.clone()
    };
    
    f.render_widget(Clear, popup_area);
    let popup = Paragraph::new(message_with_time)
        .block(
            Block::default()
                .title(" Info ")
                .title_style(Style::default().fg(TokyoNight::CYAN))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(TokyoNight::BORDER_HIGHLIGHT))
                .style(Style::default().bg(TokyoNight::BG_POPUP)),
        )
        .style(Style::default().fg(TokyoNight::FG))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(popup, popup_area);
}

// Helper functions

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn get_status_line(app_state: &AppState) -> String {
    let total = app_state.server_manager.connection_count();
    let online = app_state.server_manager.online_count();
    let sessions = app_state.server_manager.active_session_count;
    
    // Check if any servers are connecting
    let connecting_count = app_state.server_manager.connections.values()
        .filter(|conn| matches!(conn.health_status, HealthStatus::Connecting))
        .count();
    
    let mut status_parts = Vec::new();
    status_parts.push(format!("{}/{} online", online, total));
    
    if sessions > 0 {
        status_parts.push(format!("{} sessions", sessions));
    }
    
    if connecting_count > 0 {
        status_parts.push(format!("{} {} connecting", app_state.get_globe_char(), connecting_count));
    }
    
    status_parts.join(" | ")
}

fn get_health_color(status: &HealthStatus) -> Color {
    match status {
        HealthStatus::Online => TokyoNight::STATUS_ONLINE,
        HealthStatus::Offline => TokyoNight::STATUS_OFFLINE,
        HealthStatus::Connecting => TokyoNight::STATUS_CONNECTING,
        HealthStatus::Warning => TokyoNight::STATUS_WARNING,
        HealthStatus::Unknown => TokyoNight::STATUS_UNKNOWN,
    }
}

fn get_security_color(status: &SecurityStatus) -> Color {
    match status {
        SecurityStatus::Secure => TokyoNight::GREEN,
        SecurityStatus::Vulnerable => TokyoNight::ORANGE,
        SecurityStatus::Compromised => TokyoNight::RED,
        SecurityStatus::Unknown => TokyoNight::COMMENT,
    }
}

/// Render a mini sparkline for latency history
fn render_latency_sparkline(history: &[u32]) -> String {
    if history.is_empty() {
        return "▁▁▁▁▁".to_string();
    }
    
    let max_latency = *history.iter().max().unwrap_or(&1) as f32;
    let chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    
    history.iter().rev().take(8).rev().map(|&latency| {
        let normalized = (latency as f32 / max_latency * (chars.len() - 1) as f32) as usize;
        chars[normalized.min(chars.len() - 1)]
    }).collect::<String>().chars().rev().collect::<String>().chars().take(8).collect()
}


/// Render the analytics dashboard
fn render_analytics_dashboard(f: &mut Frame, area: Rect, app_state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Overview stats
            Constraint::Min(0),     // Detailed analytics
        ])
        .split(area);
    
    // Render overview statistics
    render_analytics_overview(f, chunks[0], app_state);
    
    // Render detailed analytics
    render_analytics_details(f, chunks[1], app_state);
}

/// Render analytics overview section
fn render_analytics_overview(f: &mut Frame, area: Rect, app_state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(25), Constraint::Percentage(25), Constraint::Percentage(25)])
        .split(area);
    
    // Total connections
    let total_connections = app_state.server_manager.connections.values()
        .map(|c| c.stats.connection_count)
        .sum::<u32>();
    
    let total_failures = app_state.server_manager.connections.values()
        .map(|c| c.stats.failed_attempts)
        .sum::<u32>();
        
    let success_rate = if total_connections > 0 {
        (total_connections - total_failures) as f32 / total_connections as f32 * 100.0
    } else {
        0.0
    };
    
    // Render stat boxes
    let stats = vec![
        ("Total Connections", total_connections.to_string(), TokyoNight::CYAN),
        ("Success Rate", format!("{:.1}%", success_rate), TokyoNight::GREEN),
        ("Active Sessions", app_state.server_manager.active_session_count.to_string(), TokyoNight::BLUE),
        ("Online Servers", format!("{}/{}", app_state.server_manager.online_count(), app_state.server_manager.connection_count()), TokyoNight::THEME_GREEN),
    ];
    
    for (i, (label, value, color)) in stats.iter().enumerate() {
        if let Some(chunk) = chunks.get(i) {
            let stat_text = vec![
                Line::from(
                    Span::styled(value, Style::default().fg(*color).add_modifier(Modifier::BOLD))
                ),
                Line::from(
                    Span::styled(*label, Style::default().fg(TokyoNight::COMMENT))
                ),
            ];
            
            let stat_block = Paragraph::new(stat_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(TokyoNight::BORDER))
                        .style(Style::default().bg(TokyoNight::BG))
                )
                .alignment(Alignment::Center);
                
            f.render_widget(stat_block, *chunk);
        }
    }
}

/// Render detailed analytics section
fn render_analytics_details(f: &mut Frame, area: Rect, app_state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);
    
    // Render most used servers
    render_most_used_servers(f, chunks[0], app_state);
    
    // Render connection insights
    render_connection_insights(f, chunks[1], app_state);
}

/// Render most used servers list
fn render_most_used_servers(f: &mut Frame, area: Rect, app_state: &AppState) {
    let mut servers: Vec<_> = app_state.server_manager.connections.values().collect();
    servers.sort_by(|a, b| b.stats.connection_count.cmp(&a.stats.connection_count));
    
    let items: Vec<ListItem> = servers.iter().take(10).enumerate().map(|(i, conn)| {
        let rank_color = match i {
            0 => TokyoNight::GREEN,
            1 => TokyoNight::BLUE,  
            2 => TokyoNight::ORANGE,
            _ => TokyoNight::COMMENT,
        };
        
        let content = vec![
            Line::from(vec![
                Span::styled(format!("{:2}.", i + 1), Style::default().fg(rank_color).add_modifier(Modifier::BOLD)),
                Span::raw(" "),
                Span::styled(&conn.name, Style::default().fg(TokyoNight::FG)),
                Span::raw(" "),
                Span::styled(format!("({})", conn.stats.connection_count), Style::default().fg(TokyoNight::CYAN)),
            ]),
            Line::from(vec![
                Span::raw("    "),
                Span::styled(conn.connection_string(), Style::default().fg(TokyoNight::COMMENT)),
                if let Some(latency) = conn.stats.latency {
                    Span::styled(format!(" • {}ms", latency.as_millis()), Style::default().fg(TokyoNight::GREEN))
                } else {
                    Span::raw("")
                },
            ]),
        ];
        
        ListItem::new(content)
    }).collect();
    
    let most_used = List::new(items)
        .block(
            Block::default()
                .title(" 📈 Most Used Servers ")
                .title_style(Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(TokyoNight::BORDER))
                .style(Style::default().bg(TokyoNight::BG))
        )
        .style(Style::default().fg(TokyoNight::FG));
        
    f.render_widget(most_used, area);
}

/// Render connection insights panel  
fn render_connection_insights(f: &mut Frame, area: Rect, app_state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
        
    // Server health distribution
    let (online, offline, connecting, warning) = app_state.server_manager.connections.values().fold(
        (0, 0, 0, 0),
        |(online, offline, connecting, warning), conn| {
            match conn.health_status {
                HealthStatus::Online => (online + 1, offline, connecting, warning),
                HealthStatus::Offline => (online, offline + 1, connecting, warning),
                HealthStatus::Connecting => (online, offline, connecting + 1, warning),
                HealthStatus::Warning => (online, offline, connecting, warning + 1),
                _ => (online, offline, connecting, warning),
            }
        }
    );
    
    let health_stats = vec![
        Line::from(Span::styled("📊 Server Health Distribution", 
            Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("●", Style::default().fg(TokyoNight::STATUS_ONLINE)),
            Span::styled(format!(" Online: {}", online), Style::default().fg(TokyoNight::FG)),
        ]),
        Line::from(vec![
            Span::styled("●", Style::default().fg(TokyoNight::STATUS_OFFLINE)),
            Span::styled(format!(" Offline: {}", offline), Style::default().fg(TokyoNight::FG)),
        ]),
        Line::from(vec![
            Span::styled("◐", Style::default().fg(TokyoNight::STATUS_CONNECTING)),
            Span::styled(format!(" Connecting: {}", connecting), Style::default().fg(TokyoNight::FG)),
        ]),
        Line::from(vec![
            Span::styled("▲", Style::default().fg(TokyoNight::STATUS_WARNING)),
            Span::styled(format!(" Warning: {}", warning), Style::default().fg(TokyoNight::FG)),
        ]),
    ];
    
    let health_panel = Paragraph::new(health_stats)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(TokyoNight::BORDER))
                .style(Style::default().bg(TokyoNight::BG))
        )
        .wrap(Wrap { trim: true });
        
    f.render_widget(health_panel, chunks[0]);
    
    // Connection history summary
    let recent_connections = app_state.server_manager.connection_history.len();
    let avg_latency = app_state.server_manager.connections.values()
        .filter_map(|c| c.stats.latency)
        .map(|l| l.as_millis() as f64)
        .collect::<Vec<_>>();
        
    let avg_latency_str = if !avg_latency.is_empty() {
        format!("{:.0}ms", avg_latency.iter().sum::<f64>() / avg_latency.len() as f64)
    } else {
        "N/A".to_string()
    };
    
    let insights_text = vec![
        Line::from(Span::styled("🔍 Connection Insights", 
            Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![
            Span::styled("Recent Connections: ", Style::default().fg(TokyoNight::COMMENT)),
            Span::styled(recent_connections.to_string(), Style::default().fg(TokyoNight::FG)),
        ]),
        Line::from(vec![
            Span::styled("Average Latency: ", Style::default().fg(TokyoNight::COMMENT)),
            Span::styled(avg_latency_str, Style::default().fg(TokyoNight::GREEN)),
        ]),
        Line::from(vec![
            Span::styled("Total Servers: ", Style::default().fg(TokyoNight::COMMENT)),
            Span::styled(app_state.server_manager.connection_count().to_string(), Style::default().fg(TokyoNight::FG)),
        ]),
    ];
    
    let insights_panel = Paragraph::new(insights_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(TokyoNight::BORDER))
                .style(Style::default().bg(TokyoNight::BG))
        )
        .wrap(Wrap { trim: true });
        
    f.render_widget(insights_panel, chunks[1]);
}

fn render_sessions_view(f: &mut Frame, area: Rect, app_state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),  // Summary header
            Constraint::Min(0),     // Main content
        ])
        .split(area);

    // Render session summary header
    render_session_summary_header(f, chunks[0], app_state);

    // Main content layout
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(chunks[1]);

    // Render session list
    render_session_list(f, main_chunks[0], app_state);
    
    // Render session details panel
    render_session_details(f, main_chunks[1], app_state);
}

fn render_session_list(f: &mut Frame, area: Rect, app_state: &AppState) {
    let sessions = app_state.get_filtered_sessions();
    
    let items: Vec<ListItem> = sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let is_selected = i == app_state.session_selected_index;
            let style = if is_selected {
                Style::default()
                    .bg(TokyoNight::BG_HIGHLIGHT)
                    .fg(TokyoNight::THEME_GREEN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(TokyoNight::FG)
            };

            let (status_color, status_symbol, status_text) = if session.is_idle {
                (TokyoNight::ORANGE, "💤", "IDLE")
            } else {
                (TokyoNight::STATUS_ONLINE, "⚡", "ACTIVE")
            };
            
            let formatted_duration = session.format_duration();
            let duration_color = get_duration_color(&formatted_duration);
            
            // Create a visual progress bar for long sessions
            let progress_bar = create_duration_progress_bar(session.duration());
            
            let content = vec![
                Line::from(vec![
                    // Status indicator
                    Span::styled(format!(" {} ", status_symbol), 
                        Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
                    // Server name
                    Span::styled(&session.server_name, 
                        if is_selected { 
                            Style::default().fg(TokyoNight::THEME_GREEN).add_modifier(Modifier::BOLD) 
                        } else { 
                            Style::default().fg(TokyoNight::FG).add_modifier(Modifier::BOLD) 
                        }),
                    // Status badge
                    Span::raw(" "),
                    Span::styled(format!("[{}]", status_text), 
                        Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    // PID with icon
                    Span::styled("🔸 ", Style::default().fg(TokyoNight::BLUE)),
                    Span::styled(format!("PID: {}", session.pid), 
                        Style::default().fg(TokyoNight::COMMENT)),
                    Span::raw(" │ "),
                    // Duration with color coding
                    Span::styled("⏱ ", Style::default().fg(TokyoNight::PURPLE)),
                    Span::styled(formatted_duration, 
                        Style::default().fg(duration_color).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    // Progress bar
                    Span::styled(progress_bar, Style::default().fg(TokyoNight::CYAN)),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    // Connection info
                    Span::styled("🔗 ", Style::default().fg(TokyoNight::CYAN)),
                    Span::styled(session.window_title.chars().take(40).collect::<String>(), 
                        Style::default().fg(TokyoNight::COMMENT)),
                    if session.window_title.len() > 40 { 
                        Span::styled("...", Style::default().fg(TokyoNight::COMMENT)) 
                    } else { 
                        Span::raw("") 
                    },
                ]),
            ];

            ListItem::new(content).style(style)
        })
        .collect();

    let title = format!(" 📋 Active SSH Sessions [{}] ", sessions.len());

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .title_style(Style::default().fg(TokyoNight::THEME_GREEN).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(TokyoNight::BORDER))
                .style(Style::default().bg(TokyoNight::BG)),
        )
        .style(Style::default().fg(TokyoNight::FG));

    f.render_widget(list, area);
}

fn render_session_summary_header(f: &mut Frame, area: Rect, app_state: &AppState) {
    let sessions = app_state.get_filtered_sessions();
    let (active_count, idle_count) = sessions.iter().fold((0, 0), |(active, idle), session| {
        if session.is_idle { (active, idle + 1) } else { (active + 1, idle) }
    });

    // Calculate total session time
    let total_duration: std::time::Duration = sessions.iter()
        .map(|s| s.duration())
        .sum();
    
    let total_duration_str = format_std_duration(total_duration);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(25), Constraint::Percentage(25), Constraint::Percentage(25)])
        .split(area);

    let stats = vec![
        ("📊 Total", sessions.len().to_string(), TokyoNight::CYAN),
        ("⚡ Active", active_count.to_string(), TokyoNight::STATUS_ONLINE),
        ("💤 Idle", idle_count.to_string(), TokyoNight::ORANGE),
        ("⏱ Total Time", total_duration_str, TokyoNight::PURPLE),
    ];

    for (i, (label, value, color)) in stats.iter().enumerate() {
        if let Some(chunk) = chunks.get(i) {
            let stat_text = vec![
                Line::from(vec![
                    Span::styled(format!("{} {}", label, value), 
                        Style::default().fg(*color).add_modifier(Modifier::BOLD)),
                ]),
            ];

            let stat_block = Paragraph::new(stat_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(TokyoNight::BORDER))
                        .style(Style::default().bg(TokyoNight::BG))
                )
                .alignment(Alignment::Center);

            f.render_widget(stat_block, *chunk);
        }
    }
}

fn render_session_details(f: &mut Frame, area: Rect, app_state: &AppState) {
    let sessions = app_state.get_filtered_sessions();
    
    if let Some(session) = sessions.get(app_state.session_selected_index) {
        let details = vec![
            Line::from(vec![
                Span::styled("Server: ", Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(&session.server_name, Style::default().fg(TokyoNight::FG)),
            ]),
            Line::from(vec![
                Span::styled("Window Title: ", Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(&session.window_title, Style::default().fg(TokyoNight::FG)),
            ]),
            Line::from(vec![
                Span::styled("PID: ", Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(session.pid.to_string(), Style::default().fg(TokyoNight::FG)),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(
                    if session.is_idle { "Idle" } else { "Active" },
                    Style::default().fg(if session.is_idle { TokyoNight::ORANGE } else { TokyoNight::STATUS_ONLINE })
                ),
            ]),
            Line::from(vec![
                Span::styled("Duration: ", Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(session.format_duration(), Style::default().fg(TokyoNight::FG)),
            ]),
            Line::from(vec![
                Span::styled("Started: ", Style::default().fg(TokyoNight::CYAN).add_modifier(Modifier::BOLD)),
                Span::styled(
                    session.started_at.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                    Style::default().fg(TokyoNight::FG)
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Controls:", Style::default().fg(TokyoNight::PURPLE).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("  d ", Style::default().fg(TokyoNight::THEME_GREEN).add_modifier(Modifier::BOLD)),
                Span::styled("Kill session", Style::default().fg(TokyoNight::FG)),
            ]),
            Line::from(vec![
                Span::styled("  Enter ", Style::default().fg(TokyoNight::THEME_GREEN).add_modifier(Modifier::BOLD)),
                Span::styled("Show session info", Style::default().fg(TokyoNight::FG)),
            ]),
            Line::from(vec![
                Span::styled("  r ", Style::default().fg(TokyoNight::THEME_GREEN).add_modifier(Modifier::BOLD)),
                Span::styled("Refresh sessions", Style::default().fg(TokyoNight::FG)),
            ]),
        ];

        let details_paragraph = Paragraph::new(details)
            .block(
                Block::default()
                    .title(" Session Details ")
                    .title_style(Style::default().fg(TokyoNight::THEME_GREEN).add_modifier(Modifier::BOLD))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(TokyoNight::BORDER))
                    .style(Style::default().bg(TokyoNight::BG)),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(details_paragraph, area);
    } else {
        let empty_message = vec![
            Line::from(vec![
                Span::styled("No active sessions", Style::default().fg(TokyoNight::COMMENT)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Connect to a server to start a session", Style::default().fg(TokyoNight::COMMENT)),
            ]),
        ];

        let empty_paragraph = Paragraph::new(empty_message)
            .block(
                Block::default()
                    .title(" Session Details ")
                    .title_style(Style::default().fg(TokyoNight::THEME_GREEN).add_modifier(Modifier::BOLD))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(TokyoNight::BORDER))
                    .style(Style::default().bg(TokyoNight::BG)),
            )
            .alignment(Alignment::Center);

        f.render_widget(empty_paragraph, area);
    }
}

/// Get color for duration based on how long the session has been running
fn get_duration_color(duration_str: &str) -> Color {
    if duration_str.contains('h') {
        // Long running sessions (hours) - red
        TokyoNight::RED
    } else if duration_str.contains('m') {
        let minutes: i32 = duration_str.split('m').next()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);
        if minutes > 30 {
            TokyoNight::ORANGE  // 30+ minutes - orange
        } else {
            TokyoNight::YELLOW  // Less than 30 minutes - yellow
        }
    } else {
        TokyoNight::GREEN  // Seconds only - green
    }
}

/// Create a visual progress bar for session duration
fn create_duration_progress_bar(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    
    // Scale: 0-5min = ▁, 5-15min = ▂, 15-30min = ▃, 30min-1h = ▄, 1h-2h = ▅, 2h+ = ▆
    let bar_char = match total_seconds {
        0..=300 => "▁",        // 0-5 minutes
        301..=900 => "▂",       // 5-15 minutes  
        901..=1800 => "▃",      // 15-30 minutes
        1801..=3600 => "▄",     // 30min-1hour
        3601..=7200 => "▅",     // 1-2 hours
        _ => "▆",               // 2+ hours
    };
    
    // Create a 5-character progress bar
    bar_char.repeat(5)
}

/// Format std::time::Duration for display
fn format_std_duration(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    
    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}






