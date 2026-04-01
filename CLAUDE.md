# CLAUDE.md

Guidance for Claude Code when working in this repository.

## Project Overview

A Rust statusline generator for Claude Code. Reads JSON from stdin, outputs an ANSI-colored statusline showing: working directory (fish-style shortened), git branch, worktree indicator, model name, context window usage (progress bar + percentage), session cost, lines changed, agent name, and session duration.

## Key Commands

```bash
cargo build --release     # Release build
cargo run < test.json     # Manual test with sample data
cargo check               # Quick type checking
cargo clippy              # Lint
cargo fmt                 # Format
cargo test                # Run unit tests
just ci                   # Run all CI checks (fmt, clippy, tests)
```

**Important**: Never generate test JSON files. Use `test.json` in the repo root or look in `~/.claude`.

## Architecture

### Entry Point

**src/main.rs**: Calls `statusline()` from the library and prints the result. No CLI argument handling.

**src/lib.rs**: All logic lives here.

### Input Structs

`StatusInput` is the top-level serde struct. All fields use `#[serde(default)]` so missing fields deserialize gracefully.

| Struct | Key Fields | Purpose |
|--------|-----------|---------|
| `StatusInput` | workspace, model, output_style, context_window, cost, worktree, agent | Top-level container |
| `Workspace` | `current_dir: Option<String>` | Working directory (required for meaningful output) |
| `Model` | `display_name: Option<String>` | Model name shown in statusline |
| `OutputStyle` | `name: Option<String>` | Style label (e.g. "explanatory") shown in parens |
| `ContextWindow` | `context_window_size`, `used_percentage`, `current_usage` | Context usage data |
| `CurrentUsage` | `input_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens` | Token breakdown for manual % calculation |
| `Cost` | `total_cost_usd`, `total_duration_ms`, `total_lines_added`, `total_lines_removed` | Session cost and line change stats |
| `Worktree` | `name`, `branch` | Worktree indicator |
| `Agent` | `name: Option<String>` | Agent name when running as a sub-agent |

### Statusline Assembly

`statusline()` builds these display components, then joins non-empty ones with `•` separators:

1. **Path**: Fish-style shortened (`fish_shorten_path`), colored cyan
2. **Git branch**: Via `git rev-parse --abbrev-ref HEAD`, colored green, with `↟` worktree suffix
3. **Lines changed**: `+N -M` from cost data, green/red
4. **Model**: Nerd Font icon + model name in orange, optional style suffix in gray
5. **Context bar**: 15-char progress bar (█/░) + percentage, color-coded by usage (red ≥90%, orange ≥70%, yellow ≥50%, gray <50%)
6. **Cost**: Dollar amount, color-coded (green <$5, yellow <$20, red ≥$20)
7. **Agent**: Agent name in gray with icon
8. **Duration**: Formatted as `Nh Mm` or `<1m`, from `total_duration_ms`

### ANSI Colors

Constants defined at the top of `lib.rs`: `RESET`, `RED`, `GREEN`, `YELLOW`, `CYAN`, `GRAY`, `ORANGE`, `LIGHT_CYAN`, `LIGHT_BLUE`, `LIGHT_MAGENTA`, `GOLD`. Uses standard ANSI escapes and 256-color codes.

### Key Functions

- `statusline()` — Main orchestrator, returns the assembled String
- `read_input()` — Reads stdin, deserializes to `StatusInput`
- `is_git_repo(dir)` — Checks `git rev-parse --is-inside-work-tree`
- `get_git_branch(dir)` — Gets current branch name via git
- `fish_shorten_path(path)` — Replaces $HOME with ~, shortens intermediate dirs to first char (hidden dirs keep dot + first char)
- `format_cost(f64)` — 3 decimal places below $0.01, 2 above
- `format_duration(ms)` — Formats milliseconds as `Nh Mm`, `Nm`, or `<1m`

### Display Format

```
path  branch(+N -M) • 󰊭 Model (style) • 󱦛 ███░░░░░░░░░░░░ 45% • 󰊖 $7.50 • 󰚩 agent • 󰔚 15m
```

## Dependencies

- **serde** + **serde_json**: JSON deserialization only
- **External**: `git` (required for branch/repo detection)

## Input Format

JSON on stdin. See `test.json` for the full structure. Only `workspace.current_dir` is required for meaningful output; all other fields are optional and degrade gracefully when absent.
