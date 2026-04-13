use std::ops::Range;

use super::diff::{BlockKind, DiffResult};
use super::display::{self, DisplayRow};
use super::io::Content;
use crate::text::expand_tabs;
use crate::undo::UndoStack;

const DEFAULT_UNDO_CAPACITY: usize = 100;

/// Which side of the diff was modified.
#[derive(Clone, Debug)]
enum Side {
    Left,
    Right,
}

/// A reversible edit: stores only the affected block's lines, not the full file.
/// Memory cost is O(block size) instead of O(file size).
#[derive(Clone, Debug)]
struct UndoEntry {
    side: Side,
    /// Start position in the target file where the splice occurred.
    start: usize,
    /// Lines that were present before the edit (used to undo).
    original_lines: Vec<String>,
    /// Lines that replaced the originals (used to redo).
    replacement_lines: Vec<String>,
    /// Whether the modified side was already dirty before this edit.
    was_dirty: bool,
    /// Block navigation index before this edit.
    previous_block: usize,
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
    undo_stack: UndoStack<UndoEntry>,
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

    /// Build an undo entry for a copy operation, capturing the target's
    /// current lines before they are overwritten.
    fn build_entry(
        &self,
        side: Side,
        target_range: Range<usize>,
        source_lines: Vec<String>,
    ) -> UndoEntry {
        let original_lines = match side {
            Side::Left => self.left_content.lines()[target_range.clone()].to_vec(),
            Side::Right => self.right_content.lines()[target_range.clone()].to_vec(),
        };
        let was_dirty = match side {
            Side::Left => self.left_dirty,
            Side::Right => self.right_dirty,
        };
        UndoEntry {
            side,
            start: target_range.start,
            original_lines,
            replacement_lines: source_lines,
            was_dirty,
            previous_block: self.current_block,
        }
    }

    /// Apply an entry forward: splice replacement lines in.
    fn apply_forward(&mut self, entry: &UndoEntry) {
        let range = entry.start..(entry.start + entry.original_lines.len());
        match entry.side {
            Side::Left => {
                self.left_content
                    .splice_lines(range, entry.replacement_lines.clone());
                self.left_dirty = true;
            }
            Side::Right => {
                self.right_content
                    .splice_lines(range, entry.replacement_lines.clone());
                self.right_dirty = true;
            }
        }
        self.recompute_diff();
    }

    /// Apply an entry backward: splice original lines back in.
    fn apply_backward(&mut self, entry: &UndoEntry) {
        let range = entry.start..(entry.start + entry.replacement_lines.len());
        match entry.side {
            Side::Left => {
                self.left_content
                    .splice_lines(range, entry.original_lines.clone());
                self.left_dirty = entry.was_dirty;
            }
            Side::Right => {
                self.right_content
                    .splice_lines(range, entry.original_lines.clone());
                self.right_dirty = entry.was_dirty;
            }
        }
        self.current_block = entry.previous_block;
        self.recompute_diff();
    }

    /// Copy the active block's content from the left side to the right side.
    pub fn copy_left_to_right(&mut self) {
        if self.change_block_indices.is_empty() {
            return;
        }
        let block_index = self.change_block_indices[self.current_block];
        let block = &self.diff.blocks[block_index];
        let source: Vec<String> = self.left_content.lines()[block.left_range.clone()].to_vec();
        let entry = self.build_entry(Side::Right, block.right_range.clone(), source.clone());
        self.undo_stack.push(entry);
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
        let entry = self.build_entry(Side::Left, block.left_range.clone(), source.clone());
        self.undo_stack.push(entry);
        self.left_content
            .splice_lines(block.left_range.clone(), source);
        self.left_dirty = true;
        self.recompute_diff();
    }

    /// Undo the most recent copy operation.
    pub fn undo(&mut self) {
        if let Some(entry) = self.undo_stack.pop_undo() {
            self.apply_backward(&entry);
            self.undo_stack.push_redo(entry);
        }
    }

    /// Redo the most recently undone operation.
    pub fn redo(&mut self) {
        if let Some(entry) = self.undo_stack.pop_redo() {
            self.apply_forward(&entry);
            self.undo_stack.push_undo(entry);
        }
    }

    /// Whether there are entries available to undo.
    pub fn can_undo(&self) -> bool {
        self.undo_stack.can_undo()
    }

    /// Whether there are entries available to redo.
    pub fn can_redo(&self) -> bool {
        self.undo_stack.can_redo()
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

#[cfg(test)]
mod tests {
    use super::super::io::LineEnding;
    use super::*;
    use std::path::PathBuf;

    fn content_from(lines: &[&str]) -> Content {
        Content {
            path: PathBuf::from("test"),
            lines: lines.iter().map(|s| s.to_string()).collect(),
            line_ending: LineEnding::Lf,
            has_trailing_newline: true,
        }
    }

    #[test]
    fn copy_left_to_right_updates_content() {
        let left = content_from(&["a", "b", "c"]);
        let right = content_from(&["a", "X", "c"]);
        let mut model = DiffModel::new(left, right);

        assert_eq!(model.change_count, 1);
        model.copy_left_to_right();
        assert_eq!(model.right_content.lines(), &["a", "b", "c"]);
        assert!(model.right_dirty);
        assert_eq!(model.change_count, 0);
    }

    #[test]
    fn copy_right_to_left_updates_content() {
        let left = content_from(&["a", "b", "c"]);
        let right = content_from(&["a", "X", "c"]);
        let mut model = DiffModel::new(left, right);

        model.copy_right_to_left();
        assert_eq!(model.left_content.lines(), &["a", "X", "c"]);
        assert!(model.left_dirty);
    }

    #[test]
    fn undo_restores_original_content() {
        let left = content_from(&["a", "b", "c"]);
        let right = content_from(&["a", "X", "c"]);
        let mut model = DiffModel::new(left, right);

        model.copy_left_to_right();
        assert_eq!(model.change_count, 0);

        model.undo();
        assert_eq!(model.right_content.lines(), &["a", "X", "c"]);
        assert!(!model.right_dirty);
        assert_eq!(model.change_count, 1);
    }

    #[test]
    fn redo_reapplies_undone_copy() {
        let left = content_from(&["a", "b", "c"]);
        let right = content_from(&["a", "X", "c"]);
        let mut model = DiffModel::new(left, right);

        model.copy_left_to_right();
        model.undo();
        model.redo();
        assert_eq!(model.right_content.lines(), &["a", "b", "c"]);
        assert!(model.right_dirty);
        assert_eq!(model.change_count, 0);
    }

    #[test]
    fn undo_redo_with_line_count_change() {
        // Left has 2 lines in the changed block, right has 1.
        let left = content_from(&["a", "b", "c", "d"]);
        let right = content_from(&["a", "X", "d"]);
        let mut model = DiffModel::new(left, right);

        model.copy_left_to_right();
        assert_eq!(model.right_content.lines(), &["a", "b", "c", "d"]);

        model.undo();
        assert_eq!(model.right_content.lines(), &["a", "X", "d"]);

        model.redo();
        assert_eq!(model.right_content.lines(), &["a", "b", "c", "d"]);
    }

    #[test]
    fn multiple_undo_redo_round_trips() {
        let left = content_from(&["a", "b", "c"]);
        let right = content_from(&["a", "X", "c"]);
        let mut model = DiffModel::new(left, right);

        model.copy_left_to_right();
        for _ in 0..3 {
            model.undo();
            assert_eq!(model.right_content.lines(), &["a", "X", "c"]);
            model.redo();
            assert_eq!(model.right_content.lines(), &["a", "b", "c"]);
        }
    }

    #[test]
    fn new_copy_after_undo_clears_redo() {
        let left = content_from(&["a", "b"]);
        let right = content_from(&["a", "X"]);
        let mut model = DiffModel::new(left, right);

        model.copy_left_to_right();
        model.undo();
        assert!(model.can_redo());

        // Redo a fresh copy — redo stack should be cleared.
        model.copy_left_to_right();
        assert!(!model.can_redo());
    }

    #[test]
    fn undo_noop_on_empty_stack() {
        let left = content_from(&["a"]);
        let right = content_from(&["a"]);
        let mut model = DiffModel::new(left, right);

        model.undo(); // should not panic
        assert!(!model.can_undo());
    }

    #[test]
    fn redo_noop_on_empty_stack() {
        let left = content_from(&["a"]);
        let right = content_from(&["a"]);
        let mut model = DiffModel::new(left, right);

        model.redo(); // should not panic
        assert!(!model.can_redo());
    }

    #[test]
    fn dirty_flag_restored_on_undo() {
        let left = content_from(&["a", "b"]);
        let right = content_from(&["a", "X"]);
        let mut model = DiffModel::new(left, right);

        assert!(!model.right_dirty);
        model.copy_left_to_right();
        assert!(model.right_dirty);
        model.undo();
        assert!(!model.right_dirty);
    }
}
