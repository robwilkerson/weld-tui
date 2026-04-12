# Weld

A cross-platform TUI diff and merge tool — a terminal-native alternative to
Meld, Beyond Compare, and DiffMerge.

## Goals

- Side-by-side file diff with synchronized scrolling
- Interactive two-way merge with conflict resolution
- Vim-style keybindings for efficient keyboard-driven workflows
- Fast startup, low resource usage, works over SSH

## Status

Early development. Core file diff and two-way merge are functional.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) >= 1.94

## Usage

```bash
weld <left-file> <right-file>
```

### Navigation

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll down / up one line |
| `Shift+j` / `Shift+k` | Jump to next / previous change block |
| `gg` | Jump to first change block |
| `Shift+g` | Jump to last change block |
| `h` / `l` | Scroll left / right |
| `0` / `$` | Jump to left / right edge |
| `q` | Quit |

### Merging

| Key | Action |
|-----|--------|
| `Shift+l` | Copy active block left → right |
| `Shift+h` | Copy active block right → left |
| `u` | Undo last copy |
| `Ctrl+r` | Redo last undo |

## Contributing

### Prerequisites

- [just](https://github.com/casey/just) — command runner
- [Kingfisher](https://github.com/trufflesecurity/kingfisher) — secrets scanning (pre-commit hook)

### Getting Started

```bash
just bootstrap
```

## AI Disclosure

AI tools (specifically [Claude](https://claude.ai)) were used during the
development of this project as a brainstorming partner and typist — similar to
how one might work with a pair programmer. The human developer drove all
design decisions, reviewed all output, and maintains full responsibility for the
final result. AI was a tool in the workshop, not the carpenter.
