use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use aineer_api::ToolDefinition;
use aineer_engine::{ToolError, ToolExecutor};
use aineer_tools::GlobalToolRegistry;

use crate::cli::{discover_mcp_tools, AllowedToolSet, SharedMcpManager};
use crate::render::TerminalRenderer;

pub(crate) struct CliToolExecutor {
    renderer: TerminalRenderer,
    emit_output: bool,
    allowed_tools: Option<AllowedToolSet>,
    tool_registry: GlobalToolRegistry,
    mcp_manager: SharedMcpManager,
    async_runtime: Arc<tokio::runtime::Runtime>,
    cached_mcp_tools: Arc<Mutex<Option<Vec<ToolDefinition>>>>,
    activated_tools: Arc<Mutex<HashSet<String>>>,
}

impl CliToolExecutor {
    pub(crate) fn new(
        allowed_tools: Option<AllowedToolSet>,
        emit_output: bool,
        tool_registry: GlobalToolRegistry,
        mcp_manager: SharedMcpManager,
        async_runtime: Arc<tokio::runtime::Runtime>,
        cached_mcp_tools: Arc<Mutex<Option<Vec<ToolDefinition>>>>,
        activated_tools: Arc<Mutex<HashSet<String>>>,
    ) -> Self {
        Self {
            renderer: TerminalRenderer::new(),
            emit_output,
            allowed_tools,
            tool_registry,
            mcp_manager,
            async_runtime,
            cached_mcp_tools,
            activated_tools,
        }
    }

    fn gutter_stdout() -> crate::render::GutterWriter<std::io::Stdout> {
        let (first, cont) = crate::render::gutter_prefixes();
        crate::render::GutterWriter::new(std::io::stdout(), first, cont)
    }

    fn execute_tool_search(&self, input: &str) -> Result<String, ToolError> {
        let search_input: aineer_tools::ToolSearchInput = serde_json::from_str(input)
            .map_err(|e| ToolError::new(format!("invalid ToolSearch input: {e}")))?;
        let pending = self
            .mcp_manager
            .lock()
            .ok()
            .map(|guard| {
                guard
                    .unsupported_servers()
                    .iter()
                    .map(|s| s.server_name.clone())
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty());
        let mut extra = self.tool_registry.plugin_tool_definitions();
        let mcp = {
            let mut guard = self
                .cached_mcp_tools
                .lock()
                .map_err(|e| ToolError::new(format!("MCP tool cache lock poisoned: {e}")))?;
            guard
                .get_or_insert_with(|| discover_mcp_tools(&self.async_runtime, &self.mcp_manager))
                .clone()
        };
        extra.extend(mcp);
        let output = aineer_tools::execute_tool_search_with_context(
            search_input,
            pending,
            &extra,
            self.allowed_tools.as_ref(),
        );
        if let Ok(mut activated) = self.activated_tools.lock() {
            for name in &output.matches {
                activated.insert(name.clone());
            }
        }
        serde_json::to_string_pretty(&output)
            .map_err(|e| ToolError::new(format!("failed to serialize ToolSearch output: {e}")))
    }

    fn execute_mcp_tool(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        let arguments: Option<serde_json::Value> = if input.trim().is_empty() {
            None
        } else {
            Some(
                serde_json::from_str(input)
                    .map_err(|e| ToolError::new(format!("invalid MCP tool input JSON: {e}")))?,
            )
        };
        let mut guard = self
            .mcp_manager
            .lock()
            .map_err(|e| ToolError::new(format!("MCP manager lock poisoned: {e}")))?;
        let response = self
            .async_runtime
            .block_on(guard.call_tool(tool_name, arguments))
            .map_err(|e| ToolError::new(format!("MCP tool call failed: {e}")))?;
        match response.result {
            Some(result) => {
                let text = result
                    .content
                    .iter()
                    .filter_map(|block| block.data.get("text").and_then(|v| v.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(text)
            }
            None => {
                if let Some(error) = response.error {
                    Err(ToolError::new(format!(
                        "MCP error ({}): {}",
                        error.code, error.message
                    )))
                } else {
                    Ok(String::new())
                }
            }
        }
    }

    fn render_tool_output(&mut self, tool_name: &str, output: &str, is_error: bool) {
        if !self.emit_output {
            return;
        }
        let markdown = crate::tool_display::format_tool_result(tool_name, output, is_error);
        let _ = self
            .renderer
            .stream_markdown(&markdown, &mut Self::gutter_stdout());
    }
}

impl ToolExecutor for CliToolExecutor {
    fn is_concurrency_safe(&self, tool_name: &str) -> bool {
        // MCP tools and ToolSearch are never run concurrently.
        if tool_name.starts_with("mcp__") || tool_name == "ToolSearch" {
            return false;
        }
        self.tool_registry.is_concurrency_safe(tool_name)
    }

    fn execute_batch(
        &self,
        calls: &[(&str, &str)],
    ) -> Option<Vec<Result<String, aineer_engine::ToolError>>> {
        let batch = self.tool_registry.execute_batch(calls);
        Some(
            batch
                .into_iter()
                .map(|r| r.map_err(aineer_engine::ToolError::new))
                .collect(),
        )
    }

    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        if self
            .allowed_tools
            .as_ref()
            .is_some_and(|allowed| !allowed.contains(tool_name))
        {
            return Err(ToolError::new(format!(
                "tool `{tool_name}` is not enabled by the current --allowedTools setting"
            )));
        }

        if tool_name.starts_with("mcp__") {
            let result = self.execute_mcp_tool(tool_name, input);
            match &result {
                Ok(output) => self.render_tool_output(tool_name, output, false),
                Err(error) => self.render_tool_output(tool_name, &error.to_string(), true),
            }
            return result;
        }

        if tool_name == "ToolSearch" {
            return self.execute_tool_search(input);
        }

        let value = serde_json::from_str(input)
            .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))?;
        match self.tool_registry.execute(tool_name, value) {
            Ok(output) => {
                self.render_tool_output(tool_name, &output, false);
                Ok(output)
            }
            Err(error) => {
                self.render_tool_output(tool_name, &error, true);
                Err(ToolError::new(error))
            }
        }
    }
}
