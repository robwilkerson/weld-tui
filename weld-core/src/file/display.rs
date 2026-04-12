use super::diff::{BlockKind, DiffResult};

/// What a single row in the display represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayRow {
    /// Line index in the left file, or None for a padding row.
    pub left_line: Option<usize>,
    /// Line index in the right file, or None for a padding row.
    pub right_line: Option<usize>,
    /// The kind of diff block this row belongs to.
    pub kind: BlockKind,
    /// Index of the originating DiffBlock in DiffResult.blocks.
    pub block_index: usize,
}

/// Flatten diff blocks into a sequence of display rows with alignment padding.
pub fn build_display_rows(diff: &DiffResult) -> Vec<DisplayRow> {
    let mut rows = Vec::new();

    for (block_index, block) in diff.blocks.iter().enumerate() {
        match block.kind {
            BlockKind::Equal => {
                for (i, j) in block.left_range.clone().zip(block.right_range.clone()) {
                    rows.push(DisplayRow {
                        left_line: Some(i),
                        right_line: Some(j),
                        kind: BlockKind::Equal,
                        block_index,
                    });
                }
            }
            BlockKind::Delete => {
                for i in block.left_range.clone() {
                    rows.push(DisplayRow {
                        left_line: Some(i),
                        right_line: None,
                        kind: BlockKind::Delete,
                        block_index,
                    });
                }
            }
            BlockKind::Insert => {
                for j in block.right_range.clone() {
                    rows.push(DisplayRow {
                        left_line: None,
                        right_line: Some(j),
                        kind: BlockKind::Insert,
                        block_index,
                    });
                }
            }
            BlockKind::Replace => {
                let left_len = block.left_range.len();
                let right_len = block.right_range.len();
                let max_len = left_len.max(right_len);

                for offset in 0..max_len {
                    let left_line = if offset < left_len {
                        Some(block.left_range.start + offset)
                    } else {
                        None
                    };
                    let right_line = if offset < right_len {
                        Some(block.right_range.start + offset)
                    } else {
                        None
                    };
                    rows.push(DisplayRow {
                        left_line,
                        right_line,
                        kind: BlockKind::Replace,
                        block_index,
                    });
                }
            }
        }
    }

    rows
}

#[cfg(test)]
mod tests {
    use super::super::diff::DiffBlock;
    use super::*;

    fn equal(left_start: usize, right_start: usize, len: usize) -> DiffBlock {
        DiffBlock {
            kind: BlockKind::Equal,
            left_range: left_start..left_start + len,
            right_range: right_start..right_start + len,
            inline_diffs: vec![],
        }
    }

    fn delete(left_start: usize, left_len: usize, right_pos: usize) -> DiffBlock {
        DiffBlock {
            kind: BlockKind::Delete,
            left_range: left_start..left_start + left_len,
            right_range: right_pos..right_pos,
            inline_diffs: vec![],
        }
    }

    fn insert(left_pos: usize, right_start: usize, right_len: usize) -> DiffBlock {
        DiffBlock {
            kind: BlockKind::Insert,
            left_range: left_pos..left_pos,
            right_range: right_start..right_start + right_len,
            inline_diffs: vec![],
        }
    }

    fn replace(
        left_start: usize,
        left_len: usize,
        right_start: usize,
        right_len: usize,
    ) -> DiffBlock {
        DiffBlock {
            kind: BlockKind::Replace,
            left_range: left_start..left_start + left_len,
            right_range: right_start..right_start + right_len,
            inline_diffs: vec![],
        }
    }

    #[test]
    fn equal_block_produces_paired_rows() {
        let diff = DiffResult {
            blocks: vec![equal(0, 0, 3)],
        };
        let rows = build_display_rows(&diff);
        assert_eq!(rows.len(), 3);
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.left_line, Some(i));
            assert_eq!(row.right_line, Some(i));
            assert_eq!(row.kind, BlockKind::Equal);
        }
    }

    #[test]
    fn delete_block_pads_right_side() {
        let diff = DiffResult {
            blocks: vec![delete(2, 3, 2)],
        };
        let rows = build_display_rows(&diff);
        assert_eq!(rows.len(), 3);
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.left_line, Some(2 + i));
            assert_eq!(row.right_line, None);
            assert_eq!(row.kind, BlockKind::Delete);
        }
    }

    #[test]
    fn insert_block_pads_left_side() {
        let diff = DiffResult {
            blocks: vec![insert(1, 1, 2)],
        };
        let rows = build_display_rows(&diff);
        assert_eq!(rows.len(), 2);
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.left_line, None);
            assert_eq!(row.right_line, Some(1 + i));
            assert_eq!(row.kind, BlockKind::Insert);
        }
    }

    #[test]
    fn replace_block_pairs_then_pads_shorter_side() {
        let diff = DiffResult {
            blocks: vec![replace(5, 2, 5, 4)],
        };
        let rows = build_display_rows(&diff);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].left_line, Some(5));
        assert_eq!(rows[0].right_line, Some(5));
        assert_eq!(rows[1].left_line, Some(6));
        assert_eq!(rows[1].right_line, Some(6));
        assert_eq!(rows[2].left_line, None);
        assert_eq!(rows[2].right_line, Some(7));
        assert_eq!(rows[3].left_line, None);
        assert_eq!(rows[3].right_line, Some(8));
    }

    #[test]
    fn mixed_blocks_produce_correct_sequence() {
        let diff = DiffResult {
            blocks: vec![
                equal(0, 0, 2),
                delete(2, 1, 2),
                equal(3, 2, 1),
                insert(4, 3, 2),
            ],
        };
        let rows = build_display_rows(&diff);
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[0].kind, BlockKind::Equal);
        assert_eq!(rows[1].kind, BlockKind::Equal);
        assert_eq!(rows[2].kind, BlockKind::Delete);
        assert_eq!(rows[3].kind, BlockKind::Equal);
        assert_eq!(rows[4].kind, BlockKind::Insert);
        assert_eq!(rows[5].kind, BlockKind::Insert);
    }

    #[test]
    fn empty_diff_produces_no_rows() {
        let diff = DiffResult { blocks: vec![] };
        let rows = build_display_rows(&diff);
        assert!(rows.is_empty());
    }
}
