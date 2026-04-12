# aineer-cli

The CLI mode library for Aineer. This crate provides the interactive REPL and command-line interface, and is embedded in the main `aineer` Tauri desktop application (activated via `--cli` flag).

```bash
# The CLI is bundled in the desktop app:
aineer --cli                         # Launch CLI mode from the desktop binary

# Or install standalone:
cargo install aineer                   # Installs the unified binary (GUI + CLI)
brew install andeya/aineer/aineer     # Homebrew (macOS / Linux)
```

## Architecture

`aineer-cli` is a **library crate** (not a standalone binary). It is called by the main `aineer` binary in `app/` when the `--cli` flag is passed:

```
┌─────────────────────────────────────────────────────────────────┐
│  aineer (app/)  — Tauri 2 desktop application                   │
│  ┌──────────────────────┐  ┌──────────────────────────────────┐ │
│  │  GUI mode (default)  │  │  CLI mode (--cli)                │ │
│  │  React + shadcn/ui   │  │  aineer-cli library              │ │
│  └──────────────────────┘  │  REPL · rendering · auto-update  │ │
│                            └──────────────────────────────────┘ │
├─────────────┬──────────────┬────────────────────────────────────┤
│  commands   │    tools     │     plugins                        │
├─────────────┴──────────────┴────────────────────────────────────┤
│  engine · settings · provider · memory · terminal               │
├──────────┬─────────┬────────────┬───────────────────────────────┤
│   api    │   mcp   │    lsp     │   protocol                    │
└──────────┴─────────┴────────────┴───────────────────────────────┘
```

## Key modules

| Module            | Responsibility                                           |
| ----------------- | -------------------------------------------------------- |
| `lib.rs`          | `run_cli()` entry point, called by `app/src/main.rs`     |
| `cli.rs`          | Argument parsing, flag definitions, `CliAction` enum     |
| `live_cli.rs`     | Interactive REPL loop with input, rendering, and history |
| `render.rs`       | Streaming response rendering with progress indicators    |
| `auto_update.rs`  | Self-update: GitHub Releases check, binary replacement   |
| `help.rs`         | `aineer help` and in-REPL `/help` output                 |
| `style.rs`        | Terminal color palette and ANSI escape helpers           |
| `runtime_client/` | Builds the conversation runtime for CLI use              |

## Subcommands

| Command                   | Description                        |
| ------------------------- | ---------------------------------- |
| `aineer --cli`            | Start interactive REPL             |
| `aineer --cli "<prompt>"` | One-shot prompt                    |
| `aineer help`             | Full help with examples            |
| `aineer update`           | Check for updates and self-install |
| `aineer init`             | Scaffold `.aineer/` directory      |
| `aineer login`            | OAuth authentication               |
| `aineer status`           | Show auth and config status        |
| `aineer models`           | List available models              |
| `aineer config`           | Read/write settings                |
| `aineer agents`           | List agent definitions             |
| `aineer skills`           | List skill templates               |

## For contributors

The main binary lives in `app/` (not in this crate). This crate exposes a `run_cli()` function. All library logic lives in sibling crates. When adding features:

- CLI argument parsing goes in `cli.rs`
- Slash command specs go in the `commands` crate
- Tool implementations go in the `tools` crate
- Runtime behavior goes in the `engine` crate

Run verification before submitting:

```bash
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

See the root [CONTRIBUTING.md](../../CONTRIBUTING.md) for the full guide.
