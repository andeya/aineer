use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use tokio::sync::watch;

use aineer_api::{
    ContentBlockDelta, InputContentBlock, InputMessage, MessageRequest, ProviderClient,
    StreamEvent, SystemBlock,
};

use crate::config::GatewayConfig;
use crate::types::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GatewayStatus {
    Starting,
    Running,
    Stopped,
    Error,
}

pub struct GatewayServer {
    config: GatewayConfig,
    status_tx: watch::Sender<GatewayStatus>,
    status_rx: watch::Receiver<GatewayStatus>,
}

struct AppState {
    config: GatewayConfig,
}

impl GatewayServer {
    pub fn new(config: GatewayConfig) -> Self {
        let (status_tx, status_rx) = watch::channel(GatewayStatus::Stopped);
        Self {
            config,
            status_tx,
            status_rx,
        }
    }

    pub fn status(&self) -> GatewayStatus {
        *self.status_rx.borrow()
    }

    pub fn status_rx(&self) -> watch::Receiver<GatewayStatus> {
        self.status_rx.clone()
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        if !self.config.enabled {
            tracing::info!("Gateway is disabled in config");
            return Ok(());
        }

        let addr: SocketAddr = self.config.listen_addr.parse()?;
        let state = Arc::new(AppState {
            config: self.config.clone(),
        });

        let app = Router::new()
            .route("/health", get(health_handler))
            .route("/v1/chat/completions", post(completions_handler))
            .route("/v1/models", get(models_handler))
            .with_state(state);

        tracing::info!("Aineer Gateway listening on {}", addr);
        self.status_tx.send_replace(GatewayStatus::Starting);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        self.status_tx.send_replace(GatewayStatus::Running);

        if let Err(e) = axum::serve(listener, app).await {
            self.status_tx.send_replace(GatewayStatus::Error);
            return Err(e.into());
        }

        self.status_tx.send_replace(GatewayStatus::Stopped);
        Ok(())
    }
}

async fn health_handler() -> &'static str {
    "ok"
}

async fn models_handler() -> Json<ModelListResponse> {
    let now = now_secs();

    let known_models = aineer_api::list_known_models(None);
    let data: Vec<ModelInfo> = known_models
        .into_iter()
        .map(|(id, kind)| ModelInfo {
            id: id.to_string(),
            object: "model".to_string(),
            created: now,
            owned_by: format!("{kind:?}"),
        })
        .collect();

    Json(ModelListResponse {
        object: "list".to_string(),
        data,
    })
}

async fn completions_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatCompletionRequest>,
) -> impl IntoResponse {
    let model = if req.model.is_empty() {
        state
            .config
            .default_model
            .clone()
            .unwrap_or_else(|| "auto".to_string())
    } else {
        req.model.clone()
    };

    let (system, messages) = convert_messages(&req.messages);
    let max_tokens = req
        .max_tokens
        .unwrap_or_else(|| aineer_api::max_tokens_for_model(&model));

    let tools = req.tools.as_ref().and_then(|t| convert_tools(t));
    let tool_choice = req.tool_choice.as_ref().and_then(convert_tool_choice);

    let api_request = MessageRequest {
        model: model.clone(),
        max_tokens,
        messages,
        system,
        tools,
        tool_choice,
        stream: req.stream.unwrap_or(false),
        thinking: None,
        gemini_cached_content: None,
    };

    let client = match ProviderClient::from_model(&api_request.model) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    format!("Failed to resolve provider for model '{}': {}", model, e),
                    "invalid_request_error",
                )),
            )
                .into_response();
        }
    };

    if req.stream.unwrap_or(false) {
        match handle_streaming(client, api_request, &model).await {
            Ok(sse) => sse.into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string(), "api_error")),
            )
                .into_response(),
        }
    } else {
        match handle_non_streaming(client, api_request, &model).await {
            Ok(json) => json.into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(e.to_string(), "api_error")),
            )
                .into_response(),
        }
    }
}

async fn handle_non_streaming(
    client: ProviderClient,
    request: MessageRequest,
    model: &str,
) -> anyhow::Result<Json<ChatCompletionResponse>> {
    let response = client.send_message(&request).await?;

    let content_text = response
        .content
        .iter()
        .filter_map(|block| {
            if let aineer_api::OutputContentBlock::Text { text } = block {
                Some(text.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("");

    let now = now_secs();

    Ok(Json(ChatCompletionResponse {
        id: response.id,
        object: "chat.completion".to_string(),
        created: now,
        model: model.to_string(),
        choices: vec![ChatChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: Some(serde_json::Value::String(content_text)),
            },
            finish_reason: response.stop_reason.map(|r| map_finish_reason(&r)),
        }],
        usage: Some(UsageInfo {
            prompt_tokens: response.usage.input_tokens,
            completion_tokens: response.usage.output_tokens,
            total_tokens: response.usage.input_tokens + response.usage.output_tokens,
        }),
    }))
}

async fn handle_streaming(
    client: ProviderClient,
    request: MessageRequest,
    model: &str,
) -> anyhow::Result<Sse<impl futures_core::Stream<Item = Result<Event, anyhow::Error>>>> {
    let mut stream = client.stream_message(&request).await?;
    let model = model.to_string();
    let now = now_secs();

    let sse_stream = async_stream::stream! {
        let id = format!("chatcmpl-{now}");

        let initial_chunk = ChatCompletionChunk {
            id: id.clone(),
            object: "chat.completion.chunk".to_string(),
            created: now,
            model: model.clone(),
            choices: vec![ChatChunkChoice {
                index: 0,
                delta: ChatDelta {
                    role: Some("assistant".to_string()),
                    content: None,
                },
                finish_reason: None,
            }],
        };
        yield Ok(Event::default().data(serde_json::to_string(&initial_chunk).unwrap_or_default()));

        loop {
            match stream.next_event().await {
                Ok(Some(event)) => {
                    match event {
                        StreamEvent::ContentBlockDelta(delta_event) => {
                            let text = match &delta_event.delta {
                                ContentBlockDelta::TextDelta { text } => Some(text.clone()),
                                _ => None,
                            };

                            if let Some(text) = text {
                                let chunk = ChatCompletionChunk {
                                    id: id.clone(),
                                    object: "chat.completion.chunk".to_string(),
                                    created: now,
                                    model: model.clone(),
                                    choices: vec![ChatChunkChoice {
                                        index: 0,
                                        delta: ChatDelta {
                                            role: None,
                                            content: Some(text),
                                        },
                                        finish_reason: None,
                                    }],
                                };
                                yield Ok(Event::default().data(serde_json::to_string(&chunk).unwrap_or_default()));
                            }
                        }
                        StreamEvent::MessageStop(_) => {
                            let final_chunk = ChatCompletionChunk {
                                id: id.clone(),
                                object: "chat.completion.chunk".to_string(),
                                created: now,
                                model: model.clone(),
                                choices: vec![ChatChunkChoice {
                                    index: 0,
                                    delta: ChatDelta {
                                        role: None,
                                        content: None,
                                    },
                                    finish_reason: Some("stop".to_string()),
                                }],
                            };
                            yield Ok(Event::default().data(serde_json::to_string(&final_chunk).unwrap_or_default()));
                            yield Ok(Event::default().data("[DONE]"));
                            break;
                        }
                        _ => {}
                    }
                }
                Ok(None) => {
                    yield Ok(Event::default().data("[DONE]"));
                    break;
                }
                Err(e) => {
                    let err = ErrorResponse::new(e.to_string(), "stream_error");
                    yield Ok(Event::default().data(serde_json::to_string(&err).unwrap_or_default()));
                    break;
                }
            }
        }
    };

    Ok(Sse::new(sse_stream).keep_alive(KeepAlive::default()))
}

fn convert_messages(messages: &[ChatMessage]) -> (Option<Vec<SystemBlock>>, Vec<InputMessage>) {
    let mut system_blocks: Vec<SystemBlock> = Vec::new();
    let mut input_messages: Vec<InputMessage> = Vec::new();

    for msg in messages {
        let text = match &msg.content {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => String::new(),
        };

        match msg.role.as_str() {
            "system" => {
                system_blocks.extend(SystemBlock::from_plain(&text));
            }
            "assistant" => {
                input_messages.push(InputMessage {
                    role: "assistant".to_string(),
                    content: vec![InputContentBlock::Text {
                        text,
                        cache_control: None,
                    }],
                });
            }
            "tool" => {
                if let Some(tool_call_id) = msg
                    .content
                    .as_ref()
                    .and_then(|v| v.get("tool_call_id"))
                    .and_then(|v| v.as_str())
                {
                    input_messages.push(InputMessage {
                        role: "user".to_string(),
                        content: vec![InputContentBlock::ToolResult {
                            tool_use_id: tool_call_id.to_string(),
                            content: vec![aineer_api::ToolResultContentBlock::Text { text }],
                            is_error: false,
                            cache_control: None,
                        }],
                    });
                } else {
                    input_messages.push(InputMessage {
                        role: "user".to_string(),
                        content: vec![InputContentBlock::Text {
                            text,
                            cache_control: None,
                        }],
                    });
                }
            }
            _ => {
                input_messages.push(InputMessage {
                    role: "user".to_string(),
                    content: vec![InputContentBlock::Text {
                        text,
                        cache_control: None,
                    }],
                });
            }
        }
    }

    let system = if system_blocks.is_empty() {
        None
    } else {
        Some(system_blocks)
    };

    (system, input_messages)
}

fn convert_tools(tools: &[serde_json::Value]) -> Option<Vec<aineer_api::ToolDefinition>> {
    let mut result = Vec::new();
    for tool in tools {
        let func = tool.get("function")?;
        let name = func.get("name")?.as_str()?.to_string();
        let description = func
            .get("description")
            .and_then(|d| d.as_str())
            .map(String::from);
        let input_schema = func
            .get("parameters")
            .cloned()
            .unwrap_or(serde_json::json!({"type": "object"}));
        result.push(aineer_api::ToolDefinition {
            name,
            description,
            input_schema,
            cache_control: None,
        });
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn convert_tool_choice(tc: &serde_json::Value) -> Option<aineer_api::ToolChoice> {
    match tc {
        serde_json::Value::String(s) => match s.as_str() {
            "auto" => Some(aineer_api::ToolChoice::Auto),
            "any" | "required" => Some(aineer_api::ToolChoice::Any),
            "none" => None,
            _ => Some(aineer_api::ToolChoice::Auto),
        },
        serde_json::Value::Object(obj) => {
            if let Some(name) = obj
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
            {
                Some(aineer_api::ToolChoice::Tool {
                    name: name.to_string(),
                })
            } else {
                Some(aineer_api::ToolChoice::Auto)
            }
        }
        _ => None,
    }
}

fn map_finish_reason(reason: &str) -> String {
    match reason {
        "end_turn" | "stop" => "stop".to_string(),
        "max_tokens" => "length".to_string(),
        "tool_use" => "tool_calls".to_string(),
        other => other.to_string(),
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
