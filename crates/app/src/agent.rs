use std::sync::mpsc;

use api::{
    ContentBlockDelta, InputContentBlock, InputMessage, MessageRequest, OutputContentBlock,
    ProviderClient, StreamEvent,
};
use tokio::sync::mpsc as tokio_mpsc;
use tools::GlobalToolRegistry;

const MAX_TOOL_ITERATIONS: usize = 20;

#[derive(Debug, Clone)]
pub enum ToolApproval {
    Allow,
    Deny,
    AllowAll,
}

pub enum AgentRequest {
    Chat {
        prompt: String,
        context: Vec<String>,
        response_tx: mpsc::Sender<AgentEvent>,
        approval_rx: tokio_mpsc::Receiver<ToolApproval>,
    },
}

#[derive(Debug, Clone)]
pub enum AgentEvent {
    TextDelta(String),
    ToolPending {
        tool_use_id: String,
        name: String,
        input: String,
    },
    ToolRunning {
        tool_use_id: String,
    },
    ToolResult {
        tool_use_id: String,
        #[allow(dead_code)]
        name: String,
        output: String,
        is_error: bool,
    },
    Done,
    Error(String),
}

pub struct AgentHandle {
    request_tx: mpsc::Sender<AgentRequest>,
}

impl AgentHandle {
    pub fn spawn(rt: &tokio::runtime::Runtime) -> Self {
        let (request_tx, request_rx) = mpsc::channel::<AgentRequest>();

        rt.spawn(async move {
            agent_worker(request_rx).await;
        });

        Self { request_tx }
    }

    /// Returns (event_receiver, approval_sender).
    pub fn send_chat(
        &self,
        prompt: String,
        context: Vec<String>,
    ) -> (mpsc::Receiver<AgentEvent>, tokio_mpsc::Sender<ToolApproval>) {
        let (response_tx, response_rx) = mpsc::channel();
        let (approval_tx, approval_rx) = tokio_mpsc::channel(8);
        let _ = self.request_tx.send(AgentRequest::Chat {
            prompt,
            context,
            response_tx,
            approval_rx,
        });
        (response_rx, approval_tx)
    }
}

async fn agent_worker(request_rx: mpsc::Receiver<AgentRequest>) {
    while let Ok(request) = request_rx.recv() {
        match request {
            AgentRequest::Chat {
                prompt,
                context,
                response_tx,
                mut approval_rx,
            } => {
                handle_chat(&prompt, &context, &response_tx, &mut approval_rx).await;
            }
        }
    }
}

fn build_tool_definitions() -> Vec<api::ToolDefinition> {
    let registry = GlobalToolRegistry::builtin();
    registry.definitions(None)
}

async fn handle_chat(
    prompt: &str,
    context: &[String],
    tx: &mpsc::Sender<AgentEvent>,
    approval_rx: &mut tokio_mpsc::Receiver<ToolApproval>,
) {
    let model = api::auto_detect_default_model()
        .unwrap_or("auto")
        .to_string();

    let client = match ProviderClient::from_model(&model) {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(AgentEvent::Error(format!(
                "No provider configured. Use `aineer --cli login` to set up API keys.\n\nDetails: {e}"
            )));
            let _ = tx.send(AgentEvent::Done);
            return;
        }
    };

    let mut messages = Vec::new();

    if !context.is_empty() {
        let ctx_text = context.join("\n---\n");
        messages.push(InputMessage {
            role: "user".to_string(),
            content: vec![InputContentBlock::Text {
                text: format!("Context from previous cards:\n{ctx_text}"),
                cache_control: None,
            }],
        });
        messages.push(InputMessage {
            role: "assistant".to_string(),
            content: vec![InputContentBlock::Text {
                text: "I've reviewed the context. How can I help?".to_string(),
                cache_control: None,
            }],
        });
    }

    messages.push(InputMessage {
        role: "user".to_string(),
        content: vec![InputContentBlock::Text {
            text: prompt.to_string(),
            cache_control: None,
        }],
    });

    let tool_defs = build_tool_definitions();
    let max_tokens = api::max_tokens_for_model(&model);
    let mut auto_approve = false;

    for _ in 0..MAX_TOOL_ITERATIONS {
        let request = MessageRequest {
            model: model.clone(),
            max_tokens,
            messages: messages.clone(),
            system: None,
            tools: if tool_defs.is_empty() {
                None
            } else {
                Some(tool_defs.clone())
            },
            tool_choice: None,
            stream: true,
            thinking: None,
            gemini_cached_content: None,
        };

        let mut pending_tool_calls: Vec<PendingToolCall> = Vec::new();
        let mut accumulated_text = String::new();

        match client.stream_message(&request).await {
            Ok(mut stream) => loop {
                match stream.next_event().await {
                    Ok(Some(event)) => match event {
                        StreamEvent::ContentBlockStart(start) => {
                            if let OutputContentBlock::ToolUse { id, name, .. } =
                                start.content_block
                            {
                                pending_tool_calls.push(PendingToolCall {
                                    id,
                                    name,
                                    input_json: String::new(),
                                });
                            }
                        }
                        StreamEvent::ContentBlockDelta(delta_event) => match delta_event.delta {
                            ContentBlockDelta::TextDelta { text } => {
                                accumulated_text.push_str(&text);
                                let _ = tx.send(AgentEvent::TextDelta(text));
                            }
                            ContentBlockDelta::InputJsonDelta { partial_json } => {
                                if let Some(tc) = pending_tool_calls.last_mut() {
                                    tc.input_json.push_str(&partial_json);
                                }
                            }
                            _ => {}
                        },
                        StreamEvent::MessageStop(_) => break,
                        _ => {}
                    },
                    Ok(None) => break,
                    Err(e) => {
                        let _ = tx.send(AgentEvent::Error(e.to_string()));
                        let _ = tx.send(AgentEvent::Done);
                        return;
                    }
                }
            },
            Err(e) => {
                let _ = tx.send(AgentEvent::Error(e.to_string()));
                let _ = tx.send(AgentEvent::Done);
                return;
            }
        }

        if pending_tool_calls.is_empty() {
            break;
        }

        // Build assistant message with text + tool_use blocks
        let mut assistant_content: Vec<InputContentBlock> = Vec::new();
        if !accumulated_text.is_empty() {
            assistant_content.push(InputContentBlock::Text {
                text: accumulated_text,
                cache_control: None,
            });
        }
        for tc in &pending_tool_calls {
            let input_val: serde_json::Value =
                serde_json::from_str(&tc.input_json).unwrap_or(serde_json::Value::Null);
            assistant_content.push(InputContentBlock::ToolUse {
                id: tc.id.clone(),
                name: tc.name.clone(),
                input: input_val,
            });
        }
        messages.push(InputMessage {
            role: "assistant".to_string(),
            content: assistant_content,
        });

        // Execute tools with approval gating
        let mut tool_results: Vec<InputContentBlock> = Vec::new();
        for tc in &pending_tool_calls {
            let _ = tx.send(AgentEvent::ToolPending {
                tool_use_id: tc.id.clone(),
                name: tc.name.clone(),
                input: tc.input_json.clone(),
            });

            let approved = if auto_approve {
                true
            } else {
                match approval_rx.recv().await {
                    Some(ToolApproval::Allow) => true,
                    Some(ToolApproval::AllowAll) => {
                        auto_approve = true;
                        true
                    }
                    Some(ToolApproval::Deny) => false,
                    None => false,
                }
            };

            if approved {
                let _ = tx.send(AgentEvent::ToolRunning {
                    tool_use_id: tc.id.clone(),
                });

                let input_val: serde_json::Value =
                    serde_json::from_str(&tc.input_json).unwrap_or(serde_json::Value::Null);

                let (result_text, is_error) = match tools::execute_tool(&tc.name, input_val) {
                    Ok(output) => (output.content, output.is_error),
                    Err(e) => (format!("Tool error: {e}"), true),
                };

                let _ = tx.send(AgentEvent::ToolResult {
                    tool_use_id: tc.id.clone(),
                    name: tc.name.clone(),
                    output: result_text.clone(),
                    is_error,
                });

                tool_results.push(InputContentBlock::ToolResult {
                    tool_use_id: tc.id.clone(),
                    content: vec![api::ToolResultContentBlock::Text { text: result_text }],
                    is_error,
                    cache_control: None,
                });
            } else {
                let denied_msg = format!("Tool '{}' was denied by user.", tc.name);
                let _ = tx.send(AgentEvent::ToolResult {
                    tool_use_id: tc.id.clone(),
                    name: tc.name.clone(),
                    output: denied_msg.clone(),
                    is_error: true,
                });

                tool_results.push(InputContentBlock::ToolResult {
                    tool_use_id: tc.id.clone(),
                    content: vec![api::ToolResultContentBlock::Text { text: denied_msg }],
                    is_error: true,
                    cache_control: None,
                });
            }
        }

        messages.push(InputMessage {
            role: "user".to_string(),
            content: tool_results,
        });
    }

    let _ = tx.send(AgentEvent::Done);
}

struct PendingToolCall {
    id: String,
    name: String,
    input_json: String,
}
