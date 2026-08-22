mod app;
mod config;
mod forms;
mod health;
mod models;
mod ssh;
mod ssh_config;
mod themes;
mod tui;
mod ui;

use app::App;
use clap::Parser;
use ssh::ConnectionMode;
use std::time::Duration;

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

    /// Import hosts from an SSH config file (default: ~/.ssh/config) and exit
    #[arg(long, value_name = "PATH", num_args = 0..=1, default_missing_value = "")]
    import_ssh_config: Option<String>,

    /// With --import-ssh-config: list what would be imported without saving
    #[arg(long, requires = "import_ssh_config")]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Restore the terminal before the default panic handler prints. Without
    // this, any panic leaves the user in raw mode on the alternate screen with
    // an invisible cursor — an unusable shell that needs a blind `reset`.
    tui::install_panic_hook();

    if let Some(path) = args.import_ssh_config {
        let path = if path.is_empty() { None } else { Some(path) };
        return app::run_ssh_config_import(path.as_deref(), args.dry_run);
    }

    // Determine the connection mode from arguments
    let connection_mode = if args.new_terminal {
        ConnectionMode::NewTerminal
    } else if args.direct {
        ConnectionMode::Direct
    } else {
        args.connection_mode
    };

    let mut app = App::new(Duration::from_millis(50), connection_mode)?;
    app.run().await
}
