# lazygrep

`lazygrep` is a terminal UI for interactively searching with `rg` (ripgrep) or `grep`.
As you type a query, it starts a search process and shows the first matching lines.

## Features

- Interactive text input with live search refresh
- Uses `rg` automatically when available
- Falls back to `grep -R` if `rg` is not installed
- Supports custom search command via CLI flag

## Requirements

- Rust toolchain (edition 2024 project)
- A search program:
  - preferred: `rg`
  - fallback: `grep`

## Build

```bash
cargo build
```

Run tests:

```bash
cargo test
```

## Run

Use the default search program detection:

```bash
cargo run
```

Use a custom search command:

```bash
cargo run -- --grep-program "rg --hidden --line-number"
```

The value passed to `--grep-program` is shell-split and used as the command prefix before the query.

## TUI controls

- Type to update the query
- `Ctrl+C`: quit
- `Enter` / `Ctrl+M`: ignored (no submit action; search updates while typing)

## Project layout

- `src/main.rs`: CLI setup and app startup
- `src/ui.rs`: terminal UI and key handling
- `src/grep.rs`: grep process orchestration
- `src/proc.rs`: managed child process wrapper
