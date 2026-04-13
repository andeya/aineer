use super::openai_compat_sse::parse_sse_frame;
use super::{
    build_chat_completion_request, chat_completions_endpoint, normalize_finish_reason,
    openai_tool_choice, parse_tool_arguments, OpenAiCompatClient, OpenAiCompatConfig,
};
use crate::cache_strategy::is_gemini_model;
use crate::error::ApiError;
use crate::types::{
    InputContentBlock, InputMessage, MessageRequest, SystemBlock, ToolChoice, ToolDefinition,
    ToolResultContentBlock,
};
use serde_json::json;
use std::sync::{Mutex, OnceLock};

#[test]
fn request_translation_uses_openai_compatible_shape() {
    let payload = build_chat_completion_request(&MessageRequest {
        model: "grok-3".to_string(),
        max_tokens: 64,
        messages: vec![InputMessage {
            role: "user".to_string(),
            content: vec![
                InputContentBlock::Text {
                    text: "hello".to_string(),
                    cache_control: None,
                },
                InputContentBlock::ToolResult {
                    tool_use_id: "tool_1".to_string(),
                    content: vec![ToolResultContentBlock::Json {
                        value: json!({"ok": true}),
                    }],
                    is_error: false,
                    cache_control: None,
                },
            ],
        }],
        system: Some(SystemBlock::from_plain("be helpful")),
        tools: Some(vec![ToolDefinition {
            name: "weather".to_string(),
            description: Some("Get weather".to_string()),
            input_schema: json!({"type": "object"}),
            cache_control: None,
        }]),
        tool_choice: Some(ToolChoice::Auto),
        stream: false,
        thinking: None,
        gemini_cached_content: None,
    });

    assert_eq!(payload["messages"][0]["role"], json!("system"));
    assert_eq!(payload["messages"][1]["role"], json!("user"));
    assert_eq!(payload["messages"][2]["role"], json!("tool"));
    assert_eq!(payload["tools"][0]["type"], json!("function"));
    assert_eq!(payload["tool_choice"], json!("auto"));
}

#[test]
fn chat_completion_strips_custom_provider_prefix_for_upstream_model() {
    let payload = build_chat_completion_request(&MessageRequest {
        model: "acme-corp/qwen-plus-2025-07-28".to_string(),
        max_tokens: 64,
        messages: vec![InputMessage {
            role: "user".to_string(),
            content: vec![InputContentBlock::Text {
                text: "hi".to_string(),
                cache_control: None,
            }],
        }],
        system: None,
        tools: None,
        tool_choice: None,
        stream: false,
        thinking: None,
        gemini_cached_content: None,
    });
    assert_eq!(payload["model"], json!("qwen-plus-2025-07-28"));
}

#[test]
fn chat_completion_clamps_max_tokens_to_openai_compat_cap() {
    let payload = build_chat_completion_request(&MessageRequest {
        model: "gpt-4o".to_string(),
        max_tokens: 100_000,
        messages: vec![InputMessage::user_text("hi")],
        system: None,
        tools: None,
        tool_choice: None,
        stream: false,
        thinking: None,
        gemini_cached_content: None,
    });
    assert_eq!(payload["max_tokens"], json!(32_768));
}

#[test]
fn chat_completion_clamps_max_tokens_minimum_to_one() {
    let payload = build_chat_completion_request(&MessageRequest {
        model: "gpt-4o".to_string(),
        max_tokens: 0,
        messages: vec![InputMessage::user_text("hi")],
        system: None,
        tools: None,
        tool_choice: None,
        stream: false,
        thinking: None,
        gemini_cached_content: None,
    });
    assert_eq!(payload["max_tokens"], json!(1));
}

#[test]
fn sse_parses_delta_with_reasoning_content_only() {
    let frame = "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"reasoning_content\":\"2\"}}]}\n\n";
    let parsed = parse_sse_frame(frame).expect("parse").expect("chunk");
    assert_eq!(parsed.choices.len(), 1);
    assert_eq!(
        parsed.choices[0].delta.stream_reasoning_fragment(),
        Some("2".to_string())
    );
    assert_eq!(parsed.choices[0].delta.stream_content_fragment(), None);
}

#[test]
fn sse_parses_delta_with_thought_field() {
    let frame = "data: {\"id\":\"1\",\"choices\":[{\"delta\":{\"thought\":\"x\"}}]}\n\n";
    let parsed = parse_sse_frame(frame).expect("parse").expect("chunk");
    assert_eq!(
        parsed.choices[0].delta.stream_reasoning_fragment(),
        Some("x".to_string())
    );
    assert_eq!(parsed.choices[0].delta.stream_content_fragment(), None);
}

#[test]
fn sse_parses_delta_with_content_array() {
    let frame =
        r#"data: {"id":"1","choices":[{"delta":{"content":[{"type":"text","text":"hi"}]}}]}"#
            .to_string()
            + "\n\n";
    let parsed = parse_sse_frame(&frame).expect("parse").expect("chunk");
    assert_eq!(
        parsed.choices[0].delta.stream_content_fragment(),
        Some("hi".to_string())
    );
    assert_eq!(parsed.choices[0].delta.stream_reasoning_fragment(), None);
}

#[test]
fn chat_completion_preserves_openrouter_model_after_provider_prefix() {
    let payload = build_chat_completion_request(&MessageRequest {
        model: "openrouter/meta-llama/llama-3.1-8b:free".to_string(),
        max_tokens: 64,
        messages: vec![InputMessage {
            role: "user".to_string(),
            content: vec![InputContentBlock::Text {
                text: "hi".to_string(),
                cache_control: None,
            }],
        }],
        system: None,
        tools: None,
        tool_choice: None,
        stream: false,
        thinking: None,
        gemini_cached_content: None,
    });
    assert_eq!(payload["model"], json!("meta-llama/llama-3.1-8b:free"));
}

#[test]
fn chat_completion_with_gemini_cached_content_omits_system_and_tools() {
    let payload = build_chat_completion_request(&MessageRequest {
        model: "gemini/gemini-2.0-flash".to_string(),
        max_tokens: 64,
        messages: vec![InputMessage::user_text("hi")],
        system: Some(SystemBlock::from_plain("sys")),
        tools: Some(vec![ToolDefinition {
            name: "t".to_string(),
            description: None,
            input_schema: json!({}),
            cache_control: None,
        }]),
        tool_choice: Some(ToolChoice::Auto),
        stream: false,
        thinking: None,
        gemini_cached_content: Some("cachedContents/xyz".to_string()),
    });
    assert_eq!(payload["cachedContent"], json!("cachedContents/xyz"));
    assert!(payload.get("tools").is_none());
    assert!(payload.get("tool_choice").is_none());
    assert_eq!(payload["messages"].as_array().unwrap().len(), 1);
    assert_eq!(payload["messages"][0]["role"], json!("user"));
}

#[test]
fn is_gemini_model_detects_prefixed_and_plain_ids() {
    assert!(is_gemini_model("gemini-2.0-flash"));
    assert!(is_gemini_model("models/gemini-1.5-pro"));
    assert!(is_gemini_model("google/gemini-2.0-flash"));
    assert!(!is_gemini_model("gpt-4o"));
}

#[test]
fn tool_choice_translation_supports_required_function() {
    assert_eq!(openai_tool_choice(&ToolChoice::Any), json!("required"));
    assert_eq!(
        openai_tool_choice(&ToolChoice::Tool {
            name: "weather".to_string(),
        }),
        json!({"type": "function", "function": {"name": "weather"}})
    );
}

#[test]
fn parses_tool_arguments_fallback() {
    assert_eq!(
        parse_tool_arguments("{\"city\":\"Paris\"}"),
        json!({"city": "Paris"})
    );
    assert_eq!(parse_tool_arguments("not-json"), json!({"raw": "not-json"}));
}

#[test]
fn missing_xai_api_key_is_provider_specific() {
    let _lock = env_lock();
    std::env::remove_var("XAI_API_KEY");
    let error = OpenAiCompatClient::from_env(OpenAiCompatConfig::xai())
        .expect_err("missing key should error");
    assert!(matches!(
        error,
        ApiError::MissingCredentials {
            provider: "xAI",
            ..
        }
    ));
}

#[test]
fn endpoint_builder_accepts_base_urls_and_full_endpoints() {
    assert_eq!(
        chat_completions_endpoint("https://api.x.ai/v1", None),
        "https://api.x.ai/v1/chat/completions"
    );
    assert_eq!(
        chat_completions_endpoint("https://api.x.ai/v1/", None),
        "https://api.x.ai/v1/chat/completions"
    );
    assert_eq!(
        chat_completions_endpoint("https://api.x.ai/v1/chat/completions", None),
        "https://api.x.ai/v1/chat/completions"
    );
}

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .expect("env lock")
}

#[test]
fn normalizes_stop_reasons() {
    assert_eq!(normalize_finish_reason("stop"), "end_turn");
    assert_eq!(normalize_finish_reason("tool_calls"), "tool_use");
}
