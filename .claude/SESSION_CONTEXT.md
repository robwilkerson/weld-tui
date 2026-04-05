# Session Context
Last updated: 2026-04-05

## Status
Phase 1 in progress. TUI shell, file loading, and module extraction complete. FileContent aligned with plan. Ready for diff engine.

## Completed
- Project scaffolding: workspace, CI, hooks, tooling, README, LICENSE
- TUI shell: bordered panes, Catppuccin Macchiato theme, event loop
- File loading: FileContent with LineEnding detection, save(), text(), Clone
- Scrolling: vim keybindings (j/k/h/l/gg/G/0/$), synchronized panes
- Tab expansion for display, home dir shortening (path-safe)
- Module extraction: file_diff/view.rs, input.rs, main.rs slimmed to ~60 lines
- Architecture reviews (app + systems architects) — tickets created
- CodeRabbit + internal code reviews addressed
- Panic hook for terminal restore

## Merged PRs
- #1 — Rust version pin
- #2 — TUI shell with Catppuccin theme
- #5 — File loading with scrollable panes
- #13 — Extract rendering/input modules

## Open PRs
- #14 — Align FileContent with plan (ready to merge)

## Next Session
1. Fix #15 — gg (go to top) broken (HIGH PRIORITY)
2. Block-level diff highlighting (using `similar` crate)
3. Character-level diff highlighting within changed blocks

## Open Tickets (prioritized)
- #15 — gg does not work (HIGH)
- #3 — $ scroll uses longest visible line
- #4 — j scroll cap at viewport bottom
- #8 — Restructure App (FileDiffModel, Mode enum)
- #9 — Render only visible rows (perf)
- #10 — u32 scroll offsets (65K limit)
- #12 — Cross-platform shorten_dir

## Decisions
- Rust + ratatui + crossterm stack
- Cargo workspace: weld-core (lib) + weld-tui (bin)
- Catppuccin Macchiato default theme
- TUI-first development: visual shell before core library
- Tabs expanded for display only, original content preserved
- CodeRabbit reviews on all non-trivial PRs
- Aspirational Tauri GUI frontend (weld-core stays frontend-agnostic)
