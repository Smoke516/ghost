use crate::models::{HealthStatus, SecurityStatus, ServerConnection};
use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::time::timeout;
use std::process::Command;

/// SSH connection timeout in seconds
const CONNECTION_TIMEOUT: u64 = 10;

/// Available terminal emulators for spawning SSH sessions
#[derive(Debug, Clone, PartialEq)]
pub enum AvailableTerminal {
    GnomeTerminal,
    XTerm,
    Konsole,
    XfceTerminal,
    WindowsTerminal,
    MacTerminal,
    Alacritty,
    Kitty,
    Wezterm,
    Ghostty,
    Warp,
    None, // Fallback to direct mode
}

/// Connection mode preference
#[derive(Debug, Clone, PartialEq, clap::ValueEnum)]
pub enum ConnectionMode {
    Auto,        // Try new terminal, fallback to direct
    NewTerminal, // Force new terminal (fail if none available)
    Direct,      // Always use current direct approach
}

impl AvailableTerminal {
    /// Get the command name for this terminal
    pub fn command_name(&self) -> Option<&'static str> {
        match self {
            AvailableTerminal::GnomeTerminal => Some("gnome-terminal"),
            AvailableTerminal::XTerm => Some("xterm"),
            AvailableTerminal::Konsole => Some("konsole"),
            AvailableTerminal::XfceTerminal => Some("xfce4-terminal"),
            AvailableTerminal::WindowsTerminal => Some("wt"),
            AvailableTerminal::MacTerminal => Some("osascript"),
            AvailableTerminal::Alacritty => Some("alacritty"),
            AvailableTerminal::Kitty => Some("kitty"),
            AvailableTerminal::Wezterm => Some("wezterm"),
            AvailableTerminal::Ghostty => Some("ghostty"),
            AvailableTerminal::Warp => Some("warp"),
            AvailableTerminal::None => None,
        }
    }

    /// Check if this terminal is available on the system
    pub fn is_available(&self) -> bool {
        match self.command_name() {
            Some(cmd) => {
                // Special case for macOS Terminal
                if *self == AvailableTerminal::MacTerminal {
                    return cfg!(target_os = "macos");
                }
                
                // Special case for Warp - check if it's running rather than command availability
                if *self == AvailableTerminal::Warp {
                    // Check if Warp is currently running by looking for the process
                    if let Ok(output) = Command::new("pgrep").arg("-f").arg("warp-terminal").output() {
                        return output.status.success();
                    }
                    return false;
                }
                
                // Check if command exists in PATH using cross-platform approach
                #[cfg(unix)]
                {
                    Command::new("which")
                        .arg(cmd)
                        .output()
                        .map(|output| output.status.success())
                        .unwrap_or(false)
                }
                
                #[cfg(windows)]
                {
                    Command::new("where")
                        .arg(cmd)
                        .output()
                        .map(|output| output.status.success())
                        .unwrap_or(false)
                }
            },
            None => false,
        }
    }

    /// Get command arguments for spawning SSH session
    pub fn get_ssh_command(&self, connection: &ServerConnection) -> Option<Vec<String>> {
        let ssh_cmd = format!("ssh {}@{}", connection.username, connection.host);
        
        match self {
            AvailableTerminal::GnomeTerminal => {
                Some(vec![
                    "--".to_string(),
                    "bash".to_string(),
                    "-c".to_string(),
                    ssh_cmd
                ])
            },
            AvailableTerminal::XTerm => {
                Some(vec![
                    "-e".to_string(),
                    ssh_cmd
                ])
            },
            AvailableTerminal::Konsole => {
                Some(vec![
                    "-e".to_string(),
                    ssh_cmd
                ])
            },
            AvailableTerminal::XfceTerminal => {
                Some(vec![
                    "-e".to_string(),
                    ssh_cmd
                ])
            },
            AvailableTerminal::WindowsTerminal => {
                Some(vec![
                    "ssh".to_string(),
                    format!("{}@{}", connection.username, connection.host)
                ])
            },
            AvailableTerminal::MacTerminal => {
                Some(vec![
                    "-e".to_string(),
                    format!(
                        "tell application \"Terminal\" to do script \"{}\"",
                        ssh_cmd
                    )
                ])
            },
            AvailableTerminal::Alacritty => {
                Some(vec![
                    "-e".to_string(),
                    "ssh".to_string(),
                    format!("{}@{}", connection.username, connection.host)
                ])
            },
            AvailableTerminal::Kitty => {
                Some(vec![
                    "ssh".to_string(),
                    format!("{}@{}", connection.username, connection.host)
                ])
            },
            AvailableTerminal::Wezterm => {
                Some(vec![
                    "start".to_string(),
                    "--".to_string(),
                    "ssh".to_string(),
                    format!("{}@{}", connection.username, connection.host)
                ])
            },
            AvailableTerminal::Ghostty => {
                Some(vec![
                    "-e".to_string(),
                    "ssh".to_string(),
                    format!("{}@{}", connection.username, connection.host)
                ])
            },
            AvailableTerminal::Warp => {
                // Warp Terminal doesn't have direct SSH spawning like other terminals
                // Use the system's default terminal opener instead
                Some(vec![
                    "ssh".to_string(),
                    format!("{}@{}", connection.username, connection.host)
                ])
            },
            AvailableTerminal::None => None,
        }
    }
}

/// Detect the best available terminal emulator
pub fn detect_available_terminal() -> AvailableTerminal {
    // Check environment variables first for better detection
    if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            "Apple_Terminal" => return AvailableTerminal::MacTerminal,
            "Alacritty" => return AvailableTerminal::Alacritty,
            "kitty" => return AvailableTerminal::Kitty,
            "WezTerm" => return AvailableTerminal::Wezterm,
            "ghostty" => return AvailableTerminal::Ghostty,
            "WarpTerminal" => {
                // Warp Terminal doesn't support spawning new windows programmatically
                eprintln!("⚠️  Warp Terminal detected but doesn't support new window spawning.");
                eprintln!("    Falling back to direct connection mode.");
                return AvailableTerminal::None;
            },
            _ => {}
        }
    }

    // Check for Windows Terminal
    if cfg!(target_os = "windows") {
        if AvailableTerminal::WindowsTerminal.is_available() {
            return AvailableTerminal::WindowsTerminal;
        }
    }

    // Try terminals in order of preference (excluding Warp since it doesn't work)
    let terminals = vec![
        AvailableTerminal::Ghostty,
        AvailableTerminal::Alacritty,
        AvailableTerminal::Kitty,
        AvailableTerminal::Wezterm,
        AvailableTerminal::GnomeTerminal,
        AvailableTerminal::Konsole,
        AvailableTerminal::XfceTerminal,
        AvailableTerminal::XTerm,
        AvailableTerminal::MacTerminal,
    ];

    for terminal in terminals {
        if terminal.is_available() {
            return terminal;
        }
    }

    AvailableTerminal::None
}

/// SSH connection manager
pub struct SSHManager {
    connections: HashMap<String, bool>, // Simple connection tracking for now
}

impl SSHManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    /// Test SSH connection to a server (simplified to TCP + SSH port check)
    pub async fn test_connection(&mut self, server: &ServerConnection) -> Result<ConnectionTestResult> {
        let start_time = Instant::now();
        let result = self.perform_simple_connection_test(server).await;
        let latency = start_time.elapsed();

        match result {
            Ok(is_ssh_service) => Ok(ConnectionTestResult {
                status: HealthStatus::Online,
                security_status: if is_ssh_service { 
                    // Use the consistent security assessment
                    self.assess_security_status(server)
                } else { 
                    SecurityStatus::Vulnerable // Port open but not SSH
                },
                latency: Some(latency),
                error_message: None,
            }),
            Err(e) => Ok(ConnectionTestResult {
                status: HealthStatus::Offline,
                security_status: SecurityStatus::Unknown,
                latency: Some(latency),
                error_message: Some(e.to_string()),
            }),
        }
    }

    /// Perform a simple connection test (TCP + basic SSH protocol check)
    async fn perform_simple_connection_test(&mut self, server: &ServerConnection) -> Result<bool> {
        let address = format!("{}:{}", server.host, server.port);
        
        let _stream = timeout(
            Duration::from_secs(CONNECTION_TIMEOUT),
            TcpStream::connect(&address)
        ).await
        .context("Connection timeout")?
        .context("Failed to establish TCP connection")?;

        // For now, just assume it's SSH if we can connect to the port
        // In a real implementation, you would:
        // 1. Read the SSH banner
        // 2. Perform SSH protocol handshake
        // 3. Check supported authentication methods
        
        // If we got this far, the port is open and responsive
        Ok(true)
    }

    /// Perform a simple connectivity test with security assessment
    pub async fn quick_health_check(&self, server: &ServerConnection) -> Result<ConnectionTestResult> {
        let start_time = Instant::now();
        let address = format!("{}:{}", server.host, server.port);
        
        let result = timeout(
            Duration::from_secs(5), // Quick timeout for health checks
            TcpStream::connect(&address)
        ).await;

        let latency = start_time.elapsed();

        match result {
            Ok(Ok(_)) => {
                // Connection successful - assess security based on configuration
                let security_status = self.assess_security_status(server);
                Ok(ConnectionTestResult {
                    status: HealthStatus::Online,
                    security_status,
                    latency: Some(latency),
                    error_message: None,
                })
            },
            Ok(Err(e)) => Ok(ConnectionTestResult {
                status: HealthStatus::Offline,
                security_status: SecurityStatus::Unknown,
                latency: Some(latency),
                error_message: Some(format!("Connection failed: {}", e)),
            }),
            Err(_) => Ok(ConnectionTestResult {
                status: HealthStatus::Offline,
                security_status: SecurityStatus::Unknown,
                latency: Some(latency),
                error_message: Some("Connection timeout".to_string()),
            }),
        }
    }
    
    /// Assess security status based on server configuration
    fn assess_security_status(&self, server: &ServerConnection) -> SecurityStatus {
        match &server.auth_method {
            crate::models::AuthMethod::PublicKey { .. } => SecurityStatus::Secure,
            crate::models::AuthMethod::Agent => SecurityStatus::Secure,
            crate::models::AuthMethod::Password => {
                if server.port != 22 {
                    SecurityStatus::Secure // Non-standard port + password = decent security
                } else {
                    SecurityStatus::Vulnerable // Standard port + password = vulnerable
                }
            },
            crate::models::AuthMethod::Interactive => SecurityStatus::Unknown,
        }
    }

    /// Connect to a server interactively by launching SSH in the terminal
    /// Returns the PID of the spawned terminal process
    pub async fn connect_interactive(&mut self, server: &ServerConnection) -> Result<u32> {
        self.connect_with_mode(server, ConnectionMode::Auto).await
    }
    
    /// Connect to a server with a specific connection mode
    pub async fn connect_with_mode(&mut self, server: &ServerConnection, mode: ConnectionMode) -> Result<u32> {
        eprintln!("🚀 DEBUG: Starting connect for server: {} with mode: {:?}", server.name, mode);
        
        // First, test if the server is reachable
        let test_result = self.test_connection(server).await?;
        
        match test_result.status {
            HealthStatus::Online => {
                self.connections.insert(server.id.clone(), true);
                
                match mode {
                    ConnectionMode::Auto => {
                        // Try new terminal first, fallback to direct if unavailable
                        let available_terminal = detect_available_terminal();
                        if available_terminal != AvailableTerminal::None {
                            eprintln!("🚀 Using terminal: {:?}", available_terminal);
                            self.launch_ssh_in_new_terminal(server, available_terminal).await
                        } else {
                            eprintln!("⚠️  No suitable terminal found for new window. Using direct connection.");
                            self.launch_ssh_session(server).await
                        }
                    },
                    ConnectionMode::NewTerminal => {
                        let available_terminal = detect_available_terminal();
                        if available_terminal != AvailableTerminal::None {
                            eprintln!("🚀 Forcing new terminal: {:?}", available_terminal);
                            self.launch_ssh_in_new_terminal(server, available_terminal).await
                        } else {
                            Err(anyhow::anyhow!("No terminal emulator available for new terminal mode. Available terminals: Ghostty, Alacritty, Kitty, Wezterm, GNOME Terminal, Konsole, XFCE Terminal, XTerm"))
                        }
                    },
                    ConnectionMode::Direct => {
                        eprintln!("🚀 Using direct connection mode");
                        self.launch_ssh_session(server).await
                    }
                }
            }
            _ => {
                Err(anyhow::anyhow!(
                    "Cannot connect: {}", 
                    test_result.error_message.unwrap_or_else(|| "Connection failed".to_string())
                ))
            }
        }
    }
    
    /// Launch SSH session in a new terminal window
    async fn launch_ssh_in_new_terminal(&self, server: &ServerConnection, terminal: AvailableTerminal) -> Result<u32> {
        let mut terminal_cmd = Command::new(
            terminal.command_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid terminal type"))?
        );
        
        // Get SSH command arguments for the terminal
        let ssh_args = terminal.get_ssh_command(server)
            .ok_or_else(|| anyhow::anyhow!("Cannot generate SSH command for this terminal"))?;
        
        // Add the SSH arguments to the terminal command
        terminal_cmd.args(&ssh_args);
        
        // Add SSH options for the connection
        let mut ssh_options = Vec::new();
        
        // Add port if not default
        if server.port != 22 {
            ssh_options.push("-p".to_string());
            ssh_options.push(server.port.to_string());
        }
        
        // Add authentication method specific parameters
        match &server.auth_method {
            crate::models::AuthMethod::PublicKey { key_path } => {
                let expanded_path = shellexpand::tilde(key_path);
                ssh_options.push("-i".to_string());
                ssh_options.push(expanded_path.to_string());
            }
            crate::models::AuthMethod::Agent => {
                // SSH agent is the default, no special flags needed
            }
            crate::models::AuthMethod::Password => {
                ssh_options.push("-o".to_string());
                ssh_options.push("PreferredAuthentications=password".to_string());
            }
            crate::models::AuthMethod::Interactive => {
                ssh_options.push("-o".to_string());
                ssh_options.push("PreferredAuthentications=keyboard-interactive".to_string());
            }
        }
        
        // Add useful SSH options
        ssh_options.extend(vec![
            "-o".to_string(), "ServerAliveInterval=60".to_string(),
            "-o".to_string(), "ServerAliveCountMax=3".to_string(),
            "-o".to_string(), "ConnectTimeout=10".to_string(),
            "-o".to_string(), "BatchMode=no".to_string(),
        ]);
        
        // For terminals that need the full SSH command, rebuild it
        if matches!(terminal, AvailableTerminal::GnomeTerminal | AvailableTerminal::XTerm | 
                             AvailableTerminal::Konsole | AvailableTerminal::XfceTerminal) {
            // Replace the simple SSH command with our enhanced version
            terminal_cmd = Command::new(
                terminal.command_name().unwrap()
            );
            
            let full_ssh_cmd = format!(
                "ssh {} {}@{}",
                ssh_options.join(" "),
                server.username,
                server.host
            );
            
            match terminal {
                AvailableTerminal::GnomeTerminal => {
                    terminal_cmd.args(&["--", "bash", "-c", &full_ssh_cmd]);
                },
                _ => {
                    terminal_cmd.args(&["-e", &full_ssh_cmd]);
                }
            }
        } else {
            // For other terminals, add SSH options as separate arguments
            terminal_cmd.args(&ssh_options);
        }
        
        eprintln!("🚀 DEBUG: Launching terminal: {:?}", terminal);
        eprintln!("🚀 DEBUG: Terminal command: {:?}", terminal_cmd);
        
        // Spawn the terminal process with proper detachment
        use std::process::Stdio;
        
        let child = terminal_cmd
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to spawn terminal process")?;
        
        let pid = child.id();
        eprintln!("✅ Spawned terminal with PID: {}", pid);
        
        // Don't wait for the child process - let it run independently
        // This prevents terminal output interference with Ghost's TUI
        std::mem::forget(child);
        
        // Small delay to ensure terminal has time to launch
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        Ok(pid)
    }
    
    /// Launch SSH session directly in the current terminal
    async fn launch_ssh_session(&self, server: &ServerConnection) -> Result<u32> {
        use std::process::Command;
        
        // Build SSH command based on authentication method
        let mut ssh_cmd = Command::new("ssh");
        
        // Add basic connection parameters
        ssh_cmd.arg("-p").arg(server.port.to_string());
        ssh_cmd.arg(format!("{}@{}", server.username, server.host));
        
        // Add authentication method specific parameters
        match &server.auth_method {
            crate::models::AuthMethod::PublicKey { key_path } => {
                let expanded_path = shellexpand::tilde(key_path);
                ssh_cmd.arg("-i").arg(&*expanded_path);
            }
            crate::models::AuthMethod::Agent => {
                // SSH agent is the default, no special flags needed
            }
            crate::models::AuthMethod::Password => {
                // For password auth, we might want to disable other methods
                ssh_cmd.arg("-o").arg("PreferredAuthentications=password");
            }
            crate::models::AuthMethod::Interactive => {
                ssh_cmd.arg("-o").arg("PreferredAuthentications=keyboard-interactive");
            }
        }
        
        // Add some useful SSH options
        ssh_cmd.arg("-o").arg("ServerAliveInterval=60");
        ssh_cmd.arg("-o").arg("ServerAliveCountMax=3");
        ssh_cmd.arg("-o").arg("ConnectTimeout=10");
        ssh_cmd.arg("-o").arg("BatchMode=no"); // Ensure interactive prompts work
        
        // Execute SSH directly in current terminal
        self.execute_ssh_direct(ssh_cmd, server).await
    }
    
    /// Execute SSH directly in the current terminal
    async fn execute_ssh_direct(&self, mut ssh_cmd: std::process::Command, server: &ServerConnection) -> Result<u32> {
        use crossterm::terminal::{disable_raw_mode, enable_raw_mode, LeaveAlternateScreen, EnterAlternateScreen};
        use crossterm::ExecutableCommand;
        use std::io::stdout;
        
        // Suspend Ghost's TUI - disable raw mode and leave alternate screen
        if let Err(_) = disable_raw_mode() {
            eprintln!("Warning: Failed to disable raw mode");
        }
        if let Err(_) = stdout().execute(LeaveAlternateScreen) {
            eprintln!("Warning: Failed to leave alternate screen");
        }
        
        // Clear screen and show connection status
        println!("\x1b[2J\x1b[H"); // Clear screen and move cursor to top
        println!("🔗 Connecting to {}...", server.name);
        println!("   Host: {}:{}", server.host, server.port);
        println!("   User: {}", server.username);
        
        match &server.auth_method {
            crate::models::AuthMethod::PublicKey { key_path } => {
                let expanded_path = shellexpand::tilde(key_path);
                println!("   Auth: Public Key ({})", expanded_path);
            }
            crate::models::AuthMethod::Agent => {
                println!("   Auth: SSH Agent");
            }
            crate::models::AuthMethod::Password => {
                println!("   Auth: Password");
            }
            crate::models::AuthMethod::Interactive => {
                println!("   Auth: Interactive");
            }
        }
        
        println!("\nPress Ctrl+C to disconnect and return to Ghost\n");
        
        // Execute SSH command with proper stdio inheritance
        ssh_cmd.stdin(std::process::Stdio::inherit());
        ssh_cmd.stdout(std::process::Stdio::inherit());
        ssh_cmd.stderr(std::process::Stdio::inherit());
        
        let status = ssh_cmd.status();
        
        let result = match status {
            Ok(exit_status) => {
                println!("\n{}", "=".repeat(50));
                if exit_status.success() {
                    println!("✅ Disconnected from {} successfully", server.name);
                } else {
                    let code = exit_status.code().unwrap_or(-1);
                    println!("❌ Connection to {} ended with exit code: {}", server.name, code);
                }
                println!("Press any key to return to Ghost...");
                
                // Wait for user input before returning to Ghost UI
                use std::io::{self, BufRead};
                let stdin = io::stdin();
                let _ = stdin.lock().read_line(&mut String::new());
                
                // Return a dummy PID since we're not spawning a separate process
                Ok(std::process::id())
            }
            Err(e) => {
                println!("\n❌ Failed to execute SSH command: {}", e);
                println!("Press any key to return to Ghost...");
                
                use std::io::{self, BufRead};
                let stdin = io::stdin();
                let _ = stdin.lock().read_line(&mut String::new());
                
                Err(anyhow::anyhow!("SSH execution failed: {}", e))
            }
        };
        
        // Restore Ghost's TUI - re-enable raw mode and enter alternate screen
        if let Err(_) = stdout().execute(EnterAlternateScreen) {
            eprintln!("Warning: Failed to enter alternate screen");
        }
        if let Err(_) = enable_raw_mode() {
            eprintln!("Warning: Failed to enable raw mode");
        }
        
        // Force terminal to clear and prepare for Ghost's redraw
        use crossterm::terminal::Clear;
        use crossterm::terminal::ClearType;
        use crossterm::cursor::MoveTo;
        use std::io::Write;
        let _ = stdout().execute(Clear(ClearType::All));
        let _ = stdout().execute(MoveTo(0, 0));
        let _ = stdout().flush(); // Ensure all terminal commands are executed
        
        result
    }


}

/// Result of a connection test
#[derive(Debug, Clone)]
pub struct ConnectionTestResult {
    pub status: HealthStatus,
    pub security_status: SecurityStatus,
    pub latency: Option<Duration>,
    pub error_message: Option<String>,
}


impl ConnectionTestResult {
    pub fn update_server_stats(&self, server: &mut ServerConnection) {
        server.health_status = self.status.clone();
        server.security_status = self.security_status.clone();
        
        // Update connection stats
        server.stats.latency = self.latency;
        server.stats.last_connected = Some(Utc::now());
        
        match self.status {
            HealthStatus::Online => {
                server.stats.connection_count += 1;
                // Simple uptime calculation (this would be more sophisticated in a real app)
                server.stats.uptime_percentage = 
                    (server.stats.connection_count as f32 / (server.stats.connection_count + server.stats.failed_attempts) as f32) * 100.0;
            }
            HealthStatus::Offline => {
                server.stats.failed_attempts += 1;
                server.stats.uptime_percentage = 
                    (server.stats.connection_count as f32 / (server.stats.connection_count + server.stats.failed_attempts) as f32) * 100.0;
            }
            _ => {}
        }
    }
}

