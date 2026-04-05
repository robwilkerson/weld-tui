use std::path::PathBuf;

use weld_core::file_io::FileContent;

use crate::theme::Theme;

/// Top-level application state.
pub struct App {
    pub theme: Theme,
    pub running: bool,
    pub left_dir: String,
    pub left_filename: String,
    pub right_dir: String,
    pub right_filename: String,
    pub left_content: FileContent,
    pub right_content: FileContent,
    /// Synchronized vertical scroll offset.
    pub scroll_y: u16,
    /// Synchronized horizontal scroll offset.
    pub scroll_x: u16,
    /// Last-known viewport height (rows visible in code area).
    pub viewport_height: u16,
    /// Last-known viewport width (columns visible in code area).
    pub viewport_width: u16,
}

/// Replace the home directory prefix with ~ for display.
/// Uses path-based prefix matching to avoid false positives
/// (e.g., /Users/al matching /Users/alice).
fn shorten_dir(path: &str) -> String {
    let path = std::path::Path::new(path);
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        if let Ok(rest) = path.strip_prefix(&home) {
            return if rest.as_os_str().is_empty() {
                "~".to_string()
            } else {
                format!("~/{}", rest.display())
            };
        }
    }
    path.display().to_string()
}

impl App {
    pub fn new(left: PathBuf, right: PathBuf) -> Result<Self, std::io::Error> {
        let left_content = FileContent::load(&left)?;
        let right_content = FileContent::load(&right)?;

        let left_abs = left.canonicalize().unwrap_or(left);
        let right_abs = right.canonicalize().unwrap_or(right);

        Ok(App {
            theme: Theme::default(),
            running: true,
            left_dir: shorten_dir(
                &left_abs
                    .parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            ),
            left_filename: left_abs
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default(),
            right_dir: shorten_dir(
                &right_abs
                    .parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            ),
            right_filename: right_abs
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default(),
            left_content,
            right_content,
            scroll_y: 0,
            scroll_x: 0,
            viewport_height: 0,
            viewport_width: 0,
        })
    }
}
