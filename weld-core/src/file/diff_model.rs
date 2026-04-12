use super::diff::{BlockKind, DiffResult};
use super::display::{self, DisplayRow};
use super::io::Content;
use crate::text::expand_tabs;
use crate::undo::UndoStack;

const DEFAULT_UNDO_CAPACITY: usize = 100;

/// Snapshot of mutable state captured before a mutation for undo/redo.
#[derive(Clone)]
pub struct Snapshot {
    pub left_content: Content,
    pub right_content: Content,
    pub left_dirty: bool,
    pub right_dirty: bool,
    pub current_block: usize,
}

/// State derived from the current file contents — recomputed after every mutation.
struct DerivedState {
    diff: DiffResult,
    display_rows: Vec<DisplayRow>,
    max_content_width: usize,
    change_block_indices: Vec<usize>,
    change_count: usize,
}

fn compute_derived(left: &Content, right: &Content) -> DerivedState {
    let diff = DiffResult::compute(left, right);
    let display_rows = display::build_display_rows(&diff);

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

/// Frontend-agnostic diff model: file contents, diff state, block navigation,
/// dirty flags, and undo/redo. Shared between TUI and future GUI frontends.
pub struct DiffModel {
    pub left_content: Content,
    pub right_content: Content,
    pub diff: DiffResult,
    pub display_rows: Vec<DisplayRow>,
    pub max_content_width: usize,
    pub change_count: usize,
    pub change_block_indices: Vec<usize>,
    pub current_block: usize,
    pub left_dirty: bool,
    pub right_dirty: bool,
    pub undo_stack: UndoStack<Snapshot>,
}

impl DiffModel {
    pub fn new(left_content: Content, right_content: Content) -> Self {
        let derived = compute_derived(&left_content, &right_content);

        DiffModel {
            left_content,
            right_content,
            diff: derived.diff,
            display_rows: derived.display_rows,
            max_content_width: derived.max_content_width,
            change_count: derived.change_count,
            change_block_indices: derived.change_block_indices,
            current_block: 0,
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
