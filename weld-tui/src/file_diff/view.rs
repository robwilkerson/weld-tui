use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Paragraph};

use weld_core::diff::{BlockKind, DiffResult};
use weld_core::display::DisplayRow;
use weld_core::inline_diff::InlineKind;

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
    diff: &DiffResult,
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
        let gutter_style = Style::default()
            .fg(theme.line_number_fg)
            .bg(theme.gutter_bg);

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

        // Code — for Replace rows with inline diffs, highlight changed characters.
        if row.kind == BlockKind::Replace {
            if let Some(inline) = inline_diff_for_row(row, side, diff) {
                let base_style = Style::default().fg(theme.fg).bg(theme.diff_bg);
                let emphasis_style = Style::default().fg(theme.fg).bg(theme.diff_emphasis_bg);

                let segments = match side {
                    Side::Left => &inline.left_segments,
                    Side::Right => &inline.right_segments,
                };

                let mut spans: Vec<Span<'static>> = Vec::new();
                spans.push(Span::styled(" ".to_string(), base_style)); // leading space

                for seg in segments {
                    let text = expand_tabs(&seg.text);
                    let style = match seg.kind {
                        InlineKind::Equal => base_style,
                        InlineKind::Changed => emphasis_style,
                    };
                    spans.push(Span::styled(text, style));
                }

                // Pad to max width for uniform highlight block.
                let current_width: usize = spans.iter().map(|s| s.content.len()).sum();
                if current_width < max_content_width {
                    spans.push(Span::styled(
                        " ".repeat(max_content_width - current_width),
                        base_style,
                    ));
                }

                code.push(ratatui::text::Line::from(spans));
                continue;
            }
        }

        // Default: uniform style for the whole line.
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

/// Look up the InlineDiff for a Replace row, if one exists.
fn inline_diff_for_row<'a>(
    row: &DisplayRow,
    side: Side,
    diff: &'a DiffResult,
) -> Option<&'a weld_core::inline_diff::InlineDiff> {
    let block = &diff.blocks[row.block_index];
    // Compute offset of this row within its block.
    let offset = match side {
        Side::Left => {
            let line = row.left_line?;
            line.checked_sub(block.left_range.start)?
        }
        Side::Right => {
            let line = row.right_line?;
            line.checked_sub(block.right_range.start)?
        }
    };
    block.inline_diffs.get(offset)
}

/// Shared parameters for rendering a file pane.
struct PaneContext<'a> {
    dir: &'a str,
    filename: &'a str,
    lines: &'a [String],
    display_rows: &'a [DisplayRow],
    diff: &'a DiffResult,
    side: Side,
    scroll_y: u16,
    scroll_x: u16,
    digit_width: usize,
    max_content_width: usize,
    theme: &'a Theme,
}

/// Render a file side using display rows.
fn render_file_pane(frame: &mut Frame, area: ratatui::layout::Rect, ctx: &PaneContext) {
    let theme = ctx.theme;
    let border_style = Style::default().fg(theme.gutter_bg);

    let [header_area, content_area] =
        Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(area);

    // Header
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            format!(" {} ", ctx.dir),
            Style::default().fg(theme.status_bar_fg),
        ))
        .style(Style::default().bg(theme.bg));
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!(" {}", ctx.filename),
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

    let gutter_width = (ctx.digit_width + 2) as u16;
    let [gutter_area, code_area] =
        Layout::horizontal([Constraint::Length(gutter_width), Constraint::Min(0)]).areas(inner);

    let side_lines = build_side_lines(
        ctx.display_rows,
        ctx.lines,
        ctx.side,
        ctx.digit_width,
        gutter_width,
        ctx.max_content_width,
        ctx.diff,
        theme,
    );

    frame.render_widget(
        Paragraph::new(side_lines.gutter).scroll((ctx.scroll_y, 0)),
        gutter_area,
    );
    frame.render_widget(
        Paragraph::new(side_lines.code).scroll((ctx.scroll_y, ctx.scroll_x)),
        code_area,
    );
}

/// Top-level UI: two file panes side by side + status bar.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let [body, status] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

    let (left_area, right_area, minimap_area) = if app.minimap_width > 0 {
        let [panes, minimap] =
            Layout::horizontal([Constraint::Min(0), Constraint::Length(app.minimap_width)])
                .areas(body);

        let [left, right] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .spacing(1)
                .areas(panes);

        (left, right, Some(minimap))
    } else {
        let [left, right] =
            Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                .spacing(1)
                .areas(body);

        (left, right, None)
    };

    let max_lines = app
        .left_content
        .lines()
        .len()
        .max(app.right_content.lines().len());
    let digit_width = max_lines.to_string().len().max(2);
    let header_height = 3u16;

    // Update viewport dimensions early so initial scroll has correct bounds.
    let content_height = left_area.height.saturating_sub(header_height);
    let inner_height = content_height.saturating_sub(2);
    let gutter_cols = (digit_width as u16) + 2;
    let inner_code_width = left_area
        .width
        .saturating_sub(2)
        .saturating_sub(gutter_cols);
    app.viewport.height = inner_height;
    app.viewport.width = inner_code_width;
    app.viewport
        .clamp(app.display_rows.len(), app.max_content_width as u16);

    // On first render, scroll to center the first change block.
    if app.needs_initial_scroll {
        app.needs_initial_scroll = false;
        crate::input::scroll_to_current_block(app);
    }

    // All mutation done — borrow theme for rendering.
    let theme = &app.theme;
    let max_content_width = app.max_content_width;

    render_file_pane(
        frame,
        left_area,
        &PaneContext {
            dir: &app.left_dir,
            filename: &app.left_filename,
            lines: app.left_content.lines(),
            display_rows: &app.display_rows,
            diff: &app.diff,
            side: Side::Left,
            scroll_y: app.viewport.scroll_y,
            scroll_x: app.viewport.scroll_x,
            digit_width,
            max_content_width,
            theme,
        },
    );
    render_file_pane(
        frame,
        right_area,
        &PaneContext {
            dir: &app.right_dir,
            filename: &app.right_filename,
            lines: app.right_content.lines(),
            display_rows: &app.display_rows,
            diff: &app.diff,
            side: Side::Right,
            scroll_y: app.viewport.scroll_y,
            scroll_x: app.viewport.scroll_x,
            digit_width,
            max_content_width,
            theme,
        },
    );

    // Dot indicator — render ● in the 1-column gap, centered on the current block.
    if !app.change_block_indices.is_empty() {
        let block_index = app.change_block_indices[app.current_block];

        // Find the display row range for this block.
        let block_rows: Vec<usize> = app
            .display_rows
            .iter()
            .enumerate()
            .filter(|(_, r)| r.block_index == block_index)
            .map(|(i, _)| i)
            .collect();

        if let (Some(&first), Some(&last)) = (block_rows.first(), block_rows.last()) {
            let center_row = (first + last) / 2;
            let scroll_y = app.viewport.scroll_y as usize;

            if center_row >= scroll_y && center_row < scroll_y + inner_height as usize {
                let screen_row = (center_row - scroll_y) as u16;
                // Gap column is between left_area and right_area.
                let gap_x = left_area.x + left_area.width;
                // Offset by header (3) + top border (1) to align with content.
                let gap_y = left_area.y + header_height + 1 + screen_row;
                let dot_style = Style::default().fg(theme.gutter_dot);
                frame.buffer_mut()[(gap_x, gap_y)]
                    .set_symbol("●")
                    .set_style(dot_style);
            }
        }
    }

    // Minimap — aligned to the content viewport, not the full pane height.
    if let Some(minimap_area) = minimap_area {
        let content_top = header_height + 1; // header + top border
        let minimap_content = ratatui::layout::Rect {
            x: minimap_area.x,
            y: minimap_area.y + content_top,
            width: minimap_area.width,
            height: minimap_area
                .height
                .saturating_sub(content_top)
                .min(inner_height),
        };
        super::minimap::render(
            frame.buffer_mut(),
            minimap_content,
            &app.display_rows,
            app.viewport.scroll_y,
            app.viewport.height,
            theme,
        );
    }

    // Status bar
    let change_count = app.change_count;
    let hint_text = if change_count == 0 {
        " Files are identical  [q → quit]".to_string()
    } else {
        format!(" {}/{}  [q → quit]", app.current_block + 1, change_count,)
    };
    let hint_style = Style::default().fg(theme.status_bar_fg);
    frame.render_widget(
        Paragraph::new(ratatui::text::Line::from(vec![Span::styled(
            hint_text, hint_style,
        )]))
        .alignment(Alignment::Center),
        status,
    );
}
