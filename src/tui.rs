//! Terminal setup / teardown, centralised so every exit path — clean quit,
//! error, panic, or a direct SSH handoff — leaves the user's terminal usable.

use anyhow::Result;
use crossterm::{
    cursor::Show,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{stdout, Stdout};

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Put the terminal into TUI mode and hand back a ratatui `Terminal`.
pub fn init() -> Result<Tui> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    Ok(terminal)
}

/// Return the terminal to normal mode.
///
/// Every step is best-effort and independent: if leaving the alternate screen
/// fails we still want raw mode disabled, because a raw-mode shell is the far
/// more painful failure for the user.
pub fn restore() {
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), DisableMouseCapture, LeaveAlternateScreen, Show);
}

/// Chain terminal restoration in front of the default panic handler so the
/// backtrace is actually readable and the shell is left in a sane state.
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        default_hook(info);
    }));
}
