use std::path::PathBuf;

use weld_core::diff::{BlockKind, DiffResult};
use weld_core::display::DisplayRow;
use weld_core::file_io::{FileContent, shorten_dir};

use crate::file_diff::view::expand_tabs;
use crate::theme::Theme;
use crate::viewport::Viewport;

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
    /// Computed diff between left and right files.
    pub diff: DiffResult,
    /// Display rows: flattened diff blocks with alignment padding.
    pub display_rows: Vec<DisplayRow>,
    /// Max tab-expanded content width across both files (for diff highlight padding).
    pub max_content_width: usize,
    /// Number of non-equal diff blocks (for status bar).
    pub change_count: usize,
    /// Indices of non-Equal blocks within `diff.blocks` (for block navigation).
    pub change_block_indices: Vec<usize>,
    /// Index into `change_block_indices` — which change block is "current".
    pub current_block: usize,
    /// True until the first render sets viewport dimensions and scrolls to the first block.
    pub needs_initial_scroll: bool,
    /// Scroll position and visible dimensions.
    pub viewport: Viewport,
    /// Multi-key input state machine.
    pub input: InputState,
    /// Width of the minimap bar in terminal columns (0 = hidden).
    pub minimap_width: u16,
    /// Whether the left file has been modified by a copy operation.
    pub left_dirty: bool,
    /// Whether the right file has been modified by a copy operation.
    pub right_dirty: bool,
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

        let change_block_indices: Vec<usize> = diff
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.kind != BlockKind::Equal)
            .map(|(i, _)| i)
            .collect();
        let change_count = change_block_indices.len();

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
            change_block_indices,
            current_block: 0,
            needs_initial_scroll: true,
            viewport: Viewport::default(),
            input: InputState::default(),
            minimap_width: 1,
            left_dirty: false,
            right_dirty: false,
        })
    }

    /// Copy the active block's content from the left side to the right side.
    pub fn copy_left_to_right(&mut self) {
        if self.change_block_indices.is_empty() {
            return;
        }
        let block_index = self.change_block_indices[self.current_block];
        let block = &self.diff.blocks[block_index];
        let source: Vec<String> = self.left_content.lines()[block.left_range.clone()].to_vec();
        self.right_content
            .splice_lines(block.right_range.clone(), source);
        self.right_dirty = true;
        self.recompute_diff();
    }

    /// Copy the active block's content from the right side to the left side.
    pub fn copy_right_to_left(&mut self) {
        if self.change_block_indices.is_empty() {
            return;
        }
        let block_index = self.change_block_indices[self.current_block];
        let block = &self.diff.blocks[block_index];
        let source: Vec<String> = self.right_content.lines()[block.right_range.clone()].to_vec();
        self.left_content
            .splice_lines(block.left_range.clone(), source);
        self.left_dirty = true;
        self.recompute_diff();
    }

    /// Recompute diff, display rows, and navigation indices after a copy operation.
    fn recompute_diff(&mut self) {
        self.diff = DiffResult::compute(&self.left_content, &self.right_content);
        self.display_rows = weld_core::display::build_display_rows(&self.diff);

        self.max_content_width = self
            .left_content
            .lines()
            .iter()
            .chain(self.right_content.lines().iter())
            .map(|l| expand_tabs(l).len() + 1)
            .max()
            .unwrap_or(0);

        self.change_block_indices = self
            .diff
            .blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.kind != BlockKind::Equal)
            .map(|(i, _)| i)
            .collect();
        self.change_count = self.change_block_indices.len();

        if self.change_block_indices.is_empty() {
            self.current_block = 0;
        } else if self.current_block >= self.change_block_indices.len() {
            self.current_block = self.change_block_indices.len() - 1;
        }
    }
}
