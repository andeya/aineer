use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;

use serde_json::json;

use crate::{PluginError, PluginHooks, PluginRegistry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
}

impl HookEvent {
    fn as_str(self) -> &'static str {
        match self {
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookRunResult {
    denied: bool,
    messages: Vec<String>,
}

impl HookRunResult {
    #[must_use]
    pub fn allow(messages: Vec<String>) -> Self {
        Self {
            denied: false,
            messages,
        }
    }

    #[must_use]
    pub fn is_denied(&self) -> bool {
        self.denied
    }

    #[must_use]
    pub fn messages(&self) -> &[String] {
        &self.messages
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HookRunner {
    hooks: PluginHooks,
}

impl HookRunner {
    #[must_use]
    pub fn new(hooks: PluginHooks) -> Self {
        Self { hooks }
    }

    pub fn from_registry(plugin_registry: &PluginRegistry) -> Result<Self, PluginError> {
        Ok(Self::new(plugin_registry.aggregated_hooks()?))
    }

    #[must_use]
    pub fn run_pre_tool_use(&self, tool_name: &str, tool_input: &str) -> HookRunResult {
        run_hook_commands(
            HookEvent::PreToolUse,
            &self.hooks.pre_tool_use,
            tool_name,
            tool_input,
            None,
            false,
        )
    }

    #[must_use]
    pub fn run_post_tool_use(
        &self,
        tool_name: &str,
        tool_input: &str,
        tool_output: &str,
        is_error: bool,
    ) -> HookRunResult {
        run_hook_commands(
            HookEvent::PostToolUse,
            &self.hooks.post_tool_use,
            tool_name,
            tool_input,
            Some(tool_output),
            is_error,
        )
    }
}

fn run_hook_commands(
    event: HookEvent,
    commands: &[String],
    tool_name: &str,
    tool_input: &str,
    tool_output: Option<&str>,
    is_error: bool,
) -> HookRunResult {
    if commands.is_empty() {
        return HookRunResult::allow(Vec::new());
    }

    let payload = json!({
        "hook_event_name": event.as_str(),
        "tool_name": tool_name,
        "tool_input": parse_tool_input(tool_input),
        "tool_input_json": tool_input,
        "tool_output": tool_output,
        "tool_result_is_error": is_error,
    })
    .to_string();

    let mut messages = Vec::new();

    let context = HookContext {
        event,
        tool_name,
        tool_input,
        tool_output,
        is_error,
        payload: &payload,
    };

    for command in commands {
        match run_hook_command(command, &context) {
            HookCommandOutcome::Allow { message } => {
                if let Some(message) = message {
                    messages.push(message);
                }
            }
            HookCommandOutcome::Deny { message } => {
                messages.push(message.unwrap_or_else(|| {
                    format!("{} hook denied tool `{tool_name}`", event.as_str())
                }));
                return HookRunResult {
                    denied: true,
                    messages,
                };
            }
            HookCommandOutcome::Warn { message } => messages.push(message),
        }
    }

    HookRunResult::allow(messages)
}

struct HookContext<'a> {
    event: HookEvent,
    tool_name: &'a str,
    tool_input: &'a str,
    tool_output: Option<&'a str>,
    is_error: bool,
    payload: &'a str,
}

fn run_hook_command(command: &str, ctx: &HookContext<'_>) -> HookCommandOutcome {
    let mut child = shell_command(command);
    child.stdin(std::process::Stdio::piped());
    child.stdout(std::process::Stdio::piped());
    child.stderr(std::process::Stdio::piped());
    child.env("HOOK_EVENT", ctx.event.as_str());
    child.env("HOOK_TOOL_NAME", ctx.tool_name);
    child.env("HOOK_TOOL_INPUT", ctx.tool_input);
    child.env("HOOK_TOOL_IS_ERROR", if ctx.is_error { "1" } else { "0" });
    if let Some(tool_output) = ctx.tool_output {
        child.env("HOOK_TOOL_OUTPUT", tool_output);
    }

    match child.output_with_stdin(ctx.payload.as_bytes()) {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let message = (!stdout.is_empty()).then_some(stdout);
            match output.status.code() {
                Some(0) => HookCommandOutcome::Allow { message },
                Some(2) => HookCommandOutcome::Deny { message },
                Some(code) => HookCommandOutcome::Warn {
                    message: format_hook_warning(
                        command,
                        code,
                        message.as_deref(),
                        stderr.as_str(),
                    ),
                },
                None => HookCommandOutcome::Warn {
                    message: format!(
                        "{} hook `{command}` terminated by signal while handling `{}`",
                        ctx.event.as_str(),
                        ctx.tool_name,
                    ),
                },
            }
        }
