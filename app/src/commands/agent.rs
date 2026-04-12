use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use tauri::Emitter;

use aineer_cli::desktop::{self, ShellContextSnippet};

use super::next_block_id;

/// Active agent tasks: `block_id` -> cancel flag (set by `stop_agent`).
static AGENT_ABORT: LazyLock<Mutex<HashMap<u64, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentRequest {
    pub goal: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub shell_context: Vec<ShellContextSnippet>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentEvent {
    block_id: u64,
    kind: String,
    data: String,
}

/// Run one agent turn (tools enabled, `PermissionMode::Allow` — GUI stdin prompts are not used).
#[tauri::command]
pub async fn start_agent(app: tauri::AppHandle, request: AgentRequest) -> AppResult<u64> {
    let block_id = next_block_id();
    let cwd = super::workspace_cwd_from(request.cwd.as_deref());
    let goal = request.goal.clone();
    let model = request.model.clone();
    let shell_context = request.shell_context.clone();

    tracing::info!(
        "start_agent: block_id={block_id}, cwd={}, goal_len={}",
        cwd.display(),
        goal.len()
    );

    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut map = AGENT_ABORT
            .lock()
            .map_err(|e| AppError::Agent(format!("agent registry lock poisoned: {e}")))?;
        map.insert(block_id, Arc::clone(&cancel));
    }

    let app_clone = app.clone();
    tokio::task::spawn_blocking(move || {
        let result = desktop::run_desktop_agent_turn(&cwd, model.as_deref(), &goal, &shell_context);

        let (kind, data) = match result {
            Ok(text) => ("text".to_string(), text),
            Err(e) => ("error".to_string(), e.to_string()),
        };

        let _ = app_clone.emit(
            "agent_event",
            AgentEvent {
                block_id,
                kind,
                data,
            },
        );
        let _ = app_clone.emit(
            "agent_event",
            AgentEvent {
                block_id,
                kind: "done".into(),
                data: String::new(),
            },
        );

        if let Ok(mut map) = AGENT_ABORT.lock() {
            map.remove(&block_id);
        }
    });

    Ok(block_id)
}

#[tauri::command]
pub async fn approve_tool(block_id: u64) -> AppResult<()> {
    tracing::info!("approve_tool: block_id={block_id} (no-op until GUI approval is wired)");
    Ok(())
}

#[tauri::command]
pub async fn deny_tool(block_id: u64) -> AppResult<()> {
    tracing::info!("deny_tool: block_id={block_id} (no-op until GUI approval is wired)");
    Ok(())
}

#[tauri::command]
pub async fn stop_agent(block_id: u64) -> AppResult<()> {
    tracing::info!("stop_agent: block_id={block_id}");
    let map = AGENT_ABORT
        .lock()
        .map_err(|e| AppError::Agent(format!("agent registry lock poisoned: {e}")))?;
    if let Some(flag) = map.get(&block_id) {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}
