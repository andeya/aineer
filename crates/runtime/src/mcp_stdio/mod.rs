mod manager;
mod process;
pub(crate) mod types;

#[cfg(all(test, unix))]
mod tests;

pub use manager::McpServerManager;
pub use process::McpStdioProcess;
pub use types::{
    ManagedMcpTool, McpServerManagerError, McpTool, McpToolCallContent, McpToolCallResult,
    UnsupportedMcpServer,
};
