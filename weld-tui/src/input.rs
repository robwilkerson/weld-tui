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
    let max_x = app
        .left_content
        .lines
        .iter()
        .chain(app.right_content.lines.iter())
        .map(|l| expand_tabs(l).len())
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
            app.scroll_x = max_x.saturating_sub(app.viewport_width);
        }
        KeyCode::Char('g') => {
            app.pending_g = true;
        }
        KeyCode::Char('G') => {
            app.scroll_y = max_y.saturating_sub(app.viewport_height.saturating_sub(1));
        }
        _ => {}
    }
}
