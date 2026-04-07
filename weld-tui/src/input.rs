use crossterm::event::KeyCode;

use crate::app::App;
use crate::file_diff::view::expand_tabs;

/// Handle a key press, updating app state.
pub fn handle_key(app: &mut App, code: KeyCode) {
    let max_y = app
        .left_content
        .lines
        .len()
        .max(app.right_content.lines.len())
        .saturating_sub(1) as u16;
    let scroll_y_max = max_y.saturating_sub(app.viewport_height.saturating_sub(1));

    // Horizontal max based on visible lines only, not entire file.
    let visible_start = app.scroll_y as usize;
    let visible_end = visible_start + app.viewport_height as usize;
    let max_x = app
        .left_content
        .lines
        .iter()
        .enumerate()
        .chain(app.right_content.lines.iter().enumerate())
        .filter(|(i, _)| *i >= visible_start && *i < visible_end)
        .map(|(_, l)| expand_tabs(l).len() + 1) // +1 for leading space in rendered content
        .max()
        .unwrap_or(0) as u16;

    // Handle `gg` — two consecutive `g` presses jump to top
    if app.pending_g {
        app.pending_g = false;
        if code == KeyCode::Char('g') {
            app.scroll_y = 0;
            return;
        }
        // First `g` was not followed by `g` — fall through to normal handling
    }

    match code {
        KeyCode::Char('q') => app.running = false,
        KeyCode::Char('j') | KeyCode::Down => {
            app.scroll_y = app.scroll_y.saturating_add(1).min(scroll_y_max);
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
            app.scroll_x = max_x.saturating_sub(app.viewport_width);
        }
        KeyCode::Char('g') => {
            app.pending_g = true;
        }
        KeyCode::Char('G') => {
            app.scroll_y = scroll_y_max;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use weld_core::file_io::{FileContent, LineEnding};

    use crate::theme::Theme;

    fn test_app(left_lines: &[&str], right_lines: &[&str], viewport: (u16, u16)) -> App {
        let make_content = |lines: &[&str]| FileContent {
            path: PathBuf::new(),
            lines: lines.iter().map(|s| s.to_string()).collect(),
            line_ending: LineEnding::Lf,
            has_trailing_newline: true,
        };
        App {
            theme: Theme::default(),
            running: true,
            left_dir: String::new(),
            left_filename: String::new(),
            right_dir: String::new(),
            right_filename: String::new(),
            left_content: make_content(left_lines),
            right_content: make_content(right_lines),
            scroll_y: 0,
            scroll_x: 0,
            viewport_height: viewport.1,
            viewport_width: viewport.0,
            pending_g: false,
        }
    }

    #[test]
    fn j_caps_at_viewport_bottom() {
        // 20 lines, viewport shows 10 rows → max scroll_y = 20 - 10 = 10
        let lines = vec!["line"; 20];
        let mut app = test_app(&lines, &lines, (40, 10));

        for _ in 0..25 {
            handle_key(&mut app, KeyCode::Char('j'));
        }

        assert_eq!(app.scroll_y, 10);
    }

    #[test]
    fn j_and_g_agree_on_max_scroll() {
        let lines = vec!["content"; 50];
        let mut app = test_app(&lines, &lines, (40, 20));

        handle_key(&mut app, KeyCode::Char('G'));
        let g_pos = app.scroll_y;

        app.scroll_y = 0;
        for _ in 0..100 {
            handle_key(&mut app, KeyCode::Char('j'));
        }

        assert_eq!(app.scroll_y, g_pos);
    }

    #[test]
    fn dollar_uses_visible_lines_only() {
        // Short visible lines at top, long line at index 50 (not visible)
        let mut left: Vec<&str> = vec!["short"; 51];
        let long = &"a".repeat(200);
        left[50] = long;

        let mut app = test_app(&left, &["tiny"; 5], (40, 10));

        // Viewport shows lines 0-9, all short
        handle_key(&mut app, KeyCode::Char('$'));

        assert_eq!(app.scroll_x, 0, "$ should not scroll for short visible lines");
    }

    #[test]
    fn dollar_adapts_when_scrolled_to_long_line() {
        let long = "x".repeat(100);
        let mut lines: Vec<String> = vec!["short".to_string(); 20];
        lines[15] = long;
        let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();

        let mut app = test_app(&line_refs, &vec!["tiny"; 20], (40, 10));

        // Scroll so line 15 is visible (viewport shows lines 10-19)
        app.scroll_y = 10;
        handle_key(&mut app, KeyCode::Char('$'));

        // max_x = 101 (100 + leading space), viewport_width = 40 → scroll_x = 61
        assert_eq!(app.scroll_x, 61, "$ should use the long line now visible");
    }
}
