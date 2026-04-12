use crossterm::event::{KeyCode, KeyEvent};

use crate::app::App;

/// Handle a key press, updating app state.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    let total_rows = app.display_rows.len();
    let max_x = app.max_content_width as u16;
    let code = key.code;

    // Handle `gg` — two consecutive `g` presses jump to first change block
    if app.input.pending_g {
        app.input.pending_g = false;
        if code == KeyCode::Char('g') {
            first_block(app);
            return;
        }
    }

    match code {
        KeyCode::Char('q') => app.running = false,
        KeyCode::Char('j') | KeyCode::Down => {
            app.viewport.scroll_down(total_rows);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.viewport.scroll_up();
        }
        KeyCode::Char('J') => {
            next_block(app);
        }
        KeyCode::Char('K') => {
            prev_block(app);
        }
        KeyCode::Char('l') | KeyCode::Right => {
            app.viewport.scroll_right(2, max_x);
        }
        KeyCode::Char('h') | KeyCode::Left => {
            app.viewport.scroll_left(2);
        }
        KeyCode::Char('0') | KeyCode::Home => {
            app.viewport.scroll_to_left();
        }
        KeyCode::Char('$') | KeyCode::End => {
            app.viewport.scroll_to_right(max_x);
        }
        KeyCode::Char('g') => {
            app.input.pending_g = true;
        }
        KeyCode::Char('G') => {
            last_block(app);
        }
        KeyCode::Char('L') => {
            app.copy_left_to_right();
            scroll_to_current_block(app);
        }
        KeyCode::Char('H') => {
            app.copy_right_to_left();
            scroll_to_current_block(app);
        }
        _ => {}
    }
}

/// Jump to the first change block.
fn first_block(app: &mut App) {
    if app.change_block_indices.is_empty() {
        return;
    }
    app.current_block = 0;
    scroll_to_current_block(app);
}

/// Jump to the last change block.
fn last_block(app: &mut App) {
    if app.change_block_indices.is_empty() {
        return;
    }
    app.current_block = app.change_block_indices.len() - 1;
    scroll_to_current_block(app);
}

/// Advance to the next change block (clamped at last).
fn next_block(app: &mut App) {
    if app.change_block_indices.is_empty() {
        return;
    }
    if app.current_block < app.change_block_indices.len() - 1 {
        app.current_block += 1;
    }
    scroll_to_current_block(app);
}

/// Retreat to the previous change block (clamped at first).
fn prev_block(app: &mut App) {
    if app.change_block_indices.is_empty() {
        return;
    }
    app.current_block = app.current_block.saturating_sub(1);
    scroll_to_current_block(app);
}

/// Scroll so the current change block is vertically centered in the viewport.
pub fn scroll_to_current_block(app: &mut App) {
    if app.change_block_indices.is_empty() || app.viewport.height == 0 {
        return;
    }

    let block_index = app.change_block_indices[app.current_block];
    let block_start = app
        .display_rows
        .iter()
        .position(|r| r.block_index == block_index)
        .unwrap_or(0) as u16;

    let half_vp = app.viewport.height / 2;
    let target = block_start.saturating_sub(half_vp);
    let max = app.viewport.scroll_y_max(app.display_rows.len());
    app.viewport.scroll_y = target.min(max);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;
    use weld_core::diff::{BlockKind, DiffResult};
    use weld_core::display;
    use weld_core::file_io::FileContent;

    use crate::app::InputState;
    use crate::file_diff::view::expand_tabs;
    use crate::theme::Theme;
    use crate::viewport::Viewport;

    /// Build a plain KeyEvent (no modifiers) from a KeyCode.
    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, crossterm::event::KeyModifiers::NONE)
    }

    fn test_app(left_lines: &[&str], right_lines: &[&str], viewport: (u16, u16)) -> App {
        let left_content = FileContent::from_lines(left_lines);
        let right_content = FileContent::from_lines(right_lines);
        let diff = DiffResult::compute(&left_content, &right_content);
        let display_rows = display::build_display_rows(&diff);

        let max_content_width = left_content
            .lines()
            .iter()
            .chain(right_content.lines().iter())
            .map(|l| expand_tabs(l).len() + 1)
            .max()
            .unwrap_or(0);

        let change_block_indices: Vec<usize> = diff
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.kind != BlockKind::Equal)
            .map(|(i, _)| i)
            .collect();
        let change_count = change_block_indices.len();

        App {
            theme: Theme::default(),
            running: true,
            left_dir: String::new(),
            left_filename: String::new(),
            right_dir: String::new(),
            right_filename: String::new(),
            left_content,
            right_content,
            diff,
            display_rows,
            max_content_width,
            change_count,
            change_block_indices,
            current_block: 0,
            needs_initial_scroll: false,
            viewport: Viewport {
                scroll_y: 0,
                scroll_x: 0,
                height: viewport.1,
                width: viewport.0,
            },
            input: InputState::default(),
            minimap_width: 1,
            left_dirty: false,
            right_dirty: false,
        }
    }

    #[test]
    fn j_caps_at_viewport_bottom() {
        let lines = vec!["line"; 20];
        let mut app = test_app(&lines, &lines, (40, 10));

        for _ in 0..25 {
            handle_key(&mut app, key(KeyCode::Char('j')));
        }

        // 20 identical lines = 20 display rows. max scroll = 20 - 10 = 10
        assert_eq!(app.viewport.scroll_y, 10);
    }

    #[test]
    fn j_scrolls_to_max() {
        let lines = vec!["content"; 50];
        let mut app = test_app(&lines, &lines, (40, 20));

        for _ in 0..100 {
            handle_key(&mut app, key(KeyCode::Char('j')));
        }

        assert_eq!(app.viewport.scroll_y, app.viewport.scroll_y_max(50));
    }

    #[test]
    fn dollar_uses_global_max_across_both_files() {
        let mut left: Vec<&str> = vec!["short"; 51];
        let long = &"a".repeat(200);
        left[50] = long;

        let mut app = test_app(&left, &["short"; 51], (40, 10));

        handle_key(&mut app, key(KeyCode::Char('$')));

        // Global max = 201 (200 + leading space), viewport = 40 → scroll_x = 161
        assert_eq!(
            app.viewport.scroll_x, 161,
            "$ should use global max even if long line is off-screen"
        );
    }

    #[test]
    fn dollar_adapts_when_scrolled_to_long_line() {
        let long = "x".repeat(100);
        let mut lines: Vec<String> = vec!["short".to_string(); 20];
        lines[15] = long;
        let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();

        let mut app = test_app(&line_refs, &line_refs, (40, 10));

        app.viewport.scroll_y = 10;
        handle_key(&mut app, key(KeyCode::Char('$')));

        // max_x = 101 (100 + leading space), viewport_width = 40 → scroll_x = 61
        assert_eq!(
            app.viewport.scroll_x, 61,
            "$ should use the long line now visible"
        );
    }

    #[test]
    fn gg_jumps_to_first_block() {
        let left = vec!["a", "b", "c", "d", "e", "f", "g", "h"];
        let right = vec!["a", "X", "c", "d", "e", "Y", "g", "h"];
        let mut app = test_app(&left, &right, (40, 10));
        app.current_block = 1;

        handle_key(&mut app, key(KeyCode::Char('g')));
        handle_key(&mut app, key(KeyCode::Char('g')));

        assert_eq!(app.current_block, 0);
    }

    #[test]
    fn shift_g_jumps_to_last_block() {
        let left = vec!["a", "b", "c", "d", "e", "f", "g", "h"];
        let right = vec!["a", "X", "c", "d", "e", "Y", "g", "h"];
        let mut app = test_app(&left, &right, (40, 10));

        assert_eq!(app.current_block, 0);
        handle_key(&mut app, key(KeyCode::Char('G')));
        assert_eq!(app.current_block, app.change_block_indices.len() - 1);
    }

    #[test]
    fn l_and_dollar_agree_on_max_scroll() {
        let long = "x".repeat(100);
        let mut app = test_app(&[&long], &[&long], (40, 10));

        handle_key(&mut app, key(KeyCode::Char('$')));
        let dollar_pos = app.viewport.scroll_x;

        app.viewport.scroll_x = 0;
        for _ in 0..200 {
            handle_key(&mut app, key(KeyCode::Char('l')));
        }

        assert_eq!(
            app.viewport.scroll_x, dollar_pos,
            "l max should equal $ position"
        );
    }

    #[test]
    fn g_then_non_g_does_not_jump() {
        let lines = vec!["line"; 50];
        let mut app = test_app(&lines, &lines, (40, 10));
        app.viewport.scroll_y = 30;

        handle_key(&mut app, key(KeyCode::Char('g')));
        handle_key(&mut app, key(KeyCode::Char('j')));

        assert_eq!(app.viewport.scroll_y, 31, "g then j should just move down");
    }

    #[test]
    fn display_rows_include_padding_for_inserts() {
        let left = vec!["a", "b", "c"];
        let right = vec!["a", "x", "y", "b", "c"];
        let app = test_app(&left, &right, (40, 20));

        // Display rows should include padding for alignment
        assert!(app.display_rows.len() >= 5);
    }

    #[test]
    fn shift_j_advances_to_next_block() {
        // Equal lines, then a change, then equal, then a change
        let left = vec!["a", "b", "c", "d", "e", "f", "g", "h"];
        let right = vec!["a", "X", "c", "d", "e", "Y", "g", "h"];
        let mut app = test_app(&left, &right, (40, 10));

        assert_eq!(app.current_block, 0);
        handle_key(&mut app, key(KeyCode::Char('J')));
        assert_eq!(app.current_block, 1);
    }

    #[test]
    fn shift_k_retreats_to_previous_block() {
        let left = vec!["a", "b", "c", "d", "e", "f", "g", "h"];
        let right = vec!["a", "X", "c", "d", "e", "Y", "g", "h"];
        let mut app = test_app(&left, &right, (40, 10));

        app.current_block = 1;
        handle_key(&mut app, key(KeyCode::Char('K')));
        assert_eq!(app.current_block, 0);
    }

    #[test]
    fn shift_j_clamps_at_last_block() {
        let left = vec!["a", "b"];
        let right = vec!["a", "X"];
        let mut app = test_app(&left, &right, (40, 10));

        // Only one change block — repeated Ctrl+j should stay at 0
        handle_key(&mut app, key(KeyCode::Char('J')));
        handle_key(&mut app, key(KeyCode::Char('J')));
        assert_eq!(app.current_block, 0);
    }

    #[test]
    fn shift_k_clamps_at_first_block() {
        let left = vec!["a", "b"];
        let right = vec!["a", "X"];
        let mut app = test_app(&left, &right, (40, 10));

        handle_key(&mut app, key(KeyCode::Char('K')));
        assert_eq!(app.current_block, 0);
    }

    #[test]
    fn scroll_to_block_centers_vertically() {
        // 30 equal lines, then a change, then more equal lines
        let mut left: Vec<&str> = vec!["same"; 30];
        left.push("old");
        left.extend(vec!["same"; 20]);
        let mut right: Vec<&str> = vec!["same"; 30];
        right.push("new");
        right.extend(vec!["same"; 20]);

        let mut app = test_app(&left, &right, (40, 10));

        scroll_to_current_block(&mut app);

        // Block starts at display row 30. Center in viewport of height 10 → scroll_y = 30 - 5 = 25
        assert_eq!(app.viewport.scroll_y, 25);
    }

    #[test]
    fn copy_left_to_right_replaces_content() {
        let left = vec!["a", "b", "c"];
        let right = vec!["a", "X", "c"];
        let mut app = test_app(&left, &right, (40, 10));

        handle_key(&mut app, key(KeyCode::Char('L')));

        assert_eq!(app.right_content.lines(), &["a", "b", "c"]);
        assert!(app.right_dirty);
        assert!(!app.left_dirty);
    }

    #[test]
    fn copy_right_to_left_replaces_content() {
        let left = vec!["a", "b", "c"];
        let right = vec!["a", "X", "c"];
        let mut app = test_app(&left, &right, (40, 10));

        handle_key(&mut app, key(KeyCode::Char('H')));

        assert_eq!(app.left_content.lines(), &["a", "X", "c"]);
        assert!(app.left_dirty);
        assert!(!app.right_dirty);
    }

    #[test]
    fn copy_removes_change_block() {
        let left = vec!["a", "b", "c"];
        let right = vec!["a", "X", "c"];
        let mut app = test_app(&left, &right, (40, 10));

        assert_eq!(app.change_count, 1);
        handle_key(&mut app, key(KeyCode::Char('L')));
        assert_eq!(app.change_count, 0);
    }

    #[test]
    fn copy_clamps_current_block() {
        // Two change blocks; navigate to the last, then copy it away.
        let left = vec!["a", "b", "c", "d", "e"];
        let right = vec!["a", "X", "c", "Y", "e"];
        let mut app = test_app(&left, &right, (40, 10));

        handle_key(&mut app, key(KeyCode::Char('J'))); // move to second block
        assert_eq!(app.current_block, 1);

        handle_key(&mut app, key(KeyCode::Char('L'))); // copy it away
        assert_eq!(app.change_count, 1);
        assert_eq!(app.current_block, 0); // clamped back
    }

    #[test]
    fn copy_insert_block_left_to_right() {
        // Right has extra lines — copying left→right removes them.
        let left = vec!["a", "b"];
        let right = vec!["a", "X", "Y", "b"];
        let mut app = test_app(&left, &right, (40, 10));

        handle_key(&mut app, key(KeyCode::Char('L')));

        assert_eq!(app.right_content.lines(), &["a", "b"]);
        assert_eq!(app.change_count, 0);
    }

    #[test]
    fn copy_noop_when_no_changes() {
        let left = vec!["a", "b", "c"];
        let right = vec!["a", "b", "c"];
        let mut app = test_app(&left, &right, (40, 10));

        handle_key(&mut app, key(KeyCode::Char('L')));

        assert!(!app.left_dirty);
        assert!(!app.right_dirty);
    }
}
