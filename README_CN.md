<p align="center">
  <img src="assets/logo-light.svg" alt="Codineer" width="360">
  <br>
  <em>你的本地 AI 编程助手 — 单一二进制，零云端锁定。</em>
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
  <a href="README.md">English</a>
</p>

---

**Codineer** 将你的终端变成 AI 驱动的编程伙伴。它能读取工作区、理解项目上下文，帮你编写、重构、调试和交付代码 — 全程无需离开命令行。

使用安全 Rust 构建，编译为**单个独立二进制文件**。无守护进程，无云端依赖 — 自带 API Key 即可开始编码。

## 为什么选择 Codineer？

- **隐私优先** — 代码始终留在本地，只有你主动发送的提示词才会离开终端
- **工作区感知** — 每轮对话前自动读取 `CODINEER.md`、项目配置、Git 状态和 LSP 诊断信息
- **工具丰富** — Shell 执行、文件读写编辑、全局搜索、网页抓取、待办管理、Notebook 编辑等
- **高度可扩展** — 支持 MCP 服务器、本地插件、自定义 Agent 和 Skill（通过 `.codineer/` 目录）
- **安全沙箱** — 可选的进程隔离：Linux 命名空间 或 macOS Seatbelt 沙箱
- **多供应商** — 支持 Anthropic (Claude)、xAI (Grok)、OpenAI 以及任何 OpenAI 兼容 API（Ollama 等）
- **全平台支持** — 提供 macOS、Linux、Windows 原生二进制

## 安装

### 下载预编译二进制

前往 **[Releases](https://github.com/andeya/codineer/releases)** 页面，下载适合你平台的二进制文件：

| 平台 | 文件 |
|------|------|
| macOS (Apple Silicon) | `codineer-*-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `codineer-*-x86_64-apple-darwin.tar.gz` |
| Linux (x86_64) | `codineer-*-x86_64-unknown-linux-gnu.tar.gz` |
| Linux (ARM64) | `codineer-*-aarch64-unknown-linux-gnu.tar.gz` |
| Windows (x86_64) | `codineer-*-x86_64-pc-windows-msvc.zip` |

解压后将 `codineer` 可执行文件放入你的 `PATH` 中即可。

### 从源码安装

```bash
cargo install --path crates/codineer-cli --locked
```

### Homebrew（macOS/Linux）

```bash
brew install andeya/tap/codineer
```

## 快速开始

### 1. 设置 API Key

```bash
# Anthropic (Claude)
export ANTHROPIC_API_KEY="sk-ant-..."

# xAI (Grok)
export XAI_API_KEY="xai-..."

# OpenAI
export OPENAI_API_KEY="sk-..."

# 或使用 Anthropic OAuth：
codineer login
```

Codineer 会自动检测可用的 API 供应商，无需额外配置。

### 2. 开始编码

```bash
# 交互式 REPL
codineer

# 一次性提示
codineer prompt "解释这个项目的架构"

# JSON 输出（适合脚本集成）
codineer -p "列出所有 TODO 项" --output-format json
```

## 核心功能

| 功能 | 说明 |
|------|------|
| **交互式 REPL** | 对话式编程会话，支持 Vim 键绑定、Tab 补全和历史记录 |
| **工作区工具** | `bash`、`read_file`、`write_file`、`edit_file`、`glob`、`grep`、`web_fetch`、`web_search`、`todo_write`、`notebook_edit` |
| **斜杠命令** | `/status`、`/compact`、`/config`、`/cost`、`/model`、`/permissions`、`/resume`、`/clear`、`/init`、`/diff`、`/export` |
| **Agent 与 Skill 系统** | 从 `.codineer/agents/` 和 `.codineer/skills/` 发现并运行自定义智能体和技能 |
| **插件系统** | 安装、管理和扩展自定义插件与钩子 |
| **MCP 支持** | 通过 Model Context Protocol 连接外部工具服务器（stdio、SSE、HTTP、WebSocket） |
| **Git 集成** | 分支检测、工作树管理、提交/PR 工作流 |
| **会话管理** | 保存、恢复和续接编程会话 |
| **安全沙箱** | Linux `unshare` 或 macOS `sandbox-exec` 进程隔离 |

## 配置

Codineer 按以下优先级加载配置：

1. `.codineer/settings.local.json` — 本地覆盖（已 gitignore）
2. `.codineer/settings.json` — 项目级配置
3. `~/.codineer/settings.json` — 用户全局配置

关键配置项：`model`、`permissionMode`、`mcpServers`、`sandbox`、`hooks`、`enabledPlugins`。

运行 `codineer help` 查看完整的环境变量和配置文件文档。

## 项目结构

```text
crates/
├── api/              # AI 供应商客户端 + 流式传输
├── codineer-cli/     # 交互式 CLI 二进制
├── commands/         # 斜杠命令与 Agent/Skill 发现
├── lsp/              # Language Server Protocol 客户端
├── plugins/          # 插件系统与钩子
├── runtime/          # 会话、配置、MCP、提示词、沙箱
└── tools/            # AI 可调用的工具定义
```

## 开发

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## 许可证

[MIT](LICENSE)

---

<p align="center">
  由 <a href="https://github.com/andeya">andeya</a> 使用 🦀 构建
</p>
