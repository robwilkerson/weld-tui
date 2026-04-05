mod app;
mod event;
mod theme;

use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Paragraph};

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

    let mut terminal = ratatui::init();
    let mut app = App::new(cli.left, cli.right);

    let result = main_loop(&mut terminal, &mut app);

    ratatui::restore();
    result
}

fn main_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
) -> Result<(), Box<dyn std::error::Error>> {
    while app.running {
        terminal.draw(|frame| ui(frame, app))?;

        if let Some(Event::Key(key)) = event::poll_event(Duration::from_millis(50))?
            && key.kind == KeyEventKind::Press
            && key.code == KeyCode::Char('q')
        {
            app.running = false;
        }
    }
    Ok(())
}

fn ui(frame: &mut Frame, app: &App) {
    let theme = &app.theme;
    let border_style = Style::default().fg(theme.gutter_bg);

    // Outer vertical: body (fill) | status bar (1)
    let [body, status] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

    // Body: left pane | gutter gap | right pane
    let [left_area, right_area] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .spacing(1)
            .areas(body);

    // Left pane — bordered block with file title
    let left_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            format!(" {} ", app.left_title),
            Style::default().fg(theme.header_fg),
        ))
        .style(Style::default().bg(theme.bg));
    frame.render_widget(left_block, left_area);

    // Right pane — bordered block with file title
    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            format!(" {} ", app.right_title),
            Style::default().fg(theme.header_fg),
        ))
        .style(Style::default().bg(theme.bg));
    frame.render_widget(right_block, right_area);

    // Status bar — dim keybinding hints
    let hint_style = Style::default().fg(theme.status_bar_fg);
    frame.render_widget(
        Paragraph::new(ratatui::text::Line::from(vec![Span::styled(
            " [q → quit]",
            hint_style,
        )]))
        .alignment(Alignment::Center),
        status,
    );
}
