# aineer-mcp

[Aineer](https://github.com/andeya/aineer) 的 MCP（Model Context Protocol）客户端、服务器管理与配置类型。

[English](README.md)

---

本 crate 实现了完整的 MCP 客户端协议栈：

| 层级     | 组件                                                                            |
| -------- | ------------------------------------------------------------------------------- |
| **传输** | `McpStdioProcess`（stdin/stdout JSON-RPC）、`McpRemoteClient`（HTTP/WebSocket） |
| **管理** | `McpServerManager` — 生命周期、工具发现、分页、批量分发                         |
| **配置** | `McpServerConfig`、`McpConfigCollection`、`ScopedMcpServerConfig`               |
| **资源** | `list_resources`、`read_resource` — MCP 资源一级支持                            |
| **提示** | `list_prompts`、`get_prompt` — MCP 提示模板支持                                 |
| **认证** | `McpClientAuth`、OAuth PKCE 流程类型（`OAuthAuthorizeParams`）                  |
| **命名** | 工具名称作用域、服务器签名、代理 URL 处理                                       |

### 错误处理

所有传输操作返回 `Result<T, McpTransportError>`，这是一个结构化的 `thiserror` 枚举，包含连接、超时、协议、ID 不匹配、服务器错误、WebSocket、HTTP、I/O 和 JSON 等失败变体。管理器将其包装为 `McpServerManagerError` 供上层调用。

### 支持的传输方式

| 传输方式  | 配置变体                   | 实现              |
| --------- | -------------------------- | ----------------- |
| stdio     | `McpStdioServerConfig`     | `McpStdioProcess` |
| HTTP/SSE  | `McpRemoteServerConfig`    | `McpRemoteClient` |
| WebSocket | `McpWebSocketServerConfig` | `McpRemoteClient` |
| SDK       | `McpSdkServerConfig`       | 通过托管代理      |

## 说明

本 crate 是 Aineer 项目的内部组件，作为 `aineer-cli` 的依赖发布到 crates.io，不用于独立使用。在 Aineer 工作区之外不保证 API 稳定性。

## 许可证

MIT — 详见 [LICENSE](https://github.com/andeya/aineer/blob/main/LICENSE)。
