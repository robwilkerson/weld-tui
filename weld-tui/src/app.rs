use std::path::PathBuf;

use weld_core::diff::{BlockKind, DiffResult};
use weld_core::display::DisplayRow;
use weld_core::file_io::{FileContent, shorten_dir};

use crate::file_diff::view::expand_tabs;
use crate::theme::Theme;

/// Tracks multi-key input sequences (e.g., `gg`, future counts/search).
#[derive(Default)]
pub struct InputState {
    /// Whether the previous keypress was `g` (waiting for `gg`).
    pub pending_g: bool,
}

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
    /// Computed diff between left and right files (used by merge/navigation).
    #[allow(dead_code)]
    pub diff: DiffResult,
    /// Display rows: flattened diff blocks with alignment padding.
    pub display_rows: Vec<DisplayRow>,
    /// Max tab-expanded content width across both files (for diff highlight padding).
    pub max_content_width: usize,
    /// Number of non-equal diff blocks (for status bar).
    pub change_count: usize,
    /// Synchronized vertical scroll offset.
    pub scroll_y: u16,
    /// Synchronized horizontal scroll offset.
    pub scroll_x: u16,
    /// Last-known viewport height (rows visible in code area).
    pub viewport_height: u16,
    /// Last-known viewport width (columns visible in code area).
    pub viewport_width: u16,
    /// Multi-key input state machine.
    pub input: InputState,
    /// Width of the minimap bar in terminal columns (0 = hidden).
    pub minimap_width: u16,
}

impl App {
    pub fn new(left: PathBuf, right: PathBuf) -> Result<Self, std::io::Error> {
        let left_content = FileContent::load(&left)?;
        let right_content = FileContent::load(&right)?;

        let diff = DiffResult::compute(&left_content, &right_content);
        let display_rows = weld_core::display::build_display_rows(&diff);

        let max_content_width = left_content
            .lines()
            .iter()
            .chain(right_content.lines().iter())
            .map(|l| expand_tabs(l).len() + 1)
            .max()
            .unwrap_or(0);

        let change_count = diff
            .blocks
            .iter()
            .filter(|b| b.kind != BlockKind::Equal)
            .count();

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
            diff,
            display_rows,
            max_content_width,
            change_count,
            scroll_y: 0,
            scroll_x: 0,
            viewport_height: 0,
            viewport_width: 0,
            input: InputState::default(),
            minimap_width: 1,
        })
    }
}
