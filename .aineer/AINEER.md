# AINEER.md

This file provides persistent context to Aineer when working with this repository.

## Project

**Aineer** is a local AI coding-agent CLI written in Rust. It is the project that _builds_ the
`aineer` binary — meaning this repo is both the tool and its own dogfood environment.

- Binary crate: `aineer-cli` → produces the `aineer` executable
- Workspace root: repo root (`Cargo.toml` with `members = ["crates/*"]`)
- Current version: `0.6.9` (shared across all crates via `workspace.package.version`)

## Repository layout

```
crates/
  aineer-core/  # Shared foundational types: events, observer, config, errors, cancel,
                  #   prompt types, elicitation, telemetry, loop state
  api/            # HTTP client, provider abstractions (Anthropic, OpenAI-compat, aineer-provider)
  mcp/            # MCP (Model Context Protocol) client: stdio/remote transport, resource/prompt
                  #   management, OAuth PKCE flow, JSON-RPC types
  runtime/        # Core engine: config, session, permissions, sandbox, prompts, hooks,
                  #   file ops (ripgrep-core grep/glob, PDF/image, atomic writes, mtime conflict),
                  #   conversation orchestration, error recovery, compaction, swarm
  tools/          # Built-in tool implementations:
                  #   file I/O, bash/PowerShell/REPL, web fetch/search, notebook editing,
                  #   sub-agent orchestration, LSP bridge, task management, plan mode,
                  #   git worktree, cron jobs, MCP resources, agent collaboration
  plugins/        # Plugin system: manifest, discovery, install, bundled embedding
  commands/       # Slash-command specs, discovery (skills, agents), git helpers
  lsp/            # LSP client: JSON-RPC transport, hover, completion, go-to-definition,
                  #   references, symbols, rename, formatting, diagnostics polling
  aineer-cli/   # CLI entry point, REPL, banner, init, session store, bootstrap
.aineer/        # Project config committed to repo (settings.json, AINEER.md, .gitignore)
```

## Languages & toolchain

- **Language**: Rust (edition 2021, MSRV declared in workspace `Cargo.toml`)
- **Build**: `cargo build` / `cargo build --release`
- **No runtime dependencies** — single static binary

## Verification commands

Run all three from the **repo root** before every commit:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

> `cargo fmt` without `--check` auto-fixes formatting; `--check` is used in CI.

## Coding conventions

- **No external `lazy_static`** — use `std::sync::OnceLock` for one-time initialization.
- **Error types**: use `CliError` (thiserror enum) at CLI boundaries; `McpTransportError` for MCP
  transport; `ApiError` for API clients; `RuntimeError` for the core engine. Avoid `Box<dyn Error>`.
- **Paths**: always use `runtime::aineer_runtime_dir(cwd)` (not raw `.aineer` joins) for
  runtime artifacts (sessions, agents, todos, sandbox dirs). Use `runtime::find_project_aineer_dir(cwd)`
  to locate the nearest initialized `.aineer/` without falling back to home.
- **Config loading**: `ConfigLoader::default_for(cwd)` walks ancestor dirs to find the project
  `.aineer/settings.json`; the global config is always `~/.aineer/settings.json`.
- **Plugin manifests**: `plugin.json` lives at the plugin directory root (not in `.aineer-plugin/`).
- **No `.aineer.json` flat config** — only directory-based `settings.json` is supported.
- **Async bridging**: use dedicated tokio runtimes (`OnceLock<Runtime>`) for subsystems that
  need async (LSP, web). Bridge with `block_on` / `block_in_place`; never nest runtimes.
- **File safety**: all writes go through `atomic_write`; edits use mtime-based conflict detection;
  file size limits apply to reads and writes.
- Comments should explain _why_, not _what_. Avoid narrating obvious code.
- Commit messages in English; code comments in English.

## Key design decisions

- `.aineer/` in the project is only created by `aineer init`; the binary never auto-creates it
  on startup (only `~/.aineer/` is auto-scaffolded).
- Runtime artifacts (sessions, todos, agents, sandbox) fall back to `~/.aineer/` when no project
  `.aineer/settings.json` exists in the ancestor chain.
- `is_initialized` (banner hint) checks only `cwd/.aineer/settings.json` — no ancestor walk —
  to avoid false-positives from `~/.aineer/settings.json`.
- Bundled plugins are embedded via `include_str!` and extracted to `~/.aineer/plugins/` on
  startup; they are NOT auto-discovered from `<project>/.aineer/plugins/`.

## Working agreement

- Keep shared project defaults in `.aineer/settings.json`; machine-local overrides in
  `.aineer/settings.local.json` (gitignored).
- Prefer small, focused commits. Run the three verification commands above before pushing.
- Update this file when architecture or conventions change.
