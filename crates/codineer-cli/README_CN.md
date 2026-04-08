# codineer-cli

提供 `codineer` 命令行工具的二进制 crate。这是你需要安装的包。

```bash
cargo install codineer-cli              # 从 crates.io
brew install andeya/codineer/codineer   # Homebrew（macOS / Linux）
```

## 功能概述

`codineer-cli` 将工作区中的所有 crate 组装为一个交互式终端应用：

```
┌─────────────────────────────────────────────────┐
│                  codineer-cli                    │
│  CLI 解析 · REPL · 渲染 · 自动更新              │
├─────────────┬──────────────┬────────────────────┤
│  commands   │    tools     │     plugins        │
├─────────────┴──────────────┴────────────────────┤
│                   runtime                        │
│  对话 · 配置 · 会话 · 权限                       │
├──────────┬─────────┬────────────┬───────────────┤
│   api    │   mcp   │    lsp     │     core      │
└──────────┴─────────┴────────────┴───────────────┘
```

## 核心模块

| 模块               | 职责                                          |
| ------------------ | --------------------------------------------- |
| `main.rs`          | 入口，子命令分发                              |
| `cli.rs`           | 参数解析、标志定义、`CliAction` 枚举          |
| `live_cli.rs`      | 交互式 REPL 循环（输入、渲染、历史记录）      |
| `render.rs`        | 流式响应渲染和进度指示                        |
| `auto_update.rs`   | 自更新：GitHub Releases 检查、二进制替换       |
| `help.rs`          | `codineer help` 和 REPL 内 `/help` 输出       |
| `style.rs`         | 终端调色板和 ANSI 转义辅助                    |
| `runtime_client/`  | 为 CLI 构建对话运行时                         |

## 子命令

| 命令               | 说明                    |
| ------------------ | ----------------------- |
| `codineer`         | 启动交互式 REPL         |
| `codineer "<提示>"` | 一次性提问             |
| `codineer help`    | 完整帮助及示例          |
| `codineer update`  | 检查更新并自动安装      |
| `codineer init`    | 创建 `.codineer/` 目录  |
| `codineer login`   | OAuth 认证              |
| `codineer status`  | 查看认证和配置状态      |
| `codineer models`  | 列出可用模型            |
| `codineer config`  | 读写设置                |
| `codineer agents`  | 列出 Agent 定义         |
| `codineer skills`  | 列出 Skill 模板         |

## 贡献者须知

此 crate 是工作区中**唯一的二进制**。所有库逻辑在兄弟 crate 中实现。新增功能时：

- CLI 参数解析 → `cli.rs`
- 斜杠命令定义 → `commands` crate
- 工具实现 → `tools` crate
- 运行时逻辑 → `runtime` crate

提交前运行验证：

```bash
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

完整指南见根目录 [CONTRIBUTING.md](../../CONTRIBUTING.md)。
