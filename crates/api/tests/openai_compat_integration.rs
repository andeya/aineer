use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::Arc;
use std::sync::OnceLock;

use api::{
    ContentBlockDelta, ContentBlockDeltaEvent, ContentBlockStartEvent, ContentBlockStopEvent,
    InputContentBlock, InputMessage, MessageRequest, OpenAiCompatClient, OpenAiCompatConfig,
    OutputContentBlock, ProviderClient, StreamEvent, ToolChoice, ToolDefinition,
};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[tokio::test]
async fn send_message_uses_openai_compatible_endpoint_and_auth() {
    let state = Arc::new(Mutex::new(Vec::<CapturedRequest>::new()));
    let body = concat!(
        "{",
        "\"id\":\"chatcmpl_test\",",
        "\"model\":\"grok-3\",",
        "\"choices\":[{",
        "\"message\":{\"role\":\"assistant\",\"content\":\"Hello from Grok\",\"tool_calls\":[]},",
        "\"finish_reason\":\"stop\"",
        "}],",
        "\"usage\":{\"prompt_tokens\":11,\"completion_tokens\":5}",
        "}"
    );
    let server = spawn_server(
        state.clone(),
        vec![http_response("200 OK", "application/json", body)],
    )
    .await;

    let client = OpenAiCompatClient::new("xai-test-key", OpenAiCompatConfig::xai())
        .with_base_url(server.base_url());
    let response = client
        .send_message(&sample_request(false))
        .await
        .expect("request should succeed");

    assert_eq!(response.model, "grok-3");
    assert_eq!(response.total_tokens(), 16);
    assert_eq!(
        response.content,
        vec![OutputContentBlock::Text {
            text: "Hello from Grok".to_string(),
        }]
    );

    let captured = state.lock().await;
    let request = captured.first().expect("server should capture request");
    assert_eq!(request.path, "/chat/completions");
    assert_eq!(
        request.headers.get("authorization").map(String::as_str),
        Some("Bearer xai-test-key")
    );
    let body: serde_json::Value = serde_json::from_str(&request.body).expect("json body");
    assert_eq!(body["model"], json!("grok-3"));
    assert_eq!(body["messages"][0]["role"], json!("system"));
    assert_eq!(body["tools"][0]["type"], json!("function"));
}

#[tokio::test]
async fn send_message_accepts_full_chat_completions_endpoint_override() {
    let state = Arc::new(Mutex::new(Vec::<CapturedRequest>::new()));
    let body = concat!(
        "{",
        "\"id\":\"chatcmpl_full_endpoint\",",
        "\"model\":\"grok-3\",",
        "\"choices\":[{",
        "\"message\":{\"role\":\"assistant\",\"content\":\"Endpoint override works\",\"tool_calls\":[]},",
        "\"finish_reason\":\"stop\"",
        "}],",
        "\"usage\":{\"prompt_tokens\":7,\"completion_tokens\":3}",
        "}"
    );
    let server = spawn_server(
        state.clone(),
        vec![http_response("200 OK", "application/json", body)],
    )
    .await;

    let endpoint_url = format!("{}/chat/completions", server.base_url());
    let client = OpenAiCompatClient::new("xai-test-key", OpenAiCompatConfig::xai())
        .with_base_url(endpoint_url);
    let response = client
        .send_message(&sample_request(false))
        .await
        .expect("request should succeed");

    assert_eq!(response.total_tokens(), 10);
