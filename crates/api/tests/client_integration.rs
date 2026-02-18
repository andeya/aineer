use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use api::{
    ApiClient, ApiError, AuthSource, ContentBlockDelta, ContentBlockDeltaEvent,
    ContentBlockStartEvent, InputContentBlock, InputMessage, MessageDeltaEvent, MessageRequest,
    OutputContentBlock, ProviderClient, StreamEvent, ToolChoice, ToolDefinition,
};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[tokio::test]
async fn send_message_posts_json_and_parses_response() {
    let state = Arc::new(Mutex::new(Vec::<CapturedRequest>::new()));
    let body = concat!(
        "{",
        "\"id\":\"msg_test\",",
        "\"type\":\"message\",",
        "\"role\":\"assistant\",",
        "\"content\":[{\"type\":\"text\",\"text\":\"Hello from Codineer\"}],",
        "\"model\":\"claude-sonnet-4-6\",",
        "\"stop_reason\":\"end_turn\",",
        "\"stop_sequence\":null,",
        "\"usage\":{\"input_tokens\":12,\"output_tokens\":4},",
        "\"request_id\":\"req_body_123\"",
        "}"
    );
    let server = spawn_server(
        state.clone(),
        vec![http_response("200 OK", "application/json", body)],
    )
    .await;

    let client = ApiClient::new("test-key")
        .with_auth_token(Some("proxy-token".to_string()))
        .with_base_url(server.base_url());
    let response = client
        .send_message(&sample_request(false))
        .await
        .expect("request should succeed");

    assert_eq!(response.id, "msg_test");
    assert_eq!(response.total_tokens(), 16);
    assert_eq!(response.request_id.as_deref(), Some("req_body_123"));
    assert_eq!(
        response.content,
