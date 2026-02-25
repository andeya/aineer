use std::collections::BTreeMap;

use crate::config::{McpOAuthConfig, McpServerConfig, ScopedMcpServerConfig};
use crate::mcp::{mcp_server_signature, mcp_tool_prefix, normalize_name_for_mcp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpClientTransport {
    Stdio(McpStdioTransport),
    Sse(McpRemoteTransport),
    Http(McpRemoteTransport),
    WebSocket(McpRemoteTransport),
    Sdk(McpSdkTransport),
    ManagedProxy(McpManagedProxyTransport),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpStdioTransport {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRemoteTransport {
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub headers_helper: Option<String>,
    pub auth: McpClientAuth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpSdkTransport {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpManagedProxyTransport {
    pub url: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpClientAuth {
    None,
    OAuth(McpOAuthConfig),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpClientBootstrap {
    pub server_name: String,
    pub normalized_name: String,
    pub tool_prefix: String,
    pub signature: Option<String>,
    pub transport: McpClientTransport,
}

impl McpClientBootstrap {
    #[must_use]
    pub fn from_scoped_config(server_name: &str, config: &ScopedMcpServerConfig) -> Self {
