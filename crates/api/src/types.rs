use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub use aineer_protocol::prompt_types::{
    BlockKind, CacheControl, CacheScope, CacheType, SystemBlock, ThinkingConfig, ThinkingMode,
};

/// Gemini cached content reference for context caching.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeminiCachedContent {
    /// The name/ID of the cached content resource (e.g. "cachedContents/abc123")
    pub name: String,
    /// When this cache expires
    #[serde(skip_serializing_if = "Option::is_none", alias = "expireTime")]
    pub expire_time: Option<String>,
}

/// Stable fingerprint for Gemini context cache lookup (SHA-256, first 8 bytes).
#[must_use]
pub fn gemini_cache_key_hash(
    system: Option<&[SystemBlock]>,
    tools: Option<&[ToolDefinition]>,
) -> u64 {
    let mut hasher = Sha256::new();
    if let Some(blocks) = system.filter(|b| !b.is_empty()) {
        for block in blocks {
            hasher.update(block.text.as_bytes());
            hasher.update([0u8]);
        }
    }
    if let Some(tools) = tools.filter(|t| !t.is_empty()) {
        if let Ok(encoded) = serde_json::to_string(tools) {
            hasher.update(encoded.as_bytes());
        }
    }
    let digest = hasher.finalize();
    u64::from_be_bytes(digest[..8].try_into().expect("8 bytes"))
}

// ── Message request ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<InputMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Vec<SystemBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    /// When set, OpenAI-compat requests include Google's `cachedContent` and omit duplicated system/tools.
    #[serde(skip_serializing_if = "Option::is_none", rename = "cachedContent")]
    pub gemini_cached_content: Option<String>,
}

impl MessageRequest {
    #[must_use]
    pub fn with_streaming(mut self) -> Self {
        self.stream = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputMessage {
    pub role: String,
    pub content: Vec<InputContentBlock>,
}

impl InputMessage {
    #[must_use]
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: vec![InputContentBlock::Text {
                text: text.into(),
                cache_control: None,
            }],
        }
    }

    #[must_use]
    pub fn user_tool_result(
        tool_use_id: impl Into<String>,
        content: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self {
            role: "user".to_string(),
            content: vec![InputContentBlock::ToolResult {
                tool_use_id: tool_use_id.into(),
                content: vec![ToolResultContentBlock::Text {
                    text: content.into(),
                }],
                is_error,
                cache_control: None,
            }],
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputContentBlock {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
    Image {
        source: ImageSource,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: Vec<ToolResultContentBlock>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        is_error: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
}

/// Base64-encoded image payload matching the Anthropic Messages API format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolResultContentBlock {
    Text { text: String },
    Json { value: Value },
    Image { source: ImageSource },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    Any,
    Tool { name: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub role: String,
    pub content: Vec<OutputContentBlock>,
    pub model: String,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub stop_sequence: Option<String>,
    pub usage: Usage,
    #[serde(default)]
    pub request_id: Option<String>,
}

impl MessageResponse {
    #[must_use]
    pub fn total_tokens(&self) -> u32 {
        self.usage.total_tokens()
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    Thinking {
        #[serde(default)]
        thinking: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
    RedactedThinking {
        data: Value,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    #[serde(default)]
    pub cache_read_input_tokens: u32,
    pub output_tokens: u32,
}

impl Usage {
    #[must_use]
    pub const fn total_tokens(&self) -> u32 {
        self.input_tokens
            + self.output_tokens
            + self.cache_creation_input_tokens
            + self.cache_read_input_tokens
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageStartEvent {
    pub message: MessageResponse,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageDeltaEvent {
    pub delta: MessageDelta,
    pub usage: Usage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDelta {
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub stop_sequence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentBlockStartEvent {
    pub index: u32,
    pub content_block: OutputContentBlock,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentBlockDeltaEvent {
    pub index: u32,
    pub delta: ContentBlockDelta,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
    ThinkingDelta { thinking: String },
    SignatureDelta { signature: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentBlockStopEvent {
    pub index: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageStopEvent {}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    MessageStart(MessageStartEvent),
    MessageDelta(MessageDeltaEvent),
    ContentBlockStart(ContentBlockStartEvent),
    ContentBlockDelta(ContentBlockDeltaEvent),
    ContentBlockStop(ContentBlockStopEvent),
    MessageStop(MessageStopEvent),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn message_request_serializes_with_streaming_and_tools() {
        let request = MessageRequest {
            model: "claude-opus-4-6".to_string(),
            max_tokens: 4096,
            messages: vec![InputMessage::user_text("hello")],
            system: Some(SystemBlock::from_plain("you are helpful")),
            tools: Some(vec![ToolDefinition {
                name: "read_file".to_string(),
                description: Some("Read a file".to_string()),
                input_schema: json!({"type": "object", "properties": {"path": {"type": "string"}}}),
                cache_control: None,
            }]),
            tool_choice: Some(ToolChoice::Auto),
            stream: false,
            thinking: None,
            gemini_cached_content: None,
        };
        let json = serde_json::to_string(&request).expect("serialize");
        let deserialized: MessageRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, request);
        assert!(!json.contains("\"stream\""));
        assert!(!json.contains("\"thinking\""));

        let streaming = request.with_streaming();
        let json = serde_json::to_string(&streaming).expect("serialize streaming");
        assert!(json.contains("\"stream\":true"));
    }

    #[test]
    fn system_block_serializes_with_cache_control() {
        let block = SystemBlock::cached("static section", CacheControl::global_1h());
        let json = serde_json::to_value(&block).expect("serialize");
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "static section");
        assert_eq!(json["cache_control"]["type"], "ephemeral");
        assert_eq!(json["cache_control"]["ttl"], "1h");
        assert_eq!(json["cache_control"]["scope"], "global");
    }

    #[test]
    fn thinking_config_round_trips() {
        let enabled = ThinkingConfig::enabled(10000);
        let json = serde_json::to_value(&enabled).expect("serialize");
        assert_eq!(json["type"], "enabled");
        assert_eq!(json["budget_tokens"], 10000);
        let rt: ThinkingConfig = serde_json::from_value(json).expect("deserialize");
        assert_eq!(rt, enabled);

        let disabled = ThinkingConfig::disabled();
        let json = serde_json::to_value(&disabled).expect("serialize");
        assert_eq!(json["type"], "disabled");
        assert!(json.get("budget_tokens").is_none());
    }

    #[test]
    fn input_content_block_round_trips_all_variants() {
        let text = InputContentBlock::Text {
            text: "hello".to_string(),
            cache_control: None,
        };
        let image = InputContentBlock::Image {
            source: ImageSource {
                source_type: "base64".to_string(),
                media_type: "image/png".to_string(),
                data: "iVBOR...".to_string(),
            },
        };
        let tool_use = InputContentBlock::ToolUse {
            id: "t1".to_string(),
            name: "bash".to_string(),
            input: json!({"command": "ls"}),
        };
        let tool_result = InputContentBlock::ToolResult {
            tool_use_id: "t1".to_string(),
            content: vec![ToolResultContentBlock::Text {
                text: "output".to_string(),
            }],
            is_error: false,
            cache_control: None,
        };
        for block in [text, image, tool_use, tool_result] {
            let json = serde_json::to_value(&block).expect("serialize");
            let deserialized: InputContentBlock =
                serde_json::from_value(json).expect("deserialize");
            assert_eq!(deserialized, block);
        }
    }

    #[test]
    fn image_source_serializes_to_anthropic_format() {
        let block = InputContentBlock::Image {
            source: ImageSource {
                source_type: "base64".to_string(),
                media_type: "image/jpeg".to_string(),
                data: "abc123".to_string(),
            },
        };
        let json = serde_json::to_value(&block).expect("serialize");
        assert_eq!(json["type"], "image");
        assert_eq!(json["source"]["type"], "base64");
        assert_eq!(json["source"]["media_type"], "image/jpeg");
        assert_eq!(json["source"]["data"], "abc123");
    }

    #[test]
    fn tool_result_image_variant_round_trips() {
        let block = ToolResultContentBlock::Image {
            source: ImageSource {
                source_type: "base64".to_string(),
                media_type: "image/png".to_string(),
                data: "data==".to_string(),
            },
        };
        let json = serde_json::to_value(&block).expect("serialize");
        let deserialized: ToolResultContentBlock =
            serde_json::from_value(json).expect("deserialize");
        assert_eq!(deserialized, block);
    }

    #[test]
    fn message_response_deserializes_with_defaults() {
        let json = json!({
            "id": "msg-1",
            "type": "message",
            "model": "claude-opus-4-6",
            "role": "assistant",
            "content": [{"type": "text", "text": "hi"}],
            "usage": {"input_tokens": 10, "output_tokens": 5}
        });
        let response: MessageResponse = serde_json::from_value(json).expect("deserialize");
        assert_eq!(response.id, "msg-1");
        assert_eq!(response.role, "assistant");
        assert_eq!(response.usage.total_tokens(), 15);
    }

    #[test]
    fn output_content_block_round_trips_including_thinking() {
        let blocks = vec![
            OutputContentBlock::Text {
                text: "hello".to_string(),
            },
            OutputContentBlock::ToolUse {
                id: "t1".to_string(),
                name: "bash".to_string(),
                input: json!({"command": "ls"}),
            },
            OutputContentBlock::Thinking {
                thinking: "hmm".to_string(),
                signature: Some("sig".to_string()),
            },
        ];
        for block in blocks {
            let json = serde_json::to_value(&block).expect("serialize");
            let deserialized: OutputContentBlock =
                serde_json::from_value(json).expect("deserialize");
            assert_eq!(deserialized, block);
        }
    }

    #[test]
    fn tool_choice_variants_serialize_with_type_tag() {
        let auto_json = serde_json::to_value(ToolChoice::Auto).expect("auto");
        assert_eq!(auto_json, json!({"type": "auto"}));
        let any_json = serde_json::to_value(ToolChoice::Any).expect("any");
        assert_eq!(any_json, json!({"type": "any"}));
        let tool_json = serde_json::to_value(ToolChoice::Tool {
            name: "bash".to_string(),
        })
        .expect("tool");
        assert_eq!(tool_json, json!({"type": "tool", "name": "bash"}));
    }

    #[test]
    fn stream_event_deserializes_all_variants() {
        let msg_start = json!({
            "type": "message_start",
            "message": {
                "id": "msg-1", "type": "message", "model": "m",
                "role": "assistant", "content": [],
                "usage": {"input_tokens": 0, "output_tokens": 0}
            }
        });
        let parsed: StreamEvent = serde_json::from_value(msg_start).expect("message_start");
        assert!(matches!(parsed, StreamEvent::MessageStart(_)));

        let delta = json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {"type": "text_delta", "text": "hi"}
        });
        let parsed: StreamEvent = serde_json::from_value(delta).expect("content_block_delta");
        assert!(matches!(parsed, StreamEvent::ContentBlockDelta(_)));
    }

    #[test]
    fn usage_computes_total_tokens() {
        let usage = Usage {
            input_tokens: 100,
            output_tokens: 50,
            cache_read_input_tokens: 20,
            cache_creation_input_tokens: 10,
        };
        assert_eq!(usage.total_tokens(), 180);
    }

    #[test]
    fn gemini_cached_content_round_trips_json() {
        let v = GeminiCachedContent {
            name: "cachedContents/abc".to_string(),
            expire_time: Some("2025-01-01T00:00:00Z".to_string()),
        };
        let json = serde_json::to_value(&v).expect("serialize");
        assert_eq!(json["name"], "cachedContents/abc");
        assert_eq!(json["expire_time"], "2025-01-01T00:00:00Z");
        let back: GeminiCachedContent = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn gemini_cached_content_deserializes_google_expire_time_field() {
        let json = json!({
            "name": "cachedContents/abc",
            "expireTime": "2025-01-01T00:00:00Z"
        });
        let v: GeminiCachedContent = serde_json::from_value(json).expect("deserialize");
        assert_eq!(v.expire_time, Some("2025-01-01T00:00:00Z".to_string()));
    }

    #[test]
    fn gemini_cache_key_hash_stable_for_same_inputs() {
        let sys = SystemBlock::from_plain("sys");
        let tools = vec![ToolDefinition {
            name: "t".to_string(),
            description: None,
            input_schema: json!({}),
            cache_control: None,
        }];
        let a = gemini_cache_key_hash(Some(&sys), Some(&tools));
        let b = gemini_cache_key_hash(Some(&sys), Some(&tools));
        assert_eq!(a, b);
        assert_ne!(
            a,
            gemini_cache_key_hash(Some(&SystemBlock::from_plain("other")), Some(&tools))
        );
    }

    #[test]
    fn content_block_delta_all_variants_round_trip() {
        let deltas = vec![
            ContentBlockDelta::TextDelta {
                text: "hi".to_string(),
            },
            ContentBlockDelta::InputJsonDelta {
                partial_json: "{\"a\"".to_string(),
            },
            ContentBlockDelta::ThinkingDelta {
                thinking: "hmm".to_string(),
            },
            ContentBlockDelta::SignatureDelta {
                signature: "sig".to_string(),
            },
        ];
        for delta in deltas {
            let json = serde_json::to_value(&delta).expect("serialize");
            let deserialized: ContentBlockDelta =
                serde_json::from_value(json).expect("deserialize");
            assert_eq!(deserialized, delta);
        }
    }
}
