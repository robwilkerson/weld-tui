use std::path::PathBuf;

use weld_core::diff::{BlockKind, DiffResult};
use weld_core::display::DisplayRow;
use weld_core::file_io::{FileContent, shorten_dir};
use weld_core::text::expand_tabs;
use weld_core::undo::UndoStack;

use crate::theme::Theme;
use crate::viewport::Viewport;

const DEFAULT_UNDO_CAPACITY: usize = 100;

/// Snapshot of mutable state captured before a mutation for undo/redo.
#[derive(Clone)]
pub struct Snapshot {
    pub left_content: FileContent,
    pub right_content: FileContent,
    pub left_dirty: bool,
    pub right_dirty: bool,
    pub current_block: usize,
}

/// Tracks multi-key input sequences (e.g., `gg`, future counts/search).
#[derive(Default)]
pub struct InputState {
    /// Whether the previous keypress was `g` (waiting for `gg`).
    pub pending_g: bool,
}

/// State derived from the current file contents — recomputed after every mutation.
struct DerivedState {
    diff: DiffResult,
    display_rows: Vec<DisplayRow>,
    max_content_width: usize,
    change_block_indices: Vec<usize>,
    change_count: usize,
}

fn compute_derived(left: &FileContent, right: &FileContent) -> DerivedState {
    let diff = DiffResult::compute(left, right);
    let display_rows = weld_core::display::build_display_rows(&diff);

    let max_content_width = left
        .lines()
        .iter()
        .chain(right.lines().iter())
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

    DerivedState {
        diff,
        display_rows,
        max_content_width,
        change_block_indices,
        change_count,
    }
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
    /// Undo/redo stack for copy operations.
    pub undo_stack: UndoStack<Snapshot>,
}

impl App {
    pub fn new(left: PathBuf, right: PathBuf) -> Result<Self, std::io::Error> {
        let left_content = FileContent::load(&left)?;
        let right_content = FileContent::load(&right)?;

        let left_abs = left.canonicalize().unwrap_or(left);
        let right_abs = right.canonicalize().unwrap_or(right);

        let mut app = Self::from_contents(left_content, right_content);
        app.left_dir = shorten_dir(
            &left_abs
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
        );
        app.left_filename = left_abs
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        app.right_dir = shorten_dir(
            &right_abs
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
        );
        app.right_filename = right_abs
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        app.needs_initial_scroll = true;

        Ok(app)
    }

    /// Construct an App from pre-loaded file contents (no filesystem access).
    pub fn from_contents(left_content: FileContent, right_content: FileContent) -> Self {
        let derived = compute_derived(&left_content, &right_content);

        App {
            theme: Theme::default(),
            running: true,
            left_dir: String::new(),
            left_filename: String::new(),
            right_dir: String::new(),
            right_filename: String::new(),
            left_content,
            right_content,
            diff: derived.diff,
            display_rows: derived.display_rows,
            max_content_width: derived.max_content_width,
            change_count: derived.change_count,
            change_block_indices: derived.change_block_indices,
            current_block: 0,
            needs_initial_scroll: false,
            viewport: Viewport::default(),
            input: InputState::default(),
            minimap_width: 1,
            left_dirty: false,
            right_dirty: false,
            undo_stack: UndoStack::new(DEFAULT_UNDO_CAPACITY),
        }
    }

    /// Capture current mutable state as a snapshot.
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            left_content: self.left_content.clone(),
            right_content: self.right_content.clone(),
            left_dirty: self.left_dirty,
            right_dirty: self.right_dirty,
            current_block: self.current_block,
        }
    }

    /// Restore mutable state from a snapshot and recompute derived state.
    fn restore(&mut self, snapshot: &Snapshot) {
        self.left_content = snapshot.left_content.clone();
        self.right_content = snapshot.right_content.clone();
        self.left_dirty = snapshot.left_dirty;
        self.right_dirty = snapshot.right_dirty;
        self.current_block = snapshot.current_block;
        self.recompute_diff();
    }

    /// Copy the active block's content from the left side to the right side.
    pub fn copy_left_to_right(&mut self) {
        if self.change_block_indices.is_empty() {
            return;
        }
        self.undo_stack.push(self.snapshot());
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
        self.undo_stack.push(self.snapshot());
        let block_index = self.change_block_indices[self.current_block];
        let block = &self.diff.blocks[block_index];
        let source: Vec<String> = self.right_content.lines()[block.right_range.clone()].to_vec();
        self.left_content
            .splice_lines(block.left_range.clone(), source);
        self.left_dirty = true;
        self.recompute_diff();
    }

    /// Undo the most recent copy operation.
    pub fn undo(&mut self) {
        if let Some(previous) = self.undo_stack.pop_undo() {
            let current = self.snapshot();
            self.restore(&previous);
            self.undo_stack.push_redo(current);
        }
    }

    /// Redo the most recently undone operation.
    pub fn redo(&mut self) {
        if let Some(next) = self.undo_stack.pop_redo() {
            let current = self.snapshot();
            self.restore(&next);
            self.undo_stack.push_undo(current);
        }
    }

    /// Recompute diff, display rows, and navigation indices after a mutation.
    fn recompute_diff(&mut self) {
        let derived = compute_derived(&self.left_content, &self.right_content);
        self.diff = derived.diff;
        self.display_rows = derived.display_rows;
        self.max_content_width = derived.max_content_width;
        self.change_block_indices = derived.change_block_indices;
        self.change_count = derived.change_count;

        if self.change_block_indices.is_empty() {
            self.current_block = 0;
        } else if self.current_block >= self.change_block_indices.len() {
            self.current_block = self.change_block_indices.len() - 1;
        }
    }
}
