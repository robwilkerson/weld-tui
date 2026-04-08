use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use weld_core::diff::BlockKind;
use weld_core::display::DisplayRow;

use crate::theme::Theme;

/// For a given minimap row, determine whether it represents a diff region.
/// When multiple display rows map to one minimap row, returns true if ANY is a diff.
pub fn is_diff_at_minimap_row(
    display_rows: &[DisplayRow],
    minimap_row: u16,
    minimap_height: u16,
) -> bool {
    let total = display_rows.len();
    if total == 0 || minimap_height == 0 {
        return false;
    }

    // Range of display rows that map to this minimap row.
    let start = (minimap_row as usize) * total / (minimap_height as usize);
    let end = ((minimap_row as usize) + 1) * total / (minimap_height as usize);
    let end = end.max(start + 1).min(total);

    display_rows[start..end]
        .iter()
        .any(|r| r.kind != BlockKind::Equal)
}

/// Compute the viewport indicator's top row and height within the minimap.
/// Returns (top, height) in minimap rows.
pub fn viewport_indicator(
    scroll_y: u16,
    viewport_height: u16,
    total_display_rows: usize,
    minimap_height: u16,
) -> (u16, u16) {
    if total_display_rows == 0 || minimap_height == 0 {
        return (0, minimap_height);
    }

    let total = total_display_rows as u32;
    let mh = minimap_height as u32;

    let top = (scroll_y as u32) * mh / total;
    let height = (viewport_height as u32) * mh / total;
    let height = height.max(1).min(mh - top);

    (top as u16, height as u16)
}

/// Render the minimap into the given area of the buffer.
pub fn render(
    buf: &mut Buffer,
    area: Rect,
    display_rows: &[DisplayRow],
    scroll_y: u16,
    viewport_height: u16,
    theme: &Theme,
) {
    let minimap_height = area.height;
    if minimap_height == 0 || area.width == 0 {
        return;
    }

    let bg_style = Style::default().bg(theme.minimap_bg);
    let diff_style = Style::default().bg(theme.minimap_diff);

    // Fill background and diff markers.
    for row in 0..minimap_height {
        let style = if is_diff_at_minimap_row(display_rows, row, minimap_height) {
            diff_style
        } else {
            bg_style
        };
        for col in 0..area.width {
            buf[(area.x + col, area.y + row)]
                .set_symbol(" ")
                .set_style(style);
        }
    }

    // Draw viewport indicator outline.
    let (vp_top, vp_height) = viewport_indicator(
        scroll_y,
        viewport_height,
        display_rows.len(),
        minimap_height,
    );
    let vp_bottom = vp_top + vp_height.saturating_sub(1);
    let vp_style = Style::default().fg(theme.minimap_viewport_fg);

    for row in vp_top..=vp_bottom {
        let y = area.y + row;
        if y >= area.y + area.height {
            break;
        }

        // Left edge
        if row == vp_top {
            buf[(area.x, y)].set_symbol("┌").set_style(vp_style);
        } else if row == vp_bottom {
            buf[(area.x, y)].set_symbol("└").set_style(vp_style);
        } else {
            buf[(area.x, y)].set_symbol("│").set_style(vp_style);
        }

        // Right edge (only if width > 1)
        if area.width > 1 {
            let rx = area.x + area.width - 1;
            if row == vp_top {
                buf[(rx, y)].set_symbol("┐").set_style(vp_style);
            } else if row == vp_bottom {
                buf[(rx, y)].set_symbol("┘").set_style(vp_style);
            } else {
                buf[(rx, y)].set_symbol("│").set_style(vp_style);
            }
        }

        // Top/bottom edges (fill between corners)
        if (row == vp_top || row == vp_bottom) && area.width > 2 {
            for col in 1..(area.width - 1) {
                buf[(area.x + col, y)].set_symbol("─").set_style(vp_style);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rows(kinds: &[BlockKind]) -> Vec<DisplayRow> {
        kinds
            .iter()
            .enumerate()
            .map(|(i, &kind)| DisplayRow {
                left_line: Some(i),
                right_line: Some(i),
                kind,
                block_index: 0,
            })
            .collect()
    }

    #[test]
    fn empty_display_rows_returns_false() {
        assert!(!is_diff_at_minimap_row(&[], 0, 10));
    }

    #[test]
    fn all_equal_rows_returns_false() {
        let rows = make_rows(&[BlockKind::Equal; 20]);
        for r in 0..10 {
            assert!(!is_diff_at_minimap_row(&rows, r, 10));
        }
    }

    #[test]
    fn diff_row_maps_to_correct_minimap_row() {
        // 10 display rows, minimap height 10 → 1:1 mapping.
        let mut kinds = vec![BlockKind::Equal; 10];
        kinds[5] = BlockKind::Replace;
        let rows = make_rows(&kinds);

        assert!(!is_diff_at_minimap_row(&rows, 4, 10));
        assert!(is_diff_at_minimap_row(&rows, 5, 10));
        assert!(!is_diff_at_minimap_row(&rows, 6, 10));
    }

    #[test]
    fn multiple_display_rows_per_minimap_row() {
        // 20 display rows, minimap height 10 → 2 display rows per minimap row.
        // Put a diff at display row 11 → should appear at minimap row 5.
        let mut kinds = vec![BlockKind::Equal; 20];
        kinds[11] = BlockKind::Insert;
        let rows = make_rows(&kinds);

        assert!(!is_diff_at_minimap_row(&rows, 4, 10));
        assert!(is_diff_at_minimap_row(&rows, 5, 10));
        assert!(!is_diff_at_minimap_row(&rows, 6, 10));
    }

    #[test]
    fn fewer_display_rows_than_minimap_rows() {
        // 5 display rows, minimap height 10 → some minimap rows share display rows.
        let mut kinds = vec![BlockKind::Equal; 5];
        kinds[2] = BlockKind::Delete;
        let rows = make_rows(&kinds);

        // Display row 2 maps to minimap rows 4 and 5 (2*10/5=4, 3*10/5=6).
        assert!(!is_diff_at_minimap_row(&rows, 3, 10));
        assert!(is_diff_at_minimap_row(&rows, 4, 10));
        assert!(is_diff_at_minimap_row(&rows, 5, 10));
        assert!(!is_diff_at_minimap_row(&rows, 6, 10));
    }

    #[test]
    fn viewport_indicator_full_file_visible() {
        // 20 display rows, viewport 20, minimap 10 → indicator spans full height.
        let (top, height) = viewport_indicator(0, 20, 20, 10);
        assert_eq!(top, 0);
        assert_eq!(height, 10);
    }

    #[test]
    fn viewport_indicator_half_scrolled() {
        // 100 display rows, viewport 20, scroll_y 50, minimap 50.
        let (top, height) = viewport_indicator(50, 20, 100, 50);
        assert_eq!(top, 25);
        assert_eq!(height, 10);
    }

    #[test]
    fn viewport_indicator_minimum_height() {
        // Very large file, tiny viewport → indicator height must be at least 1.
        let (_, height) = viewport_indicator(0, 1, 10000, 20);
        assert_eq!(height, 1);
    }

    #[test]
    fn viewport_indicator_empty_file() {
        let (top, height) = viewport_indicator(0, 10, 0, 20);
        assert_eq!(top, 0);
        assert_eq!(height, 20);
    }
}
