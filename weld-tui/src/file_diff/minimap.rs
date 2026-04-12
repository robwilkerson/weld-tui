use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use weld_core::file::diff::BlockKind;
use weld_core::file::display::DisplayRow;

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

/// For a given minimap row, determine whether it contains the active diff block.
pub fn is_active_at_minimap_row(
    display_rows: &[DisplayRow],
    minimap_row: u16,
    minimap_height: u16,
    active_block_index: Option<usize>,
) -> bool {
    let active = match active_block_index {
        Some(idx) => idx,
        None => return false,
    };
    let total = display_rows.len();
    if total == 0 || minimap_height == 0 {
        return false;
    }

    let start = (minimap_row as usize) * total / (minimap_height as usize);
    let end = ((minimap_row as usize) + 1) * total / (minimap_height as usize);
    let end = end.max(start + 1).min(total);

    display_rows[start..end]
        .iter()
        .any(|r| r.block_index == active && r.kind != BlockKind::Equal)
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
    let height = height.max(1).min(mh.saturating_sub(top).max(1));

    (top as u16, height as u16)
}

/// Render the minimap into the given area of the buffer.
pub fn render(
    buf: &mut Buffer,
    area: Rect,
    display_rows: &[DisplayRow],
    scroll_y: u16,
    viewport_height: u16,
    active_block_index: Option<usize>,
    theme: &Theme,
) {
    let minimap_height = area.height;
    if minimap_height == 0 || area.width == 0 {
        return;
    }

    let bg_style = Style::default().bg(theme.minimap_bg);
    let diff_style = Style::default().bg(theme.minimap_diff);
    let diff_active_style = Style::default().bg(theme.minimap_diff_active);

    // Fill background and diff markers.
    for row in 0..minimap_height {
        let style =
            if is_active_at_minimap_row(display_rows, row, minimap_height, active_block_index) {
                diff_active_style
            } else if is_diff_at_minimap_row(display_rows, row, minimap_height) {
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

    // Draw viewport indicator as │ overlay, preserving diff background.
    let (vp_top, vp_height) = viewport_indicator(
        scroll_y,
        viewport_height,
        display_rows.len(),
        minimap_height,
    );
    let vp_bottom = vp_top + vp_height.saturating_sub(1);
    let vp_fg = theme.minimap_viewport_fg;

    for row in vp_top..=vp_bottom {
        let y = area.y + row;
        if y >= area.y + area.height {
            break;
        }
        for col in 0..area.width {
            let cell = &mut buf[(area.x + col, y)];
            cell.set_symbol("│");
            cell.fg = vp_fg;
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

    /// Build display rows with realistic block indices (new block index on each kind transition).
    fn make_rows_with_blocks(kinds: &[BlockKind]) -> Vec<DisplayRow> {
        let mut rows = Vec::new();
        let mut block_index = 0;
        for (i, &kind) in kinds.iter().enumerate() {
            if i > 0 && kind != kinds[i - 1] {
                block_index += 1;
            }
            rows.push(DisplayRow {
                left_line: Some(i),
                right_line: Some(i),
                kind,
                block_index,
            });
        }
        rows
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

    #[test]
    fn viewport_indicator_scrolled_to_end() {
        // 100 display rows, viewport 20, scrolled to max (80), minimap 50.
        // Indicator should not overflow the minimap.
        let (top, height) = viewport_indicator(80, 20, 100, 50);
        assert!(
            top + height <= 50,
            "indicator must not overflow minimap: top={top}, height={height}"
        );
        assert!(height >= 1, "indicator height must be at least 1");
    }

    #[test]
    fn active_none_returns_false() {
        let rows = make_rows(&[BlockKind::Replace; 10]);
        assert!(!is_active_at_minimap_row(&rows, 0, 10, None));
    }

    #[test]
    fn active_matches_correct_block() {
        // Equal(block 0) → Replace(block 1) → Equal(block 2) → Insert(block 3)
        let kinds = vec![
            BlockKind::Equal,
            BlockKind::Equal,
            BlockKind::Replace,
            BlockKind::Replace,
            BlockKind::Equal,
            BlockKind::Equal,
            BlockKind::Insert,
        ];
        let rows = make_rows_with_blocks(&kinds);
        // 7 display rows, minimap height 7 → 1:1 mapping.

        // Block 1 (Replace) is active — rows 2 and 3 should match.
        assert!(!is_active_at_minimap_row(&rows, 1, 7, Some(1)));
        assert!(is_active_at_minimap_row(&rows, 2, 7, Some(1)));
        assert!(is_active_at_minimap_row(&rows, 3, 7, Some(1)));
        assert!(!is_active_at_minimap_row(&rows, 4, 7, Some(1)));

        // Block 3 (Insert) is active — only row 6 should match.
        assert!(!is_active_at_minimap_row(&rows, 5, 7, Some(3)));
        assert!(is_active_at_minimap_row(&rows, 6, 7, Some(3)));
    }

    #[test]
    fn active_ignores_equal_rows_with_matching_block_index() {
        // Equal rows happen to have block_index 0, but active should not highlight them.
        let rows = make_rows(&[BlockKind::Equal; 5]);
        assert!(!is_active_at_minimap_row(&rows, 0, 5, Some(0)));
    }
}
