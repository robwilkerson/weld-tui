use crossterm::event::KeyCode;

use crate::app::App;

/// Handle a key press, updating app state.
pub fn handle_key(app: &mut App, code: KeyCode) {
    let max_y = app.display_rows.len().saturating_sub(1) as u16;
    let scroll_y_max = max_y.saturating_sub(app.viewport_height.saturating_sub(1));

    let max_x = app.max_content_width as u16;

    // Handle `gg` — two consecutive `g` presses jump to top
    if app.pending_g {
        app.pending_g = false;
        if code == KeyCode::Char('g') {
            app.scroll_y = 0;
            return;
        }
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
            app.scroll_x = app
                .scroll_x
                .saturating_add(2)
                .min(max_x.saturating_sub(app.viewport_width));
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
    use weld_core::diff::{BlockKind, DiffResult};
    use weld_core::display;
    use weld_core::file_io::{FileContent, LineEnding};

    use crate::file_diff::view::expand_tabs;
    use crate::theme::Theme;

    fn test_app(left_lines: &[&str], right_lines: &[&str], viewport: (u16, u16)) -> App {
        let make_content = |lines: &[&str]| FileContent {
            path: PathBuf::new(),
            lines: lines.iter().map(|s| s.to_string()).collect(),
            line_ending: LineEnding::Lf,
            has_trailing_newline: true,
        };
        let left_content = make_content(left_lines);
        let right_content = make_content(right_lines);
        let diff = DiffResult::compute(&left_content, &right_content);
        let display_rows = display::build_display_rows(&diff);

        let max_content_width = left_content
            .lines
            .iter()
            .chain(right_content.lines.iter())
            .map(|l| expand_tabs(l).len() + 1)
            .max()
            .unwrap_or(0);

        let change_count = diff
            .blocks
            .iter()
            .filter(|b| b.kind != BlockKind::Equal)
            .count();

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
            scroll_y: 0,
            scroll_x: 0,
            viewport_height: viewport.1,
            viewport_width: viewport.0,
            pending_g: false,
        }
    }

    #[test]
    fn j_caps_at_viewport_bottom() {
        let lines = vec!["line"; 20];
        let mut app = test_app(&lines, &lines, (40, 10));

        for _ in 0..25 {
            handle_key(&mut app, KeyCode::Char('j'));
        }

        // 20 identical lines = 20 display rows. max scroll = 20 - 10 = 10
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
    fn dollar_uses_global_max_across_both_files() {
        let mut left: Vec<&str> = vec!["short"; 51];
        let long = &"a".repeat(200);
        left[50] = long;

        let mut app = test_app(&left, &["short"; 51], (40, 10));

        handle_key(&mut app, KeyCode::Char('$'));

        // Global max = 201 (200 + leading space), viewport = 40 → scroll_x = 161
        assert_eq!(
            app.scroll_x, 161,
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

        app.scroll_y = 10;
        handle_key(&mut app, KeyCode::Char('$'));

        // max_x = 101 (100 + leading space), viewport_width = 40 → scroll_x = 61
        assert_eq!(app.scroll_x, 61, "$ should use the long line now visible");
    }

    #[test]
    fn gg_jumps_to_top() {
        let lines = vec!["line"; 50];
        let mut app = test_app(&lines, &lines, (40, 10));
        app.scroll_y = 30;

        handle_key(&mut app, KeyCode::Char('g'));
        handle_key(&mut app, KeyCode::Char('g'));

        assert_eq!(app.scroll_y, 0);
    }

    #[test]
    fn l_and_dollar_agree_on_max_scroll() {
        let long = "x".repeat(100);
        let mut app = test_app(&[&long], &[&long], (40, 10));

        handle_key(&mut app, KeyCode::Char('$'));
        let dollar_pos = app.scroll_x;

        app.scroll_x = 0;
        for _ in 0..200 {
            handle_key(&mut app, KeyCode::Char('l'));
        }

        assert_eq!(app.scroll_x, dollar_pos, "l max should equal $ position");
    }

    #[test]
    fn g_then_non_g_does_not_jump() {
        let lines = vec!["line"; 50];
        let mut app = test_app(&lines, &lines, (40, 10));
        app.scroll_y = 30;

        handle_key(&mut app, KeyCode::Char('g'));
        handle_key(&mut app, KeyCode::Char('j'));

        assert_eq!(app.scroll_y, 31, "g then j should just move down");
    }

    #[test]
    fn display_rows_include_padding_for_inserts() {
        let left = vec!["a", "b", "c"];
        let right = vec!["a", "x", "y", "b", "c"];
        let app = test_app(&left, &right, (40, 20));

        // Display rows should include padding for alignment
        assert!(app.display_rows.len() >= 5);
    }
}
