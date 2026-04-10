# Session Context
Last updated: 2026-04-08

## Status
Minimap complete. Block-level diff highlighting complete. Ready for character-level highlighting.

## Completed
- Project scaffolding: workspace, CI, hooks, tooling, README, LICENSE
- TUI shell: bordered panes, Catppuccin Macchiato theme, event loop
- File loading: FileContent with LineEnding detection, save(), text(), Clone
- Scrolling: vim keybindings (j/k/h/l/gg/G/0/$), synchronized panes
- Tab expansion for display, home dir shortening (moved to weld-core)
- Module extraction: file_diff/view.rs, input.rs, main.rs slimmed
- Panic hook for terminal restore
- Diff engine: line-level (DiffResult/DiffBlock) + character-level (InlineDiff)
- Display row abstraction: alignment padding for insert/delete/replace blocks
- Block-level diff highlighting: single neutral color, full-width padding, cross-pane max
- Scroll fixes: $, j, l capping; gg fix; display-row-aware scrolling
- Minimap: 1-column bar, proportional diff markers (pale yellow), │ viewport indicator, content-aligned
- Code review items: PaneContext struct, cached max_content_width/change_count, InputState extraction, FileContent encapsulation, block_index on DisplayRow, removed empty merge.rs
- Pre-commit hook formatting active (.githooks/pre-commit)
- Clippy clean, CI should be green
- 54 tests passing (37 weld-core, 17 weld-tui)

## Closed Tickets
- #3 — $ scroll (fixed in e8b58e5)
- #4 — j scroll cap (fixed in e8b58e5)
- #15 — gg broken (fixed in 6e4a96a)

## Open Tickets (prioritized)
- #8 — Restructure App (FileDiffModel, Mode enum)
- #9 — Render only visible rows (perf)
- #10 — u32 scroll offsets (65K limit)
- #12 — Cross-platform shorten_dir

## Next Session
1. Character-level diff highlighting within changed blocks (InlineDiff already built, diff_emphasis_bg in theme)
2. Then: jump navigation between diffs (ctrl+j, ctrl+k)
3. Then: merging diffs (ctrl+h, ctrl+l)
4. Then: unsaved file visual cue

## Decisions
- Rust + ratatui + crossterm stack
- Cargo workspace: weld-core (lib) + weld-tui (bin)
- Catppuccin Macchiato default theme
- TUI-first development: visual shell before core library
- Tabs expanded for display only, original content preserved
- Aspirational Tauri GUI frontend (weld-core stays frontend-agnostic)
- Use "Fixes #N" in commits to auto-close GitHub issues
- No TDD enforcement; test after implementation is fine
- Single diff highlight color for all block types (not red/green)
- Minimap: 1-column default, configurable width (0=hidden), pale yellow markers
