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

    let mut app = App::new(cli.left, cli.right)?;
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
        terminal.draw(|frame| ui(frame, &mut *app))?;

        if let Some(Event::Key(key)) = event::poll_event(Duration::from_millis(50))?
            && key.kind == KeyEventKind::Press
        {
            let max_y = app
                .left_content
                .lines
                .len()
                .max(app.right_content.lines.len())
                .saturating_sub(1) as u16;
            let max_x = app
                .left_content
                .lines
                .iter()
                .chain(app.right_content.lines.iter())
                .map(|l| expand_tabs(l).len())
                .max()
                .unwrap_or(0) as u16;
            match key.code {
                KeyCode::Char('q') => app.running = false,
                KeyCode::Char('j') | KeyCode::Down => {
                    app.scroll_y = app.scroll_y.saturating_add(1).min(max_y);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    app.scroll_y = app.scroll_y.saturating_sub(1);
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    app.scroll_x = app.scroll_x.saturating_add(2).min(max_x);
                }
                KeyCode::Char('h') | KeyCode::Left => {
                    app.scroll_x = app.scroll_x.saturating_sub(2);
                }
                KeyCode::Char('0') | KeyCode::Home => {
                    app.scroll_x = 0;
                }
                KeyCode::Char('$') | KeyCode::End => {
                    // Scroll so the longest line's end sits at the right edge
                    app.scroll_x = max_x.saturating_sub(app.viewport_width);
                }
                KeyCode::Char('g') => {
                    app.scroll_y = 0;
                }
                KeyCode::Char('G') => {
                    // Scroll so the last line sits at the bottom of the viewport
                    app.scroll_y = max_y.saturating_sub(app.viewport_height.saturating_sub(1));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// Data needed to render one side of the diff.
struct PaneData<'a> {
    dir: &'a str,
    filename: &'a str,
    lines: &'a [String],
    scroll_y: u16,
    scroll_x: u16,
    /// Width of line number column (based on max line count across both files).
    digit_width: usize,
    /// Max line count across both files (for extending the gutter).
    max_lines: usize,
}

/// Expand tabs to spaces for display, respecting tab stops.
/// The original content is never modified — this is render-only.
const TAB_WIDTH: usize = 4;

fn expand_tabs(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut col = 0;
    for ch in s.chars() {
        if ch == '\t' {
            let spaces = TAB_WIDTH - (col % TAB_WIDTH);
            result.extend(std::iter::repeat_n(' ', spaces));
            col += spaces;
        } else {
            result.push(ch);
            col += 1;
        }
    }
    result
}

/// Render a file side: header block + content block with line number gutter.
fn render_file_pane(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    pane: &PaneData,
    theme: &crate::theme::Theme,
) {
    let border_style = Style::default().fg(theme.gutter_bg);

    // Two blocks stacked directly — their adjacent borders create a thin gap
    let [header_area, content_area] = Layout::vertical([
        Constraint::Length(3), // border + filename + border
        Constraint::Min(0),
    ])
    .areas(area);

    // Header block — directory as border title, filename as content
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            format!(" {} ", pane.dir),
            Style::default().fg(theme.status_bar_fg),
        ))
        .style(Style::default().bg(theme.bg));
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!(" {}", pane.filename),
            Style::default().fg(theme.header_fg),
        ))
        .block(header_block),
        header_area,
    );

    // Content pane — full borders, 1-cell gap on all sides (consistent).
    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(theme.bg));
    let inner = content_block.inner(content_area);
    frame.render_widget(content_block, content_area);

    let lines = pane.lines;
    let digit_width = pane.digit_width;
    let gutter_width = (digit_width + 2) as u16; // 1 pad each side

    // Split inner area: fixed gutter | scrollable code
    let [gutter_area, code_area] =
        Layout::horizontal([Constraint::Length(gutter_width), Constraint::Min(0)]).areas(inner);

    // Gutter — fixed, scrolls only vertically (synced with code)
    let gutter_style = Style::default()
        .fg(theme.line_number_fg)
        .bg(theme.gutter_bg);
    let visible_rows = gutter_area.height as usize;
    let total_rows = pane.max_lines.max(visible_rows);
    let gutter_lines: Vec<ratatui::text::Line> = (0..total_rows)
        .map(|i| {
            if i < lines.len() {
                ratatui::text::Line::from(Span::styled(
                    format!(" {:>width$} ", i + 1, width = digit_width),
                    gutter_style,
                ))
            } else {
                ratatui::text::Line::from(Span::styled(
                    " ".repeat(gutter_width as usize),
                    gutter_style,
                ))
            }
        })
        .collect();
    frame.render_widget(
        Paragraph::new(gutter_lines).scroll((pane.scroll_y, 0)),
        gutter_area,
    );

    // Code — scrolls both axes
    let content_style = Style::default().fg(theme.fg);
    let code_lines: Vec<ratatui::text::Line> = lines
        .iter()
        .map(|l| {
            let expanded = expand_tabs(l);
            ratatui::text::Line::from(Span::styled(format!(" {}", expanded), content_style))
        })
        .collect();
    frame.render_widget(
        Paragraph::new(code_lines).scroll((pane.scroll_y, pane.scroll_x)),
        code_area,
    );
}

fn ui(frame: &mut Frame, app: &mut App) {
    let theme = &app.theme;

    // Outer vertical: body (fill) | status bar (1)
    let [body, status] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

    // Body: left side | gap | right side
    let [left_area, right_area] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .spacing(1)
            .areas(body);

    let max_lines = app
        .left_content
        .lines
        .len()
        .max(app.right_content.lines.len());
    let digit_width = max_lines.to_string().len().max(2);

    let left_pane = PaneData {
        dir: &app.left_dir,
        filename: &app.left_filename,
        lines: &app.left_content.lines,
        scroll_y: app.scroll_y,
        scroll_x: app.scroll_x,
        digit_width,
        max_lines,
    };
    let right_pane = PaneData {
        dir: &app.right_dir,
        filename: &app.right_filename,
        lines: &app.right_content.lines,
        scroll_y: app.scroll_y,
        scroll_x: app.scroll_x,
        digit_width,
        max_lines,
    };
    render_file_pane(frame, left_area, &left_pane, theme);
    render_file_pane(frame, right_area, &right_pane, theme);

    // Store viewport dimensions for scroll clamping.
    // Compute from the left pane layout (both sides are identical).
    let header_height = 3u16;
    let content_height = left_area.height.saturating_sub(header_height);
    // inner = content minus 2 (top/bottom border)
    let inner_height = content_height.saturating_sub(2);
    let gutter_cols = (digit_width as u16) + 2;
    // inner width = pane width minus 2 (left/right border) minus gutter
    let inner_code_width = left_area
        .width
        .saturating_sub(2)
        .saturating_sub(gutter_cols);
    app.viewport_height = inner_height;
    app.viewport_width = inner_code_width;

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
