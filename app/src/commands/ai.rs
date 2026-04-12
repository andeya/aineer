use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use tauri::Emitter;

use aineer_cli::desktop::{self, ShellContextSnippet, StreamDelta};

use super::next_block_id;

/// Active AI streams: `block_id` -> cancel flag (set by `stop_ai_stream`).
static AI_STREAM_ABORT: LazyLock<Mutex<HashMap<u64, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Serialize, Deserialize)]
pub struct AiMessageRequest {
    pub message: String,
    pub model: Option<String>,
    /// Workspace / project root for settings discovery (defaults to current dir).
    #[serde(default)]
    pub cwd: Option<String>,
    /// Recent shell runs from the UI (command + output) for zero-shot context.
    #[serde(default)]
    pub shell_context: Vec<ShellContextSnippet>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AiStreamDelta {
    block_id: u64,
    delta: String,
    /// `"text"` for formal output, `"thinking"` for model reasoning, empty on done.
    kind: String,
    done: bool,
}

/// Send a user message to the configured provider and stream assistant text via `ai_stream_delta`.
#[tauri::command]
pub async fn send_ai_message(app: tauri::AppHandle, request: AiMessageRequest) -> AppResult<u64> {
    let block_id = next_block_id();
    let cwd = super::workspace_cwd_from(request.cwd.as_deref());
    let message = request.message.clone();
    let model = request.model.clone();
    let shell_context = request.shell_context.clone();

    tracing::info!(
        "send_ai_message: block_id={block_id}, cwd={}, model={:?}",
        cwd.display(),
        model
    );

    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut map = AI_STREAM_ABORT
            .lock()
            .map_err(|e| AppError::Ai(format!("stream registry lock poisoned: {e}")))?;
        map.insert(block_id, Arc::clone(&cancel));
    }

    let app_clone = app.clone();
    // `stream_desktop_chat` uses `dyn Write` internally; its future is not `Send`, so run it on a
    // current-thread runtime inside `spawn_blocking` instead of `tokio::spawn`.
    tokio::task::spawn_blocking(move || {
        let emit_delta = |delta: &str, kind: &str, done: bool| {
            let _ = app_clone.emit(
                "ai_stream_delta",
                AiStreamDelta {
                    block_id,
                    delta: delta.to_string(),
                    kind: kind.to_string(),
                    done,
                },
            );
        };

        let rt_result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();

        let result = match rt_result {
            Ok(rt) => rt.block_on(async {
                desktop::stream_desktop_chat(
                    &cwd,
                    model.as_deref(),
                    &message,
                    &shell_context,
                    Arc::clone(&cancel),
                    |d| match d {
                        StreamDelta::Text(t) => emit_delta(t, "text", false),
                        StreamDelta::Thinking(t) => emit_delta(t, "thinking", false),
                    },
                )
                .await
            }),
            Err(e) => Err(aineer_cli::desktop::DesktopStreamError::Cli(
                aineer_cli::error::CliError::Other(format!("tokio runtime: {e}")),
            )),
        };

        match result {
            Ok(_) => emit_delta("", "", true),
            Err(e) => emit_delta(&format!("**Error:** {e}"), "text", true),
        }

        if let Ok(mut map) = AI_STREAM_ABORT.lock() {
            map.remove(&block_id);
        }
    });

    Ok(block_id)
}

#[tauri::command]
pub async fn stop_ai_stream(block_id: u64) -> AppResult<()> {
    tracing::info!("stop_ai_stream: block_id={block_id}");
    let map = AI_STREAM_ABORT
        .lock()
        .map_err(|e| AppError::Ai(format!("stream registry lock poisoned: {e}")))?;
    if let Some(flag) = map.get(&block_id) {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}
