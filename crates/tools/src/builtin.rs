//! Trait-based builtin tool dispatch.
//!
//! Each built-in tool implements [`BuiltinToolDispatch`] (typically via the
//! blanket impl from [`BuiltinTool`]). Tools self-register in the
//! [`BUILTIN_TOOLS`] static array, eliminating the giant match statement.

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::tool_output::{ToolError, ToolOutput};

/// Core trait for a statically-typed built-in tool.
///
/// Each implementor has a compile-time `NAME` and a typed `Input`.
/// The blanket impl on [`BuiltinToolDispatch`] handles JSON deserialization.
pub trait BuiltinTool: Sync {
    const NAME: &'static str;
    type Input: DeserializeOwned;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError>;

    /// Whether this tool is safe to run concurrently with other tools.
    /// Default: false (conservative).
    fn is_concurrency_safe(_input: &Self::Input) -> bool {
        false
    }
}

/// Object-safe dispatch wrapper for [`BuiltinTool`].
///
/// This trait enables heterogeneous storage in `&[&dyn BuiltinToolDispatch]`.
pub trait BuiltinToolDispatch: Sync {
    fn name(&self) -> &'static str;
    fn dispatch(&self, input: Value) -> Result<ToolOutput, ToolError>;
    fn check_concurrency_safe(&self, input: &Value) -> bool;
}

/// Blanket impl: any `BuiltinTool` automatically becomes `BuiltinToolDispatch`.
impl<T: BuiltinTool> BuiltinToolDispatch for T {
    fn name(&self) -> &'static str {
        T::NAME
    }

    fn dispatch(&self, input: Value) -> Result<ToolOutput, ToolError> {
        let typed: T::Input = serde_json::from_value(input)?;
        T::execute(typed)
    }

    fn check_concurrency_safe(&self, input: &Value) -> bool {
        match serde_json::from_value::<T::Input>(input.clone()) {
            Ok(typed) => T::is_concurrency_safe(&typed),
            Err(_) => false,
        }
    }
}

/// Look up a builtin tool by name.
pub fn find_builtin(name: &str) -> Option<&'static dyn BuiltinToolDispatch> {
    BUILTIN_TOOLS.iter().find(|t| t.name() == name).copied()
}

// -- Built-in tools implemented in this module --

struct SleepTool;

impl BuiltinTool for SleepTool {
    const NAME: &'static str = "Sleep";
    type Input = crate::types::SleepInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        let output = crate::execute_sleep(input);
        serde_json::to_string_pretty(&output)
            .map(ToolOutput::ok)
            .map_err(ToolError::from)
    }

    fn is_concurrency_safe(_input: &Self::Input) -> bool {
        true
    }
}

struct ToolSearchTool;

impl BuiltinTool for ToolSearchTool {
    const NAME: &'static str = "ToolSearch";
    type Input = crate::types::ToolSearchInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        let output = crate::execute_tool_search_with_context(input, None, &[], None);
        serde_json::to_string_pretty(&output)
            .map(ToolOutput::ok)
            .map_err(ToolError::from)
    }

    fn is_concurrency_safe(_input: &Self::Input) -> bool {
        true
    }
}

struct StructuredOutputTool;

impl BuiltinTool for StructuredOutputTool {
    const NAME: &'static str = "StructuredOutput";
    type Input = crate::types::StructuredOutputInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        let output = crate::execute_structured_output(input);
        serde_json::to_string_pretty(&output)
            .map(ToolOutput::ok)
            .map_err(ToolError::from)
    }

    fn is_concurrency_safe(_input: &Self::Input) -> bool {
        true
    }
}

/// All registered builtin tools.
static BUILTIN_TOOLS: &[&dyn BuiltinToolDispatch] = &[
    &SleepTool,
    &StructuredOutputTool,
    &ToolSearchTool,
    &crate::AgentTool,
    &crate::AskUserQuestionTool,
    &crate::BashTool,
    &crate::BriefTool,
    &crate::ConfigTool,
    &crate::cron::CronCreateTool,
    &crate::cron::CronDeleteTool,
    &crate::cron::CronListTool,
    &crate::EditFileTool,
    &crate::plan_mode::EnterPlanModeTool,
    &crate::worktree::EnterWorktreeTool,
    &crate::plan_mode::ExitPlanModeTool,
    &crate::worktree::ExitWorktreeTool,
    &crate::GlobSearchTool,
    &crate::GrepSearchTool,
    &crate::mcp_resource::ListMcpResourcesTool,
    &crate::lsp_tool::LspTool,
    &crate::mcp_resource::McpSearchTool,
    &crate::notebook::NotebookEditTool,
    &crate::MultiEditTool,
    &crate::powershell::PowerShellTool,
    &crate::ReadFileTool,
    &crate::mcp_resource::ReadMcpResourceTool,
    &crate::ReplTool,
    &crate::collab::SendMessageTool,
    &crate::SendUserMessageTool,
    &crate::SkillTool,
    &crate::collab::SlashCommandTool,
    &crate::task::TaskCreateTool,
    &crate::task::TaskGetTool,
    &crate::task::TaskListTool,
    &crate::task::TaskStopTool,
    &crate::task::TaskUpdateTool,
    &crate::collab::TeamCreateTool,
    &crate::collab::TeamDeleteTool,
    &crate::TodoWriteTool,
    &crate::web::WebFetchTool,
    &crate::web::WebSearchTool,
    &crate::WriteFileTool,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_registered_tool() {
        assert!(find_builtin("Sleep").is_some());
        assert!(find_builtin("StructuredOutput").is_some());
        assert!(find_builtin("ToolSearch").is_some());
        assert!(find_builtin("nonexistent").is_none());
    }

    #[test]
    fn tool_search_dispatch_finds_extended_tool() {
        let input = serde_json::json!({ "query": "web fetch", "max_results": 3 });
        let result = find_builtin("ToolSearch").unwrap().dispatch(input);
        assert!(result.is_ok());
        let content = result.unwrap().content;
        assert!(content.contains("WebFetch"));
    }

    #[test]
    fn tool_search_category_filters() {
        let input = serde_json::json!({
            "query": "list",
            "category": "mcp",
            "max_results": 5
        });
        let result = find_builtin("ToolSearch").unwrap().dispatch(input);
        assert!(result.is_ok());
        assert!(result.unwrap().content.contains("ListMcpResources"));
    }

    #[test]
    fn sleep_tool_dispatch() {
        let input = serde_json::json!({ "duration_ms": 1 });
        let result = find_builtin("Sleep").unwrap().dispatch(input);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.is_error);
        assert!(output.content.contains("Slept for 1ms"));
    }

    #[test]
    fn concurrency_safe_check() {
        let input = serde_json::json!({ "duration_ms": 1 });
        assert!(find_builtin("Sleep")
            .unwrap()
            .check_concurrency_safe(&input));
    }

    #[test]
    fn bad_input_returns_error() {
        let bad_input = serde_json::json!({ "wrong_field": true });
        let result = find_builtin("Sleep").unwrap().dispatch(bad_input);
        assert!(matches!(result, Err(ToolError::InputError(_))));
    }
}
