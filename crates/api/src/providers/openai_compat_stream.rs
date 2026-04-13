use std::collections::{BTreeMap, VecDeque};

use serde_json::json;

use crate::types::{
    ContentBlockDelta, ContentBlockDeltaEvent, ContentBlockStartEvent, ContentBlockStopEvent,
    MessageDelta, MessageDeltaEvent, MessageResponse, MessageStopEvent, OutputContentBlock,
    StreamEvent, Usage,
};

use super::openai_compat_sse::{ChatCompletionChunk, DeltaToolCall};

#[derive(Debug)]
pub(super) struct StreamState {
    model: String,
    message_started: bool,
    /// Next content block index (thinking then text, or text-only at 0).
    next_block_index: u32,
    thinking_open: bool,
    thinking_index: u32,
    text_open: bool,
    text_index: u32,
    finished: bool,
    stop_reason: Option<String>,
    usage: Option<Usage>,
    tool_calls: BTreeMap<u32, ToolCallState>,
}

impl StreamState {
    pub fn new(model: String) -> Self {
        Self {
            model,
            message_started: false,
            next_block_index: 0,
            thinking_open: false,
            thinking_index: 0,
            text_open: false,
            text_index: 0,
            finished: false,
            stop_reason: None,
            usage: None,
            tool_calls: BTreeMap::new(),
        }
    }

    fn close_thinking_if_open(&mut self, events: &mut Vec<StreamEvent>) {
        if self.thinking_open {
            events.push(StreamEvent::ContentBlockStop(ContentBlockStopEvent {
                index: self.thinking_index,
            }));
            self.thinking_open = false;
        }
    }

    fn close_text_if_open(&mut self, events: &mut Vec<StreamEvent>) {
        if self.text_open {
            events.push(StreamEvent::ContentBlockStop(ContentBlockStopEvent {
                index: self.text_index,
            }));
            self.text_open = false;
        }
    }

    fn ensure_thinking_block(&mut self, events: &mut Vec<StreamEvent>) -> u32 {
        if !self.thinking_open {
            self.thinking_index = self.next_block_index;
            self.next_block_index += 1;
            self.thinking_open = true;
            events.push(StreamEvent::ContentBlockStart(ContentBlockStartEvent {
                index: self.thinking_index,
                content_block: OutputContentBlock::Thinking {
                    thinking: String::new(),
                    signature: None,
                },
            }));
        }
        self.thinking_index
    }

    fn ensure_text_block(&mut self, events: &mut Vec<StreamEvent>) -> u32 {
        if !self.text_open {
            self.text_index = self.next_block_index;
            self.next_block_index += 1;
            self.text_open = true;
            events.push(StreamEvent::ContentBlockStart(ContentBlockStartEvent {
                index: self.text_index,
                content_block: OutputContentBlock::Text {
                    text: String::new(),
                },
            }));
        }
        self.text_index
    }

    pub fn ingest_chunk(&mut self, chunk: ChatCompletionChunk) -> Vec<StreamEvent> {
        let mut events = Vec::new();
        if !self.message_started {
            self.message_started = true;
            events.push(StreamEvent::MessageStart(crate::types::MessageStartEvent {
                message: MessageResponse {
                    id: chunk.id.clone(),
                    kind: "message".to_string(),
                    role: "assistant".to_string(),
                    content: Vec::new(),
                    model: chunk.model.clone().unwrap_or_else(|| self.model.clone()),
                    stop_reason: None,
                    stop_sequence: None,
                    usage: Usage {
                        input_tokens: 0,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                        output_tokens: 0,
                    },
                    request_id: None,
                },
            }));
        }

        if let Some(usage) = chunk.usage {
            self.usage = Some(Usage {
                input_tokens: usage.prompt_tokens,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
                output_tokens: usage.completion_tokens,
            });
        }

        for choice in chunk.choices {
            let delta = &choice.delta;
            if let Some(reasoning) = delta.stream_reasoning_fragment() {
                let idx = self.ensure_thinking_block(&mut events);
                events.push(StreamEvent::ContentBlockDelta(ContentBlockDeltaEvent {
                    index: idx,
                    delta: ContentBlockDelta::ThinkingDelta {
                        thinking: reasoning,
                    },
                }));
            }
            if let Some(content) = delta.stream_content_fragment() {
                self.close_thinking_if_open(&mut events);
                let idx = self.ensure_text_block(&mut events);
                events.push(StreamEvent::ContentBlockDelta(ContentBlockDeltaEvent {
                    index: idx,
                    delta: ContentBlockDelta::TextDelta { text: content },
                }));
            }

            for tool_call in choice.delta.tool_calls {
                let key = tool_call.index;
                {
                    let state = self.tool_calls.entry(key).or_default();
                    state.apply(tool_call);
                }
                if self
                    .tool_calls
                    .get(&key)
                    .is_some_and(|s| s.resolved_block_index.is_none())
                {
                    self.close_thinking_if_open(&mut events);
                    self.close_text_if_open(&mut events);
                    let idx = self.next_block_index;
                    self.next_block_index += 1;
                    self.tool_calls
                        .get_mut(&key)
                        .expect("entry above")
                        .resolved_block_index = Some(idx);
                }
                let state = self.tool_calls.get_mut(&key).expect("just inserted");
                let block_index = state.resolved_block_index.expect("set above");
                if !state.started {
                    if let Some(start_event) = state.start_event(block_index) {
                        state.started = true;
                        events.push(StreamEvent::ContentBlockStart(start_event));
                    } else {
                        continue;
                    }
                }
                if let Some(delta_event) = state.delta_event(block_index) {
                    events.push(StreamEvent::ContentBlockDelta(delta_event));
                }
                if choice.finish_reason.as_deref() == Some("tool_calls") && !state.stopped {
                    state.stopped = true;
                    events.push(StreamEvent::ContentBlockStop(ContentBlockStopEvent {
                        index: block_index,
                    }));
                }
            }

            if let Some(finish_reason) = choice.finish_reason {
                self.stop_reason = Some(super::normalize_finish_reason(&finish_reason));
                if finish_reason == "tool_calls" {
                    for state in self.tool_calls.values_mut() {
                        if state.started && !state.stopped {
                            state.stopped = true;
                            let idx = state.resolved_block_index.expect("tool block index");
                            events.push(StreamEvent::ContentBlockStop(ContentBlockStopEvent {
                                index: idx,
                            }));
                        }
                    }
                }
            }
        }

        events
    }

    pub fn finish(&mut self) -> Vec<StreamEvent> {
        if self.finished {
            return Vec::new();
        }
        self.finished = true;

        let mut events = Vec::new();
        if self.thinking_open {
            events.push(StreamEvent::ContentBlockStop(ContentBlockStopEvent {
                index: self.thinking_index,
            }));
            self.thinking_open = false;
        }
        if self.text_open {
            events.push(StreamEvent::ContentBlockStop(ContentBlockStopEvent {
                index: self.text_index,
            }));
            self.text_open = false;
        }

        for state in self.tool_calls.values_mut() {
            if !state.started {
                if state.resolved_block_index.is_none() {
                    state.resolved_block_index = Some(self.next_block_index);
                    self.next_block_index += 1;
                }
                let idx = state.resolved_block_index.expect("tool index");
                if let Some(start_event) = state.start_event(idx) {
                    state.started = true;
                    events.push(StreamEvent::ContentBlockStart(start_event));
                    if let Some(delta_event) = state.delta_event(idx) {
                        events.push(StreamEvent::ContentBlockDelta(delta_event));
                    }
                }
            }
            if state.started && !state.stopped {
                state.stopped = true;
                let idx = state.resolved_block_index.expect("tool index");
                events.push(StreamEvent::ContentBlockStop(ContentBlockStopEvent {
                    index: idx,
                }));
            }
        }

        if self.message_started {
            events.push(StreamEvent::MessageDelta(MessageDeltaEvent {
                delta: MessageDelta {
                    stop_reason: Some(
                        self.stop_reason
                            .clone()
                            .unwrap_or_else(|| "end_turn".to_string()),
                    ),
                    stop_sequence: None,
                },
                usage: self.usage.clone().unwrap_or(Usage {
                    input_tokens: 0,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                    output_tokens: 0,
                }),
            }));
            events.push(StreamEvent::MessageStop(MessageStopEvent {}));
        }
        events
    }
}

#[derive(Debug)]
pub struct MessageStream {
    pub(super) request_id: Option<String>,
    pub(super) response: reqwest::Response,
    pub(super) parser: super::openai_compat_sse::OpenAiSseParser,
    pub(super) pending: VecDeque<StreamEvent>,
    pub(super) done: bool,
    pub(super) state: StreamState,
}

impl MessageStream {
    #[must_use]
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    pub async fn next_event(&mut self) -> Result<Option<StreamEvent>, crate::error::ApiError> {
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Ok(Some(event));
            }

            if self.done {
                self.pending.extend(self.state.finish());
                if let Some(event) = self.pending.pop_front() {
                    return Ok(Some(event));
                }
                return Ok(None);
            }

            match self.response.chunk().await? {
                Some(chunk) => {
                    for parsed in self.parser.push(&chunk)? {
                        self.pending.extend(self.state.ingest_chunk(parsed));
                    }
                }
                None => {
                    self.done = true;
                }
            }
        }
    }
}

#[derive(Debug, Default)]
struct ToolCallState {
    openai_index: u32,
    id: Option<String>,
    name: Option<String>,
    arguments: String,
    emitted_len: usize,
    started: bool,
    stopped: bool,
    /// Allocated after any thinking/text blocks so indices stay unique.
    resolved_block_index: Option<u32>,
}

impl ToolCallState {
    fn apply(&mut self, tool_call: DeltaToolCall) {
        self.openai_index = tool_call.index;
        if let Some(id) = tool_call.id {
            self.id = Some(id);
        }
        if let Some(name) = tool_call.function.name {
            self.name = Some(name);
        }
        if let Some(arguments) = tool_call.function.arguments {
            self.arguments.push_str(&arguments);
        }
    }

    fn start_event(&self, block_index: u32) -> Option<ContentBlockStartEvent> {
        let name = self.name.clone()?;
        let id = self
            .id
            .clone()
            .unwrap_or_else(|| format!("tool_call_{}", self.openai_index));
        Some(ContentBlockStartEvent {
            index: block_index,
            content_block: OutputContentBlock::ToolUse {
                id,
                name,
                input: json!({}),
            },
        })
    }

    fn delta_event(&mut self, block_index: u32) -> Option<ContentBlockDeltaEvent> {
        if self.emitted_len >= self.arguments.len() {
            return None;
        }
        let delta = self.arguments[self.emitted_len..].to_string();
        self.emitted_len = self.arguments.len();
        Some(ContentBlockDeltaEvent {
            index: block_index,
            delta: ContentBlockDelta::InputJsonDelta {
                partial_json: delta,
            },
        })
    }
}
