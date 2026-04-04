# Weld TUI — Project Instructions

## Identity

**Weld** is a cross-platform TUI diff and merge tool — a terminal-native alternative to meld, BeyondCompare, and DiffMerge. Side-by-side file diff with interactive two-way merge, vim-style keybindings.

## Architecture

Cargo workspace with two crates:

- **`weld-core`** — diff engine, merge logic, file I/O. Library crate, tested independently.
- **`weld-tui`** — ratatui-based UI, keybindings, rendering. Binary crate, depends on `weld-core`.

## Tech Stack

- **Language:** Rust (latest stable)
- **TUI:** ratatui + crossterm
- **Diffing:** `similar` crate (line-level and character-level)
- **CLI:** clap + clap_complete

## Key Documents

- **Spec:** `docs/superpowers/specs/2026-04-02-weld-tui-diff-design.md`
- **Phase 1 Plan:** `docs/superpowers/plans/2026-04-02-weld-phase1-file-diff-mvp.md`

## Development

```bash
# Build
cargo build

# Run
cargo run --bin weld-tui -- <left-file> <right-file>

# Test
cargo test
```

## Conventions

- Follow idiomatic Rust (clippy clean, rustfmt formatted)
- `weld-core` must have no TUI dependencies — keep it frontend-agnostic
- Fail loudly on missing/unreadable files rather than silently degrading
- Tests in `weld-core` are unit tests; integration tests use `test-fixtures/`

## Related Repos

None yet.
