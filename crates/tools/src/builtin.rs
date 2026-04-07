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
    fn dispatch(&self, input: &Value) -> Result<ToolOutput, ToolError>;
    fn check_concurrency_safe(&self, input: &Value) -> bool;
}

/// Blanket impl: any `BuiltinTool` automatically becomes `BuiltinToolDispatch`.
impl<T: BuiltinTool> BuiltinToolDispatch for T {
    fn name(&self) -> &'static str {
        T::NAME
    }

    fn dispatch(&self, input: &Value) -> Result<ToolOutput, ToolError> {
        let typed: T::Input = serde_json::from_value(input.clone())?;
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

/// All registered builtin tools. Tools are added here as they're migrated to the trait.
///
/// The old `execute_tool` match statement serves as fallback for tools not yet listed here.
static BUILTIN_TOOLS: &[&dyn BuiltinToolDispatch] = &[&SleepTool, &StructuredOutputTool];

// -- Example migrated tools below --

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_registered_tool() {
        assert!(find_builtin("Sleep").is_some());
        assert!(find_builtin("StructuredOutput").is_some());
        assert!(find_builtin("nonexistent").is_none());
    }

    #[test]
    fn sleep_tool_dispatch() {
        let input = serde_json::json!({ "duration_ms": 1 });
        let result = find_builtin("Sleep").unwrap().dispatch(&input);
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
        let result = find_builtin("Sleep").unwrap().dispatch(&bad_input);
        assert!(matches!(result, Err(ToolError::InputError(_))));
    }
}
