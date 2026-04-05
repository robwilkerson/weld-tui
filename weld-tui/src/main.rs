mod app;
mod event;
mod file_diff;
mod input;
mod theme;

use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{Event, KeyEventKind};

use crate::app::App;

#[derive(Parser)]
#[command(name = "weld", version, about = "TUI diff and merge tool")]
struct Cli {
    /// Left file to compare
    left: PathBuf,
    /// Right file to compare
    right: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let mut app = App::new(cli.left, cli.right)?;

    // Restore the terminal on panic so it doesn't stay in raw mode.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        default_hook(info);
    }));

    let mut terminal = ratatui::init();

    let result = main_loop(&mut terminal, &mut app);

    ratatui::restore();
    result
}

fn main_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    while app.running {
        terminal.draw(|frame| file_diff::view::draw(frame, &mut *app))?;

        if let Some(Event::Key(key)) = event::poll_event(Duration::from_millis(50))?
            && key.kind == KeyEventKind::Press
        {
            input::handle_key(app, key.code);
        } else {
            // Clear pending key state (e.g., `gg`) on non-key events like resize
            app.pending_g = false;
        }
    }
    Ok(())
}
