# Weld — TUI Diff & Merge Tool

**Date:** 2026-04-02
**Status:** Draft

## Overview

Weld is a cross-platform TUI application for visualizing and merging file and directory diffs. It targets the same use cases as meld, BeyondCompare, and DiffMerge — comparing arbitrary files and directories with interactive merge capabilities — but in a terminal-native interface with vim-style keybindings.

**Name rationale:** weld ≈ meld; welding is fusing things together.

## Goals

- Side-by-side file diff with interactive two-way merge
- Directory diff with file-level operations
- Vim-native keybindings
- Cross-platform (macOS, Linux, Windows), terminal-agnostic
- Start as a personal tool, move to open source quickly after viable MVP

## Non-Goals (for now)

- Three-way merge
- Git integration
- Inline editing within diff panes
- Structural / AST-level diffs

## Architecture

Cargo workspace with two crates:

- **`weld-core`** — diff engine, merge logic, file I/O, directory comparison. Library crate, testable independently.
- **`weld-tui`** — ratatui-based UI, keybindings, rendering. Binary crate, depends on `weld-core`.

This structure cleanly separates concerns and enables future frontends (GUI, CLI-only batch mode, etc.) by adding crates to the workspace.

## `weld-core` — Diff & Merge Engine

### Diff Computation

- Uses the `similar` crate for line-level and character-level diffs.
- Produces a list of `DiffBlock` structs representing contiguous regions of change.
- Each `Replace` block includes character-level diff info for inline highlighting.
- Operates on lines (text files only for now).

### Data Model

```
DiffResult {
    left: FileContent,           // lines + metadata
    right: FileContent,
    blocks: Vec<DiffBlock>,      // ordered list of diff regions
}

DiffBlock {
    kind: Equal | Insert | Delete | Replace,
    left_range: Range<usize>,    // line range in left file
    right_range: Range<usize>,   // line range in right file
    char_diffs: Option<Vec<InlineDiff>>,  // for Replace blocks
}

InlineDiff {
    kind: Equal | Changed,
    left_text: String,
    right_text: String,
}
```

### Merge Operations

- `copy_block_left_to_right(block_index)` — replaces right side with left content for the given block.
- `copy_block_right_to_left(block_index)` — replaces left side with right content for the given block.
- After a copy, the diff is recomputed.
- Tracks which side(s) have been modified (dirty flags).

### File I/O

- Load files as UTF-8 text with line-ending detection (preserve original endings on save).
- Save with explicit command — writes only dirty side(s).
- If both sides are dirty, the decision of which to save is delegated to the TUI layer (prompt).

### Directory Comparison

- Walk both directories, match files by relative path.
- Classify each entry: `LeftOnly`, `RightOnly`, `Identical`, `Modified`, `TypeMismatch` (file vs dir).
- No recursive diff content at this level — just the inventory.

## `weld-tui` — UI Architecture

### App Structure

- Elm-style architecture: Model → View → Update loop.
- `App` holds the current mode (`FileDiff` or `DirDiff`) and shared state.
- Each mode has its own model, view function, and input handler.
- Crossterm backend for cross-platform terminal support.

### File Diff View

#### Layout

```
┌──────────────────┬───┬──────────────────┐
│  left/file.rs    │   │  right/file.rs ● │
├──────────────────┤   ├──────────────────┤
│                  │   │                  │
│  Left content   ▐│ ● │▌  Right content   │
│                  │   │                  │
│                 ▐│   │▌                  │
├──────────────────┴───┴──────────────────┤
│ Block 1 of 3          j/k H/L :w :q ?  │
└─────────────────────────────────────────┘
```

- **Left/right panes:** line-numbered content with horizontal scrolling.
- **Scrollbar:** thin scrollbar on the right edge of each pane (same size, synced position).
- **Gutter:** narrow column between panes. Shows a filled dot vertically centered within the active diff block's height. The gutter's header segment matches the header background; the content segment has a darker background.
- **Header:** file paths with a dirty indicator dot (●) after the filename when that side has unsaved modifications.
- **Status bar:** current block position (e.g., "Block 1 of 3"), key hints.

#### Synchronized Scrolling

- **Vertical:** both panes scroll together, aligned by diff blocks. Equal regions stay aligned. Insert/Delete blocks show blank padding lines on the shorter side to maintain alignment.
- **Horizontal:** locked together — scrolling one pane scrolls both equally.

#### Highlighting

- **Changed blocks:** light background tint across entire lines (soft red for deletions, soft green for insertions).
- **Character diffs (Replace blocks):** darker/stronger highlight on the specific changed characters within the line.
- **Active diff block:** the block currently targeted by `j`/`k` navigation, indicated by the gutter dot.
- Colors are defined in a `Theme` struct for future configurability. Ships with sensible defaults that work on both light and dark terminals.

#### Keybindings

| Key | Action |
|-----|--------|
| `j` / `k` | Jump to next / previous diff block |
| `H` (Shift+h) | Copy active block from right → left |
| `L` (Shift+l) | Copy active block from left → right |
| `Ctrl+d` / `Ctrl+u` | Half-page scroll down / up |
| `g` / `G` | Jump to top / bottom |
| `:w` | Save (prompt if both sides dirty) |
| `:q` | Quit (warn if unsaved changes) |
| `:q!` | Discard all changes and quit |
| `:wq` | Save and quit |
| `:e!` | Discard all changes and reload from disk (reset) |
| `q` | Quit (warn if unsaved) |
| `?` | Show help overlay |

#### Edge Cases

- **Same file:** if both paths resolve to the same file, show a warning message and allow the user to quit.
- **Identical files:** load both files and display them side-by-side normally, but show a dismissible overlay banner: "Files are identical." No diff blocks to navigate.

### Directory Diff View

#### Layout

Same two-pane structure as file diff. Header shows directory paths (with dirty dot if files have been copied/modified). Gutter between panes with the same visual treatment.

#### Entry Display

Each pane lists files and subdirectories sorted alphabetically. Entries are classified:

- **Both sides, identical** — normal/dimmed text, no highlight.
- **Both sides, modified** — highlighted.
- **Left only** — shown in left pane, blank/placeholder row in right pane.
- **Right only** — blank/placeholder in left pane, shown in right pane.

#### Keybindings

| Key | Action |
|-----|--------|
| `j` / `k` | Move cursor up/down through file list |
| `Enter` | Open file diff view for the selected file |
| `H` (Shift+h) | Copy selected file/dir from right → left |
| `L` (Shift+l) | Copy selected file/dir from left → right |
| `Backspace` / `-` | Navigate up to parent directory |
| `:e!` | Discard all changes and reload from disk |
| `q` / `:q` | Quit |
| `?` | Help overlay |

#### Navigation

Drill-in model: `Enter` on a subdirectory replaces the current view with that subdirectory's contents. `Backspace` or `-` navigates back up. No inline tree expansion — keeps the implementation simple and consistent with file manager conventions.

## CLI Interface

### Invocation

```
weld <left-path> <right-path>
```

- Both paths are files → file diff view.
- Both paths are directories → directory diff view.
- Mismatched types (file vs directory) → error with clear message.
- Either path doesn't exist → error.
- Same file → warning overlay, allow quit.

### Shell Completions

```
weld --completions <shell>
```

Generates completions for bash, zsh, fish, and powershell via `clap_complete`.

### Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Clean exit (files identical, or user saved and quit) |
| `1` | Files differ and user quit without saving |
| `2` | Error (bad arguments, missing files, etc.) |

Enables scripting: `weld a.rs b.rs || echo "files still differ"`

### Configuration (Phase 2)

- Config file at `~/.config/weld/config.toml` (respects `XDG_CONFIG_HOME`).
- Theme file at `~/.config/weld/theme.toml`.
- Not needed for MVP — hardcoded defaults. The rendering layer reads from a `Theme` struct so theming is a clean addition.

## Dependencies

| Crate | Purpose |
|-------|---------|
| `ratatui` + `crossterm` | TUI framework + cross-platform terminal backend |
| `similar` | Line-level and character-level diff algorithm |
| `syntect` | Syntax highlighting (Phase 2) |
| `clap` + `clap_complete` | CLI argument parsing + shell completions |
| `walkdir` | Directory traversal (Phase 3) |

## Phasing

### Phase 1 — File Diff MVP

- Project scaffolding (Cargo workspace, CI, build)
- `weld-core`: diff engine with `similar`, line + character-level diffs
- `weld-tui`: side-by-side file diff view with synchronized scrolling, gutter dot indicator, two-level highlighting (block + character)
- Vim keybindings (`j`/`k`, `H`/`L`, `:w`, `:q`, `:q!`, `:e!`, `Ctrl+d`/`Ctrl+u`, `g`/`G`)
- Save semantics: write dirty side(s), prompt if both dirty
- Same-file warning, identical-files overlay
- CLI with `clap`, shell completions
- Exit codes

### Phase 2 — Polish & Adoption

- Syntax highlighting via `syntect`
- Theming support (`config.toml` / `theme.toml`)
- `cargo install` / `brew` packaging
- README, demo GIFs, release workflow

### Phase 3 — Directory Diff

- `weld-core`: directory comparison engine
- `weld-tui`: directory diff view with drill-in navigation
- File/directory copy operations (`H`/`L`)

### Future (Not Scoped)

- Three-way merge
- Git integration
- Inline editing within diff panes
- `tree-sitter` for structural diffs
