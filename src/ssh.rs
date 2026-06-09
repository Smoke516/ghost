use crate::models::{AuthStrength, HealthStatus, ServerConnection};
use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::time::timeout;
use std::process::Command;

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

    /// Wrap a program and its arguments into this terminal's argv form, WITHOUT
    /// going through a shell. Every element is passed as a separate argument, so
    /// server fields (host / username / key path) can never be interpreted as
    /// shell syntax. The macOS Terminal path is the one exception — osascript
    /// requires a command string — so it is explicitly shell- and AppleScript-
    /// escaped.
    pub fn wrap_command(&self, program: &str, args: &[String]) -> Option<Vec<String>> {
        // program followed by its args, as owned strings
        let prog_and_args = || {
            let mut v = Vec::with_capacity(args.len() + 1);
            v.push(program.to_string());
            v.extend_from_slice(args);
            v
        };

        match self {
            // `terminal -- program args...`
            AvailableTerminal::GnomeTerminal => {
                let mut v = vec!["--".to_string()];
                v.extend(prog_and_args());
                Some(v)
            }
            // `terminal -e program args...` — these exec the argv directly.
            AvailableTerminal::XTerm
            | AvailableTerminal::Konsole
            | AvailableTerminal::Alacritty
            | AvailableTerminal::Ghostty => {
                let mut v = vec!["-e".to_string()];
                v.extend(prog_and_args());
                Some(v)
            }
            // xfce4-terminal: `-e` takes a single (shell-parsed) string, but
            // `-x` consumes the rest of the argv directly — use that.
            AvailableTerminal::XfceTerminal => {
                let mut v = vec!["-x".to_string()];
                v.extend(prog_and_args());
                Some(v)
            }
            // `wezterm start -- program args...`
            AvailableTerminal::Wezterm => {
                let mut v = vec!["start".to_string(), "--".to_string()];
                v.extend(prog_and_args());
                Some(v)
            }
            // kitty / Windows Terminal / Warp run the program as positional argv.
            AvailableTerminal::Kitty
            | AvailableTerminal::WindowsTerminal
            | AvailableTerminal::Warp => Some(prog_and_args()),
            // macOS Terminal via osascript needs a string: shell-quote each
            // argument, then AppleScript-escape the whole command.
            AvailableTerminal::MacTerminal => {
                let cmd = std::iter::once(program.to_string())
                    .chain(args.iter().cloned())
                    .map(|a| shell_quote(&a))
                    .collect::<Vec<_>>()
                    .join(" ");
                let script = format!(
                    "tell application \"Terminal\" to do script \"{}\"",
                    applescript_escape(&cmd)
                );
                Some(vec!["-e".to_string(), script])
            }
            AvailableTerminal::None => None,
        }
    }
}

/// Build the argument vector passed to `ssh` (everything after the `ssh`
/// program name) as separate argv elements — never assembled into a shell
/// string. Shared by both the direct and new-terminal launch paths.
fn build_ssh_args(server: &ServerConnection) -> Vec<String> {
    let mut args = Vec::new();

    if server.port != 22 {
        args.push("-p".to_string());
        args.push(server.port.to_string());
    }

    match &server.auth_method {
        crate::models::AuthMethod::PublicKey { key_path } => {
            let expanded = shellexpand::tilde(key_path).to_string();
            args.push("-i".to_string());
            args.push(expanded);
        }
        crate::models::AuthMethod::Agent => {}
        crate::models::AuthMethod::Password => {
            args.push("-o".to_string());
            args.push("PreferredAuthentications=password".to_string());
        }
        crate::models::AuthMethod::Interactive => {
            args.push("-o".to_string());
            args.push("PreferredAuthentications=keyboard-interactive".to_string());
        }
    }

    args.extend([
        "-o".to_string(), "ServerAliveInterval=60".to_string(),
        "-o".to_string(), "ServerAliveCountMax=3".to_string(),
        "-o".to_string(), "ConnectTimeout=10".to_string(),
        "-o".to_string(), "BatchMode=no".to_string(),
    ]);

    args.push(format!("{}@{}", server.username, server.host));
    args
}

/// POSIX-shell single-quote escaping: wrap in '...' and escape embedded quotes.
fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

/// Escape a string for embedding inside an AppleScript double-quoted literal.
fn applescript_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
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
    if cfg!(target_os = "windows")
        && AvailableTerminal::WindowsTerminal.is_available() {
            return AvailableTerminal::WindowsTerminal;
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

impl Default for SSHManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SSHManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
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
                // Reachable — surface the configured auth method as a hint.
                let auth_strength = self.assess_auth_strength(server);
                Ok(ConnectionTestResult {
                    status: HealthStatus::Online,
                    auth_strength,
                    latency: Some(latency),
                    error_message: None,
                })
            },
            Ok(Err(e)) => Ok(ConnectionTestResult {
                status: HealthStatus::Offline,
                auth_strength: AuthStrength::Unknown,
                latency: Some(latency),
                error_message: Some(format!("Connection failed: {}", e)),
            }),
            Err(_) => Ok(ConnectionTestResult {
                status: HealthStatus::Offline,
                auth_strength: AuthStrength::Unknown,
                latency: Some(latency),
                error_message: Some("Connection timeout".to_string()),
            }),
        }
    }

    /// Map the configured auth method to an at-a-glance strength hint.
    ///
    /// This is a reflection of LOCAL config only — it does not (and cannot, from
    /// a plain TCP connect) audit the remote host's actual security posture. Its
    /// only job is to make weaker auth choices (password) visually stand out.
    fn assess_auth_strength(&self, server: &ServerConnection) -> AuthStrength {
        match &server.auth_method {
            crate::models::AuthMethod::PublicKey { .. } => AuthStrength::Key,
            crate::models::AuthMethod::Agent => AuthStrength::Agent,
            crate::models::AuthMethod::Password => AuthStrength::Password,
            crate::models::AuthMethod::Interactive => AuthStrength::Interactive,
        }
    }

    /// Connect to a server with a specific connection mode
    pub async fn connect_with_mode(&mut self, server: &ServerConnection, mode: ConnectionMode) -> Result<u32> {
        // We deliberately do NOT pre-gate on a raw TCP reachability probe.
        // Hosts behind a bastion/ProxyJump, with port-knocking, or that drop
        // port scans are perfectly connectable via ssh even when a direct TCP
        // test to host:port fails. Let ssh itself be the authority on whether
        // the connection succeeds, and surface its exit status instead.
        self.connections.insert(server.id.clone(), true);

        match mode {
            ConnectionMode::Auto => {
                // Try a new terminal first, fall back to direct if none available.
                let available_terminal = detect_available_terminal();
                if available_terminal != AvailableTerminal::None {
                    self.launch_ssh_in_new_terminal(server, available_terminal).await
                } else {
                    self.launch_ssh_session(server).await
                }
            }
            ConnectionMode::NewTerminal => {
                let available_terminal = detect_available_terminal();
                if available_terminal != AvailableTerminal::None {
                    self.launch_ssh_in_new_terminal(server, available_terminal).await
                } else {
                    Err(anyhow::anyhow!("No terminal emulator available for new terminal mode. Available terminals: Ghostty, Alacritty, Kitty, Wezterm, GNOME Terminal, Konsole, XFCE Terminal, XTerm"))
                }
            }
            ConnectionMode::Direct => self.launch_ssh_session(server).await,
        }
    }
    
    /// Launch SSH session in a new terminal window.
    ///
    /// The ssh invocation is assembled as a vector of discrete argv elements
    /// (`build_ssh_args`) and wrapped into the terminal's launch form via
    /// `wrap_command`, so no untrusted server field is ever interpolated into a
    /// shell command string.
    async fn launch_ssh_in_new_terminal(&self, server: &ServerConnection, terminal: AvailableTerminal) -> Result<u32> {
        let cmd_name = terminal
            .command_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid terminal type"))?;

        let ssh_args = build_ssh_args(server);
        let terminal_args = terminal
            .wrap_command("ssh", &ssh_args)
            .ok_or_else(|| anyhow::anyhow!("Cannot generate SSH command for this terminal"))?;

        let mut terminal_cmd = Command::new(cmd_name);
        terminal_cmd.args(&terminal_args);

        // Spawn the terminal detached so its I/O can't interfere with the TUI.
        use std::process::Stdio;
        let child = terminal_cmd
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to spawn terminal process")?;

        let pid = child.id();

        // Reap the terminal in the background when it eventually exits. This lets
        // the window run independently of Ghost without leaking a zombie process
        // (the previous `mem::forget` left the child unwaited-for).
        std::thread::spawn(move || {
            let mut child = child;
            let _ = child.wait();
        });

        // Small delay to ensure the terminal has time to launch.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        Ok(pid)
    }
    
    /// Launch SSH session directly in the current terminal.
    async fn launch_ssh_session(&self, server: &ServerConnection) -> Result<u32> {
        // Same discrete-argv construction as the new-terminal path: ssh receives
        // each option as its own argument, so nothing is shell-interpreted.
        let mut ssh_cmd = Command::new("ssh");
        ssh_cmd.args(build_ssh_args(server));

        // Execute SSH directly in the current terminal
        self.execute_ssh_direct(ssh_cmd, server).await
    }
    
    /// Execute SSH directly in the current terminal
    async fn execute_ssh_direct(&self, mut ssh_cmd: std::process::Command, server: &ServerConnection) -> Result<u32> {
        use crossterm::terminal::{disable_raw_mode, enable_raw_mode, LeaveAlternateScreen, EnterAlternateScreen};
        use crossterm::ExecutableCommand;
        use std::io::stdout;
        
        // Suspend Ghost's TUI - disable raw mode and leave alternate screen
        if disable_raw_mode().is_err() {
            eprintln!("Warning: Failed to disable raw mode");
        }
        if stdout().execute(LeaveAlternateScreen).is_err() {
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
        if stdout().execute(EnterAlternateScreen).is_err() {
            eprintln!("Warning: Failed to enter alternate screen");
        }
        if enable_raw_mode().is_err() {
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
    pub auth_strength: AuthStrength,
    pub latency: Option<Duration>,
    pub error_message: Option<String>,
}


impl ConnectionTestResult {
    pub fn update_server_stats(&self, server: &mut ServerConnection) {
        server.health_status = self.status.clone();
        server.auth_strength = self.auth_strength.clone();
        
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ServerConnection;

    fn server(host: &str, user: &str, port: u16) -> ServerConnection {
        ServerConnection::new("test".to_string(), host.to_string(), port, user.to_string())
    }

    #[test]
    fn target_with_metacharacters_stays_one_argv_element() {
        // A host carrying shell syntax must remain a SINGLE argv element so it
        // can never be interpreted as a separate command.
        let s = server("evil; touch /tmp/pwned", "root", 2222);
        let args = build_ssh_args(&s);

        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"2222".to_string()));
        assert_eq!(args.last().unwrap(), "root@evil; touch /tmp/pwned");
        // The payload lives wholly inside one element, never split out.
        assert_eq!(args.iter().filter(|a| a.contains("touch")).count(), 1);
    }

    #[test]
    fn default_port_omits_p_flag() {
        let args = build_ssh_args(&server("example.com", "me", 22));
        assert!(!args.contains(&"-p".to_string()));
        assert_eq!(args.last().unwrap(), "me@example.com");
    }

    #[test]
    fn gnome_wrap_uses_argv_not_a_shell() {
        let s = server("evil$(id)", "root", 22);
        let args = build_ssh_args(&s);
        let wrapped = AvailableTerminal::GnomeTerminal
            .wrap_command("ssh", &args)
            .unwrap();

        // No shell is spawned: `bash -c` must not appear anywhere.
        assert!(!wrapped.iter().any(|a| a == "bash" || a == "-c"));
        assert_eq!(wrapped[0], "--");
        assert_eq!(wrapped[1], "ssh");
        // The dangerous host survives as a single, inert argument.
        assert!(wrapped.iter().any(|a| a == "root@evil$(id)"));
    }

    #[test]
    fn xfce_uses_x_flag_for_direct_argv() {
        let args = build_ssh_args(&server("h", "u", 22));
        let wrapped = AvailableTerminal::XfceTerminal
            .wrap_command("ssh", &args)
            .unwrap();
        // `-x` consumes argv directly; `-e` (string, shell-parsed) is avoided.
        assert_eq!(wrapped[0], "-x");
        assert_eq!(wrapped[1], "ssh");
    }

    #[test]
    fn mac_terminal_command_is_escaped() {
        // A host with a double-quote could otherwise close the AppleScript
        // `do script "..."` literal early.
        let s = server("h\"; rm -rf ~", "u", 22);
        let args = build_ssh_args(&s);
        let wrapped = AvailableTerminal::MacTerminal
            .wrap_command("ssh", &args)
            .unwrap();
        assert_eq!(wrapped[0], "-e");
        // The embedded quote is AppleScript-escaped (\").
        assert!(wrapped[1].contains("\\\""));
    }

    #[test]
    fn shell_quote_neutralizes_metacharacters() {
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
        assert_eq!(shell_quote("x; rm -rf ~"), "'x; rm -rf ~'");
    }
}
