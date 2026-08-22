use crate::models::{AppMode, AppState, AuthStrength, HealthStatus};
use crate::themes::Theme;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn ui(f: &mut Frame, app_state: &mut AppState) {
    let size = f.area();
    let theme = *app_state.theme_manager.current_theme();

    // Create main layout
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer
        ])
        .split(size);

    // Render header
    render_header(f, main_chunks[0], app_state);

    // Render content based on mode
    match app_state.mode.clone() {
        AppMode::Normal | AppMode::Search => render_main_view(f, main_chunks[1], app_state),
        AppMode::Help => render_help_popup(f, size, app_state),
        AppMode::History => render_history_popup(f, size, app_state),
        AppMode::Analytics => render_analytics_dashboard(f, main_chunks[1], app_state),
        AppMode::Sessions => render_sessions_view(f, main_chunks[1], app_state),
        AppMode::Topology => render_topology_view(f, main_chunks[1], app_state),
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
        AppMode::ThemeSelector => {
            render_main_view(f, main_chunks[1], app_state);
            render_theme_selector(f, size, app_state);
        }
        AppMode::ConfirmDiscard => {
            render_main_view(f, main_chunks[1], app_state);
            render_server_form_popup(f, size, app_state);
            render_confirm_discard_popup(f, size, theme);
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
    let theme = *app_state.theme_manager.current_theme();
    let header_text = vec![Line::from(vec![
        Span::styled("👻 ", Style::default().fg(theme.theme_primary)),
        Span::styled(
            "GHOST",
            Style::default()
                .fg(theme.theme_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" SSH Manager ", Style::default().fg(theme.fg)),
        Span::styled(app_state.get_globe_char(), Style::default().fg(theme.fg)),
        Span::styled(" ", Style::default()),
        Span::styled(
            format!("[{}]", get_status_line(app_state)),
            Style::default().fg(theme.cyan),
        ),
    ])];

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
    let theme = *app_state.theme_manager.current_theme();
    let selected = app_state.server_manager.selected_index;
    let globe = app_state.get_globe_char();
    let total = app_state.server_manager.connection_count();
    let filter = app_state.server_manager.filter.clone();
    let searching = app_state.mode == AppMode::Search;
    let connections = app_state.server_manager.filtered_connections();

    // Nothing to list: say why, and say what to do about it.
    if connections.is_empty() {
        render_empty_server_list(f, area, theme, total, &filter);
        return;
    }

    let items: Vec<ListItem> = connections
        .iter()
        .enumerate()
        .map(|(i, conn)| {
            let style = if i == selected {
                Style::default()
                    .bg(theme.bg_highlight)
                    .fg(theme.theme_primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };

            let health_color = get_health_color(&conn.health_status, theme);
            let auth_color = get_auth_color(&conn.auth_strength, theme);

            // Use spinning globe for connecting servers, otherwise use normal symbol
            let health_symbol = if matches!(conn.health_status, HealthStatus::Connecting) {
                globe
            } else {
                conn.health_status.symbol()
            };

            let session_indicator = if conn.has_active_sessions() {
                format!(" [{}]", conn.session_count())
            } else {
                String::new()
            };

            // Quick-connect numbers only mean anything for the first nine rows.
            let quick_num = if i < 9 {
                format!("{}:", i + 1)
            } else {
                "  ".to_string()
            };

            let content = vec![
                Line::from(vec![
                    Span::styled(quick_num, Style::default().fg(theme.comment)),
                    Span::styled(health_symbol, Style::default().fg(health_color)),
                    Span::raw(" "),
                    Span::styled(conn.auth_strength.symbol(), Style::default().fg(auth_color)),
                    Span::raw(" "),
                    Span::styled(&conn.name, style),
                    if conn.has_active_sessions() {
                        Span::styled(
                            session_indicator,
                            Style::default()
                                .fg(theme.green)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        Span::raw("")
                    },
                ]),
                Line::from(vec![
                    Span::raw("     "),
                    Span::styled(conn.connection_string(), Style::default().fg(theme.comment)),
                ]),
            ];

            ListItem::new(content).style(style)
        })
        .collect();

    let count = connections.len();
    let title = if filter.is_empty() {
        format!(" Servers [{}] ", total)
    } else {
        format!(" Servers [{}/{}] ", count, total)
    };

    // A live search gets a highlighted border so it's obvious the list is
    // filtered and where the keystrokes are going.
    let border_color = if searching {
        theme.theme_primary
    } else {
        theme.border
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .title_style(
                    Style::default()
                        .fg(theme.theme_primary)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(theme.bg)),
        )
        .style(Style::default().fg(theme.fg))
        .highlight_style(
            Style::default()
                .bg(theme.bg_highlight)
                .add_modifier(Modifier::BOLD),
        );

    // Stateful rendering is what makes the list scroll: ratatui keeps an offset
    // in ListState and moves it to keep the selection visible. Rendering
    // statelessly (the old behaviour) meant selecting row 40 of 60 highlighted
    // a row the user could not see.
    app_state.server_list_state.select(Some(selected));
    f.render_stateful_widget(list, area, &mut app_state.server_list_state);
}

/// Empty-state panel: distinguishes "no servers configured" from "your filter
/// matched nothing", and offers the relevant next action.
fn render_empty_server_list(f: &mut Frame, area: Rect, theme: Theme, total: usize, filter: &str) {
    let lines = if total == 0 {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "No servers configured yet",
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "  i  ",
                    Style::default()
                        .fg(theme.theme_primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Import every host from ~/.ssh/config",
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "  a  ",
                    Style::default()
                        .fg(theme.theme_primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Add a server by hand", Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled(
                    "  h  ",
                    Style::default()
                        .fg(theme.theme_primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Show all keybindings", Style::default().fg(theme.fg)),
            ]),
        ]
    } else if !filter.is_empty() {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("No servers match \"{}\"", filter),
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Esc clears the search",
                Style::default().fg(theme.comment),
            )),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "No servers are online",
                Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "'f' shows offline servers too · 'r' re-checks",
                Style::default().fg(theme.comment),
            )),
        ]
    };

    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .title(format!(" Servers [{}] ", total))
                .title_style(
                    Style::default()
                        .fg(theme.theme_primary)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg)),
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(widget, area);
}

fn render_metrics_panel(f: &mut Frame, area: Rect, app_state: &AppState) {
    let theme = *app_state.theme_manager.current_theme();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // System overview
    let total = app_state.server_manager.connection_count();
    let online = app_state.server_manager.online_count();
    let sessions = app_state.server_manager.active_session_count;

    let overview_text = vec![
        Line::from(vec![Span::styled(
            "📊 Overview",
            Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Servers: ", Style::default().fg(theme.comment)),
            Span::styled(
                format!("{}/{} online", online, total),
                Style::default().fg(if online == total {
                    theme.green
                } else {
                    theme.orange
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("Sessions: ", Style::default().fg(theme.comment)),
            Span::styled(
                sessions.to_string(),
                Style::default().fg(if sessions > 0 {
                    theme.green
                } else {
                    theme.comment
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("Layout: ", Style::default().fg(theme.comment)),
            Span::styled(
                format!("{:?}", app_state.layout.mode),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Panel Sizes:",
            Style::default()
                .fg(theme.purple)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            format!(
                "  [{}% | {}% | {}%]",
                app_state.layout.panel_sizes[0],
                app_state.layout.panel_sizes[1],
                app_state.layout.panel_sizes[2]
            ),
            Style::default().fg(theme.comment),
        )]),
    ];

    let overview = Paragraph::new(overview_text)
        .block(
            Block::default()
                .title(" System Metrics ")
                .title_style(
                    Style::default()
                        .fg(theme.theme_primary)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(overview, chunks[0]);

    // Quick stats
    let stats_text = vec![
        Line::from(vec![Span::styled(
            "⚡ Quick Stats",
            Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Theme: ", Style::default().fg(theme.comment)),
            Span::styled(
                app_state.theme_manager.current_variant().name(),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled("History: ", Style::default().fg(theme.comment)),
            Span::styled(
                format!(
                    "{} entries",
                    app_state.server_manager.connection_history.len()
                ),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled("Render: ", Style::default().fg(theme.comment)),
            Span::styled(
                app_state
                    .performance
                    .ui_render_time
                    .map(|d| format!("{:.1}ms", d.as_secs_f64() * 1000.0))
                    .unwrap_or_else(|| "—".to_string()),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Config: ",
            Style::default().fg(theme.comment),
        )]),
        Line::from(vec![Span::styled(
            app_state.config_path.clone(),
            Style::default().fg(theme.comment),
        )]),
    ];

    let stats = Paragraph::new(stats_text)
        .block(
            Block::default()
                .title(" Stats ")
                .title_style(
                    Style::default()
                        .fg(theme.theme_primary)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(stats, chunks[1]);
}

fn render_details_panel(f: &mut Frame, area: Rect, app_state: &AppState) {
    let theme = *app_state.theme_manager.current_theme();
    let connections = app_state.server_manager.filtered_connections();

    if let Some(connection) = connections.get(app_state.server_manager.selected_index) {
        let details = vec![
            Line::from(vec![
                Span::styled(
                    "Name: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&connection.name, Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![]),
            Line::from(vec![
                Span::styled(
                    "Host: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&connection.host, Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled(
                    "Port: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(connection.port.to_string(), Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled(
                    "User: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&connection.username, Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![]),
            Line::from(vec![
                Span::styled(
                    "Status: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    connection.health_status.symbol(),
                    Style::default().fg(get_health_color(&connection.health_status, theme)),
                ),
                Span::raw(" "),
                Span::styled(
                    connection.health_status.as_str(),
                    Style::default().fg(get_health_color(&connection.health_status, theme)),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Auth: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    connection.auth_strength.symbol(),
                    Style::default().fg(get_auth_color(&connection.auth_strength, theme)),
                ),
                Span::raw(" "),
                Span::styled(
                    connection.auth_strength.as_str(),
                    Style::default().fg(get_auth_color(&connection.auth_strength, theme)),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Timeout: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    match connection.timeout {
                        Some(t) => format!("{}s", t),
                        None => format!("{}s (default)", crate::ssh::DEFAULT_CONNECT_TIMEOUT_SECS),
                    },
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(vec![]),
            Line::from(vec![
                Span::styled(
                    "Created: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    connection.created_at.format("%Y-%m-%d %H:%M").to_string(),
                    Style::default().fg(theme.comment),
                ),
            ]),
            Line::from(vec![]),
            Line::from(vec![
                Span::styled(
                    "Latency: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                if let Some(latency) = connection.stats.latency {
                    Span::styled(
                        format!("{}ms", latency.as_millis()),
                        Style::default().fg(theme.green),
                    )
                } else {
                    Span::styled("N/A", Style::default().fg(theme.comment))
                },
                Span::raw(" "),
                Span::styled(
                    render_latency_sparkline(&connection.stats.latency_history),
                    Style::default().fg(theme.blue),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Uptime: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                if connection.stats.probe_success + connection.stats.probe_failure == 0 {
                    Span::styled("not yet probed", Style::default().fg(theme.comment))
                } else {
                    Span::styled(
                        format!(
                            "{:.0}% of {} check(s)",
                            connection.stats.uptime_percentage,
                            connection.stats.probe_success + connection.stats.probe_failure
                        ),
                        Style::default().fg(if connection.stats.uptime_percentage >= 99.0 {
                            theme.green
                        } else if connection.stats.uptime_percentage >= 90.0 {
                            theme.yellow
                        } else {
                            theme.orange
                        }),
                    )
                },
            ]),
            Line::from(vec![
                Span::styled(
                    "Sessions opened: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    connection.stats.connection_count.to_string(),
                    Style::default().fg(theme.fg),
                ),
                if connection.stats.failed_attempts > 0 {
                    Span::styled(
                        format!(" ({} failed)", connection.stats.failed_attempts),
                        Style::default().fg(theme.red),
                    )
                } else {
                    Span::raw("")
                },
            ]),
            Line::from(vec![
                Span::styled(
                    "Last connected: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    connection
                        .stats
                        .last_connected
                        .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| "never".to_string()),
                    Style::default().fg(theme.comment),
                ),
            ]),
            Line::from(vec![]),
            Line::from(vec![
                Span::styled(
                    "Sessions: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                if connection.has_active_sessions() {
                    Span::styled(
                        format!("{} active", connection.session_count()),
                        Style::default()
                            .fg(theme.green)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled("None", Style::default().fg(theme.comment))
                },
            ]),
        ];

        let mut details = details;

        // Surface the latest health-check error, if the last check failed.
        if let Some(err) = &connection.last_error {
            details.push(Line::from(vec![]));
            details.push(Line::from(vec![
                Span::styled(
                    "⚠ Error: ",
                    Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(err.clone(), Style::default().fg(theme.red)),
            ]));
        }

        // Add session details if any are active
        if connection.has_active_sessions() {
            details.push(Line::from(vec![]));
            for (i, session) in connection.active_sessions.iter().enumerate() {
                if i < 3 {
                    // Show max 3 sessions to avoid clutter
                    details.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(
                            format!("PID {}: ", session.pid),
                            Style::default().fg(theme.comment),
                        ),
                        Span::styled(
                            session.started_at.format("%H:%M:%S").to_string(),
                            Style::default().fg(theme.fg),
                        ),
                    ]));
                }
            }
            if connection.active_sessions.len() > 3 {
                details.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("... and {} more", connection.active_sessions.len() - 3),
                        Style::default().fg(theme.comment),
                    ),
                ]));
            }
        }

        let paragraph = Paragraph::new(details)
            .block(
                Block::default()
                    .title(" Details ")
                    .title_style(
                        Style::default()
                            .fg(theme.theme_primary)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border))
                    .style(Style::default().bg(theme.bg)),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, area);
    } else {
        let no_selection = Paragraph::new("No server selected")
            .block(
                Block::default()
                    .title(" Details ")
                    .title_style(
                        Style::default()
                            .fg(theme.theme_primary)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border))
                    .style(Style::default().bg(theme.bg)),
            )
            .style(Style::default().fg(theme.comment))
            .alignment(Alignment::Center);

        f.render_widget(no_selection, area);
    }
}

fn render_footer(f: &mut Frame, area: Rect, app_state: &AppState) {
    let theme = *app_state.theme_manager.current_theme();

    // While searching, the footer becomes the search prompt — the query has to
    // be visible somewhere, and this avoids stealing a row from the list.
    if app_state.mode == AppMode::Search {
        render_search_bar(f, area, app_state, theme);
        return;
    }

    let keybindings = match app_state.mode {
        // Kept short enough to fit an 80-column terminal on one line; `h`
        // opens the full, scrollable list.
        AppMode::Normal => {
            "j/k Move · Enter Connect · / Search · a Add · i Import · e Edit · d Delete · r Refresh · h Help · q Quit"
        }
        AppMode::Help => "j/k: Scroll · h, q, or Esc: Return",
        AppMode::History => "Press H, q, or Esc to return",
        AppMode::Analytics => "Press A, q, or Esc to return",
        AppMode::Sessions => "j/k: Move · d: Terminate · r: Refresh · Enter: Details · S/q/Esc: Return",
        AppMode::ConfirmDelete(_) => "y: Confirm · n: Cancel",
        AppMode::ConfirmDiscard => "y: Discard changes · n: Keep editing",
        AppMode::Connecting(_) => "Esc: Cancel connection",
        AppMode::AddServer | AppMode::EditServer(_) => {
            "Tab/↑↓: Move between fields · Enter: Save · Esc: Cancel"
        }
        AppMode::Loading(_) => "Esc: Continue in background",
        AppMode::ThemeSelector => "j/k: Preview · Enter: Keep · Esc: Cancel",
        AppMode::Topology => "j/k: Move · Enter: Connect · m/q/Esc: Return",
        AppMode::Search => "",
    };

    // A filter left active in Normal mode is easy to forget about; call it out.
    let content = if app_state.server_manager.filter.is_empty() {
        Line::from(Span::styled(
            keybindings,
            Style::default().fg(theme.comment),
        ))
    } else {
        Line::from(vec![
            Span::styled(
                format!(" filter: {} ", app_state.server_manager.filter),
                Style::default()
                    .bg(theme.theme_primary)
                    .fg(theme.bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  Esc clears · ", Style::default().fg(theme.comment)),
            Span::styled(keybindings, Style::default().fg(theme.comment)),
        ])
    };

    let footer = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg)),
        )
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Center);

    f.render_widget(footer, area);
}

/// The live search prompt, shown in place of the footer while in Search mode.
fn render_search_bar(f: &mut Frame, area: Rect, app_state: &AppState, theme: Theme) {
    let field = &app_state.search_input;
    let matches = app_state.server_manager.filtered_connections().len();
    let total = app_state.server_manager.connection_count();

    let value_style = if field.value.is_empty() {
        Style::default().fg(theme.comment)
    } else {
        Style::default().fg(theme.fg)
    };
    let shown = if field.value.is_empty() {
        field.placeholder.clone()
    } else {
        field.value.clone()
    };

    let line = Line::from(vec![
        Span::styled(
            "/ ",
            Style::default()
                .fg(theme.theme_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(shown, value_style),
    ]);

    let title = format!(" Search — {}/{} match ", matches, total);
    let widget = Paragraph::new(line).block(
        Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(theme.theme_primary)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.theme_primary))
            .style(Style::default().bg(theme.bg)),
    );
    f.render_widget(widget, area);

    // Place the caret after the "/ " prefix, in display columns.
    if area.width > 4 {
        let x = area.x + 3 + field.cursor_display_column() as u16;
        f.set_cursor_position((x.min(area.x + area.width - 2), area.y + 1));
    }
}

fn render_help_popup(f: &mut Frame, area: Rect, app_state: &AppState) {
    let theme = *app_state.theme_manager.current_theme();
    let popup_area = centered_rect(72, 92, area);

    let heading = |t: &'static str| {
        Line::from(Span::styled(
            t,
            Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
        ))
    };

    let help_text = vec![
        Line::from(Span::styled(
            "👻 Ghost — SSH Connection Manager",
            Style::default()
                .fg(theme.theme_primary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        heading("NAVIGATION"),
        Line::from("  j/k or ↑/↓      Move through the server list"),
        Line::from("  g / G           Jump to first / last server"),
        Line::from("  PgUp / PgDn     Move ten at a time"),
        Line::from("  Enter           Connect to the selected server"),
        Line::from("  1-9             Quick connect by position"),
        Line::from("  /               Search by name, host, user, tag"),
        Line::from(""),
        heading("SERVERS"),
        Line::from("  a               Add a server"),
        Line::from("  i               Import hosts from ~/.ssh/config"),
        Line::from("  e               Edit the selected server"),
        Line::from("  d               Delete the selected server"),
        Line::from("  r               Re-check reachability of every server"),
        Line::from("  f               Toggle online-only filter"),
        Line::from(""),
        heading("VIEWS"),
        Line::from("  m               Topology — hosts grouped by bastion"),
        Line::from("  S               Active SSH sessions"),
        Line::from("  A               Analytics"),
        Line::from("  H               Connection history"),
        Line::from("  l               Cycle layout (one / two / three panels)"),
        Line::from("  [ / ]           Resize panels"),
        Line::from("  T               Next theme    ·    t  Theme selector"),
        Line::from(""),
        heading("SESSIONS"),
        Line::from("  Ctrl+X          Terminate all tracked sessions"),
        Line::from("  In S view: d terminates the selected session"),
        Line::from(""),
        heading("STATUS COLUMN"),
        Line::from("  ● online   ● offline   ◐ checking   ? not yet checked"),
        Line::from("  🔑 key or agent auth    ⚠ password auth    💬 interactive"),
        Line::from(Span::styled(
            "  Auth icons reflect your local config, not the remote host's posture.",
            Style::default().fg(theme.comment),
        )),
        Line::from(""),
        heading("COMMAND LINE"),
        Line::from("  --new-terminal          Always open a new terminal window"),
        Line::from("  --direct                Always connect in this terminal"),
        Line::from("  --connection-mode M     auto | new-terminal | direct"),
        Line::from("  --import-ssh-config     Import ~/.ssh/config and exit"),
        Line::from("  --import-ssh-config --dry-run   Preview the import"),
        Line::from(""),
        heading("GENERAL"),
        Line::from("  ?               Contextual tip     ·    F2  Toggle tips"),
        Line::from("  q / Ctrl+C      Quit               ·    Esc Clear search or quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Press h, q, or Esc to return",
            Style::default().fg(theme.cyan),
        )),
    ];

    f.render_widget(Clear, popup_area);
    let total_lines = help_text.len();
    let visible = popup_area.height.saturating_sub(2) as usize;
    let max_scroll = total_lines.saturating_sub(visible) as u16;
    let scroll = app_state.help_scroll.min(max_scroll);

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .title(" Help ")
                .title_style(
                    Style::default()
                        .fg(theme.theme_primary)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border_highlight))
                .style(Style::default().bg(theme.bg_popup)),
        )
        .style(Style::default().fg(theme.fg))
        // `trim: true` strips the leading spaces that align the key column,
        // which collapsed the help table into a ragged left edge.
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));

    f.render_widget(help, popup_area);

    // The help text is taller than most terminals; show the reader where they
    // are rather than silently cutting it off.
    if max_scroll > 0 {
        let hint = format!(
            " {}–{} of {}  (j/k to scroll) ",
            scroll as usize + 1,
            (scroll as usize + visible).min(total_lines),
            total_lines
        );
        let width = (hint.chars().count() as u16).min(popup_area.width.saturating_sub(2));
        f.render_widget(
            Paragraph::new(hint).style(Style::default().fg(theme.comment)),
            Rect::new(
                popup_area.x + popup_area.width.saturating_sub(width + 1),
                popup_area.y + popup_area.height - 1,
                width,
                1,
            ),
        );
    }
}

fn render_history_popup(f: &mut Frame, area: Rect, app_state: &AppState) {
    let theme = *app_state.theme_manager.current_theme();
    let popup_area = centered_rect(80, 70, area);

    let history_items: Vec<ListItem> = app_state
        .server_manager
        .connection_history
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let time_str = entry.connected_at.format("%Y-%m-%d %H:%M:%S").to_string();
            let content = vec![
                Line::from(vec![
                    Span::styled(format!("{}. ", i + 1), Style::default().fg(theme.comment)),
                    Span::styled(
                        &entry.server_name,
                        Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(time_str, Style::default().fg(theme.comment)),
                ]),
            ];
            ListItem::new(content)
        })
        .collect();

    let history_text = if history_items.is_empty() {
        vec![Line::from(Span::styled(
            "No connection history yet. Connect to servers to see history here.",
            Style::default().fg(theme.comment),
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
                    .title_style(Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border_highlight))
                    .style(Style::default().bg(theme.bg_popup)),
            )
            .style(Style::default().fg(theme.fg))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(history, popup_area);
    } else {
        let history_list = List::new(history_items)
            .block(
                Block::default()
                    .title(format!(
                        " Connection History ({}) ",
                        app_state.server_manager.connection_history.len()
                    ))
                    .title_style(Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border_highlight))
                    .style(Style::default().bg(theme.bg_popup)),
            )
            .style(Style::default().fg(theme.fg));
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
        .style(Style::default().fg(theme.comment))
        .alignment(Alignment::Center);
    f.render_widget(instructions, instruction_area);
}

fn render_confirm_delete_popup(f: &mut Frame, area: Rect, app_state: &AppState, server_id: &str) {
    let theme = *app_state.theme_manager.current_theme();
    let popup_area = centered_rect(50, 20, area);

    let server_name = app_state
        .server_manager
        .get_connection(server_id)
        .map(|c| c.name.as_str())
        .unwrap_or("Unknown");

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "⚠️  WARNING",
            Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("Delete server \""),
            Span::styled(server_name, Style::default().fg(theme.cyan)),
            Span::raw("\"?"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "y: Yes | n: No",
            Style::default().fg(theme.comment),
        )),
    ];

    f.render_widget(Clear, popup_area);
    let confirm = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Confirm Delete ")
                .title_style(Style::default().fg(theme.red).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.red))
                .style(Style::default().bg(theme.bg_popup)),
        )
        .style(Style::default().fg(theme.fg))
        .alignment(Alignment::Center);

    f.render_widget(confirm, popup_area);
}

fn render_confirm_discard_popup(f: &mut Frame, area: Rect, theme: Theme) {
    let popup_area = centered_rect(50, 20, area);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "⚠️  Unsaved changes",
            Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Discard your changes?"),
        Line::from(""),
        Line::from(Span::styled(
            "y: Discard | n: Keep editing",
            Style::default().fg(theme.comment),
        )),
    ];

    f.render_widget(Clear, popup_area);
    let confirm = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Discard Changes ")
                .title_style(Style::default().fg(theme.red).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.red))
                .style(Style::default().bg(theme.bg_popup)),
        )
        .style(Style::default().fg(theme.fg))
        .alignment(Alignment::Center);

    f.render_widget(confirm, popup_area);
}

fn render_connecting_popup(f: &mut Frame, area: Rect, app_state: &AppState, server_id: &str) {
    let theme = *app_state.theme_manager.current_theme();
    let popup_area = centered_rect(40, 15, area);

    let server_name = app_state
        .server_manager
        .get_connection(server_id)
        .map(|c| c.name.as_str())
        .unwrap_or("Unknown");

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(app_state.get_globe_char(), Style::default().fg(theme.fg)),
            Span::raw(" → Connecting to "),
            Span::styled(server_name, Style::default().fg(theme.cyan)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press Esc to cancel",
            Style::default().fg(theme.comment),
        )),
    ];

    f.render_widget(Clear, popup_area);
    let connecting = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Connecting... ")
                .title_style(Style::default().fg(theme.blue).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.blue))
                .style(Style::default().bg(theme.bg_popup)),
        )
        .style(Style::default().fg(theme.fg))
        .alignment(Alignment::Center);

    f.render_widget(connecting, popup_area);
}

fn render_loading_popup(
    f: &mut Frame,
    area: Rect,
    app_state: &AppState,
    context: &crate::models::LoadingContext,
) {
    use crate::models::LoadingContext;
    let theme = *app_state.theme_manager.current_theme();

    let popup_area = centered_rect(50, 18, area);

    let (title, status_text, progress_info) = match context {
        LoadingContext::RefreshingHealth { completed, total } => {
            let progress = if *total > 0 {
                *completed as f32 / *total as f32
            } else {
                0.0
            };
            let progress_bar = create_progress_bar(progress, 30);

            (
                "🔄 Refreshing Health",
                "Checking server status...".to_string(),
                format!("{}\n{}/{} servers checked", progress_bar, completed, total),
            )
        }
    };

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                app_state.get_globe_char(),
                Style::default().fg(theme.theme_primary),
            ),
            Span::raw(" "),
            Span::styled(status_text, Style::default().fg(theme.fg)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            progress_info,
            Style::default().fg(theme.comment),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press Esc to cancel",
            Style::default()
                .fg(theme.comment)
                .add_modifier(Modifier::ITALIC),
        )),
    ];

    f.render_widget(Clear, popup_area);
    let loading_popup = Paragraph::new(text)
        .block(
            Block::default()
                .title(format!(" {} ", title))
                .title_style(
                    Style::default()
                        .fg(theme.theme_primary)
                        .add_modifier(Modifier::BOLD),
                )
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
    let theme = *app_state.theme_manager.current_theme();
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
                Constraint::Length(2), // Instructions
                Constraint::Min(0),    // Form fields
                Constraint::Length(3), // Action buttons
            ])
            .split(popup_area);

        // Render instructions
        let instructions = Paragraph::new("Tab/Shift+Tab: Navigate | Enter: Save | Esc: Cancel")
            .style(Style::default().fg(theme.comment))
            .alignment(Alignment::Center);
        f.render_widget(instructions, form_chunks[0]);

        // Render form fields
        render_form_fields(f, form_chunks[1], form, theme);

        // Render action buttons
        let actions = vec![Line::from(vec![
            Span::styled(
                "[Enter]",
                Style::default()
                    .fg(theme.green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Save   "),
            Span::styled(
                "[Esc]",
                Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Cancel"),
        ])];
        let action_bar = Paragraph::new(actions)
            .style(Style::default().fg(theme.fg))
            .alignment(Alignment::Center);
        f.render_widget(action_bar, form_chunks[2]);

        // Render the main popup block
        let popup_block = Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(theme.theme_primary)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_highlight))
            .style(Style::default().bg(theme.bg_popup));
        f.render_widget(popup_block, popup_area);
    }
}

/// Render the add/edit form, scrolling so the focused row stays visible.
///
/// The form is taller than the popup on a standard 80x24 terminal. Laying every
/// row out unconditionally (the old behaviour) let ratatui silently squash the
/// overflow to zero height, so the last fields — including the auth selector —
/// simply weren't drawn and couldn't be reached.
fn render_form_fields(f: &mut Frame, area: Rect, form: &crate::forms::ServerForm, theme: Theme) {
    const FIELD_HEIGHT: u16 = 3;
    const AUTH_HEIGHT: u16 = 4;

    // Rows in focus order: the text fields, then tags, then the auth selector.
    // Index i < fields.len() is a field; i == fields.len() is tags.
    let row_count = form.fields.len() + 2;
    let auth_row = row_count - 1;
    let tags_row = form.fields.len();

    let row_height = |row: usize| {
        if row == auth_row {
            AUTH_HEIGHT
        } else {
            FIELD_HEIGHT
        }
    };

    let focused_row = if form.auth_method_focused {
        auth_row
    } else {
        form.current_field.min(tags_row)
    };

    // Total height if everything were shown at once.
    let full_height: u16 = (0..row_count).map(row_height).sum();
    let scrolling = full_height > area.height;

    // When scrolling, reserve a row top and bottom for the "more" markers so
    // they don't paint over a field's border.
    let (marker_top, fields_area, marker_bottom) = if scrolling && area.height > 2 {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);
        (Some(rows[0]), rows[1], Some(rows[2]))
    } else {
        (None, area, None)
    };
    let area = fields_area;

    // Walk back from the focused row until the visible window is full, so the
    // focused row is always on screen and we show as much context as fits.
    let mut first_visible = focused_row;
    let mut used = row_height(focused_row);
    while first_visible > 0 {
        let candidate = row_height(first_visible - 1);
        if used + candidate > area.height {
            break;
        }
        used += candidate;
        first_visible -= 1;
    }

    // Then extend forward with whatever room is left.
    let mut last_visible = focused_row;
    while last_visible + 1 < row_count {
        let candidate = row_height(last_visible + 1);
        if used + candidate > area.height {
            break;
        }
        used += candidate;
        last_visible += 1;
    }

    let visible: Vec<usize> = (first_visible..=last_visible).collect();
    if visible.is_empty() {
        return;
    }

    let constraints: Vec<Constraint> = visible
        .iter()
        .map(|&row| Constraint::Length(row_height(row)))
        .collect();

    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (slot, &row) in visible.iter().enumerate() {
        let Some(&row_area) = areas.get(slot) else {
            continue;
        };
        if row == auth_row {
            render_auth_method_field(f, row_area, form, theme);
        } else if row == tags_row {
            render_input_field(
                f,
                row_area,
                &form.tags_input,
                form.current_field == tags_row && !form.auth_method_focused,
                theme,
            );
        } else {
            render_input_field(
                f,
                row_area,
                &form.fields[row],
                form.current_field == row && !form.auth_method_focused,
                theme,
            );
        }
    }

    // Tell the user there is more above or below the window.
    let marker = |f: &mut Frame, rect: Rect, text: &'static str| {
        f.render_widget(
            Paragraph::new(text)
                .style(Style::default().fg(theme.comment))
                .alignment(Alignment::Center),
            rect,
        );
    };
    if let Some(rect) = marker_top {
        if first_visible > 0 {
            marker(f, rect, "▲ more above");
        }
    }
    if let Some(rect) = marker_bottom {
        if last_visible + 1 < row_count {
            marker(f, rect, "▼ more below");
        }
    }
}

fn render_input_field(
    f: &mut Frame,
    area: Rect,
    field: &crate::forms::InputField,
    is_focused: bool,
    theme: Theme,
) {
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
        Style::default()
            .bg(theme.bg_highlight)
            .fg(theme.theme_primary)
    } else {
        Style::default().bg(theme.bg).fg(if field.value.is_empty() {
            theme.comment
        } else {
            theme.fg
        })
    };

    let border_style = if is_focused {
        Style::default().fg(theme.theme_primary)
    } else {
        Style::default().fg(theme.border)
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
                .border_style(border_style),
        )
        .style(input_style);
    f.render_widget(input, area);

    // Render cursor if focused. Uses the display *column* (wide glyphs occupy
    // two cells), not a raw char index, so the caret tracks the text.
    if is_focused && area.width > 2 {
        let cursor_x = area.x + 1 + field.cursor_display_column() as u16;
        let cursor_y = area.y + 1;
        f.set_cursor_position((cursor_x.min(area.x + area.width - 2), cursor_y));
    }
}

fn render_auth_method_field(
    f: &mut Frame,
    area: Rect,
    form: &crate::forms::ServerForm,
    theme: Theme,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(2)])
        .split(area);

    // Render label
    let label_style = if form.auth_method_focused {
        Style::default()
            .fg(theme.theme_primary)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.cyan)
    };
    let label = Paragraph::new("Authentication:").style(label_style);
    f.render_widget(label, chunks[0]);

    // Render dropdown
    let dropdown_style = if form.auth_method_focused {
        Style::default().bg(theme.bg_highlight).fg(theme.fg)
    } else {
        Style::default().bg(theme.bg).fg(theme.fg)
    };

    let border_style = if form.auth_method_focused {
        Style::default().fg(theme.theme_primary)
    } else {
        Style::default().fg(theme.border)
    };

    let auth_text = vec![
        Line::from(vec![
            Span::styled("▼ ", Style::default().fg(theme.theme_primary)),
            Span::styled(form.auth_method.display_name(), dropdown_style),
        ]),
        Line::from(Span::styled(
            form.auth_method.description(),
            Style::default().fg(theme.comment),
        )),
    ];

    let dropdown = Paragraph::new(auth_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .style(dropdown_style);
    f.render_widget(dropdown, chunks[1]);
}

fn render_tooltip(
    f: &mut Frame,
    area: Rect,
    app_state: &AppState,
    tooltip: &crate::models::TooltipInfo,
) {
    use crate::models::TooltipCategory;
    use chrono::Utc;
    let theme = *app_state.theme_manager.current_theme();

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
            Span::styled(
                &tooltip.title,
                Style::default()
                    .fg(title_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            &tooltip.description,
            Style::default().fg(theme.fg),
        )),
    ];

    if let Some(ref key_hint) = tooltip.key_hint {
        content.push(Line::from(""));
        content.push(Line::from(vec![
            Span::styled("💡 ", Style::default().fg(theme.yellow)),
            Span::styled(
                key_hint,
                Style::default()
                    .fg(theme.comment)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));
    }

    content.push(Line::from(""));
    content.push(Line::from(vec![Span::styled(
        format!("Auto-dismiss in {}s | Esc: Close", remaining_time),
        Style::default()
            .fg(theme.comment)
            .add_modifier(Modifier::ITALIC),
    )]));

    f.render_widget(Clear, tooltip_area);
    let tooltip_widget = Paragraph::new(content)
        .block(
            Block::default()
                .title(" 💬 Tooltip ")
                .title_style(
                    Style::default()
                        .fg(title_color)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(theme.bg)),
        )
        .style(Style::default().fg(theme.fg))
        .wrap(Wrap { trim: true });

    f.render_widget(tooltip_widget, tooltip_area);
}

fn render_message_popup(f: &mut Frame, area: Rect, app_state: &AppState) {
    let theme = *app_state.theme_manager.current_theme();
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
        format!(
            "{}\n\n[Auto-dismiss in {}s | Press Enter/Esc to close]",
            app_state.popup_message, remaining_time
        )
    } else {
        app_state.popup_message.clone()
    };

    f.render_widget(Clear, popup_area);
    let popup = Paragraph::new(message_with_time)
        .block(
            Block::default()
                .title(" Info ")
                .title_style(Style::default().fg(theme.cyan))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border_highlight))
                .style(Style::default().bg(theme.bg_popup)),
        )
        .style(Style::default().fg(theme.fg))
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
    let connecting_count = app_state
        .server_manager
        .connections
        .values()
        .filter(|conn| matches!(conn.health_status, HealthStatus::Connecting))
        .count();

    let mut status_parts = Vec::new();
    status_parts.push(format!("{}/{} online", online, total));

    if sessions > 0 {
        status_parts.push(format!("{} sessions", sessions));
    }

    if connecting_count > 0 {
        status_parts.push(format!(
            "{} {} connecting",
            app_state.get_globe_char(),
            connecting_count
        ));
    }

    status_parts.join(" | ")
}

fn get_health_color(status: &HealthStatus, theme: Theme) -> Color {
    match status {
        HealthStatus::Online => theme.status_online,
        HealthStatus::Offline => theme.status_offline,
        HealthStatus::Connecting => theme.status_connecting,
        HealthStatus::Warning => theme.status_warning,
        HealthStatus::Unknown => theme.status_unknown,
    }
}

fn get_auth_color(status: &AuthStrength, theme: Theme) -> Color {
    match status {
        AuthStrength::Key | AuthStrength::Agent => theme.green,
        AuthStrength::Password => theme.orange,
        AuthStrength::Interactive => theme.comment,
        AuthStrength::Unknown => theme.comment,
    }
}

/// Render a mini sparkline for latency history
/// Render recent latency samples as a sparkline, oldest on the left.
///
/// Two bugs lived here: an empty history returned a flat five-bar sparkline
/// (implying five zero-latency samples that were never taken), and a stray
/// double `.rev()` drew the series backwards, newest-first.
fn render_latency_sparkline(history: &[u32]) -> String {
    const WIDTH: usize = 8;
    const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    if history.is_empty() {
        return String::new();
    }

    let recent: Vec<u32> = history.iter().rev().take(WIDTH).rev().copied().collect();
    let max = *recent.iter().max().unwrap_or(&1);
    // Scale against at least 50ms so a stable, healthy link doesn't render as
    // eight full-height bars just because every sample equals the maximum.
    let scale = max.max(50) as f32;

    recent
        .iter()
        .map(|&latency| {
            let idx = (latency as f32 / scale * (BARS.len() - 1) as f32).round() as usize;
            BARS[idx.min(BARS.len() - 1)]
        })
        .collect()
}

fn render_analytics_dashboard(f: &mut Frame, area: Rect, app_state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Overview stats
            Constraint::Min(0),    // Detailed analytics
        ])
        .split(area);

    // Render overview statistics
    render_analytics_overview(f, chunks[0], app_state);

    // Render detailed analytics
    render_analytics_details(f, chunks[1], app_state);
}

/// Render analytics overview section
fn render_analytics_overview(f: &mut Frame, area: Rect, app_state: &AppState) {
    let theme = *app_state.theme_manager.current_theme();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    // Total connections
    let total_connections = app_state
        .server_manager
        .connections
        .values()
        .map(|c| c.stats.connection_count)
        .sum::<u32>();

    let total_failures = app_state
        .server_manager
        .connections
        .values()
        .map(|c| c.stats.failed_attempts)
        .sum::<u32>();

    // Success rate over *attempts*, not connections. The old form computed
    // `total_connections - total_failures` on independent u32 counters, which
    // underflowed (panicking in debug, wrapping in release) as soon as a server
    // had more failed launches than successful ones.
    let total_attempts = total_connections + total_failures;
    let success_rate = if total_attempts > 0 {
        total_connections as f32 / total_attempts as f32 * 100.0
    } else {
        0.0
    };

    // Render stat boxes
    let stats = [
        (
            "Total Connections",
            total_connections.to_string(),
            theme.cyan,
        ),
        ("Success Rate", format!("{:.1}%", success_rate), theme.green),
        (
            "Active Sessions",
            app_state.server_manager.active_session_count.to_string(),
            theme.blue,
        ),
        (
            "Online Servers",
            format!(
                "{}/{}",
                app_state.server_manager.online_count(),
                app_state.server_manager.connection_count()
            ),
            theme.theme_primary,
        ),
    ];

    for (i, (label, value, color)) in stats.iter().enumerate() {
        if let Some(chunk) = chunks.get(i) {
            let stat_text = vec![
                Line::from(Span::styled(
                    value,
                    Style::default().fg(*color).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(*label, Style::default().fg(theme.comment))),
            ];

            let stat_block = Paragraph::new(stat_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme.border))
                        .style(Style::default().bg(theme.bg)),
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
    let theme = *app_state.theme_manager.current_theme();
    let mut servers: Vec<_> = app_state.server_manager.connections.values().collect();
    servers.sort_by_key(|s| std::cmp::Reverse(s.stats.connection_count));

    let items: Vec<ListItem> = servers
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, conn)| {
            let rank_color = match i {
                0 => theme.green,
                1 => theme.blue,
                2 => theme.orange,
                _ => theme.comment,
            };

            let content = vec![
                Line::from(vec![
                    Span::styled(
                        format!("{:2}.", i + 1),
                        Style::default().fg(rank_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(&conn.name, Style::default().fg(theme.fg)),
                    Span::raw(" "),
                    Span::styled(
                        format!("({})", conn.stats.connection_count),
                        Style::default().fg(theme.cyan),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(conn.connection_string(), Style::default().fg(theme.comment)),
                    if let Some(latency) = conn.stats.latency {
                        Span::styled(
                            format!(" • {}ms", latency.as_millis()),
                            Style::default().fg(theme.green),
                        )
                    } else {
                        Span::raw("")
                    },
                ]),
            ];

            ListItem::new(content)
        })
        .collect();

    let most_used = List::new(items)
        .block(
            Block::default()
                .title(" 📈 Most Used Servers ")
                .title_style(Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg)),
        )
        .style(Style::default().fg(theme.fg));

    f.render_widget(most_used, area);
}

/// Render connection insights panel  
fn render_connection_insights(f: &mut Frame, area: Rect, app_state: &AppState) {
    let theme = *app_state.theme_manager.current_theme();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Server health distribution
    let (online, offline, connecting, warning) =
        app_state.server_manager.connections.values().fold(
            (0, 0, 0, 0),
            |(online, offline, connecting, warning), conn| match conn.health_status {
                HealthStatus::Online => (online + 1, offline, connecting, warning),
                HealthStatus::Offline => (online, offline + 1, connecting, warning),
                HealthStatus::Connecting => (online, offline, connecting + 1, warning),
                HealthStatus::Warning => (online, offline, connecting, warning + 1),
                _ => (online, offline, connecting, warning),
            },
        );

    let health_stats = vec![
        Line::from(Span::styled(
            "📊 Server Health Distribution",
            Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("●", Style::default().fg(theme.status_online)),
            Span::styled(
                format!(" Online: {}", online),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled("●", Style::default().fg(theme.status_offline)),
            Span::styled(
                format!(" Offline: {}", offline),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled("◐", Style::default().fg(theme.status_connecting)),
            Span::styled(
                format!(" Connecting: {}", connecting),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled("▲", Style::default().fg(theme.status_warning)),
            Span::styled(
                format!(" Warning: {}", warning),
                Style::default().fg(theme.fg),
            ),
        ]),
    ];

    let health_panel = Paragraph::new(health_stats)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(health_panel, chunks[0]);

    // Connection history summary
    let recent_connections = app_state.server_manager.connection_history.len();
    let avg_latency = app_state
        .server_manager
        .connections
        .values()
        .filter_map(|c| c.stats.latency)
        .map(|l| l.as_millis() as f64)
        .collect::<Vec<_>>();

    let avg_latency_str = if !avg_latency.is_empty() {
        format!(
            "{:.0}ms",
            avg_latency.iter().sum::<f64>() / avg_latency.len() as f64
        )
    } else {
        "N/A".to_string()
    };

    let insights_text = vec![
        Line::from(Span::styled(
            "🔍 Connection Insights",
            Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Recent Connections: ", Style::default().fg(theme.comment)),
            Span::styled(
                recent_connections.to_string(),
                Style::default().fg(theme.fg),
            ),
        ]),
        Line::from(vec![
            Span::styled("Average Latency: ", Style::default().fg(theme.comment)),
            Span::styled(avg_latency_str, Style::default().fg(theme.green)),
        ]),
        Line::from(vec![
            Span::styled("Total Servers: ", Style::default().fg(theme.comment)),
            Span::styled(
                app_state.server_manager.connection_count().to_string(),
                Style::default().fg(theme.fg),
            ),
        ]),
    ];

    let insights_panel = Paragraph::new(insights_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(insights_panel, chunks[1]);
}

fn render_sessions_view(f: &mut Frame, area: Rect, app_state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Summary header
            Constraint::Min(0),    // Main content
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

fn render_session_list(f: &mut Frame, area: Rect, app_state: &mut AppState) {
    let theme = *app_state.theme_manager.current_theme();
    let sessions = app_state.get_filtered_sessions();

    let items: Vec<ListItem> = sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let is_selected = i == app_state.session_selected_index;
            let style = if is_selected {
                Style::default()
                    .bg(theme.bg_highlight)
                    .fg(theme.theme_primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };

            let (status_color, status_symbol, status_text) = if session.is_idle {
                (theme.orange, "💤", "IDLE")
            } else {
                (theme.status_online, "⚡", "ACTIVE")
            };

            let formatted_duration = session.format_duration();
            let duration_color = get_duration_color(&formatted_duration, theme);

            // Create a visual progress bar for long sessions
            let progress_bar = create_duration_progress_bar(session.duration());

            let content = vec![
                Line::from(vec![
                    // Status indicator
                    Span::styled(
                        format!(" {} ", status_symbol),
                        Style::default()
                            .fg(status_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    // Server name
                    Span::styled(
                        session.server_name.clone(),
                        if is_selected {
                            Style::default()
                                .fg(theme.theme_primary)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)
                        },
                    ),
                    // Status badge
                    Span::raw(" "),
                    Span::styled(
                        format!("[{}]", status_text),
                        Style::default()
                            .fg(status_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    // PID with icon
                    Span::styled("🔸 ", Style::default().fg(theme.blue)),
                    Span::styled(
                        format!("PID: {}", session.pid),
                        Style::default().fg(theme.comment),
                    ),
                    Span::raw(" │ "),
                    // Duration with color coding
                    Span::styled("⏱ ", Style::default().fg(theme.purple)),
                    Span::styled(
                        formatted_duration,
                        Style::default()
                            .fg(duration_color)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    // Progress bar
                    Span::styled(progress_bar, Style::default().fg(theme.cyan)),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    // Connection info
                    Span::styled("🔗 ", Style::default().fg(theme.cyan)),
                    Span::styled(
                        session.window_title.chars().take(40).collect::<String>(),
                        Style::default().fg(theme.comment),
                    ),
                    if session.window_title.len() > 40 {
                        Span::styled("...", Style::default().fg(theme.comment))
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
                .title_style(
                    Style::default()
                        .fg(theme.theme_primary)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg)),
        )
        .style(Style::default().fg(theme.fg));

    // Stateful so long session lists scroll with the selection. The items are
    // built already, so release the borrow on `sessions` before touching state.
    let has_sessions = !sessions.is_empty();
    drop(sessions);
    let selected = app_state.session_selected_index;
    app_state
        .session_list_state
        .select(has_sessions.then_some(selected));
    f.render_stateful_widget(list, area, &mut app_state.session_list_state);
}

fn render_session_summary_header(f: &mut Frame, area: Rect, app_state: &AppState) {
    let theme = *app_state.theme_manager.current_theme();
    let sessions = app_state.get_filtered_sessions();
    let (active_count, idle_count) = sessions.iter().fold((0, 0), |(active, idle), session| {
        if session.is_idle {
            (active, idle + 1)
        } else {
            (active + 1, idle)
        }
    });

    // Calculate total session time
    let total_duration: std::time::Duration = sessions.iter().map(|s| s.duration()).sum();

    let total_duration_str = format_std_duration(total_duration);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(area);

    let stats = [
        ("📊 Total", sessions.len().to_string(), theme.cyan),
        ("⚡ Active", active_count.to_string(), theme.status_online),
        ("💤 Idle", idle_count.to_string(), theme.orange),
        ("⏱ Total Time", total_duration_str, theme.purple),
    ];

    for (i, (label, value, color)) in stats.iter().enumerate() {
        if let Some(chunk) = chunks.get(i) {
            let stat_text = vec![Line::from(vec![Span::styled(
                format!("{} {}", label, value),
                Style::default().fg(*color).add_modifier(Modifier::BOLD),
            )])];

            let stat_block = Paragraph::new(stat_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme.border))
                        .style(Style::default().bg(theme.bg)),
                )
                .alignment(Alignment::Center);

            f.render_widget(stat_block, *chunk);
        }
    }
}

fn render_session_details(f: &mut Frame, area: Rect, app_state: &AppState) {
    let theme = *app_state.theme_manager.current_theme();
    let sessions = app_state.get_filtered_sessions();

    if let Some(session) = sessions.get(app_state.session_selected_index) {
        let details = vec![
            Line::from(vec![
                Span::styled(
                    "Server: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&session.server_name, Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled(
                    "Window Title: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&session.window_title, Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled(
                    "PID: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(session.pid.to_string(), Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled(
                    "Status: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    if session.is_idle { "Idle" } else { "Active" },
                    Style::default().fg(if session.is_idle {
                        theme.orange
                    } else {
                        theme.status_online
                    }),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    "Duration: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(session.format_duration(), Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled(
                    "Started: ",
                    Style::default().fg(theme.cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    session
                        .started_at
                        .format("%Y-%m-%d %H:%M:%S UTC")
                        .to_string(),
                    Style::default().fg(theme.fg),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Controls:",
                Style::default()
                    .fg(theme.purple)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled(
                    "  d ",
                    Style::default()
                        .fg(theme.theme_primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Kill session", Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled(
                    "  Enter ",
                    Style::default()
                        .fg(theme.theme_primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Show session info", Style::default().fg(theme.fg)),
            ]),
            Line::from(vec![
                Span::styled(
                    "  r ",
                    Style::default()
                        .fg(theme.theme_primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("Refresh sessions", Style::default().fg(theme.fg)),
            ]),
        ];

        let details_paragraph = Paragraph::new(details)
            .block(
                Block::default()
                    .title(" Session Details ")
                    .title_style(
                        Style::default()
                            .fg(theme.theme_primary)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border))
                    .style(Style::default().bg(theme.bg)),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(details_paragraph, area);
    } else {
        let empty_message = vec![
            Line::from(vec![Span::styled(
                "No active sessions",
                Style::default().fg(theme.comment),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Connect to a server to start a session",
                Style::default().fg(theme.comment),
            )]),
        ];

        let empty_paragraph = Paragraph::new(empty_message)
            .block(
                Block::default()
                    .title(" Session Details ")
                    .title_style(
                        Style::default()
                            .fg(theme.theme_primary)
                            .add_modifier(Modifier::BOLD),
                    )
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border))
                    .style(Style::default().bg(theme.bg)),
            )
            .alignment(Alignment::Center);

        f.render_widget(empty_paragraph, area);
    }
}

/// Get color for duration based on how long the session has been running
fn get_duration_color(duration_str: &str, theme: Theme) -> Color {
    if duration_str.contains('h') {
        // Long running sessions (hours) - red
        theme.red
    } else if duration_str.contains('m') {
        let minutes: i32 = duration_str
            .split('m')
            .next()
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);
        if minutes > 30 {
            theme.orange // 30+ minutes - orange
        } else {
            theme.yellow // Less than 30 minutes - yellow
        }
    } else {
        theme.green // Seconds only - green
    }
}

/// Create a visual progress bar for session duration
fn create_duration_progress_bar(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();

    // Scale: 0-5min = ▁, 5-15min = ▂, 15-30min = ▃, 30min-1h = ▄, 1h-2h = ▅, 2h+ = ▆
    let bar_char = match total_seconds {
        0..=300 => "▁",     // 0-5 minutes
        301..=900 => "▂",   // 5-15 minutes
        901..=1800 => "▃",  // 15-30 minutes
        1801..=3600 => "▄", // 30min-1hour
        3601..=7200 => "▅", // 1-2 hours
        _ => "▆",           // 2+ hours
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

/// Theme picker. Moving the selection previews the theme across the whole UI —
/// the popup itself repaints in the highlighted theme, so you see the real
/// thing before committing.
fn render_theme_selector(f: &mut Frame, area: Rect, app_state: &AppState) {
    let theme = *app_state.theme_manager.current_theme();
    let variants = crate::themes::ThemeVariant::all();
    let popup_area = centered_rect(46, 62, area);

    let items: Vec<ListItem> = variants
        .iter()
        .enumerate()
        .map(|(i, variant)| {
            let selected = i == app_state.theme_selector_index;
            let marker = if selected { "▸ " } else { "  " };
            let style = if selected {
                Style::default()
                    .bg(theme.bg_highlight)
                    .fg(theme.theme_primary)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            ListItem::new(Line::from(vec![
                Span::styled(marker, Style::default().fg(theme.theme_primary)),
                Span::styled(variant.name(), style),
                Span::raw("  "),
                // A swatch of the theme's own accents, so the list is scannable
                // without previewing every entry one at a time.
                Span::styled("●", Style::default().fg(theme.green)),
                Span::styled("●", Style::default().fg(theme.yellow)),
                Span::styled("●", Style::default().fg(theme.red)),
                Span::styled("●", Style::default().fg(theme.blue)),
                Span::styled("●", Style::default().fg(theme.purple)),
            ]))
            .style(style)
        })
        .collect();

    f.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(popup_area);

    let list = List::new(items).block(
        Block::default()
            .title(" Theme ")
            .title_style(
                Style::default()
                    .fg(theme.theme_primary)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_highlight))
            .style(Style::default().bg(theme.bg_popup)),
    );
    f.render_widget(list, chunks[0]);

    f.render_widget(
        Paragraph::new(" j/k preview · Enter keeps · Esc cancels ")
            .style(Style::default().fg(theme.comment).bg(theme.bg_popup))
            .alignment(Alignment::Center),
        chunks[1],
    );
}

/// Shade a status dot by actual round-trip time rather than just up/down.
///
/// "Online" covers everything from a 1 ms LAN box to a 400 ms satellite link;
/// colouring by latency makes that visible at a glance without reading numbers.
fn latency_color(conn: &crate::models::ServerConnection, theme: Theme) -> Color {
    use crate::models::HealthStatus;
    match conn.health_status {
        HealthStatus::Online => match conn.stats.latency.map(|d| d.as_millis()) {
            Some(ms) if ms < 25 => theme.green,
            Some(ms) if ms < 80 => theme.cyan,
            Some(ms) if ms < 200 => theme.yellow,
            Some(_) => theme.orange,
            None => theme.status_online,
        },
        _ => get_health_color(&conn.health_status, theme),
    }
}

/// Topology view: hosts grouped under the bastion they are reached through.
fn render_topology_view(f: &mut Frame, area: Rect, app_state: &mut AppState) {
    let theme = *app_state.theme_manager.current_theme();
    let globe = app_state.get_globe_char();
    let connections = app_state.server_manager.filtered_connections();
    let rows = crate::topology::build(&connections);
    let selectable = crate::topology::selectable(&rows);

    if rows.is_empty() {
        let hint = Paragraph::new("No servers to map. Press 'i' to import your ~/.ssh/config.")
            .style(Style::default().fg(theme.comment))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(" Topology ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border))
                    .style(Style::default().bg(theme.bg)),
            );
        f.render_widget(hint, area);
        return;
    }

    // Split off a path panel when there is room; below that width the tree
    // itself would get too cramped to read.
    let show_path = area.width >= 90;
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(if show_path {
            vec![Constraint::Min(46), Constraint::Length(34)]
        } else {
            vec![Constraint::Percentage(100)]
        })
        .split(area);

    let cursor = app_state
        .topology_selected
        .min(selectable.len().saturating_sub(1));
    let selected_row = selectable.get(cursor).copied();

    let tree_area = chunks[0];
    let inner_height = tree_area.height.saturating_sub(2) as usize;
    // Only spend a row on the pinned heading when the list can actually
    // scroll; on a short list it would just be a blank line at the top.
    let scrollable = rows.len() > inner_height;
    let visible = if scrollable {
        inner_height.saturating_sub(1).max(1)
    } else {
        inner_height.max(1)
    };

    // Scroll so the selection stays on screen, with the window clamped to the
    // end of the list so the last page isn't half empty.
    let anchor = selected_row.unwrap_or(0);
    let max_offset = rows.len().saturating_sub(visible);
    let offset = if anchor < app_state.topology_offset {
        anchor
    } else if anchor >= app_state.topology_offset + visible {
        anchor + 1 - visible
    } else {
        app_state.topology_offset
    }
    .min(max_offset);
    app_state.topology_offset = offset;

    // Which group governs the selection, and has it scrolled out of view? If
    // so it gets pinned, because "which bastion am I under" is the entire point
    // of this view and losing it to a scroll defeats the exercise.
    let governing = selected_row.and_then(|r| crate::topology::group_of(&rows, r));
    let pinned = governing.filter(|&g| g < offset);

    let name_width = 24usize;

    let render_row = |row: &crate::topology::Row, selected: bool| -> Line<'static> {
        let base = if selected {
            Style::default()
                .bg(theme.bg_highlight)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        match row {
            crate::topology::Row::Spacer => Line::from(""),

            crate::topology::Row::Group {
                label,
                server,
                children,
                is_bastion,
            } => {
                let mut spans = vec![Span::styled(
                    format!(
                        "{:<width$}",
                        truncate(label, name_width),
                        width = name_width
                    ),
                    base.fg(theme.theme_primary).add_modifier(Modifier::BOLD),
                )];
                match server.map(|i| connections[i]) {
                    Some(conn) => {
                        spans.push(Span::styled(
                            conn.health_status.symbol(),
                            base.fg(latency_color(conn, theme)),
                        ));
                        spans.push(Span::styled(
                            format!(" {:>7}", format_latency(conn)),
                            base.fg(theme.comment),
                        ));
                    }
                    // A bastion Ghost has no entry for: it routes traffic but
                    // cannot be probed or connected to. The synthetic "Direct"
                    // heading is neither, so it says nothing.
                    None if *is_bastion => {
                        spans.push(Span::styled("· not configured", base.fg(theme.comment)))
                    }
                    None => {}
                }
                spans.push(Span::styled(
                    if *is_bastion {
                        format!("   {} behind", children)
                    } else {
                        format!("   {} host(s)", children)
                    },
                    base.fg(theme.comment),
                ));
                Line::from(spans)
            }

            crate::topology::Row::Host { server, last } => {
                let conn = connections[*server];
                let elbow = if *last { "└── " } else { "├── " };
                let symbol =
                    if matches!(conn.health_status, crate::models::HealthStatus::Connecting) {
                        globe
                    } else {
                        conn.health_status.symbol()
                    };

                let label = format!("{}{}", elbow, conn.name);
                let mut spans = vec![
                    Span::styled(
                        format!(
                            "{:<width$}",
                            truncate(&label, name_width),
                            width = name_width
                        ),
                        base.fg(if selected {
                            theme.theme_primary
                        } else {
                            theme.fg
                        }),
                    ),
                    Span::styled(symbol, base.fg(latency_color(conn, theme))),
                    Span::styled(
                        format!(" {:>7}", format_latency(conn)),
                        base.fg(theme.comment),
                    ),
                    Span::styled(
                        format!("   {}", truncate(&conn.connection_string(), 30)),
                        base.fg(theme.comment),
                    ),
                ];
                if conn.has_active_sessions() {
                    spans.push(Span::styled(
                        format!(" [{}]", conn.session_count()),
                        base.fg(theme.green).add_modifier(Modifier::BOLD),
                    ));
                }
                Line::from(spans)
            }
        }
    };

    let mut lines: Vec<Line> = Vec::with_capacity(inner_height);
    match pinned {
        Some(g) => {
            let mut line = render_row(&rows[g], false);
            line.spans
                .push(Span::styled("  ↑", Style::default().fg(theme.comment)));
            lines.push(line);
        }
        // Keep the row budget identical whether or not a heading is pinned, so
        // the list doesn't jitter by one line as you scroll past a boundary.
        None => lines.push(Line::from("")),
    }

    for (i, row) in rows.iter().enumerate().skip(offset).take(visible) {
        lines.push(render_row(row, Some(i) == selected_row));
    }

    let hidden_above = offset;
    let hidden_below = rows.len().saturating_sub(offset + visible);
    let title = if hidden_above > 0 || hidden_below > 0 {
        format!(
            " Topology — {} host(s)  [{}/{}] ",
            connections.len(),
            cursor + 1,
            selectable.len()
        )
    } else {
        format!(" Topology — {} host(s) ", connections.len())
    };

    let tree = Paragraph::new(lines).block(
        Block::default()
            .title(title)
            .title_style(
                Style::default()
                    .fg(theme.theme_primary)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.bg)),
    );
    f.render_widget(tree, tree_area);

    if hidden_below > 0 && tree_area.height > 2 {
        f.render_widget(
            Paragraph::new(format!("{} more ▼", hidden_below))
                .style(Style::default().fg(theme.comment))
                .alignment(Alignment::Right),
            Rect::new(
                tree_area.x + 1,
                tree_area.y + tree_area.height - 1,
                tree_area.width.saturating_sub(3),
                1,
            ),
        );
    }

    if show_path {
        let selected_server = selected_row.and_then(|r| crate::topology::server_of(&rows[r]));
        render_topology_path(f, chunks[1], &connections, selected_server, theme);
    }
}

/// The path panel: how you actually reach the selected host.
///
/// This is what the topology view knows that the flat list does not — a host
/// can be perfectly healthy and still unreachable because something upstream
/// of it is down.
fn render_topology_path(
    f: &mut Frame,
    area: Rect,
    connections: &[&crate::models::ServerConnection],
    selected: Option<usize>,
    theme: Theme,
) {
    let block = Block::default()
        .title(" Path ")
        .title_style(
            Style::default()
                .fg(theme.theme_primary)
                .add_modifier(Modifier::BOLD),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg));

    let Some(index) = selected else {
        f.render_widget(
            Paragraph::new("No host selected")
                .style(Style::default().fg(theme.comment))
                .alignment(Alignment::Center)
                .block(block),
            area,
        );
        return;
    };

    let chain = crate::topology::jump_chain(connections, index);
    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        "this machine",
        Style::default().fg(theme.comment),
    ))];

    let mut blocked_by: Option<String> = None;
    for (depth, hop) in chain.iter().enumerate() {
        let indent = "  ".repeat(depth);
        match hop.server.map(|i| connections[i]) {
            Some(conn) => {
                // Only a hop that is *known* to be down blocks the path.
                // Unknown means "not probed yet", and flagging that as blocked
                // fires on every host at startup, before any check has run.
                let known_down = matches!(conn.health_status, crate::models::HealthStatus::Offline);
                if known_down && depth + 1 < chain.len() && blocked_by.is_none() {
                    blocked_by = Some(conn.name.clone());
                }
                lines.push(Line::from(vec![
                    Span::styled(format!("{}└→ ", indent), Style::default().fg(theme.comment)),
                    Span::styled(
                        truncate(&hop.label, 16),
                        Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        conn.health_status.symbol(),
                        Style::default().fg(latency_color(conn, theme)),
                    ),
                    Span::styled(
                        format!(" {}", format_latency(conn)),
                        Style::default().fg(theme.comment),
                    ),
                ]));
            }
            None => {
                if blocked_by.is_none() {
                    blocked_by = Some(hop.label.clone());
                }
                lines.push(Line::from(vec![
                    Span::styled(format!("{}└→ ", indent), Style::default().fg(theme.comment)),
                    Span::styled(truncate(&hop.label, 14), Style::default().fg(theme.orange)),
                ]));
                // On its own line: appending it wrapped the hop name mid-word.
                lines.push(Line::from(Span::styled(
                    format!("{}   not configured", indent),
                    Style::default().fg(theme.comment),
                )));
            }
        }
    }

    let target = connections[index];
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        truncate(&target.connection_string(), 30),
        Style::default().fg(theme.fg),
    )));
    lines.push(Line::from(vec![
        Span::styled("auth  ", Style::default().fg(theme.comment)),
        Span::styled(
            target.auth_strength.as_str(),
            Style::default().fg(get_auth_color(&target.auth_strength, theme)),
        ),
    ]));
    if let Some(err) = &target.last_error {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            truncate(err, 30),
            Style::default().fg(theme.red),
        )));
    }

    // Call out the structural case: the host itself may be fine, but you cannot
    // get to it because a hop in front of it is down or missing.
    if let Some(hop) = blocked_by {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("⚠ blocked at {}", truncate(&hop, 16)),
            Style::default()
                .fg(theme.orange)
                .add_modifier(Modifier::BOLD),
        )));
    }

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn format_latency(conn: &crate::models::ServerConnection) -> String {
    match conn.stats.latency {
        Some(d) if matches!(conn.health_status, crate::models::HealthStatus::Online) => {
            format!("{}ms", d.as_millis())
        }
        _ => "—".to_string(),
    }
}

/// Truncate to a display width, leaving room for an ellipsis.
fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        s.to_string()
    } else {
        let keep = width.saturating_sub(1);
        s.chars().take(keep).collect::<String>() + "…"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_latency_history_renders_nothing() {
        // A flat "▁▁▁▁▁" implied five samples at zero latency that were never
        // actually measured.
        assert_eq!(render_latency_sparkline(&[]), "");
    }

    #[test]
    fn sparkline_runs_oldest_to_newest() {
        // Regression: a stray double .rev() drew the series backwards.
        let rising = render_latency_sparkline(&[10, 200, 400]);
        let chars: Vec<char> = rising.chars().collect();
        assert_eq!(chars.len(), 3);
        assert!(
            chars[0] < chars[1],
            "expected rising bars, got {:?}",
            rising
        );
        assert!(
            chars[1] < chars[2],
            "expected rising bars, got {:?}",
            rising
        );
    }

    #[test]
    fn sparkline_is_capped_at_eight_bars() {
        let long: Vec<u32> = (0..40).collect();
        assert_eq!(render_latency_sparkline(&long).chars().count(), 8);
    }

    #[test]
    fn a_healthy_flat_link_is_not_all_full_bars() {
        // Scaling purely against the max made every stable connection render as
        // eight solid blocks, which reads as "bad".
        let flat = render_latency_sparkline(&[12, 12, 12, 12]);
        assert!(
            !flat.chars().all(|c| c == '█'),
            "flat low latency rendered as full bars: {:?}",
            flat
        );
    }

    fn probed(
        status: crate::models::HealthStatus,
        ms: Option<u64>,
    ) -> crate::models::ServerConnection {
        let mut c = crate::models::ServerConnection::new("t".into(), "h".into(), 22, "u".into());
        c.health_status = status;
        c.stats.latency = ms.map(std::time::Duration::from_millis);
        c
    }

    #[test]
    fn latency_shading_separates_fast_from_slow() {
        // Cannot be demonstrated on loopback — a local TCP connect completes in
        // ~0ms no matter what the listener does — so pin the thresholds here.
        use crate::models::HealthStatus;
        let theme = crate::themes::Theme::from_variant(crate::themes::ThemeVariant::TokyoNightDark);

        let lan = latency_color(&probed(HealthStatus::Online, Some(3)), theme);
        let regional = latency_color(&probed(HealthStatus::Online, Some(60)), theme);
        let distant = latency_color(&probed(HealthStatus::Online, Some(150)), theme);
        let awful = latency_color(&probed(HealthStatus::Online, Some(600)), theme);

        assert_eq!(lan, theme.green);
        assert_eq!(regional, theme.cyan);
        assert_eq!(distant, theme.yellow);
        assert_eq!(awful, theme.orange);
        // Every band must be visually distinct or the shading says nothing.
        let bands = [lan, regional, distant, awful];
        for (i, a) in bands.iter().enumerate() {
            for b in bands.iter().skip(i + 1) {
                assert_ne!(a, b, "two latency bands share a colour");
            }
        }
    }

    #[test]
    fn an_offline_host_is_not_shaded_by_stale_latency() {
        use crate::models::HealthStatus;
        let theme = crate::themes::Theme::from_variant(crate::themes::ThemeVariant::TokyoNightDark);
        // A host that just went down still carries its last measurement; it
        // must not keep rendering as a healthy green.
        let c = probed(HealthStatus::Offline, Some(3));
        assert_eq!(latency_color(&c, theme), theme.status_offline);
    }

    #[test]
    fn online_without_a_measurement_falls_back_to_plain_online() {
        use crate::models::HealthStatus;
        let theme = crate::themes::Theme::from_variant(crate::themes::ThemeVariant::TokyoNightDark);
        let c = probed(HealthStatus::Online, None);
        assert_eq!(latency_color(&c, theme), theme.status_online);
    }

    #[test]
    fn online_and_offline_glyphs_differ_without_colour() {
        use crate::models::HealthStatus;
        // Colour alone is not an accessible signal: these must be told apart in
        // a screenshot, a mono terminal, or by a red-green colourblind reader.
        assert_ne!(
            HealthStatus::Online.symbol(),
            HealthStatus::Offline.symbol()
        );
    }

    #[test]
    fn latency_is_only_reported_for_reachable_hosts() {
        use crate::models::HealthStatus;
        assert_eq!(
            format_latency(&probed(HealthStatus::Online, Some(42))),
            "42ms"
        );
        assert_eq!(
            format_latency(&probed(HealthStatus::Offline, Some(42))),
            "—"
        );
        assert_eq!(format_latency(&probed(HealthStatus::Online, None)), "—");
    }

    #[test]
    fn truncate_never_exceeds_the_budget() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("exactlyten", 10), "exactlyten");
        let cut = truncate("a-very-long-hostname-indeed", 10);
        assert_eq!(cut.chars().count(), 10);
        assert!(cut.ends_with('…'));
        // Must not split a multi-byte character.
        let uni = truncate("日本語のホスト名です", 5);
        assert_eq!(uni.chars().count(), 5);
    }

    #[test]
    fn progress_bar_spans_empty_to_full() {
        assert!(!create_progress_bar(0.0, 10).contains('█'));
        assert_eq!(
            create_progress_bar(1.0, 10)
                .chars()
                .filter(|&c| c == '█')
                .count(),
            10
        );
    }
}
