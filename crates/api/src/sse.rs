use crate::error::ApiError;
use crate::types::StreamEvent;
use serde_json::Value;

/// Top-level `type` values we deserialize today; anything else is ignored for forward compatibility.
const KNOWN_STREAM_EVENT_TYPES: &[&str] = &[
    "message_start",
    "message_delta",
    "content_block_start",
    "content_block_delta",
    "content_block_stop",
    "message_stop",
];

#[derive(Debug, Default)]
pub struct SseParser {
    buffer: Vec<u8>,
}

impl SseParser {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    const MAX_BUFFER_SIZE: usize = 16 * 1024 * 1024;

    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<StreamEvent>, ApiError> {
        if self.buffer.len() + chunk.len() > Self::MAX_BUFFER_SIZE {
            self.buffer.clear();
            return Err(ApiError::ResponsePayloadTooLarge {
                limit: Self::MAX_BUFFER_SIZE,
            });
        }
        self.buffer.extend_from_slice(chunk);
        let mut events = Vec::new();

        while let Some(frame) = self.next_frame() {
            if let Some(event) = parse_frame(&frame)? {
                events.push(event);
            }
        }

        Ok(events)
    }

    pub fn finish(&mut self) -> Result<Vec<StreamEvent>, ApiError> {
        if self.buffer.is_empty() {
            return Ok(Vec::new());
        }

        let trailing = std::mem::take(&mut self.buffer);
        match parse_frame(&String::from_utf8_lossy(&trailing))? {
            Some(event) => Ok(vec![event]),
            None => Ok(Vec::new()),
        }
    }

    fn next_frame(&mut self) -> Option<String> {
        let separator = self
            .buffer
            .windows(2)
            .position(|window| window == b"\n\n")
            .map(|position| (position, 2))
            .or_else(|| {
                self.buffer
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                    .map(|position| (position, 4))
            })?;

        let (position, separator_len) = separator;
        let frame = self
            .buffer
            .drain(..position + separator_len)
            .collect::<Vec<_>>();
        let frame_len = frame.len().saturating_sub(separator_len);
        Some(String::from_utf8_lossy(&frame[..frame_len]).into_owned())
    }
}

pub fn parse_frame(frame: &str) -> Result<Option<StreamEvent>, ApiError> {
    let trimmed = frame.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let mut data_lines = Vec::new();
    let mut event_name: Option<&str> = None;

    for line in trimmed.lines() {
        if line.starts_with(':') {
            continue;
        }
        if let Some(name) = line.strip_prefix("event:") {
            event_name = Some(name.trim());
            continue;
        }
        if let Some(data) = line.strip_prefix("data:") {
            data_lines.push(data.trim_start());
        }
    }

    if matches!(event_name, Some("ping")) {
        return Ok(None);
    }

    if data_lines.is_empty() {
        return Ok(None);
    }

    let payload = data_lines.join("\n");
    if payload == "[DONE]" {
        return Ok(None);
    }

    let value: Value = serde_json::from_str(&payload).map_err(ApiError::from)?;

    // Anthropic Messages SSE: `type: "error"` must surface as an error, not be dropped as unknown.
    if value.get("type").and_then(Value::as_str) == Some("error") {
        let err_obj = value.get("error");
        let error_type = err_obj
            .and_then(|e| e.get("type"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let message = err_obj
            .and_then(|e| e.get("message"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| "unknown streaming error".to_string());
        return Err(ApiError::StreamApplicationError {
            error_type,
            message,
        });
    }

    let unknown_top_level = matches!(
        value.get("type").and_then(Value::as_str),
        Some(t) if !KNOWN_STREAM_EVENT_TYPES.contains(&t)
    );
    match serde_json::from_value::<StreamEvent>(value) {
        Ok(event) => Ok(Some(event)),
        Err(err) => {
            // Forward-compatible: ignore only unknown *top-level* event kinds. A known `type` with
            // a malformed payload must still surface as `Json` so we do not drop real bugs.
            if unknown_top_level {
                return Ok(None);
            }
            Err(ApiError::from(err))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_frame, SseParser};
    use crate::error::ApiError;
    use crate::types::{ContentBlockDelta, MessageDelta, OutputContentBlock, StreamEvent, Usage};

    #[test]
    fn parses_single_frame() {
        let frame = concat!(
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"Hi\"}}\n\n"
        );

        let event = parse_frame(frame).expect("frame should parse");
        assert_eq!(
            event,
            Some(StreamEvent::ContentBlockStart(
                crate::types::ContentBlockStartEvent {
                    index: 0,
                    content_block: OutputContentBlock::Text {
                        text: "Hi".to_string(),
                    },
                },
            ))
        );
    }

    #[test]
    fn parses_chunked_stream() {
        let mut parser = SseParser::new();
        let first = b"event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hel";
        let second = b"lo\"}}\n\n";

        assert!(parser
            .push(first)
            .expect("first chunk should buffer")
            .is_empty());
        let events = parser.push(second).expect("second chunk should parse");

        assert_eq!(
            events,
            vec![StreamEvent::ContentBlockDelta(
                crate::types::ContentBlockDeltaEvent {
                    index: 0,
                    delta: ContentBlockDelta::TextDelta {
                        text: "Hello".to_string(),
                    },
                }
            )]
        );
    }

    #[test]
    fn ignores_ping_and_done() {
        let mut parser = SseParser::new();
        let payload = concat!(
            ": keepalive\n",
            "event: ping\n",
            "data: {\"type\":\"ping\"}\n\n",
            "event: message_delta\n",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\",\"stop_sequence\":null},\"usage\":{\"input_tokens\":1,\"output_tokens\":2}}\n\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n\n",
            "data: [DONE]\n\n"
        );

        let events = parser
            .push(payload.as_bytes())
            .expect("parser should succeed");
        assert_eq!(
            events,
            vec![
                StreamEvent::MessageDelta(crate::types::MessageDeltaEvent {
                    delta: MessageDelta {
                        stop_reason: Some("tool_use".to_string()),
                        stop_sequence: None,
                    },
                    usage: Usage {
                        input_tokens: 1,
                        cache_creation_input_tokens: 0,
                        cache_read_input_tokens: 0,
                        output_tokens: 2,
                    },
                }),
                StreamEvent::MessageStop(crate::types::MessageStopEvent {}),
            ]
        );
    }

    #[test]
    fn ignores_data_less_event_frames() {
        let frame = "event: ping\n\n";
        let event = parse_frame(frame).expect("frame without data should be ignored");
        assert_eq!(event, None);
    }

    #[test]
    fn parses_split_json_across_data_lines() {
        let frame = concat!(
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\n",
            "data: \"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\n"
        );

        let event = parse_frame(frame).expect("frame should parse");
        assert_eq!(
            event,
            Some(StreamEvent::ContentBlockDelta(
                crate::types::ContentBlockDeltaEvent {
                    index: 0,
                    delta: ContentBlockDelta::TextDelta {
                        text: "Hello".to_string(),
                    },
                }
            ))
        );
    }

    #[test]
    fn parses_thinking_content_block_start() {
        let frame = concat!(
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\",\"signature\":null}}\n\n"
        );

        let event = parse_frame(frame).expect("frame should parse");
        assert_eq!(
            event,
            Some(StreamEvent::ContentBlockStart(
                crate::types::ContentBlockStartEvent {
                    index: 0,
                    content_block: OutputContentBlock::Thinking {
                        thinking: String::new(),
                        signature: None,
                    },
                },
            ))
        );
    }

    #[test]
    fn parses_thinking_related_deltas() {
        let thinking = concat!(
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"step 1\"}}\n\n"
        );
        let signature = concat!(
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"signature_delta\",\"signature\":\"sig_123\"}}\n\n"
        );

        let thinking_event = parse_frame(thinking).expect("thinking delta should parse");
        let signature_event = parse_frame(signature).expect("signature delta should parse");

        assert_eq!(
            thinking_event,
            Some(StreamEvent::ContentBlockDelta(
                crate::types::ContentBlockDeltaEvent {
                    index: 0,
                    delta: ContentBlockDelta::ThinkingDelta {
                        thinking: "step 1".to_string(),
                    },
                }
            ))
        );
        assert_eq!(
            signature_event,
            Some(StreamEvent::ContentBlockDelta(
                crate::types::ContentBlockDeltaEvent {
                    index: 0,
                    delta: ContentBlockDelta::SignatureDelta {
                        signature: "sig_123".to_string(),
                    },
                }
            ))
        );
    }

    #[test]
    fn rejects_oversized_buffer() {
        let mut parser = SseParser::new();
        let big_chunk = vec![b'x'; SseParser::MAX_BUFFER_SIZE + 1];
        let err = parser.push(&big_chunk).unwrap_err();
        assert!(err.to_string().contains("limit"));
    }

    #[test]
    fn stream_error_frame_returns_err() {
        let frame = concat!(
            "event: error\n",
            "data: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"try again\"}}\n\n"
        );
        let err = parse_frame(frame).unwrap_err();
        assert!(err.is_retryable());
        match &err {
            ApiError::StreamApplicationError {
                error_type,
                message,
            } => {
                assert_eq!(error_type.as_deref(), Some("overloaded_error"));
                assert_eq!(message, "try again");
            }
            other => panic!("expected StreamApplicationError, got {other:?}"),
        }
    }

    #[test]
    fn unknown_event_type_is_skipped() {
        let frame = concat!(
            "event: future_event\n",
            "data: {\"type\":\"hypothetical_future_event\",\"index\":0}\n\n"
        );
        assert_eq!(parse_frame(frame).expect("skip unknown"), None);
    }
}
