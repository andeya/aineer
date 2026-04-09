# aineer-lsp

[Aineer](https://github.com/andeya/aineer) 的语言服务器协议（LSP）客户端集成。

[English](README.md)

---

本 crate 提供 Aineer 与语言服务器通信的 LSP 传输层，管理服务器生命周期（启动、初始化、关闭）、JSON-RPC 消息帧处理，以及通过 `tokio::sync::watch` 实现的异步诊断轮询。

### 支持的操作

| 操作       | 方法                                            |
| ---------- | ----------------------------------------------- |
| 悬浮       | `textDocument/hover`                            |
| 补全       | `textDocument/completion`                       |
| 跳转定义   | `textDocument/definition`                       |
| 查找引用   | `textDocument/references`                       |
| 文档符号   | `textDocument/documentSymbol`                   |
| 工作区符号 | `workspace/symbol`                              |
| 重命名     | `textDocument/rename`                           |
| 格式化     | `textDocument/formatting`                       |
| 诊断       | `textDocument/publishDiagnostics`（推送）+ 轮询 |

### 架构

- `LspClient` — 每服务器进程管理器，使用 stdin/stdout JSON-RPC 帧处理。
- `LspManager` — 多路复用器，根据文件扩展名将请求路由到对应的服务器，通过 `tokio::fs` 实现异步 I/O。
- 服务器能力在 `initialize` 阶段捕获并对外暴露，用于特性检测。

## 说明

本 crate 是 Aineer 项目的内部组件，作为 `aineer-cli` 的依赖发布到 crates.io，不用于独立使用。在 Aineer 工作区之外不保证 API 稳定性。

## 许可证

MIT — 详见 [LICENSE](https://github.com/andeya/aineer/blob/main/LICENSE)。
