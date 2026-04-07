use std::ops::Range;

use similar::DiffOp;

use crate::file_io::FileContent;
use crate::inline_diff::InlineDiff;

/// The kind of change a diff block represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Equal,
    Insert,
    Delete,
    Replace,
}

/// A contiguous region of diff between two files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffBlock {
    pub kind: BlockKind,
    pub left_range: Range<usize>,
    pub right_range: Range<usize>,
    /// Character-level diffs for Replace blocks. One entry per paired line.
    pub inline_diffs: Vec<InlineDiff>,
}

/// The result of diffing two files.
#[derive(Debug, Clone)]
pub struct DiffResult {
    pub blocks: Vec<DiffBlock>,
}

impl DiffResult {
    /// Compute the diff between two file contents.
    pub fn compute(left: &FileContent, right: &FileContent) -> Self {
        let left_text = left.text();
        let right_text = right.text();
        let diff = similar::TextDiff::from_lines(&left_text, &right_text);

        // Use the lines as seen by the diff engine so indices are always in bounds.
        let left_lines: Vec<&str> = left_text.lines().collect();
        let right_lines: Vec<&str> = right_text.lines().collect();

        let mut blocks: Vec<DiffBlock> = Vec::new();

        for op in diff.ops() {
            let (kind, left_range, right_range) = match *op {
                DiffOp::Equal {
                    old_index,
                    new_index,
                    len,
                } => (
                    BlockKind::Equal,
                    old_index..old_index + len,
                    new_index..new_index + len,
                ),
                DiffOp::Delete {
                    old_index,
                    old_len,
                    new_index,
                } => (
                    BlockKind::Delete,
                    old_index..old_index + old_len,
                    new_index..new_index,
                ),
                DiffOp::Insert {
                    old_index,
                    new_index,
                    new_len,
                } => (
                    BlockKind::Insert,
                    old_index..old_index,
                    new_index..new_index + new_len,
                ),
                DiffOp::Replace {
                    old_index,
                    old_len,
                    new_index,
                    new_len,
                } => (
                    BlockKind::Replace,
                    old_index..old_index + old_len,
                    new_index..new_index + new_len,
                ),
            };

            let inline_diffs = if kind == BlockKind::Replace {
                compute_inline_diffs_str(
                    &left_lines[left_range.clone()],
                    &right_lines[right_range.clone()],
                )
            } else {
                Vec::new()
            };

            blocks.push(DiffBlock {
                kind,
                left_range,
                right_range,
                inline_diffs,
            });
        }

        DiffResult { blocks }
    }

    /// Return only the blocks that represent changes (not Equal).
    pub fn change_blocks(&self) -> Vec<(usize, &DiffBlock)> {
        self.blocks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.kind != BlockKind::Equal)
            .collect()
    }

    /// Returns true if there are no differences.
    pub fn is_identical(&self) -> bool {
        self.blocks.iter().all(|b| b.kind == BlockKind::Equal)
    }
}

/// Compute inline diffs for paired lines in a Replace block.
fn compute_inline_diffs_str(left_lines: &[&str], right_lines: &[&str]) -> Vec<InlineDiff> {
    let pair_count = left_lines.len().min(right_lines.len());
    (0..pair_count)
        .map(|i| InlineDiff::compute(left_lines[i], right_lines[i]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_io::LineEnding;
    use std::path::PathBuf;

    fn make_content(lines: &[&str]) -> FileContent {
        FileContent {
            path: PathBuf::new(),
            lines: lines.iter().map(|s| s.to_string()).collect(),
            line_ending: LineEnding::Lf,
            has_trailing_newline: !lines.is_empty(),
        }
    }

    #[test]
    fn identical_files_produce_single_equal_block() {
        let left = make_content(&["hello", "world"]);
        let right = make_content(&["hello", "world"]);
        let result = DiffResult::compute(&left, &right);
        assert!(result.is_identical());
        assert_eq!(result.blocks.len(), 1);
        assert_eq!(result.blocks[0].kind, BlockKind::Equal);
    }

    #[test]
    fn insertion_detected() {
        let left = make_content(&["line1", "line3"]);
        let right = make_content(&["line1", "line2", "line3"]);
        let result = DiffResult::compute(&left, &right);
        let changes = result.change_blocks();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].1.kind, BlockKind::Insert);
        assert_eq!(changes[0].1.right_range, 1..2);
    }

    #[test]
    fn deletion_detected() {
        let left = make_content(&["line1", "line2", "line3"]);
        let right = make_content(&["line1", "line3"]);
        let result = DiffResult::compute(&left, &right);
        let changes = result.change_blocks();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].1.kind, BlockKind::Delete);
        assert_eq!(changes[0].1.left_range, 1..2);
    }

    #[test]
    fn replacement_detected_with_inline_diffs() {
        let left = make_content(&["let app = App::new();"]);
        let right = make_content(&["let app = App::with_config(&config);"]);
        let result = DiffResult::compute(&left, &right);
        let changes = result.change_blocks();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].1.kind, BlockKind::Replace);
        assert!(!changes[0].1.inline_diffs.is_empty());
    }

    #[test]
    fn change_blocks_excludes_equal() {
        let left = make_content(&["same", "different_a", "same"]);
        let right = make_content(&["same", "different_b", "same"]);
        let result = DiffResult::compute(&left, &right);
        let changes = result.change_blocks();
        assert_eq!(changes.len(), 1);
        assert_ne!(changes[0].1.kind, BlockKind::Equal);
    }

    #[test]
    fn empty_files_are_identical() {
        let left = make_content(&[]);
        let right = make_content(&[]);
        let result = DiffResult::compute(&left, &right);
        assert!(result.is_identical());
    }

    #[test]
    fn left_empty_right_has_content() {
        let left = make_content(&[]);
        let right = make_content(&["line1", "line2"]);
        let result = DiffResult::compute(&left, &right);
        let changes = result.change_blocks();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].1.kind, BlockKind::Insert);
    }
}
