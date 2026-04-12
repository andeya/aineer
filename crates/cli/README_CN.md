# aineer-cli

Aineer 的 CLI 模式库。此 crate 提供交互式 REPL 和命令行界面，嵌入在主 `aineer` Tauri 桌面应用中（通过 `--cli` 参数激活）。

```bash
# CLI 已内置于桌面应用中：
aineer --cli                         # 从桌面二进制启动 CLI 模式

# 或独立安装：
cargo install aineer                   # 安装统一二进制（GUI + CLI）
brew install andeya/aineer/aineer     # Homebrew（macOS / Linux）
```

## 架构

`aineer-cli` 是一个**库 crate**（非独立二进制）。它由 `app/` 目录中的 `aineer` 主二进制在传入 `--cli` 参数时调用：

```
┌─────────────────────────────────────────────────────────────────┐
│  aineer (app/)  — Tauri 2 桌面应用                               │
│  ┌──────────────────────┐  ┌──────────────────────────────────┐ │
│  │  GUI 模式（默认）     │  │  CLI 模式 (--cli)                │ │
│  │  React + shadcn/ui   │  │  aineer-cli 库                   │ │
│  └──────────────────────┘  │  REPL · 渲染 · 自动更新          │ │
│                            └──────────────────────────────────┘ │
├─────────────┬──────────────┬────────────────────────────────────┤
│  commands   │    tools     │     plugins                        │
├─────────────┴──────────────┴────────────────────────────────────┤
│  engine · settings · provider · memory · terminal               │
├──────────┬─────────┬────────────┬───────────────────────────────┤
│   api    │   mcp   │    lsp     │   protocol                    │
└──────────┴─────────┴────────────┴───────────────────────────────┘
```

## 核心模块

| 模块              | 职责                                          |
| ----------------- | --------------------------------------------- |
| `lib.rs`          | `run_cli()` 入口点，由 `app/src/main.rs` 调用 |
| `cli.rs`          | 参数解析、标志定义、`CliAction` 枚举          |
| `live_cli.rs`     | 交互式 REPL 循环（输入、渲染、历史记录）      |
| `render.rs`       | 流式响应渲染和进度指示                        |
| `auto_update.rs`  | 自更新：GitHub Releases 检查、二进制替换      |
| `help.rs`         | `aineer help` 和 REPL 内 `/help` 输出         |
| `style.rs`        | 终端调色板和 ANSI 转义辅助                    |
| `runtime_client/` | 为 CLI 构建对话运行时                         |

## 子命令

| 命令                    | 说明                 |
| ----------------------- | -------------------- |
| `aineer --cli`          | 启动交互式 REPL      |
| `aineer --cli "<提示>"` | 一次性提问           |
| `aineer help`           | 完整帮助及示例       |
| `aineer update`         | 检查更新并自动安装   |
| `aineer init`           | 创建 `.aineer/` 目录 |
| `aineer login`          | OAuth 认证           |
| `aineer status`         | 查看认证和配置状态   |
| `aineer models`         | 列出可用模型         |
| `aineer config`         | 读写设置             |
| `aineer agents`         | 列出 Agent 定义      |
| `aineer skills`         | 列出 Skill 模板      |

## 贡献者须知

主二进制位于 `app/`（非本 crate）。本 crate 暴露 `run_cli()` 函数。所有库逻辑在兄弟 crate 中实现。新增功能时：

- CLI 参数解析 → `cli.rs`
- 斜杠命令定义 → `commands` crate
- 工具实现 → `tools` crate
- 运行时逻辑 → `engine` crate

提交前运行验证：

```bash
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

完整指南见根目录 [CONTRIBUTING.md](../../CONTRIBUTING.md)。
