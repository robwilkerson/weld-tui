use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Paragraph};

use weld_core::diff::BlockKind;
use weld_core::display::DisplayRow;

use crate::app::App;
use crate::theme::Theme;

/// Expand tabs to spaces for display, respecting tab stops.
const TAB_WIDTH: usize = 4;

pub fn expand_tabs(s: &str) -> String {
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

/// Gutter + code lines for one side of the diff.
struct SideLines {
    gutter: Vec<ratatui::text::Line<'static>>,
    code: Vec<ratatui::text::Line<'static>>,
}

#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
}

fn build_side_lines(
    display_rows: &[DisplayRow],
    lines: &[String],
    side: Side,
    digit_width: usize,
    gutter_width: u16,
    max_content_width: usize,
    theme: &Theme,
) -> SideLines {
    let mut gutter = Vec::with_capacity(display_rows.len());
    let mut code = Vec::with_capacity(display_rows.len());

    for row in display_rows {
        let line_idx = match side {
            Side::Left => row.left_line,
            Side::Right => row.right_line,
        };

        let is_diff = row.kind != BlockKind::Equal;
        let bg = if is_diff { theme.diff_bg } else { theme.bg };

        // Gutter always uses gutter_bg
        let gutter_style = Style::default().fg(theme.line_number_fg).bg(theme.gutter_bg);

        if let Some(idx) = line_idx {
            gutter.push(ratatui::text::Line::from(Span::styled(
                format!(" {:>width$} ", idx + 1, width = digit_width),
                gutter_style,
            )));
        } else {
            gutter.push(ratatui::text::Line::from(Span::styled(
                " ".repeat(gutter_width as usize),
                gutter_style,
            )));
        }

        // Code — pad diff lines to max width for a uniform highlight block
        let line_style = Style::default().fg(theme.fg).bg(bg);
        let text = if let Some(idx) = line_idx {
            format!(" {}", expand_tabs(&lines[idx]))
        } else {
            " ".to_string()
        };
        let padded = if is_diff {
            format!("{:<width$}", text, width = max_content_width)
        } else {
            text
        };
        code.push(ratatui::text::Line::from(padded).style(line_style));
    }

    SideLines { gutter, code }
}

/// Render a file side using display rows.
fn render_file_pane(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    dir: &str,
    filename: &str,
    lines: &[String],
    display_rows: &[DisplayRow],
    side: Side,
    scroll_y: u16,
    scroll_x: u16,
    digit_width: usize,
    max_content_width: usize,
    theme: &Theme,
) {
    let border_style = Style::default().fg(theme.gutter_bg);

    let [header_area, content_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .areas(area);

    // Header
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            format!(" {} ", dir),
            Style::default().fg(theme.status_bar_fg),
        ))
        .style(Style::default().bg(theme.bg));
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!(" {}", filename),
            Style::default().fg(theme.header_fg),
        ))
        .block(header_block),
        header_area,
    );

    // Content
    let content_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(theme.bg));
    let inner = content_block.inner(content_area);
    frame.render_widget(content_block, content_area);

    let gutter_width = (digit_width + 2) as u16;
    let [gutter_area, code_area] =
        Layout::horizontal([Constraint::Length(gutter_width), Constraint::Min(0)]).areas(inner);

    let side_lines = build_side_lines(display_rows, lines, side, digit_width, gutter_width, max_content_width, theme);

    frame.render_widget(
        Paragraph::new(side_lines.gutter).scroll((scroll_y, 0)),
        gutter_area,
    );
    frame.render_widget(
        Paragraph::new(side_lines.code).scroll((scroll_y, scroll_x)),
        code_area,
    );
}

/// Top-level UI: two file panes side by side + status bar.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let theme = &app.theme;

    let [body, status] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

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

    // Max content width across both panes so diff highlights align.
    let max_content_width = app
        .left_content
        .lines
        .iter()
        .chain(app.right_content.lines.iter())
        .map(|l| expand_tabs(l).len() + 1)
        .max()
        .unwrap_or(0);

    render_file_pane(
        frame,
        left_area,
        &app.left_dir,
        &app.left_filename,
        &app.left_content.lines,
        &app.display_rows,
        Side::Left,
        app.scroll_y,
        app.scroll_x,
        digit_width,
        max_content_width,
        theme,
    );
    render_file_pane(
        frame,
        right_area,
        &app.right_dir,
        &app.right_filename,
        &app.right_content.lines,
        &app.display_rows,
        Side::Right,
        app.scroll_y,
        app.scroll_x,
        digit_width,
        max_content_width,
        theme,
    );

    // Store viewport dimensions for scroll clamping.
    let header_height = 3u16;
    let content_height = left_area.height.saturating_sub(header_height);
    let inner_height = content_height.saturating_sub(2);
    let gutter_cols = (digit_width as u16) + 2;
    let inner_code_width = left_area
        .width
        .saturating_sub(2)
        .saturating_sub(gutter_cols);
    app.viewport_height = inner_height;
    app.viewport_width = inner_code_width;

    // Status bar
    let change_count = app.diff.change_blocks().len();
    let hint_text = if change_count == 0 {
        " Files are identical  [q → quit]".to_string()
    } else {
        format!(
            " {} change{}  [q → quit]",
            change_count,
            if change_count == 1 { "" } else { "s" }
        )
    };
    let hint_style = Style::default().fg(theme.status_bar_fg);
    frame.render_widget(
        Paragraph::new(ratatui::text::Line::from(vec![Span::styled(
            hint_text,
            hint_style,
        )]))
        .alignment(Alignment::Center),
        status,
    );
}
