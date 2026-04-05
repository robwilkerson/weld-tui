# Session Context
Last updated: 2026-04-05

## Status
Phase 1 in progress. TUI shell and file loading complete. Working on architecture hardening before diff engine.

## Completed
- Project scaffolding: workspace, CI, hooks, tooling, README, LICENSE
- TUI shell: bordered panes, Catppuccin Macchiato theme, event loop
- File loading: FileContent::load(), content rendering with line numbers
- Scrolling: vim keybindings (j/k/h/l/g/G/0/$), synchronized panes
- Tab expansion for display, home dir shortening
- Branch protection rules, Renovate, CodeRabbit configured
- Architecture reviews (app + systems architects) — tickets created
- CodeRabbit review feedback addressed

## Active PR
- #5 — File loading with scrollable content panes (review fixes pushed)

## Open Tickets (prioritized)
- #11 — Panic hook for terminal restore (quick win)
- #6 — Extract rendering/input from main.rs
- #7 — Align FileContent with plan (LineEnding, save, Clone)
- #8 — Restructure App (FileDiffModel, Mode enum)
- #9 — Render only visible rows (perf)
- #10 — u32 scroll offsets (65K limit)
- #3 — $ scroll uses longest visible line
- #4 — j scroll cap at viewport bottom
- #12 — Cross-platform shorten_dir

## Decisions
- Rust + ratatui + crossterm stack
- Cargo workspace: weld-core (lib) + weld-tui (bin)
- Catppuccin Macchiato default theme (structured for multiple themes)
- TUI-first development: visual shell before core library
- Tabs expanded for display only, original content preserved
- CodeRabbit reviews on all non-trivial PRs
- Renovate via Docker (not GitHub App), runs Saturday mornings
