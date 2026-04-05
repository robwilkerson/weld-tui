use std::path::PathBuf;

use crate::theme::Theme;

/// Top-level application state.
pub struct App {
    pub theme: Theme,
    pub running: bool,
    pub left_title: String,
    pub right_title: String,
}

impl App {
    pub fn new(left: PathBuf, right: PathBuf) -> Self {
        App {
            theme: Theme::default(),
            running: true,
            left_title: left.display().to_string(),
            right_title: right.display().to_string(),
        }
    }
}
