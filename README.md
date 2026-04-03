<p align="center">
  <img src="assets/logo-light.svg" alt="Codineer" width="360">
  <br>
  <em>Your local AI coding agent — one binary, zero cloud lock-in.</em>
</p>

<p align="center">
  <a href="https://github.com/andeya/codineer/actions"><img src="https://github.com/andeya/codineer/workflows/CI/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT License"></a>
  <a href="https://github.com/andeya/codineer/releases"><img src="https://img.shields.io/github/v/release/andeya/codineer" alt="Release"></a>
  <br>
  <img src="https://img.shields.io/badge/macOS-supported-blue?logo=apple&logoColor=white" alt="macOS">
  <img src="https://img.shields.io/badge/Linux-supported-blue?logo=linux&logoColor=white" alt="Linux">
  <img src="https://img.shields.io/badge/Windows-supported-blue?logo=windows&logoColor=white" alt="Windows">
  <br>
  <a href="README_CN.md">中文文档</a>
</p>

---

**Codineer** turns your terminal into an AI-powered coding companion. It reads your workspace, understands your project context, and helps you write, refactor, debug, and ship code — all without leaving the command line.

Built in safe Rust. Ships as a **single, self-contained binary**. No daemon, no cloud dependency — bring your own API key and start coding.

## Why Codineer?

- **Private by design** — your code stays on your machine; only the prompts you send leave the terminal
- **Workspace-aware** — reads `CODINEER.md`, project configs, git state, and LSP diagnostics before every turn
- **Tool-rich** — shell execution, file read/write/edit, glob/grep search, web fetch, todo tracking, notebook editing, and more
- **Extensible** — MCP servers, local plugins, custom agents and skills via `.codineer/` directories
- **Sandboxed** — optional process isolation via Linux namespaces or macOS Seatbelt profiles
- **Multi-provider** — Anthropic (Claude), xAI (Grok), OpenAI, and any OpenAI-compatible API (Ollama, etc.)
- **Cross-platform** — native binaries for macOS, Linux, and Windows

## Install

### Download prebuilt binaries

Head to the **[Releases](https://github.com/andeya/codineer/releases)** page and download the binary for your platform:

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `codineer-*-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `codineer-*-x86_64-apple-darwin.tar.gz` |
| Linux (x86_64) | `codineer-*-x86_64-unknown-linux-gnu.tar.gz` |
| Linux (ARM64) | `codineer-*-aarch64-unknown-linux-gnu.tar.gz` |
| Windows (x86_64) | `codineer-*-x86_64-pc-windows-msvc.zip` |

Extract and place the `codineer` binary in your `PATH`.

### Install from source

```bash
cargo install --path crates/codineer-cli --locked
```

### Homebrew (macOS/Linux)

```bash
brew install andeya/tap/codineer
```

## Quick Start

### 1. Set up your API key

```bash
# Anthropic (Claude)
export ANTHROPIC_API_KEY="sk-ant-..."

# xAI (Grok)
export XAI_API_KEY="xai-..."

# OpenAI
export OPENAI_API_KEY="sk-..."

# Or use Anthropic OAuth:
codineer login
```

Codineer auto-detects which provider to use based on available credentials. No configuration needed.

### 2. Start coding

```bash
# Interactive REPL
codineer

# One-shot prompt
codineer prompt "explain the architecture of this project"

# JSON output for scripting
codineer -p "list all TODO items" --output-format json
```

## Core Features

| Feature | Description |
|---------|-------------|
| **Interactive REPL** | Conversational coding sessions with Vim keybindings, tab completion, and history |
| **Workspace Tools** | `bash`, `read_file`, `write_file`, `edit_file`, `glob`, `grep`, `web_fetch`, `web_search`, `todo_write`, `notebook_edit` |
| **Slash Commands** | `/status`, `/compact`, `/config`, `/cost`, `/model`, `/permissions`, `/resume`, `/clear`, `/init`, `/diff`, `/export` |
| **Agent & Skill System** | Discover and run agents/skills from `.codineer/agents/` and `.codineer/skills/` |
| **Plugin System** | Install, manage, and extend with custom plugins and hooks |
| **MCP Support** | Connect to external tool servers via Model Context Protocol (stdio, SSE, HTTP, WebSocket) |
| **Git Integration** | Branch detection, worktree management, commit/PR workflows |
| **Session Management** | Save, restore, and resume coding sessions |
| **Sandbox** | Process isolation with Linux `unshare` or macOS `sandbox-exec` |

## Configuration

Codineer loads configuration from multiple sources (in precedence order):

1. `.codineer/settings.local.json` — local overrides (gitignored)
2. `.codineer/settings.json` — project settings
3. `~/.codineer/settings.json` — user-global settings

Key settings: `model`, `permissionMode`, `mcpServers`, `sandbox`, `hooks`, `enabledPlugins`.

Run `codineer help` for full documentation of environment variables and configuration files.

## Project Structure

```text
crates/
├── api/              # AI provider clients + streaming
├── codineer-cli/     # Interactive CLI binary
├── commands/         # Slash commands & agent/skill discovery
├── lsp/              # Language Server Protocol client
├── plugins/          # Plugin system & hooks
├── runtime/          # Session, config, MCP, prompt, sandbox
└── tools/            # AI-callable tool definitions
```

## Development

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## License

[MIT](LICENSE)

---

<p align="center">
  Made with 🦀 by <a href="https://github.com/andeya">andeya</a>
</p>
