# aineer-mcp

MCP (Model Context Protocol) client, server management, and configuration types for [Aineer](https://github.com/andeya/aineer).

[中文文档](README_CN.md)

This crate implements the full MCP client stack:

| Layer          | Components                                                                    |
| -------------- | ----------------------------------------------------------------------------- |
| **Transport**  | `McpStdioProcess` (stdin/stdout JSON-RPC), `McpRemoteClient` (HTTP/WebSocket) |
| **Management** | `McpServerManager` — lifecycle, tool discovery, pagination, batch dispatch    |
| **Config**     | `McpServerConfig`, `McpConfigCollection`, `ScopedMcpServerConfig`             |
| **Resources**  | `list_resources`, `read_resource` — first-class MCP resource support          |
| **Prompts**    | `list_prompts`, `get_prompt` — MCP prompt template support                    |
| **Auth**       | `McpClientAuth`, OAuth PKCE flow types (`OAuthAuthorizeParams`)               |
| **Naming**     | Tool name scoping, server signatures, proxy URL handling                      |

### Error handling

All transport operations return `Result<T, McpTransportError>`, a structured `thiserror` enum with variants for connection, timeout, protocol, ID mismatch, server error, WebSocket, HTTP, I/O, and JSON failures. The manager wraps these in `McpServerManagerError` for higher-level callers.

### Supported transports

| Transport | Config variant             | Implementation    |
| --------- | -------------------------- | ----------------- |
| stdio     | `McpStdioServerConfig`     | `McpStdioProcess` |
| HTTP/SSE  | `McpRemoteServerConfig`    | `McpRemoteClient` |
| WebSocket | `McpWebSocketServerConfig` | `McpRemoteClient` |
| SDK       | `McpSdkServerConfig`       | Via managed proxy |

## Note

This is an internal crate of the Aineer project. It is published to crates.io as a dependency of `aineer-cli` and is not intended for standalone use. API stability is not guaranteed outside the Aineer workspace.

## License

MIT — see [LICENSE](https://github.com/andeya/aineer/blob/main/LICENSE) for details.
