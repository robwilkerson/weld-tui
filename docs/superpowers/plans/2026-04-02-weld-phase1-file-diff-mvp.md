# Weld Phase 1: File Diff MVP — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a working TUI file diff viewer and two-way merge tool with vim-style keybindings, side-by-side panes, synchronized scrolling, and two-level diff highlighting.

**Architecture:** Cargo workspace with two crates — `weld-core` (diff engine, merge logic, file I/O) and `weld-tui` (ratatui-based UI with crossterm backend). The core is a library crate tested independently; the TUI is a binary crate that depends on core.

**Tech Stack:** Rust, ratatui + crossterm, similar (diff algorithm), clap + clap_complete (CLI)

**Spec:** `docs/superpowers/specs/2026-04-02-weld-tui-diff-design.md`

---

## File Structure

```
weld/
├── Cargo.toml                      # Workspace manifest
├── weld-core/
│   ├── Cargo.toml                  # Library crate
│   └── src/
│       ├── lib.rs                  # Public API re-exports
│       ├── diff.rs                 # DiffResult, DiffBlock, compute_diff()
│       ├── inline_diff.rs          # InlineDiff, character-level diffing
│       ├── merge.rs                # MergeState, copy operations, dirty tracking
│       └── file_io.rs              # FileContent, load/save with line-ending preservation
├── weld-tui/
│   ├── Cargo.toml                  # Binary crate
│   └── src/
│       ├── main.rs                 # CLI parsing, terminal setup/teardown, entry point
│       ├── app.rs                  # App state, mode enum, top-level event dispatch
│       ├── event.rs                # Event loop, crossterm event polling
│       ├── input.rs                # Keybinding dispatch, command-mode parsing (:w, :q, etc.)
│       ├── file_diff/
│       │   ├── mod.rs              # Re-exports
│       │   ├── model.rs            # FileDiffModel: scroll state, active block, viewport
│       │   ├── view.rs             # Render function: panes, gutter, status bar, highlights
│       │   └── overlays.rs         # Help overlay, identical-files banner, same-file warning, save prompt
│       └── theme.rs                # Theme struct with color definitions, default theme
└── test-fixtures/
    ├── left.rs                     # Sample left file for integration tests
    └── right.rs                    # Sample right file for integration tests
```

---

### Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `weld-core/Cargo.toml`
- Create: `weld-core/src/lib.rs`
- Create: `weld-tui/Cargo.toml`
- Create: `weld-tui/src/main.rs`
- Create: `.gitignore`

- [ ] **Step 1: Initialize git repository**

```bash
cd /Users/54695/Development/lookout-software/weld
git init
```

- [ ] **Step 2: Create workspace Cargo.toml**

Create `Cargo.toml`:

```toml
[workspace]
members = ["weld-core", "weld-tui"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
```

- [ ] **Step 3: Create weld-core crate**

Create `weld-core/Cargo.toml`:

```toml
[package]
name = "weld-core"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Diff engine and merge logic for weld"

[dependencies]
similar = { version = "2", features = ["inline"] }

[dev-dependencies]
pretty_assertions = "1"
```

Create `weld-core/src/lib.rs`:

```rust
pub mod diff;
pub mod file_io;
pub mod inline_diff;
pub mod merge;
```

- [ ] **Step 4: Create weld-tui crate**

Create `weld-tui/Cargo.toml`:

```toml
[package]
name = "weld-tui"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "TUI file and directory diff viewer with merge capabilities"

[[bin]]
name = "weld"
path = "src/main.rs"

[dependencies]
weld-core = { path = "../weld-core" }
ratatui = "0.29"
crossterm = "0.28"
clap = { version = "4", features = ["derive"] }
clap_complete = "4"
```

Create `weld-tui/src/main.rs`:

```rust
fn main() {
    println!("weld v0.1.0");
}
```

- [ ] **Step 5: Create .gitignore**

Create `.gitignore`:

```
/target
.superpowers/
```

- [ ] **Step 6: Verify the workspace builds**

```bash
cargo build
```

Expected: builds successfully, produces `target/debug/weld` binary.

- [ ] **Step 7: Run the binary to verify**

```bash
cargo run -p weld-tui
```

Expected: prints `weld v0.1.0`.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml weld-core/ weld-tui/ .gitignore
git commit -m "feat: scaffold cargo workspace with weld-core and weld-tui crates"
```

---

### Task 2: File I/O — Load and Save with Line-Ending Preservation

**Files:**
- Create: `weld-core/src/file_io.rs`
- Modify: `weld-core/src/lib.rs`

- [ ] **Step 1: Write tests for file loading**

Add to `weld-core/src/file_io.rs`:

```rust
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Detected line ending style of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
}

impl LineEnding {
    pub fn as_str(&self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
        }
    }
}

/// Contents of a loaded file, split into lines with metadata.
#[derive(Debug, Clone)]
pub struct FileContent {
    pub path: PathBuf,
    pub lines: Vec<String>,
    pub line_ending: LineEnding,
}

impl FileContent {
    /// Load a file from disk as UTF-8 text.
    pub fn load(path: &Path) -> io::Result<Self> {
        let raw = fs::read_to_string(path)?;
        let line_ending = if raw.contains("\r\n") {
            LineEnding::CrLf
        } else {
            LineEnding::Lf
        };
        let normalized = raw.replace("\r\n", "\n");
        let lines: Vec<String> = normalized.split('\n').map(String::from).collect();
        // Remove trailing empty string from final newline
        let lines = if lines.last().is_some_and(|l| l.is_empty()) {
            lines[..lines.len() - 1].to_vec()
        } else {
            lines
        };
        Ok(FileContent {
            path: path.to_path_buf(),
            lines,
            line_ending,
        })
    }

    /// Save lines back to disk using the original line ending style.
    pub fn save(&self) -> io::Result<()> {
        let ending = self.line_ending.as_str();
        let content = self.lines.join(ending) + ending;
        fs::write(&self.path, content)
    }

    /// Reconstruct the full text content (LF-normalized) for diffing.
    pub fn text(&self) -> String {
        self.lines.join("\n") + "\n"
    }
}

impl fmt::Display for FileContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in &self.lines {
            writeln!(f, "{line}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_lf_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\nline2\nline3\n").unwrap();

        let content = FileContent::load(&path).unwrap();

        assert_eq!(content.lines, vec!["line1", "line2", "line3"]);
        assert_eq!(content.line_ending, LineEnding::Lf);
    }

    #[test]
    fn load_crlf_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\r\nline2\r\nline3\r\n").unwrap();

        let content = FileContent::load(&path).unwrap();

        assert_eq!(content.lines, vec!["line1", "line2", "line3"]);
        assert_eq!(content.line_ending, LineEnding::CrLf);
    }

    #[test]
    fn save_preserves_lf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\nline2\n").unwrap();

        let content = FileContent::load(&path).unwrap();
        content.save().unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        assert_eq!(raw, "line1\nline2\n");
    }

    #[test]
    fn save_preserves_crlf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\r\nline2\r\n").unwrap();

        let content = FileContent::load(&path).unwrap();
        content.save().unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        assert_eq!(raw, "line1\r\nline2\r\n");
    }

    #[test]
    fn load_missing_file_returns_error() {
        let result = FileContent::load(Path::new("/nonexistent/path.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn text_returns_lf_normalized() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "a\r\nb\r\n").unwrap();

        let content = FileContent::load(&path).unwrap();
        assert_eq!(content.text(), "a\nb\n");
    }
}
```

- [ ] **Step 2: Add tempfile dev-dependency**

Update `weld-core/Cargo.toml` `[dev-dependencies]`:

```toml
[dev-dependencies]
pretty_assertions = "1"
tempfile = "3"
```

- [ ] **Step 3: Run tests to verify they pass**

```bash
cargo test -p weld-core -- file_io
```

Expected: all 6 tests pass.

- [ ] **Step 4: Commit**

```bash
git add weld-core/
git commit -m "feat(core): add FileContent with load/save and line-ending preservation"
```

---

### Task 3: Diff Engine — Line-Level and Character-Level Diffs

**Files:**
- Create: `weld-core/src/inline_diff.rs`
- Create: `weld-core/src/diff.rs`

- [ ] **Step 1: Write InlineDiff type**

Create `weld-core/src/inline_diff.rs`:

```rust
/// A segment of an inline (character-level) diff within a single line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineSegment {
    pub kind: InlineKind,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineKind {
    Equal,
    Changed,
}

/// Character-level diff between two lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineDiff {
    pub left_segments: Vec<InlineSegment>,
    pub right_segments: Vec<InlineSegment>,
}

impl InlineDiff {
    /// Compute character-level diff between two lines.
    pub fn compute(left_line: &str, right_line: &str) -> Self {
        use similar::{ChangeTag, TextDiff};

        let diff = TextDiff::from_chars(left_line, right_line);
        let mut left_segments = Vec::new();
        let mut right_segments = Vec::new();

        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Equal => {
                    let text = change.value().to_string();
                    left_segments.push(InlineSegment {
                        kind: InlineKind::Equal,
                        text: text.clone(),
                    });
                    right_segments.push(InlineSegment {
                        kind: InlineKind::Equal,
                        text,
                    });
                }
                ChangeTag::Delete => {
                    left_segments.push(InlineSegment {
                        kind: InlineKind::Changed,
                        text: change.value().to_string(),
                    });
                }
                ChangeTag::Insert => {
                    right_segments.push(InlineSegment {
                        kind: InlineKind::Changed,
                        text: change.value().to_string(),
                    });
                }
            }
        }

        // Merge consecutive segments of the same kind
        InlineDiff {
            left_segments: merge_segments(left_segments),
            right_segments: merge_segments(right_segments),
        }
    }
}

fn merge_segments(segments: Vec<InlineSegment>) -> Vec<InlineSegment> {
    let mut merged: Vec<InlineSegment> = Vec::new();
    for seg in segments {
        if let Some(last) = merged.last_mut() {
            if last.kind == seg.kind {
                last.text.push_str(&seg.text);
                continue;
            }
        }
        merged.push(seg);
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_lines_produce_single_equal_segment() {
        let result = InlineDiff::compute("hello world", "hello world");
        assert_eq!(result.left_segments.len(), 1);
        assert_eq!(result.left_segments[0].kind, InlineKind::Equal);
        assert_eq!(result.left_segments[0].text, "hello world");
    }

    #[test]
    fn completely_different_lines() {
        let result = InlineDiff::compute("aaa", "bbb");
        assert!(result.left_segments.iter().any(|s| s.kind == InlineKind::Changed));
        assert!(result.right_segments.iter().any(|s| s.kind == InlineKind::Changed));
    }

    #[test]
    fn partial_change_detected() {
        let result = InlineDiff::compute("App::new()", "App::with_config(&config)");
        // Should have equal "App::" prefix and changed suffix
        assert!(result.left_segments.len() >= 2);
        assert_eq!(result.left_segments[0].kind, InlineKind::Equal);
        assert!(result.left_segments.iter().any(|s| s.kind == InlineKind::Changed));
    }
}
```

- [ ] **Step 2: Run inline_diff tests**

```bash
cargo test -p weld-core -- inline_diff
```

Expected: all 3 tests pass.

- [ ] **Step 3: Write DiffBlock and compute_diff**

Create `weld-core/src/diff.rs`:

```rust
use std::ops::Range;

use similar::{ChangeTag, TextDiff};

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
    /// Character-level diffs for Replace blocks. One entry per line pair.
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
        let diff = TextDiff::from_lines(&left_text, &right_text);

        let mut blocks: Vec<DiffBlock> = Vec::new();

        for op in diff.ops() {
            let (tag, old_range, new_range) = match *op {
                similar::DiffOp::Equal {
                    old_index,
                    new_index,
                    len,
                } => (
                    BlockKind::Equal,
                    old_index..old_index + len,
                    new_index..new_index + len,
                ),
                similar::DiffOp::Delete {
                    old_index,
                    old_len,
                    new_index,
                } => (
                    BlockKind::Delete,
                    old_index..old_index + old_len,
                    new_index..new_index,
                ),
                similar::DiffOp::Insert {
                    old_index,
                    new_index,
                    new_len,
                } => (
                    BlockKind::Insert,
                    old_index..old_index,
                    new_index..new_index + new_len,
                ),
                similar::DiffOp::Replace {
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

            let inline_diffs = if tag == BlockKind::Replace {
                compute_inline_diffs(
                    &left.lines[old_range.clone()],
                    &right.lines[new_range.clone()],
                )
            } else {
                Vec::new()
            };

            blocks.push(DiffBlock {
                kind: tag,
                left_range: old_range,
                right_range: new_range,
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
/// Pairs lines by index; unpaired lines get no inline diff.
fn compute_inline_diffs(left_lines: &[String], right_lines: &[String]) -> Vec<InlineDiff> {
    let pair_count = left_lines.len().min(right_lines.len());
    (0..pair_count)
        .map(|i| InlineDiff::compute(&left_lines[i], &right_lines[i]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_content(path: &str, lines: Vec<&str>) -> FileContent {
        FileContent {
            path: PathBuf::from(path),
            lines: lines.into_iter().map(String::from).collect(),
            line_ending: crate::file_io::LineEnding::Lf,
        }
    }

    #[test]
    fn identical_files_produce_single_equal_block() {
        let left = make_content("a.txt", vec!["hello", "world"]);
        let right = make_content("b.txt", vec!["hello", "world"]);

        let result = DiffResult::compute(&left, &right);

        assert!(result.is_identical());
        assert_eq!(result.blocks.len(), 1);
        assert_eq!(result.blocks[0].kind, BlockKind::Equal);
    }

    #[test]
    fn insertion_detected() {
        let left = make_content("a.txt", vec!["line1", "line3"]);
        let right = make_content("b.txt", vec!["line1", "line2", "line3"]);

        let result = DiffResult::compute(&left, &right);

        let changes = result.change_blocks();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].1.kind, BlockKind::Insert);
        assert_eq!(changes[0].1.right_range, 1..2);
    }

    #[test]
    fn deletion_detected() {
        let left = make_content("a.txt", vec!["line1", "line2", "line3"]);
        let right = make_content("b.txt", vec!["line1", "line3"]);

        let result = DiffResult::compute(&left, &right);

        let changes = result.change_blocks();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].1.kind, BlockKind::Delete);
        assert_eq!(changes[0].1.left_range, 1..2);
    }

    #[test]
    fn replacement_detected_with_inline_diffs() {
        let left = make_content("a.txt", vec!["let app = App::new();"]);
        let right = make_content("b.txt", vec!["let app = App::with_config(&config);"]);

        let result = DiffResult::compute(&left, &right);

        let changes = result.change_blocks();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].1.kind, BlockKind::Replace);
        assert!(!changes[0].1.inline_diffs.is_empty());
    }

    #[test]
    fn change_blocks_excludes_equal() {
        let left = make_content("a.txt", vec!["same", "different_a", "same"]);
        let right = make_content("b.txt", vec!["same", "different_b", "same"]);

        let result = DiffResult::compute(&left, &right);

        let changes = result.change_blocks();
        assert_eq!(changes.len(), 1);
        // All equal blocks filtered out
        assert_ne!(changes[0].1.kind, BlockKind::Equal);
    }

    #[test]
    fn empty_files_are_identical() {
        let left = make_content("a.txt", vec![]);
        let right = make_content("b.txt", vec![]);

        let result = DiffResult::compute(&left, &right);
        assert!(result.is_identical());
    }
}
```

- [ ] **Step 4: Run diff tests**

```bash
cargo test -p weld-core -- diff::tests
```

Expected: all 6 tests pass.

- [ ] **Step 5: Commit**

```bash
git add weld-core/
git commit -m "feat(core): add diff engine with line-level and character-level diffing"
```

---

### Task 4: Merge State — Copy Operations and Dirty Tracking

**Files:**
- Create: `weld-core/src/merge.rs`

- [ ] **Step 1: Write MergeState with tests**

Create `weld-core/src/merge.rs`:

```rust
use crate::diff::{BlockKind, DiffResult};
use crate::file_io::FileContent;

/// Tracks the merge state: which sides are dirty and applies copy operations.
#[derive(Debug)]
pub struct MergeState {
    pub left: FileContent,
    pub right: FileContent,
    pub diff: DiffResult,
    pub left_dirty: bool,
    pub right_dirty: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MergeError {
    /// The block index is out of range.
    InvalidBlockIndex(usize),
    /// The block is an Equal block and cannot be copied.
    BlockIsEqual(usize),
}

impl std::fmt::Display for MergeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergeError::InvalidBlockIndex(i) => write!(f, "invalid block index: {i}"),
            MergeError::BlockIsEqual(i) => write!(f, "block {i} has no changes to copy"),
        }
    }
}

impl std::error::Error for MergeError {}

impl MergeState {
    pub fn new(left: FileContent, right: FileContent) -> Self {
        let diff = DiffResult::compute(&left, &right);
        MergeState {
            left,
            right,
            diff,
            left_dirty: false,
            right_dirty: false,
        }
    }

    /// Copy the left side's content into the right side for the given block.
    pub fn copy_left_to_right(&mut self, block_index: usize) -> Result<(), MergeError> {
        self.apply_copy(block_index, CopyDirection::LeftToRight)
    }

    /// Copy the right side's content into the left side for the given block.
    pub fn copy_right_to_left(&mut self, block_index: usize) -> Result<(), MergeError> {
        self.apply_copy(block_index, CopyDirection::RightToLeft)
    }

    fn apply_copy(
        &mut self,
        block_index: usize,
        direction: CopyDirection,
    ) -> Result<(), MergeError> {
        let block = self
            .diff
            .blocks
            .get(block_index)
            .ok_or(MergeError::InvalidBlockIndex(block_index))?;

        if block.kind == BlockKind::Equal {
            return Err(MergeError::BlockIsEqual(block_index));
        }

        let left_range = block.left_range.clone();
        let right_range = block.right_range.clone();

        match direction {
            CopyDirection::LeftToRight => {
                let source: Vec<String> = self.left.lines[left_range].to_vec();
                self.right.lines.splice(right_range, source);
                self.right_dirty = true;
            }
            CopyDirection::RightToLeft => {
                let source: Vec<String> = self.right.lines[right_range].to_vec();
                self.left.lines.splice(left_range, source);
                self.left_dirty = true;
            }
        }

        // Recompute diff after modification
        self.diff = DiffResult::compute(&self.left, &self.right);
        Ok(())
    }

    /// Reload both files from disk, discarding all changes.
    pub fn reload(&mut self) -> std::io::Result<()> {
        self.left = FileContent::load(&self.left.path)?;
        self.right = FileContent::load(&self.right.path)?;
        self.diff = DiffResult::compute(&self.left, &self.right);
        self.left_dirty = false;
        self.right_dirty = false;
        Ok(())
    }

    /// Save whichever side(s) are dirty. Returns which sides were saved.
    pub fn save_dirty(&mut self) -> std::io::Result<SaveResult> {
        let saved_left = if self.left_dirty {
            self.left.save()?;
            self.left_dirty = false;
            true
        } else {
            false
        };
        let saved_right = if self.right_dirty {
            self.right.save()?;
            self.right_dirty = false;
            true
        } else {
            false
        };
        Ok(SaveResult {
            saved_left,
            saved_right,
        })
    }

    /// Returns true if either side has unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.left_dirty || self.right_dirty
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SaveResult {
    pub saved_left: bool,
    pub saved_right: bool,
}

#[derive(Debug, Clone, Copy)]
enum CopyDirection {
    LeftToRight,
    RightToLeft,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_io::LineEnding;
    use std::path::PathBuf;

    fn make_content(path: &str, lines: Vec<&str>) -> FileContent {
        FileContent {
            path: PathBuf::from(path),
            lines: lines.into_iter().map(String::from).collect(),
            line_ending: LineEnding::Lf,
        }
    }

    #[test]
    fn copy_left_to_right_makes_block_equal() {
        let left = make_content("a.txt", vec!["same", "left_version", "same"]);
        let right = make_content("b.txt", vec!["same", "right_version", "same"]);

        let mut state = MergeState::new(left, right);
        assert!(!state.diff.is_identical());

        // Find the change block index
        let change_blocks = state.diff.change_blocks();
        let (block_idx, _) = change_blocks[0];

        state.copy_left_to_right(block_idx).unwrap();

        assert!(state.diff.is_identical());
        assert!(state.right_dirty);
        assert!(!state.left_dirty);
        assert_eq!(state.right.lines[1], "left_version");
    }

    #[test]
    fn copy_right_to_left_makes_block_equal() {
        let left = make_content("a.txt", vec!["same", "left_version", "same"]);
        let right = make_content("b.txt", vec!["same", "right_version", "same"]);

        let mut state = MergeState::new(left, right);
        let change_blocks = state.diff.change_blocks();
        let (block_idx, _) = change_blocks[0];

        state.copy_right_to_left(block_idx).unwrap();

        assert!(state.diff.is_identical());
        assert!(state.left_dirty);
        assert!(!state.right_dirty);
        assert_eq!(state.left.lines[1], "right_version");
    }

    #[test]
    fn copy_equal_block_returns_error() {
        let left = make_content("a.txt", vec!["same", "different", "same"]);
        let right = make_content("b.txt", vec!["same", "other", "same"]);

        let mut state = MergeState::new(left, right);

        // Block 0 should be Equal ("same")
        let result = state.copy_left_to_right(0);
        assert_eq!(result, Err(MergeError::BlockIsEqual(0)));
    }

    #[test]
    fn invalid_block_index_returns_error() {
        let left = make_content("a.txt", vec!["a"]);
        let right = make_content("b.txt", vec!["b"]);

        let mut state = MergeState::new(left, right);

        let result = state.copy_left_to_right(999);
        assert_eq!(result, Err(MergeError::InvalidBlockIndex(999)));
    }

    #[test]
    fn copy_insertion_block() {
        let left = make_content("a.txt", vec!["line1", "line3"]);
        let right = make_content("b.txt", vec!["line1", "line2", "line3"]);

        let mut state = MergeState::new(left, right);
        let change_blocks = state.diff.change_blocks();
        let (block_idx, _) = change_blocks[0];

        // Copy right's insertion to left
        state.copy_right_to_left(block_idx).unwrap();

        assert!(state.diff.is_identical());
        assert_eq!(state.left.lines, vec!["line1", "line2", "line3"]);
    }

    #[test]
    fn is_dirty_reflects_state() {
        let left = make_content("a.txt", vec!["a"]);
        let right = make_content("b.txt", vec!["b"]);

        let mut state = MergeState::new(left, right);
        assert!(!state.is_dirty());

        let change_blocks = state.diff.change_blocks();
        let (block_idx, _) = change_blocks[0];
        state.copy_left_to_right(block_idx).unwrap();

        assert!(state.is_dirty());
    }
}
```

- [ ] **Step 2: Run merge tests**

```bash
cargo test -p weld-core -- merge::tests
```

Expected: all 6 tests pass.

- [ ] **Step 3: Commit**

```bash
git add weld-core/src/merge.rs
git commit -m "feat(core): add MergeState with copy operations, dirty tracking, and reload"
```

---

### Task 5: CLI Argument Parsing and Entry Point

**Files:**
- Modify: `weld-tui/src/main.rs`

- [ ] **Step 1: Write CLI parsing with clap**

Replace `weld-tui/src/main.rs` with:

```rust
use std::path::PathBuf;
use std::process;

use clap::{CommandFactory, Parser, ValueHint};
use clap_complete::{generate, Shell};

#[derive(Parser)]
#[command(
    name = "weld",
    version,
    about = "TUI file and directory diff viewer with merge capabilities"
)]
struct Cli {
    /// Left file or directory to compare
    #[arg(value_hint = ValueHint::AnyPath)]
    left: Option<PathBuf>,

    /// Right file or directory to compare
    #[arg(value_hint = ValueHint::AnyPath)]
    right: Option<PathBuf>,

    /// Generate shell completions
    #[arg(long, value_name = "SHELL")]
    completions: Option<Shell>,
}

fn main() {
    let cli = Cli::parse();

    if let Some(shell) = cli.completions {
        let mut cmd = Cli::command();
        generate(shell, &mut cmd, "weld", &mut std::io::stdout());
        return;
    }

    let (left, right) = match (cli.left, cli.right) {
        (Some(l), Some(r)) => (l, r),
        _ => {
            eprintln!("Usage: weld <left-path> <right-path>");
            eprintln!("Run 'weld --help' for more information.");
            process::exit(2);
        }
    };

    // Validate paths exist
    if !left.exists() {
        eprintln!("Error: path does not exist: {}", left.display());
        process::exit(2);
    }
    if !right.exists() {
        eprintln!("Error: path does not exist: {}", right.display());
        process::exit(2);
    }

    // Validate both are files or both are directories
    let left_is_file = left.is_file();
    let right_is_file = right.is_file();
    if left_is_file != right_is_file {
        eprintln!(
            "Error: cannot compare a file with a directory: {} vs {}",
            left.display(),
            right.display()
        );
        process::exit(2);
    }

    if !left_is_file {
        eprintln!("Error: directory diff is not yet supported");
        process::exit(2);
    }

    // Detect same file
    let same_file = match (left.canonicalize(), right.canonicalize()) {
        (Ok(l), Ok(r)) => l == r,
        _ => false,
    };

    // Load files and launch TUI (placeholder for now)
    match run(left, right, same_file) {
        Ok(has_differences) => {
            if has_differences {
                process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(2);
        }
    }
}

fn run(left: PathBuf, right: PathBuf, _same_file: bool) -> Result<bool, Box<dyn std::error::Error>> {
    let left_content = weld_core::file_io::FileContent::load(&left)?;
    let right_content = weld_core::file_io::FileContent::load(&right)?;
    let merge_state = weld_core::merge::MergeState::new(left_content, right_content);

    let has_differences = !merge_state.diff.is_identical();

    // TUI launch will go here in the next task
    println!(
        "Comparing {} and {} — {}",
        left.display(),
        right.display(),
        if has_differences {
            "files differ"
        } else {
            "files are identical"
        }
    );

    Ok(has_differences)
}
```

- [ ] **Step 2: Verify it builds and runs**

```bash
cargo build -p weld-tui
```

Expected: builds successfully.

- [ ] **Step 3: Test CLI argument validation**

```bash
# No args — should print usage
cargo run -p weld-tui 2>&1; echo "exit: $?"
```

Expected: prints usage message, exits with code 2.

```bash
# Missing file — should print error
cargo run -p weld-tui -- /nonexistent /also-nonexistent 2>&1; echo "exit: $?"
```

Expected: prints error about missing path, exits with code 2.

- [ ] **Step 4: Test shell completions**

```bash
cargo run -p weld-tui -- --completions zsh | head -5
```

Expected: outputs zsh completion script.

- [ ] **Step 5: Commit**

```bash
git add weld-tui/
git commit -m "feat(tui): add CLI argument parsing with clap, path validation, and shell completions"
```

---

### Task 6: Theme Struct and Default Colors

**Files:**
- Create: `weld-tui/src/theme.rs`

- [ ] **Step 1: Write theme definitions**

Create `weld-tui/src/theme.rs`:

```rust
use ratatui::style::{Color, Modifier, Style};

/// All colors and styles used by the TUI, in one place.
/// Designed to be swappable for future theming support.
pub struct Theme {
    /// Background for the entire app
    pub bg: Color,
    /// Default foreground text
    pub fg: Color,
    /// Header bar background
    pub header_bg: Color,
    /// Header file path text
    pub header_fg: Color,
    /// Dirty indicator dot in header
    pub dirty_indicator: Color,
    /// Status bar background
    pub status_bar_bg: Color,
    /// Status bar text
    pub status_bar_fg: Color,
    /// Line number foreground
    pub line_number_fg: Color,
    /// Gutter background (content area)
    pub gutter_bg: Color,
    /// Active diff dot in gutter
    pub gutter_dot: Color,
    /// Scrollbar track
    pub scrollbar_track: Color,
    /// Scrollbar thumb
    pub scrollbar_thumb: Color,
    /// Background tint for deleted lines (left side of Replace/Delete blocks)
    pub diff_delete_bg: Color,
    /// Stronger highlight for changed characters within deleted lines
    pub diff_delete_emphasis_bg: Color,
    /// Background tint for inserted lines (right side of Replace/Insert blocks)
    pub diff_insert_bg: Color,
    /// Stronger highlight for changed characters within inserted lines
    pub diff_insert_emphasis_bg: Color,
    /// Style for the currently active diff block gutter indicator
    pub active_block_style: Style,
    /// Overlay background (help, prompts)
    pub overlay_bg: Color,
    /// Overlay text
    pub overlay_fg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            bg: Color::Indexed(235),            // dark gray
            fg: Color::Indexed(252),            // light gray
            header_bg: Color::Indexed(236),     // slightly lighter dark
            header_fg: Color::Indexed(75),      // blue
            dirty_indicator: Color::Indexed(208), // orange
            status_bar_bg: Color::Indexed(238), // medium dark
            status_bar_fg: Color::Indexed(249), // medium light
            line_number_fg: Color::Indexed(242), // dim gray
            gutter_bg: Color::Indexed(233),     // very dark
            gutter_dot: Color::Indexed(204),    // pink/red
            scrollbar_track: Color::Indexed(233),
            scrollbar_thumb: Color::Indexed(245),
            diff_delete_bg: Color::Indexed(52),   // dark red
            diff_delete_emphasis_bg: Color::Indexed(88), // brighter red
            diff_insert_bg: Color::Indexed(22),   // dark green
            diff_insert_emphasis_bg: Color::Indexed(28), // brighter green
            active_block_style: Style::default().fg(Color::Indexed(204)),
            overlay_bg: Color::Indexed(237),
            overlay_fg: Color::Indexed(252),
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build -p weld-tui
```

Expected: builds successfully.

- [ ] **Step 3: Commit**

```bash
git add weld-tui/src/theme.rs
git commit -m "feat(tui): add Theme struct with default dark color scheme"
```

---

### Task 7: Event Loop and App Shell

**Files:**
- Create: `weld-tui/src/event.rs`
- Create: `weld-tui/src/app.rs`
- Modify: `weld-tui/src/main.rs`

- [ ] **Step 1: Write the event loop**

Create `weld-tui/src/event.rs`:

```rust
use std::time::Duration;

use crossterm::event::{self, Event, KeyEvent};

/// Polls for terminal events with a timeout.
/// Returns None if no event within the timeout period.
pub fn poll_event(timeout: Duration) -> std::io::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}
```

- [ ] **Step 2: Write the App struct**

Create `weld-tui/src/app.rs`:

```rust
use weld_core::merge::MergeState;

use crate::theme::Theme;

/// The action the app should take after handling an event.
pub enum Action {
    /// Continue the event loop.
    Continue,
    /// Quit the application. Bool indicates whether files still differ.
    Quit(bool),
}

/// Top-level application state.
pub struct App {
    pub merge_state: MergeState,
    pub theme: Theme,
    pub same_file: bool,
    /// Whether we should show the "files are identical" overlay.
    pub show_identical_overlay: bool,
    /// Whether we should show the "same file" warning.
    pub show_same_file_warning: bool,
    /// Current active diff block index (into change_blocks list).
    pub active_change_index: usize,
    /// Vertical scroll offset (in lines).
    pub scroll_y: usize,
    /// Horizontal scroll offset (in columns).
    pub scroll_x: usize,
    /// Whether we're in command mode (typing :w, :q, etc.).
    pub command_mode: bool,
    /// Current command buffer when in command mode.
    pub command_buffer: String,
    /// Whether to show the help overlay.
    pub show_help: bool,
    /// Pending save prompt (when both sides are dirty).
    pub save_prompt: Option<SavePrompt>,
    /// Pending quit confirmation (when there are unsaved changes).
    pub quit_prompt: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum SavePrompt {
    Both,
}

impl App {
    pub fn new(merge_state: MergeState, same_file: bool) -> Self {
        let is_identical = merge_state.diff.is_identical();
        App {
            merge_state,
            theme: Theme::default(),
            same_file,
            show_identical_overlay: is_identical && !same_file,
            show_same_file_warning: same_file,
            active_change_index: 0,
            scroll_y: 0,
            scroll_x: 0,
            command_mode: false,
            command_buffer: String::new(),
            show_help: false,
            save_prompt: None,
            quit_prompt: false,
        }
    }

    /// Total number of change blocks (non-Equal blocks).
    pub fn change_block_count(&self) -> usize {
        self.merge_state.diff.change_blocks().len()
    }

    /// Get the actual block index (into blocks vec) for the current active change.
    pub fn active_block_index(&self) -> Option<usize> {
        let changes = self.merge_state.diff.change_blocks();
        changes
            .get(self.active_change_index)
            .map(|(idx, _)| *idx)
    }
}
```

- [ ] **Step 3: Wire up terminal setup/teardown and app loop in main.rs**

Update `weld-tui/src/main.rs`. Replace the `run` function and add module declarations at the top:

Add after the existing `use` statements at the top:

```rust
mod app;
mod event;
mod theme;

// Add these imports
use std::io::{self, stdout};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
```

Replace the `run` function:

```rust
fn run(left: PathBuf, right: PathBuf, same_file: bool) -> Result<bool, Box<dyn std::error::Error>> {
    let left_content = weld_core::file_io::FileContent::load(&left)?;
    let right_content = weld_core::file_io::FileContent::load(&right)?;
    let merge_state = weld_core::merge::MergeState::new(left_content, right_content);

    let mut app = app::App::new(merge_state, same_file);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    let result = main_loop(&mut terminal, &mut app);

    // Teardown terminal (always, even on error)
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    let has_differences = result?;
    Ok(has_differences)
}

fn main_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut app::App,
) -> Result<bool, Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|frame| {
            // Rendering will be implemented in the next task
            let area = frame.area();
            let block = ratatui::widgets::Block::default()
                .title(" weld — press q to quit ")
                .borders(ratatui::widgets::Borders::ALL);
            frame.render_widget(block, area);
        })?;

        if let Some(crossterm::event::Event::Key(key)) =
            event::poll_event(std::time::Duration::from_millis(50))?
        {
            use crossterm::event::KeyCode;
            match key.code {
                KeyCode::Char('q') if !app.command_mode => {
                    let has_differences = !app.merge_state.diff.is_identical();
                    return Ok(has_differences);
                }
                _ => {}
            }
        }
    }
}
```

- [ ] **Step 4: Verify it builds and launches**

Create two temp test files and run:

```bash
echo "hello\nworld" > /tmp/weld-test-left.txt
echo "hello\nearth" > /tmp/weld-test-right.txt
cargo run -p weld-tui -- /tmp/weld-test-left.txt /tmp/weld-test-right.txt
```

Expected: TUI launches showing a bordered box. Press `q` to quit. Terminal restores properly after exit.

- [ ] **Step 5: Commit**

```bash
git add weld-tui/src/
git commit -m "feat(tui): add event loop, App state, and terminal setup/teardown"
```

---

### Task 8: File Diff View — Rendering Side-by-Side Panes

**Files:**
- Create: `weld-tui/src/file_diff/mod.rs`
- Create: `weld-tui/src/file_diff/model.rs`
- Create: `weld-tui/src/file_diff/view.rs`
- Modify: `weld-tui/src/main.rs`

- [ ] **Step 1: Create the FileDiffModel**

Create `weld-tui/src/file_diff/mod.rs`:

```rust
pub mod model;
pub mod view;
```

Create `weld-tui/src/file_diff/model.rs`:

```rust
use weld_core::diff::{BlockKind, DiffBlock};
use weld_core::merge::MergeState;

/// Computes the display lines for the side-by-side view.
/// Equal blocks map 1:1. For Insert/Delete/Replace blocks,
/// the shorter side gets padding (empty lines) to stay aligned.
#[derive(Debug, Clone)]
pub struct DisplayLine {
    pub kind: DisplayLineKind,
    pub left_line_num: Option<usize>,
    pub left_text: String,
    pub right_line_num: Option<usize>,
    pub right_text: String,
    pub block_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayLineKind {
    Equal,
    Delete,
    Insert,
    Replace,
    Padding,
}

/// Build the full list of display lines from the current merge state.
pub fn build_display_lines(merge_state: &MergeState) -> Vec<DisplayLine> {
    let mut lines = Vec::new();

    for (block_idx, block) in merge_state.diff.blocks.iter().enumerate() {
        match block.kind {
            BlockKind::Equal => {
                for (i, offset) in (block.left_range.start..block.left_range.end).enumerate() {
                    let right_offset = block.right_range.start + i;
                    lines.push(DisplayLine {
                        kind: DisplayLineKind::Equal,
                        left_line_num: Some(offset + 1),
                        left_text: merge_state.left.lines[offset].clone(),
                        right_line_num: Some(right_offset + 1),
                        right_text: merge_state.right.lines[right_offset].clone(),
                        block_index: block_idx,
                    });
                }
            }
            BlockKind::Delete => {
                for offset in block.left_range.start..block.left_range.end {
                    lines.push(DisplayLine {
                        kind: DisplayLineKind::Delete,
                        left_line_num: Some(offset + 1),
                        left_text: merge_state.left.lines[offset].clone(),
                        right_line_num: None,
                        right_text: String::new(),
                        block_index: block_idx,
                    });
                }
            }
            BlockKind::Insert => {
                for offset in block.right_range.start..block.right_range.end {
                    lines.push(DisplayLine {
                        kind: DisplayLineKind::Insert,
                        left_line_num: None,
                        left_text: String::new(),
                        right_line_num: Some(offset + 1),
                        right_text: merge_state.right.lines[offset].clone(),
                        block_index: block_idx,
                    });
                }
            }
            BlockKind::Replace => {
                let left_len = block.left_range.len();
                let right_len = block.right_range.len();
                let max_len = left_len.max(right_len);

                for i in 0..max_len {
                    let (left_num, left_text) = if i < left_len {
                        let offset = block.left_range.start + i;
                        (Some(offset + 1), merge_state.left.lines[offset].clone())
                    } else {
                        (None, String::new())
                    };

                    let (right_num, right_text) = if i < right_len {
                        let offset = block.right_range.start + i;
                        (Some(offset + 1), merge_state.right.lines[offset].clone())
                    } else {
                        (None, String::new())
                    };

                    let kind = if left_num.is_some() && right_num.is_some() {
                        DisplayLineKind::Replace
                    } else {
                        DisplayLineKind::Padding
                    };

                    lines.push(DisplayLine {
                        kind,
                        left_line_num: left_num,
                        left_text,
                        right_line_num: right_num,
                        right_text,
                        block_index: block_idx,
                    });
                }
            }
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use weld_core::file_io::{FileContent, LineEnding};
    use std::path::PathBuf;

    fn make_content(path: &str, lines: Vec<&str>) -> FileContent {
        FileContent {
            path: PathBuf::from(path),
            lines: lines.into_iter().map(String::from).collect(),
            line_ending: LineEnding::Lf,
        }
    }

    #[test]
    fn equal_files_produce_all_equal_display_lines() {
        let left = make_content("a.txt", vec!["line1", "line2"]);
        let right = make_content("b.txt", vec!["line1", "line2"]);
        let state = MergeState::new(left, right);

        let lines = build_display_lines(&state);

        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|l| l.kind == DisplayLineKind::Equal));
        assert_eq!(lines[0].left_line_num, Some(1));
        assert_eq!(lines[0].right_line_num, Some(1));
    }

    #[test]
    fn insertion_creates_lines_with_empty_left_side() {
        let left = make_content("a.txt", vec!["a", "c"]);
        let right = make_content("b.txt", vec!["a", "b", "c"]);
        let state = MergeState::new(left, right);

        let lines = build_display_lines(&state);

        let insert_lines: Vec<_> = lines.iter().filter(|l| l.kind == DisplayLineKind::Insert).collect();
        assert_eq!(insert_lines.len(), 1);
        assert_eq!(insert_lines[0].left_line_num, None);
        assert_eq!(insert_lines[0].right_text, "b");
    }

    #[test]
    fn replace_block_pads_shorter_side() {
        let left = make_content("a.txt", vec!["old1", "old2"]);
        let right = make_content("b.txt", vec!["new1", "new2", "new3"]);
        let state = MergeState::new(left, right);

        let lines = build_display_lines(&state);

        // 2 paired Replace lines + 1 Padding line (left side shorter)
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[2].left_line_num, None); // padding on left
        assert_eq!(lines[2].right_line_num, Some(3));
    }
}
```

- [ ] **Step 2: Run model tests**

```bash
cargo test -p weld-tui -- file_diff::model
```

Expected: all 3 tests pass.

- [ ] **Step 3: Write the view renderer**

Create `weld-tui/src/file_diff/view.rs`:

```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::Frame;

use weld_core::diff::BlockKind;
use weld_core::inline_diff::InlineKind;

use crate::app::App;
use crate::file_diff::model::{build_display_lines, DisplayLine, DisplayLineKind};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let display_lines = build_display_lines(&app.merge_state);
    let total_lines = display_lines.len();

    // Top-level layout: header, content, status bar
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Min(1),   // content
            Constraint::Length(1), // status bar
        ])
        .split(area);

    let header_area = vertical[0];
    let content_area = vertical[1];
    let status_area = vertical[2];

    // Content: left pane + left scrollbar + gutter + right pane + right scrollbar
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(10),    // left pane
            Constraint::Length(1),  // left scrollbar
            Constraint::Length(3),  // gutter
            Constraint::Min(10),   // right pane
            Constraint::Length(1),  // right scrollbar
        ])
        .split(content_area);

    let left_pane = horizontal[0];
    let left_scroll_area = horizontal[1];
    let gutter_area = horizontal[2];
    let right_pane = horizontal[3];
    let right_scroll_area = horizontal[4];

    // Determine which block is active
    let active_block_idx = app.active_block_index();

    // Render header
    render_header(frame, app, header_area, gutter_area.width);

    // Render panes
    let viewport_height = left_pane.height as usize;
    let visible_lines = get_visible_lines(&display_lines, app.scroll_y, viewport_height);

    render_pane(
        frame,
        app,
        left_pane,
        &visible_lines,
        PaneSide::Left,
        active_block_idx,
    );
    render_pane(
        frame,
        app,
        right_pane,
        &visible_lines,
        PaneSide::Right,
        active_block_idx,
    );

    // Render gutter
    render_gutter(frame, app, gutter_area, &visible_lines, active_block_idx);

    // Render scrollbars
    render_scrollbar(frame, left_scroll_area, app.scroll_y, total_lines, viewport_height);
    render_scrollbar(frame, right_scroll_area, app.scroll_y, total_lines, viewport_height);

    // Render status bar
    render_status_bar(frame, app, status_area);
}

fn render_header(frame: &mut Frame, app: &App, area: Rect, gutter_width: u16) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(1),  // left scrollbar spacer
            Constraint::Length(gutter_width),
            Constraint::Min(10),
            Constraint::Length(1),  // right scrollbar spacer
        ])
        .split(area);

    let left_header = layout[0];
    let gutter_header = layout[2];
    let right_header = layout[3];

    let header_style = Style::default()
        .bg(app.theme.header_bg)
        .fg(app.theme.header_fg);

    // Left filename + dirty indicator
    let left_path = app.merge_state.left.path.display().to_string();
    let mut left_spans = vec![Span::styled(format!(" {left_path}"), header_style)];
    if app.merge_state.left_dirty {
        left_spans.push(Span::styled(
            " ●",
            Style::default()
                .bg(app.theme.header_bg)
                .fg(app.theme.dirty_indicator),
        ));
    }
    frame.render_widget(
        Paragraph::new(Line::from(left_spans)).style(Style::default().bg(app.theme.header_bg)),
        left_header,
    );

    // Gutter header (matches header bg)
    frame.render_widget(
        Block::default().style(Style::default().bg(app.theme.header_bg)),
        gutter_header,
    );

    // Right filename + dirty indicator
    let right_path = app.merge_state.right.path.display().to_string();
    let mut right_spans = vec![Span::styled(format!(" {right_path}"), header_style)];
    if app.merge_state.right_dirty {
        right_spans.push(Span::styled(
            " ●",
            Style::default()
                .bg(app.theme.header_bg)
                .fg(app.theme.dirty_indicator),
        ));
    }
    frame.render_widget(
        Paragraph::new(Line::from(right_spans)).style(Style::default().bg(app.theme.header_bg)),
        right_header,
    );
}

#[derive(Clone, Copy)]
enum PaneSide {
    Left,
    Right,
}

fn render_pane(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    visible_lines: &[&DisplayLine],
    side: PaneSide,
    active_block_idx: Option<usize>,
) {
    let line_num_width = 5; // "NNNN "
    let mut text_lines: Vec<Line> = Vec::new();

    for display_line in visible_lines {
        let (line_num, text) = match side {
            PaneSide::Left => (display_line.left_line_num, &display_line.left_text),
            PaneSide::Right => (display_line.right_line_num, &display_line.right_text),
        };

        let num_str = match line_num {
            Some(n) => format!("{n:>4} "),
            None => "     ".to_string(),
        };

        let bg_color = line_bg_color(display_line, side, &app.theme);
        let is_active = active_block_idx == Some(display_line.block_index)
            && display_line.kind != DisplayLineKind::Equal;

        let num_span = Span::styled(
            num_str,
            Style::default()
                .fg(app.theme.line_number_fg)
                .bg(bg_color),
        );

        // For Replace blocks with inline diffs, render with character-level highlighting
        let text_spans = if display_line.kind == DisplayLineKind::Replace {
            build_inline_spans(app, display_line, side, bg_color)
        } else {
            let visible_text = apply_horizontal_scroll(text, app.scroll_x, area.width as usize - line_num_width);
            vec![Span::styled(
                visible_text,
                Style::default().fg(app.theme.fg).bg(bg_color),
            )]
        };

        let mut spans = vec![num_span];
        spans.extend(text_spans);
        text_lines.push(Line::from(spans));
    }

    // Fill remaining viewport with empty lines
    let remaining = area.height as usize - text_lines.len().min(area.height as usize);
    for _ in 0..remaining {
        text_lines.push(Line::from(Span::styled(
            " ".repeat(area.width as usize),
            Style::default().bg(app.theme.bg),
        )));
    }

    let paragraph = Paragraph::new(text_lines).style(Style::default().bg(app.theme.bg));
    frame.render_widget(paragraph, area);
}

fn build_inline_spans(
    app: &App,
    display_line: &DisplayLine,
    side: PaneSide,
    bg_color: Color,
) -> Vec<Span<'static>> {
    let block = &app.merge_state.diff.blocks[display_line.block_index];

    // Find which inline diff line this corresponds to
    let line_offset = match side {
        PaneSide::Left => display_line
            .left_line_num
            .map(|n| n - 1 - block.left_range.start),
        PaneSide::Right => display_line
            .right_line_num
            .map(|n| n - 1 - block.right_range.start),
    };

    let emphasis_bg = match side {
        PaneSide::Left => app.theme.diff_delete_emphasis_bg,
        PaneSide::Right => app.theme.diff_insert_emphasis_bg,
    };

    if let Some(offset) = line_offset {
        if let Some(inline) = block.inline_diffs.get(offset) {
            let segments = match side {
                PaneSide::Left => &inline.left_segments,
                PaneSide::Right => &inline.right_segments,
            };
            return segments
                .iter()
                .map(|seg| {
                    let bg = match seg.kind {
                        InlineKind::Equal => bg_color,
                        InlineKind::Changed => emphasis_bg,
                    };
                    Span::styled(
                        seg.text.clone(),
                        Style::default().fg(app.theme.fg).bg(bg),
                    )
                })
                .collect();
        }
    }

    // Fallback: no inline diff data, render as plain text
    let text = match side {
        PaneSide::Left => &display_line.left_text,
        PaneSide::Right => &display_line.right_text,
    };
    vec![Span::styled(
        text.clone(),
        Style::default().fg(app.theme.fg).bg(bg_color),
    )]
}

fn line_bg_color(display_line: &DisplayLine, side: PaneSide, theme: &Theme) -> Color {
    match (display_line.kind, side) {
        (DisplayLineKind::Equal, _) => theme.bg,
        (DisplayLineKind::Delete, PaneSide::Left) => theme.diff_delete_bg,
        (DisplayLineKind::Delete, PaneSide::Right) => theme.bg,
        (DisplayLineKind::Insert, PaneSide::Left) => theme.bg,
        (DisplayLineKind::Insert, PaneSide::Right) => theme.diff_insert_bg,
        (DisplayLineKind::Replace, PaneSide::Left) => theme.diff_delete_bg,
        (DisplayLineKind::Replace, PaneSide::Right) => theme.diff_insert_bg,
        (DisplayLineKind::Padding, _) => theme.bg,
    }
}

fn render_gutter(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    visible_lines: &[&DisplayLine],
    active_block_idx: Option<usize>,
) {
    let mut gutter_lines: Vec<Line> = Vec::new();

    // Group visible lines by block index to find the vertical center of the active block
    let active_line_range = active_block_idx.map(|idx| {
        let start = visible_lines
            .iter()
            .position(|l| l.block_index == idx && l.kind != DisplayLineKind::Equal);
        let end = visible_lines
            .iter()
            .rposition(|l| l.block_index == idx && l.kind != DisplayLineKind::Equal);
        match (start, end) {
            (Some(s), Some(e)) => Some((s + e) / 2),
            _ => None,
        }
    }).flatten();

    for (i, _display_line) in visible_lines.iter().enumerate() {
        let content = if active_line_range == Some(i) {
            Span::styled(" ● ", Style::default().fg(app.theme.gutter_dot).bg(app.theme.gutter_bg))
        } else {
            Span::styled("   ", Style::default().bg(app.theme.gutter_bg))
        };
        gutter_lines.push(Line::from(content));
    }

    // Fill remaining
    let remaining = area.height as usize - gutter_lines.len().min(area.height as usize);
    for _ in 0..remaining {
        gutter_lines.push(Line::from(Span::styled(
            "   ",
            Style::default().bg(app.theme.gutter_bg),
        )));
    }

    let paragraph = Paragraph::new(gutter_lines);
    frame.render_widget(paragraph, area);
}

fn render_scrollbar(
    frame: &mut Frame,
    area: Rect,
    scroll_offset: usize,
    total_lines: usize,
    viewport_height: usize,
) {
    if total_lines <= viewport_height {
        // No scrollbar needed — fill with background
        let block = Block::default().style(Style::default().bg(Color::Indexed(233)));
        frame.render_widget(block, area);
        return;
    }

    let mut state = ScrollbarState::new(total_lines.saturating_sub(viewport_height))
        .position(scroll_offset);

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .track_symbol(Some("│"))
        .thumb_symbol("█");

    frame.render_stateful_widget(scrollbar, area, &mut state);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let change_count = app.change_block_count();
    let current = if change_count > 0 {
        app.active_change_index + 1
    } else {
        0
    };

    let left_text = if app.command_mode {
        format!(":{}", app.command_buffer)
    } else {
        format!("Block {current} of {change_count}")
    };

    let right_text = "j/k nav · H/L copy · :w save · :q quit · ? help";

    let status_style = Style::default()
        .bg(app.theme.status_bar_bg)
        .fg(app.theme.status_bar_fg);

    let left_span = Span::styled(format!(" {left_text}"), status_style);
    let right_span = Span::styled(format!("{right_text} "), status_style);

    // Render left-aligned and right-aligned text on the status bar
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    frame.render_widget(
        Paragraph::new(Line::from(left_span)).style(status_style),
        layout[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(right_span))
            .style(status_style)
            .alignment(ratatui::layout::Alignment::Right),
        layout[1],
    );
}

fn get_visible_lines<'a>(
    display_lines: &'a [DisplayLine],
    scroll_y: usize,
    viewport_height: usize,
) -> Vec<&'a DisplayLine> {
    display_lines
        .iter()
        .skip(scroll_y)
        .take(viewport_height)
        .collect()
}

fn apply_horizontal_scroll(text: &str, scroll_x: usize, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let start = scroll_x.min(chars.len());
    let end = (start + width).min(chars.len());
    chars[start..end].iter().collect()
}
```

- [ ] **Step 4: Wire the view into main.rs**

In `weld-tui/src/main.rs`, add the module declaration:

```rust
mod file_diff;
```

Replace the `terminal.draw` closure in `main_loop`:

```rust
        terminal.draw(|frame| {
            file_diff::view::render(frame, app);
        })?;
```

- [ ] **Step 5: Build and test visually**

```bash
echo -e "fn main() {\n    let app = App::new();\n    app.run();\n}" > /tmp/weld-left.rs
echo -e "fn main() {\n    let app = App::with_config(&config);\n    app.start();\n}" > /tmp/weld-right.rs
cargo run -p weld-tui -- /tmp/weld-left.rs /tmp/weld-right.rs
```

Expected: side-by-side diff view renders with colored highlighting. Press `q` to quit.

- [ ] **Step 6: Commit**

```bash
git add weld-tui/src/file_diff/
git commit -m "feat(tui): add side-by-side file diff rendering with two-level highlighting and gutter"
```

---

### Task 9: Keybinding Dispatch and Input Handling

**Files:**
- Create: `weld-tui/src/input.rs`
- Modify: `weld-tui/src/app.rs`
- Modify: `weld-tui/src/main.rs`

- [ ] **Step 1: Write the input handler**

Create `weld-tui/src/input.rs`:

```rust
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{Action, App, SavePrompt};

pub fn handle_key(app: &mut App, key: KeyEvent) -> Action {
    // Handle overlays first
    if app.show_same_file_warning || app.show_identical_overlay {
        return handle_overlay_key(app, key);
    }

    if app.quit_prompt {
        return handle_quit_prompt(app, key);
    }

    if let Some(prompt) = app.save_prompt {
        return handle_save_prompt(app, key, prompt);
    }

    if app.show_help {
        return handle_help_key(app, key);
    }

    if app.command_mode {
        return handle_command_mode(app, key);
    }

    handle_normal_mode(app, key)
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) -> Action {
    match key.code {
        // Navigation between diff blocks
        KeyCode::Char('j') => {
            let count = app.change_block_count();
            if count > 0 && app.active_change_index < count - 1 {
                app.active_change_index += 1;
                scroll_to_active_block(app);
            }
            Action::Continue
        }
        KeyCode::Char('k') => {
            if app.active_change_index > 0 {
                app.active_change_index -= 1;
                scroll_to_active_block(app);
            }
            Action::Continue
        }

        // Copy block operations
        KeyCode::Char('L') => {
            // Copy active block from left → right
            if let Some(block_idx) = app.active_block_index() {
                let _ = app.merge_state.copy_left_to_right(block_idx);
                // Clamp active index if blocks changed
                clamp_active_index(app);
            }
            Action::Continue
        }
        KeyCode::Char('H') => {
            // Copy active block from right → left
            if let Some(block_idx) = app.active_block_index() {
                let _ = app.merge_state.copy_right_to_left(block_idx);
                clamp_active_index(app);
            }
            Action::Continue
        }

        // Scrolling
        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let half_page = app.viewport_height / 2;
            app.scroll_y = app.scroll_y.saturating_add(half_page);
            clamp_scroll(app);
            Action::Continue
        }
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let half_page = app.viewport_height / 2;
            app.scroll_y = app.scroll_y.saturating_sub(half_page);
            Action::Continue
        }
        KeyCode::Char('g') => {
            app.scroll_y = 0;
            Action::Continue
        }
        KeyCode::Char('G') => {
            let total = app.total_display_lines();
            app.scroll_y = total.saturating_sub(app.viewport_height);
            Action::Continue
        }

        // Horizontal scroll
        KeyCode::Left => {
            app.scroll_x = app.scroll_x.saturating_sub(4);
            Action::Continue
        }
        KeyCode::Right => {
            app.scroll_x = app.scroll_x.saturating_add(4);
            Action::Continue
        }

        // Enter command mode
        KeyCode::Char(':') => {
            app.command_mode = true;
            app.command_buffer.clear();
            Action::Continue
        }

        // Help
        KeyCode::Char('?') => {
            app.show_help = true;
            Action::Continue
        }

        // Quick quit
        KeyCode::Char('q') => {
            if app.merge_state.is_dirty() {
                app.quit_prompt = true;
                Action::Continue
            } else {
                Action::Quit(!app.merge_state.diff.is_identical())
            }
        }

        _ => Action::Continue,
    }
}

fn handle_command_mode(app: &mut App, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter => {
            let cmd = app.command_buffer.clone();
            app.command_mode = false;
            app.command_buffer.clear();
            execute_command(app, &cmd)
        }
        KeyCode::Esc => {
            app.command_mode = false;
            app.command_buffer.clear();
            Action::Continue
        }
        KeyCode::Backspace => {
            app.command_buffer.pop();
            if app.command_buffer.is_empty() {
                app.command_mode = false;
            }
            Action::Continue
        }
        KeyCode::Char(c) => {
            app.command_buffer.push(c);
            Action::Continue
        }
        _ => Action::Continue,
    }
}

fn execute_command(app: &mut App, cmd: &str) -> Action {
    match cmd {
        "w" => {
            if app.merge_state.left_dirty && app.merge_state.right_dirty {
                app.save_prompt = Some(SavePrompt::Both);
                Action::Continue
            } else {
                let _ = app.merge_state.save_dirty();
                Action::Continue
            }
        }
        "q" => {
            if app.merge_state.is_dirty() {
                app.quit_prompt = true;
                Action::Continue
            } else {
                Action::Quit(!app.merge_state.diff.is_identical())
            }
        }
        "q!" => Action::Quit(!app.merge_state.diff.is_identical()),
        "wq" => {
            if app.merge_state.left_dirty && app.merge_state.right_dirty {
                app.save_prompt = Some(SavePrompt::Both);
                // After save prompt resolves, we'd quit — but for simplicity,
                // save both and quit
                let _ = app.merge_state.save_dirty();
                Action::Quit(!app.merge_state.diff.is_identical())
            } else {
                let _ = app.merge_state.save_dirty();
                Action::Quit(!app.merge_state.diff.is_identical())
            }
        }
        "e!" => {
            let _ = app.merge_state.reload();
            app.active_change_index = 0;
            app.scroll_y = 0;
            app.scroll_x = 0;
            Action::Continue
        }
        _ => Action::Continue, // unknown command, ignore
    }
}

fn handle_overlay_key(app: &mut App, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Enter | KeyCode::Esc | KeyCode::Char(' ') => {
            if app.show_same_file_warning {
                // Same file warning: any key dismisses, then allow quit
                app.show_same_file_warning = false;
            }
            if app.show_identical_overlay {
                app.show_identical_overlay = false;
            }
            Action::Continue
        }
        KeyCode::Char('q') | KeyCode::Char(':') => {
            app.show_same_file_warning = false;
            app.show_identical_overlay = false;
            if key.code == KeyCode::Char('q') {
                Action::Quit(false)
            } else {
                app.command_mode = true;
                app.command_buffer.clear();
                Action::Continue
            }
        }
        _ => Action::Continue,
    }
}

fn handle_quit_prompt(app: &mut App, key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            Action::Quit(!app.merge_state.diff.is_identical())
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.quit_prompt = false;
            Action::Continue
        }
        _ => Action::Continue,
    }
}

fn handle_save_prompt(app: &mut App, key: KeyEvent, _prompt: SavePrompt) -> Action {
    match key.code {
        KeyCode::Char('l') | KeyCode::Char('L') => {
            // Save left only
            app.merge_state.left.save().ok();
            app.merge_state.left_dirty = false;
            app.save_prompt = None;
            Action::Continue
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            // Save right only
            app.merge_state.right.save().ok();
            app.merge_state.right_dirty = false;
            app.save_prompt = None;
            Action::Continue
        }
        KeyCode::Char('b') | KeyCode::Char('B') => {
            // Save both
            let _ = app.merge_state.save_dirty();
            app.save_prompt = None;
            Action::Continue
        }
        KeyCode::Esc => {
            app.save_prompt = None;
            Action::Continue
        }
        _ => Action::Continue,
    }
}

fn scroll_to_active_block(app: &mut App) {
    use crate::file_diff::model::build_display_lines;

    if let Some(block_idx) = app.active_block_index() {
        let display_lines = build_display_lines(&app.merge_state);
        // Find the first display line for this block
        if let Some(pos) = display_lines.iter().position(|l| {
            l.block_index == block_idx
                && l.kind != crate::file_diff::model::DisplayLineKind::Equal
        }) {
            // Center the block in the viewport
            let half = app.viewport_height / 2;
            app.scroll_y = pos.saturating_sub(half);
        }
    }
}

fn clamp_active_index(app: &mut App) {
    let count = app.change_block_count();
    if count == 0 {
        app.active_change_index = 0;
    } else if app.active_change_index >= count {
        app.active_change_index = count - 1;
    }
}

fn clamp_scroll(app: &mut App) {
    let total = app.total_display_lines();
    let max_scroll = total.saturating_sub(app.viewport_height);
    if app.scroll_y > max_scroll {
        app.scroll_y = max_scroll;
    }
}
```

- [ ] **Step 2: Add viewport_height and total_display_lines to App**

Add these fields/methods to `weld-tui/src/app.rs`:

Add field to `App` struct:

```rust
    /// Viewport height in lines (updated each frame).
    pub viewport_height: usize,
```

Initialize in `App::new`:

```rust
            viewport_height: 0,
```

Add method to `impl App`:

```rust
    pub fn total_display_lines(&self) -> usize {
        crate::file_diff::model::build_display_lines(&self.merge_state).len()
    }
```

- [ ] **Step 3: Wire input handling into main_loop**

In `weld-tui/src/main.rs`, add the module declaration:

```rust
mod input;
```

Replace the `main_loop` function:

```rust
fn main_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut app::App,
) -> Result<bool, Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|frame| {
            // Update viewport height for scroll calculations
            app.viewport_height = frame.area().height.saturating_sub(2) as usize; // minus header + status
            file_diff::view::render(frame, app);
        })?;

        if let Some(event) = event::poll_event(std::time::Duration::from_millis(50))? {
            if let crossterm::event::Event::Key(key) = event {
                match input::handle_key(app, key) {
                    app::Action::Continue => {}
                    app::Action::Quit(has_differences) => return Ok(has_differences),
                }
            }
        }
    }
}
```

- [ ] **Step 4: Build and test interactively**

```bash
cargo run -p weld-tui -- /tmp/weld-left.rs /tmp/weld-right.rs
```

Expected: Full interactive TUI. Test:
- `j`/`k` navigates between diff blocks (gutter dot moves)
- `Ctrl+d`/`Ctrl+u` scrolls
- `L` copies left→right, `H` copies right→left
- `:w` + Enter saves
- `:q` + Enter quits (warns if dirty)
- `:q!` + Enter force quits
- `:e!` + Enter reloads from disk
- `?` shows help (not yet rendered — next task)
- `q` quits

- [ ] **Step 5: Commit**

```bash
git add weld-tui/src/input.rs weld-tui/src/app.rs weld-tui/src/main.rs
git commit -m "feat(tui): add vim-style keybinding dispatch with command mode, merge ops, and scrolling"
```

---

### Task 10: Overlays — Help, Identical Files, Same File Warning, Save/Quit Prompts

**Files:**
- Create: `weld-tui/src/file_diff/overlays.rs`
- Modify: `weld-tui/src/file_diff/mod.rs`
- Modify: `weld-tui/src/file_diff/view.rs`

- [ ] **Step 1: Write overlay rendering functions**

Create `weld-tui/src/file_diff/overlays.rs`:

```rust
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;

/// Render any active overlay on top of the diff view.
pub fn render_overlays(frame: &mut Frame, app: &App) {
    if app.show_same_file_warning {
        render_centered_message(
            frame,
            &app,
            "Same File",
            &["Both paths point to the same file.", "", "Press q to quit or any key to dismiss."],
        );
    } else if app.show_identical_overlay {
        render_centered_message(
            frame,
            &app,
            "Files Identical",
            &["The files are identical.", "", "Press any key to dismiss."],
        );
    } else if app.quit_prompt {
        render_centered_message(
            frame,
            &app,
            "Unsaved Changes",
            &[
                "You have unsaved changes.",
                "",
                "Quit without saving? (y/n)",
            ],
        );
    } else if app.save_prompt.is_some() {
        render_centered_message(
            frame,
            &app,
            "Save",
            &[
                "Both files have been modified.",
                "",
                "(l) Save left only",
                "(r) Save right only",
                "(b) Save both",
                "(Esc) Cancel",
            ],
        );
    } else if app.show_help {
        render_help(frame, app);
    }
}

fn render_centered_message(frame: &mut Frame, app: &App, title: &str, lines: &[&str]) {
    let area = centered_rect(50, lines.len() as u16 + 4, frame.area());

    frame.render_widget(Clear, area);

    let text: Vec<Line> = lines
        .iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(app.theme.overlay_fg))))
        .collect();

    let block = Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .style(Style::default().bg(app.theme.overlay_bg).fg(app.theme.overlay_fg));

    let paragraph = Paragraph::new(text)
        .block(block)
        .wrap(Wrap { trim: false })
        .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(paragraph, area);
}

fn render_help(frame: &mut Frame, app: &App) {
    let help_lines = vec![
        ("j / k", "Next / previous diff block"),
        ("H (Shift+h)", "Copy block: right → left"),
        ("L (Shift+l)", "Copy block: left → right"),
        ("Ctrl+d / Ctrl+u", "Half-page scroll down / up"),
        ("g / G", "Jump to top / bottom"),
        ("← / →", "Scroll horizontally"),
        (":w", "Save (prompt if both dirty)"),
        (":q", "Quit (warn if unsaved)"),
        (":q!", "Discard changes and quit"),
        (":wq", "Save and quit"),
        (":e!", "Reload files from disk"),
        ("?", "Toggle this help"),
        ("q", "Quit"),
    ];

    let area = centered_rect(60, help_lines.len() as u16 + 4, frame.area());

    frame.render_widget(Clear, area);

    let text: Vec<Line> = help_lines
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(
                    format!("{key:>20}  "),
                    Style::default().fg(app.theme.header_fg),
                ),
                Span::styled(
                    desc.to_string(),
                    Style::default().fg(app.theme.overlay_fg),
                ),
            ])
        })
        .collect();

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .style(Style::default().bg(app.theme.overlay_bg).fg(app.theme.overlay_fg));

    let paragraph = Paragraph::new(text).block(block);

    frame.render_widget(paragraph, area);
}

/// Create a centered rectangle of a given percentage width and fixed height.
fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height.min(100).max(10)) / 2),
            Constraint::Length(height),
            Constraint::Percentage((100 - height.min(100).max(10)) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
```

- [ ] **Step 2: Update mod.rs and view.rs**

Add to `weld-tui/src/file_diff/mod.rs`:

```rust
pub mod overlays;
```

Add at the end of the `render` function in `weld-tui/src/file_diff/view.rs`:

```rust
    // Render overlays on top of everything
    crate::file_diff::overlays::render_overlays(frame, app);
```

- [ ] **Step 3: Build and test overlays**

```bash
# Test identical files overlay
cp /tmp/weld-left.rs /tmp/weld-identical.rs
cargo run -p weld-tui -- /tmp/weld-left.rs /tmp/weld-identical.rs
```

Expected: shows "Files Identical" overlay. Press any key to dismiss, then view files normally.

```bash
# Test same file warning
cargo run -p weld-tui -- /tmp/weld-left.rs /tmp/weld-left.rs
```

Expected: shows "Same File" warning overlay.

```bash
# Test help overlay
cargo run -p weld-tui -- /tmp/weld-left.rs /tmp/weld-right.rs
# Press ? to see help
```

Expected: help overlay renders with keybinding list.

- [ ] **Step 4: Commit**

```bash
git add weld-tui/src/file_diff/
git commit -m "feat(tui): add overlays for help, identical files, same file warning, and save/quit prompts"
```

---

### Task 11: Test Fixtures and Integration Tests

**Files:**
- Create: `test-fixtures/left.rs`
- Create: `test-fixtures/right.rs`
- Create: `weld-core/tests/integration.rs`

- [ ] **Step 1: Create test fixture files**

Create `test-fixtures/left.rs`:

```rust
fn main() {
    let config = Config::load();
    let app = App::new();
    app.run();
    println!("Done");
}
```

Create `test-fixtures/right.rs`:

```rust
fn main() {
    let config = Config::load();
    let app = App::with_config(&config);
    app.start();
    println!("Done");
}
```

- [ ] **Step 2: Write integration tests for weld-core**

Create `weld-core/tests/integration.rs`:

```rust
use std::path::Path;
use weld_core::diff::{BlockKind, DiffResult};
use weld_core::file_io::FileContent;
use weld_core::merge::MergeState;

#[test]
fn diff_fixture_files() {
    let left = FileContent::load(Path::new("../test-fixtures/left.rs")).unwrap();
    let right = FileContent::load(Path::new("../test-fixtures/right.rs")).unwrap();

    let diff = DiffResult::compute(&left, &right);

    assert!(!diff.is_identical());

    let changes = diff.change_blocks();
    // Should detect the Replace block for lines 3-4 (App::new/run vs App::with_config/start)
    assert!(!changes.is_empty());

    let replace_blocks: Vec<_> = changes
        .iter()
        .filter(|(_, b)| b.kind == BlockKind::Replace)
        .collect();
    assert!(!replace_blocks.is_empty(), "Expected at least one Replace block");

    // Verify inline diffs exist for Replace blocks
    for (_, block) in &replace_blocks {
        assert!(
            !block.inline_diffs.is_empty(),
            "Replace block should have inline diffs"
        );
    }
}

#[test]
fn merge_fixture_files_copy_left_to_right() {
    let left = FileContent::load(Path::new("../test-fixtures/left.rs")).unwrap();
    let right = FileContent::load(Path::new("../test-fixtures/right.rs")).unwrap();

    let mut state = MergeState::new(left, right);
    assert!(!state.is_dirty());

    // Copy all change blocks from left to right
    while !state.diff.is_identical() {
        let changes = state.diff.change_blocks();
        if changes.is_empty() {
            break;
        }
        let (block_idx, _) = changes[0];
        state.copy_left_to_right(block_idx).unwrap();
    }

    assert!(state.diff.is_identical());
    assert!(state.right_dirty);
    assert!(!state.left_dirty);
    assert_eq!(state.left.lines, state.right.lines);
}

#[test]
fn merge_fixture_files_copy_right_to_left() {
    let left = FileContent::load(Path::new("../test-fixtures/left.rs")).unwrap();
    let right = FileContent::load(Path::new("../test-fixtures/right.rs")).unwrap();

    let mut state = MergeState::new(left, right);

    // Copy all change blocks from right to left
    while !state.diff.is_identical() {
        let changes = state.diff.change_blocks();
        if changes.is_empty() {
            break;
        }
        let (block_idx, _) = changes[0];
        state.copy_right_to_left(block_idx).unwrap();
    }

    assert!(state.diff.is_identical());
    assert!(state.left_dirty);
    assert!(!state.right_dirty);
    assert_eq!(state.left.lines, state.right.lines);
}
```

- [ ] **Step 3: Run integration tests**

```bash
cargo test -p weld-core --test integration
```

Expected: all 3 tests pass.

- [ ] **Step 4: Run the full test suite**

```bash
cargo test --workspace
```

Expected: all tests across both crates pass.

- [ ] **Step 5: Commit**

```bash
git add test-fixtures/ weld-core/tests/
git commit -m "test: add fixture files and integration tests for diff and merge"
```

---

### Task 12: Final Polish — Error Handling and Edge Cases

**Files:**
- Modify: `weld-tui/src/main.rs`
- Modify: `weld-tui/src/input.rs`

- [ ] **Step 1: Improve error handling in main.rs**

In `weld-tui/src/main.rs`, wrap the `run` function with proper panic handling to ensure terminal cleanup:

Replace the call to `run` in `main()`:

```rust
    match run(left, right, same_file) {
        Ok(has_differences) => {
            if has_differences {
                process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(2);
        }
    }
```

With:

```rust
    // Install panic hook that restores terminal before printing panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    match run(left, right, same_file) {
        Ok(has_differences) => {
            if has_differences {
                process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(2);
        }
    }
```

- [ ] **Step 2: Handle terminal resize events**

In `weld-tui/src/main.rs`, update the event handling in `main_loop` to handle resize:

```rust
        if let Some(event) = event::poll_event(std::time::Duration::from_millis(50))? {
            match event {
                crossterm::event::Event::Key(key) => {
                    match input::handle_key(app, key) {
                        app::Action::Continue => {}
                        app::Action::Quit(has_differences) => return Ok(has_differences),
                    }
                }
                crossterm::event::Event::Resize(_, _) => {
                    // Terminal will re-render on next loop iteration
                }
                _ => {}
            }
        }
```

- [ ] **Step 3: Build and do a full smoke test**

```bash
cargo build -p weld-tui
cargo run -p weld-tui -- /tmp/weld-left.rs /tmp/weld-right.rs
```

Smoke test checklist:
- TUI renders with side-by-side panes, colored highlights
- `j`/`k` moves between diff blocks, gutter dot moves
- `L` copies left→right, right pane updates, dirty dot appears
- `H` copies right→left
- `:e!` reloads from disk (undoes all changes)
- `:w` saves
- `:q` warns if dirty, quits if clean
- `:q!` force quits
- `?` shows help overlay
- `q` quits
- Resize terminal — view re-renders correctly
- Exit code is 0 if files identical, 1 if different

- [ ] **Step 4: Run full test suite one final time**

```bash
cargo test --workspace
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add weld-tui/src/
git commit -m "feat(tui): add panic recovery, resize handling, and polish error handling"
```

---

## Summary

| Task | What it builds | Key files |
|------|---------------|-----------|
| 1 | Project scaffolding | `Cargo.toml`, both crate manifests |
| 2 | File I/O with line-ending preservation | `weld-core/src/file_io.rs` |
| 3 | Diff engine (line + character level) | `weld-core/src/diff.rs`, `inline_diff.rs` |
| 4 | Merge state with copy operations | `weld-core/src/merge.rs` |
| 5 | CLI argument parsing | `weld-tui/src/main.rs` |
| 6 | Theme struct | `weld-tui/src/theme.rs` |
| 7 | Event loop and app shell | `weld-tui/src/event.rs`, `app.rs`, `main.rs` |
| 8 | Side-by-side rendering | `weld-tui/src/file_diff/` |
| 9 | Keybinding dispatch | `weld-tui/src/input.rs` |
| 10 | Overlays (help, warnings, prompts) | `weld-tui/src/file_diff/overlays.rs` |
| 11 | Test fixtures and integration tests | `test-fixtures/`, `weld-core/tests/` |
| 12 | Polish (panic recovery, resize) | `weld-tui/src/main.rs` |
