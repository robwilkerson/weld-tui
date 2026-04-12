use std::path::PathBuf;

use weld_core::file::diff::BlockKind;
use weld_core::file::diff_model::DiffModel;
use weld_core::file::io::Content;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("test-fixtures")
}

fn load_pair(subdir: &str) -> DiffModel {
    let dir = fixtures_dir().join(subdir);
    let left = Content::load(&dir.join("left.txt")).expect("load left");
    let right = Content::load(&dir.join("right.txt")).expect("load right");
    DiffModel::new(left, right)
}

fn load_go_pair(subdir: &str) -> DiffModel {
    let dir = fixtures_dir().join(subdir);
    let left = Content::load(&dir.join("left.go")).expect("load left");
    let right = Content::load(&dir.join("right.go")).expect("load right");
    DiffModel::new(left, right)
}

// --- identical ---

#[test]
fn identical_files_produce_no_changes() {
    let model = load_pair("identical");
    assert_eq!(model.change_count, 0);
    assert!(model.change_block_indices.is_empty());
    assert!(model.diff.is_identical());
}

#[test]
fn identical_display_rows_match_line_count() {
    let model = load_pair("identical");
    // 3 lines → 3 display rows, all paired.
    assert_eq!(model.display_rows.len(), 3);
    for row in &model.display_rows {
        assert_eq!(row.kind, BlockKind::Equal);
        assert!(row.left_line.is_some());
        assert!(row.right_line.is_some());
    }
}

// --- simple-replace ---

#[test]
fn simple_replace_detects_two_changes() {
    let model = load_pair("simple-replace");
    // "bravo"→"BRAVO" and "delta"→"DELTA" = 2 replace blocks.
    assert_eq!(model.change_count, 2);
    for &idx in &model.change_block_indices {
        assert_eq!(model.diff.blocks[idx].kind, BlockKind::Replace);
    }
}

#[test]
fn simple_replace_has_inline_diffs() {
    let model = load_pair("simple-replace");
    for &idx in &model.change_block_indices {
        let block = &model.diff.blocks[idx];
        assert!(
            !block.inline_diffs.is_empty(),
            "Replace block should have inline diffs"
        );
    }
}

#[test]
fn simple_replace_display_rows_match_line_count() {
    let model = load_pair("simple-replace");
    // Both files have 5 lines, all replacements are 1:1 → 5 display rows.
    assert_eq!(model.display_rows.len(), 5);
}

// --- insert-delete ---

#[test]
fn insert_delete_detects_both_kinds() {
    let model = load_pair("insert-delete");
    let kinds: Vec<BlockKind> = model
        .change_block_indices
        .iter()
        .map(|&idx| model.diff.blocks[idx].kind)
        .collect();

    assert!(
        kinds.contains(&BlockKind::Delete) || kinds.contains(&BlockKind::Replace),
        "should have a deletion or replace for removed line 'two'"
    );
    assert!(
        kinds.contains(&BlockKind::Insert) || kinds.contains(&BlockKind::Replace),
        "should have an insertion or replace for added lines 'extra-a', 'extra-b'"
    );
}

#[test]
fn insert_delete_display_rows_have_padding() {
    let model = load_pair("insert-delete");
    // Insertions/deletions create padding rows (None on one side).
    let has_left_padding = model.display_rows.iter().any(|r| r.left_line.is_none());
    let has_right_padding = model.display_rows.iter().any(|r| r.right_line.is_none());
    assert!(
        has_left_padding || has_right_padding,
        "insert/delete should produce padding rows"
    );
}

// --- empty-vs-content ---

#[test]
fn empty_vs_content_is_single_insert() {
    let model = load_pair("empty-vs-content");
    assert_eq!(model.change_count, 1);
    let block = &model.diff.blocks[model.change_block_indices[0]];
    assert_eq!(block.kind, BlockKind::Insert);
}

#[test]
fn empty_vs_content_left_lines_all_none() {
    let model = load_pair("empty-vs-content");
    // Every display row should have left_line = None (empty file has no lines).
    for row in &model.display_rows {
        assert!(
            row.left_line.is_none(),
            "left file is empty — no left lines"
        );
    }
}

// --- mixed (Go files) ---

#[test]
fn mixed_go_has_multiple_change_types() {
    let model = load_go_pair("mixed");
    assert!(
        model.change_count >= 3,
        "Go fixture should have at least 3 change blocks, got {}",
        model.change_count
    );
}

#[test]
fn mixed_go_display_rows_exceed_max_line_count() {
    let model = load_go_pair("mixed");
    let max_lines = model
        .left_content
        .lines()
        .len()
        .max(model.right_content.lines().len());
    // Insertions/deletions add padding, so display rows >= max file length.
    assert!(
        model.display_rows.len() >= max_lines,
        "display rows ({}) should be >= max line count ({max_lines})",
        model.display_rows.len()
    );
}

#[test]
fn mixed_go_has_inline_diffs_on_replace_blocks() {
    let model = load_go_pair("mixed");
    let replace_blocks: Vec<_> = model
        .change_block_indices
        .iter()
        .map(|&idx| &model.diff.blocks[idx])
        .filter(|b| b.kind == BlockKind::Replace)
        .collect();

    assert!(!replace_blocks.is_empty(), "should have Replace blocks");
    for block in &replace_blocks {
        assert!(
            !block.inline_diffs.is_empty(),
            "Replace block should have inline diffs"
        );
    }
}

#[test]
fn mixed_go_max_content_width_is_reasonable() {
    let model = load_go_pair("mixed");
    // The Go fixture has a long /metrics line. max_content_width should reflect it.
    assert!(
        model.max_content_width > 50,
        "max_content_width ({}) should reflect long lines in fixture",
        model.max_content_width
    );
}
