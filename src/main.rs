mod app;
mod colors;
mod config;
mod forms;
mod health;
mod models;
mod ssh;
mod themes;
mod ui;

use app::App;
use clap::Parser;
use std::time::Duration;
use ssh::ConnectionMode;

#[derive(Parser, Debug)]
#[command(name = "ghost")]
#[command(about = "A modern SSH connection manager with terminal UI")]
#[command(version)]
struct Args {
    /// Connection mode preference
    #[arg(long, value_enum, default_value_t = ConnectionMode::Auto)]
    connection_mode: ConnectionMode,
    
    /// Force new terminal for SSH connections (shorthand for --connection-mode new-terminal)
    #[arg(long, conflicts_with = "connection_mode")]
    new_terminal: bool,
    
    /// Force direct connection in current terminal (shorthand for --connection-mode direct)
    #[arg(long, conflicts_with = "connection_mode")]
    direct: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    // Determine the connection mode from arguments
    let connection_mode = if args.new_terminal {
        ConnectionMode::NewTerminal
    } else if args.direct {
        ConnectionMode::Direct
    } else {
        args.connection_mode
    };
    
    eprintln!("🚀 Starting Ghost SSH Manager with connection mode: {:?}...", connection_mode);
    let mut app = App::new(Duration::from_millis(50), connection_mode)?;
    eprintln!("✅ App created successfully");
    app.run().await?;
    eprintln!("✅ App finished running");
    Ok(())
}
