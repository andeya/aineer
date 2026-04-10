use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use super::types::AgentRunStatus;
use super::{
    agent_permission_policy, allowed_tools_for_subagent, execute_agent_with_spawn,
    final_assistant_text, mvp_tool_specs, persist_agent_terminal_state, push_output_block,
    AgentInput, AgentJob, SubagentToolExecutor,
};
use aineer_api::OutputContentBlock;
use aineer_engine::{ApiRequest, AssistantEvent, ConversationRuntime, RuntimeError, Session};
use serde_json::json;
use serde_json::Value;

/// Same as [`crate::execute_tool`] but preserves the old `Result<String, String>` test API.
fn execute_tool_str(name: &str, input: &Value) -> Result<String, String> {
    super::execute_tool(name, input.clone())
        .map(|o| o.content)
        .map_err(|e| e.to_string())
}

include!("suite1.rs");
include!("suite2.rs");
