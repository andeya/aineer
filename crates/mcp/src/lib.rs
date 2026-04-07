//! MCP (Model Context Protocol) client, server management, and configuration types.

mod client;
pub mod config;
mod naming;
mod remote;
pub mod stdio;

pub use client::{
    McpClientAuth, McpClientBootstrap, McpClientTransport, McpManagedProxyTransport,
    McpRemoteTransport, McpSdkTransport, McpStdioTransport, OAuthAuthorizeParams, OAuthTokenResult,
};
pub use config::{
    ConfigSource, McpConfigCollection, McpManagedProxyServerConfig, McpOAuthConfig,
    McpRemoteServerConfig, McpSdkServerConfig, McpServerConfig, McpStdioServerConfig, McpTransport,
    McpWebSocketServerConfig, ScopedMcpServerConfig,
};
pub use naming::{
    mcp_server_signature, mcp_tool_name, mcp_tool_prefix, normalize_name_for_mcp,
    scoped_mcp_config_hash, unwrap_mcp_proxy_url,
};
pub use remote::McpRemoteClient;
pub use stdio::{
    ManagedMcpTool, McpGetPromptParams, McpGetPromptResult, McpListPromptsParams,
    McpListPromptsResult, McpListResourcesParams, McpListResourcesResult, McpPrompt,
    McpPromptArgument, McpPromptContent, McpPromptMessage, McpReadResourceParams,
    McpReadResourceResult, McpResource, McpResourceContents, McpServerManager,
    McpServerManagerError, McpStdioProcess, McpTool, McpToolCallContent, McpToolCallResult,
    McpTransportError, UnsupportedMcpServer,
};
