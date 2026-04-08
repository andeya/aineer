//! Experimental agent-collaboration tools.
//!
//! These tools enable multi-agent orchestration patterns:
//! - `TeamCreate` / `TeamDelete`: manage named groups of agent endpoints.
//! - `SendMessage`: send a message (text or tool-result) to an agent or team.
//! - `SlashCommand`: invoke a named slash-command on the local agent.
//!
//! In this release the implementation uses an in-process registry and an HTTP
//! transport backed by the same `reqwest` client used by the web tools.
//! Production deployments may replace the transport layer without changing the
//! tool interface.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::builtin::BuiltinTool;
use crate::tool_output::{ToolError, ToolOutput};
use crate::types::{SendMessageInput, SlashCommandInput, TeamCreateInput, TeamDeleteInput};
use crate::web::{block_on_web, http_client};

// ── Team registry ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Team {
    name: String,
    description: Option<String>,
    /// Agent endpoint URLs.
    members: Vec<String>,
}

type TeamRegistry = BTreeMap<String, Team>;

static TEAMS: OnceLock<Mutex<TeamRegistry>> = OnceLock::new();

fn teams() -> &'static Mutex<TeamRegistry> {
    TEAMS.get_or_init(|| Mutex::new(TeamRegistry::new()))
}

// ── Slash-command registry ────────────────────────────────────────────────────

pub type SlashCommandHandler =
    Box<dyn Fn(&str, &serde_json::Value) -> Result<String, String> + Send + Sync>;

static SLASH_COMMANDS: OnceLock<Mutex<BTreeMap<String, SlashCommandHandler>>> = OnceLock::new();

fn slash_commands() -> &'static Mutex<BTreeMap<String, SlashCommandHandler>> {
    SLASH_COMMANDS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Register a slash-command handler. Must be called before any `SlashCommand` tool invocation.
pub fn register_slash_command(name: impl Into<String>, handler: SlashCommandHandler) {
    slash_commands()
        .lock()
        .expect("slash command registry poisoned")
        .insert(name.into(), handler);
}

// ── Tool implementations ──────────────────────────────────────────────────────

pub(crate) fn execute_team_create(input: TeamCreateInput) -> Result<String, String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("team name must not be empty".to_string());
    }
    let mut guard = teams()
        .lock()
        .map_err(|e| format!("team registry lock poisoned: {e}"))?;
    if guard.contains_key(&name) {
        return Err(format!("team '{name}' already exists"));
    }
    guard.insert(
        name.clone(),
        Team {
            name: name.clone(),
            description: input.description,
            members: input.members.unwrap_or_default(),
        },
    );
    Ok(format!("Team '{name}' created."))
}

pub(crate) fn execute_team_delete(input: TeamDeleteInput) -> Result<String, String> {
    let mut guard = teams()
        .lock()
        .map_err(|e| format!("team registry lock poisoned: {e}"))?;
    guard
        .remove(input.name.trim())
        .map(|_| format!("Team '{}' deleted.", input.name))
        .ok_or_else(|| format!("team '{}' not found", input.name))
}

pub(crate) fn execute_send_message(input: SendMessageInput) -> Result<String, String> {
    let client = http_client();

    let endpoints: Vec<String> = {
        let guard = teams()
            .lock()
            .map_err(|e| format!("team registry lock poisoned: {e}"))?;
        if let Some(team) = guard.get(input.recipient.trim()) {
            team.members.clone()
        } else {
            vec![input.recipient.trim().to_string()]
        }
    };

    if endpoints.is_empty() {
        return Err(format!("no endpoints for recipient '{}'", input.recipient));
    }

    let payload = json!({
        "role": "user",
        "content": input.content,
    });

    block_on_web(async {
        let mut results: Vec<String> = Vec::new();
        for endpoint in &endpoints {
            let resp = client
                .post(endpoint)
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("send_message POST to {endpoint} failed: {e}"))?
                .text()
                .await
                .map_err(|e| format!("reading response from {endpoint}: {e}"))?;
            results.push(format!("[{endpoint}] {resp}"));
        }
        Ok(results.join("\n"))
    })
}

pub(crate) fn execute_slash_command(input: SlashCommandInput) -> Result<String, String> {
    let name = input.command.trim_start_matches('/').to_string();
    let args = input.args.unwrap_or(serde_json::Value::Null);
    let guard = slash_commands()
        .lock()
        .map_err(|e| format!("slash command registry lock poisoned: {e}"))?;
    let handler = guard
        .get(&name)
        .ok_or_else(|| format!("slash command '/{name}' not registered"))?;
    handler(&name, &args)
}

// ---------------------------------------------------------------------------
// BuiltinTool adapters
// ---------------------------------------------------------------------------

pub(crate) struct TeamCreateTool;

impl BuiltinTool for TeamCreateTool {
    const NAME: &'static str = "TeamCreate";
    type Input = TeamCreateInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        execute_team_create(input)
            .map(ToolOutput::ok)
            .map_err(ToolError::execution)
    }
}

pub(crate) struct TeamDeleteTool;

impl BuiltinTool for TeamDeleteTool {
    const NAME: &'static str = "TeamDelete";
    type Input = TeamDeleteInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        execute_team_delete(input)
            .map(ToolOutput::ok)
            .map_err(ToolError::execution)
    }
}

pub(crate) struct SendMessageTool;

impl BuiltinTool for SendMessageTool {
    const NAME: &'static str = "SendMessage";
    type Input = SendMessageInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        execute_send_message(input)
            .map(ToolOutput::ok)
            .map_err(ToolError::execution)
    }
}

pub(crate) struct SlashCommandTool;

impl BuiltinTool for SlashCommandTool {
    const NAME: &'static str = "SlashCommand";
    type Input = SlashCommandInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        execute_slash_command(input)
            .map(ToolOutput::ok)
            .map_err(ToolError::execution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SlashCommandInput, TeamCreateInput, TeamDeleteInput};

    #[test]
    fn team_create_and_delete() {
        let name = format!("test-team-{}", std::process::id());
        execute_team_create(TeamCreateInput {
            name: name.clone(),
            description: Some("unit test team".to_string()),
            members: Some(vec!["http://localhost:1234".to_string()]),
        })
        .unwrap();

        let err = execute_team_create(TeamCreateInput {
            name: name.clone(),
            description: None,
            members: None,
        })
        .unwrap_err();
        assert!(err.contains("already exists"));

        execute_team_delete(TeamDeleteInput { name: name.clone() }).unwrap();

        let err = execute_team_delete(TeamDeleteInput { name }).unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn team_create_rejects_empty_name() {
        let err = execute_team_create(TeamCreateInput {
            name: "  ".to_string(),
            description: None,
            members: None,
        })
        .unwrap_err();
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn slash_command_not_registered() {
        let err = execute_slash_command(SlashCommandInput {
            command: "/nonexistent_cmd".to_string(),
            args: None,
        })
        .unwrap_err();
        assert!(err.contains("not registered"));
    }

    #[test]
    fn slash_command_invocation() {
        register_slash_command(
            "test_echo",
            Box::new(|name, args| Ok(format!("echo: {name} {args}"))),
        );
        let result = execute_slash_command(SlashCommandInput {
            command: "/test_echo".to_string(),
            args: Some(serde_json::json!({"key": "val"})),
        })
        .unwrap();
        assert!(result.contains("echo: test_echo"));
    }
}
