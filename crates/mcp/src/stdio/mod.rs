mod manager;
mod process;
pub mod types;

#[cfg(all(test, unix))]
mod tests;

pub use manager::McpServerManager;
pub use process::McpStdioProcess;
pub use types::{
    ManagedMcpTool, McpGetPromptParams, McpGetPromptResult, McpListPromptsParams,
    McpListPromptsResult, McpListResourcesParams, McpListResourcesResult, McpPrompt,
    McpPromptArgument, McpPromptContent, McpPromptMessage, McpReadResourceParams,
    McpReadResourceResult, McpResource, McpResourceContents, McpServerManagerError, McpTool,
    McpToolCallContent, McpToolCallResult, McpTransportError, UnsupportedMcpServer,
};
