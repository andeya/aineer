# codineer-cli

The binary crate that ships the `codineer` command-line tool. This is the package you install.

```bash
cargo install codineer-cli          # from crates.io
brew install andeya/codineer/codineer   # Homebrew (macOS / Linux)
```

## What it does

`codineer-cli` wires together every other crate in the workspace into a single interactive terminal application:

```
┌─────────────────────────────────────────────────┐
│                  codineer-cli                    │
│  CLI parsing · REPL · rendering · auto-update   │
├─────────────┬──────────────┬────────────────────┤
│  commands   │    tools     │     plugins        │
├─────────────┴──────────────┴────────────────────┤
│                   runtime                        │
│  conversation · config · sessions · permissions  │
├──────────┬─────────┬────────────┬───────────────┤
│   api    │   mcp   │    lsp     │     core      │
└──────────┴─────────┴────────────┴───────────────┘
```

## Key modules

| Module             | Responsibility                                           |
| ------------------ | -------------------------------------------------------- |
| `main.rs`          | Entry point, subcommand dispatch                         |
| `cli.rs`           | Argument parsing, flag definitions, `CliAction` enum     |
| `live_cli.rs`      | Interactive REPL loop with input, rendering, and history |
| `render.rs`        | Streaming response rendering with progress indicators    |
| `auto_update.rs`   | Self-update: GitHub Releases check, binary replacement   |
| `help.rs`          | `codineer help` and in-REPL `/help` output               |
| `style.rs`         | Terminal color palette and ANSI escape helpers            |
| `runtime_client/`  | Builds the conversation runtime for CLI use              |

## Subcommands

| Command          | Description                        |
| ---------------- | ---------------------------------- |
| `codineer`       | Start interactive REPL             |
| `codineer "<prompt>"` | One-shot prompt               |
| `codineer help`  | Full help with examples            |
| `codineer update`| Check for updates and self-install |
| `codineer init`  | Scaffold `.codineer/` directory    |
| `codineer login` | OAuth authentication               |
| `codineer status`| Show auth and config status        |
| `codineer models`| List available models              |
| `codineer config`| Read/write settings                |
| `codineer agents`| List agent definitions             |
| `codineer skills`| List skill templates               |

## For contributors

This crate is the **only binary** in the workspace. All library logic lives in sibling crates. When adding features:

- CLI argument parsing goes in `cli.rs`
- Slash command specs go in the `commands` crate
- Tool implementations go in the `tools` crate
- Runtime behavior goes in the `runtime` crate

Run verification before submitting:

```bash
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

See the root [CONTRIBUTING.md](../../CONTRIBUTING.md) for the full guide.
