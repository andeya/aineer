//! Anthropic and OpenAI-compatible API client with streaming support.

mod cache_strategy;
mod client;
mod error;
mod providers;
mod sse;
mod types;

fn default_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .unwrap_or_default()
}

pub use cache_strategy::{
    is_gemini_model, GeminiCacheStrategy, NoCacheStrategy, ProviderCacheStrategy,
};
pub use client::{
    read_base_url, read_xai_base_url, resolve_saved_oauth_token, resolve_startup_auth_source,
    MessageStream, OAuthTokenSet, ProviderClient,
};
pub use error::ApiError;
pub use providers::aineer_provider::{AineerApiClient, AuthSource};
pub use providers::openai_compat::{OpenAiCompatClient, OpenAiCompatConfig};
pub use providers::{
    auto_detect_default_model, builtin_preset, detect_provider_kind, list_known_models,
    max_tokens_for_model, parse_custom_provider_prefix, provider_kind_by_name, resolve_model_alias,
    BuiltinProviderPreset, ProviderKind, RetryPolicy, BUILTIN_PROVIDER_PRESETS,
};
pub use sse::{parse_frame, SseParser};

/// Fetch the model list from an OpenAI-compatible `/models` endpoint.
///
/// Returns a sorted list of model IDs. On any HTTP or parsing error the
/// reqwest error is propagated as [`ApiError::Http`].
pub async fn fetch_remote_model_ids(
    base_url: &str,
    api_key: Option<&str>,
    headers: Option<&std::collections::BTreeMap<String, String>>,
) -> Result<Vec<String>, ApiError> {
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let mut req = client.get(&url);
    if let Some(key) = api_key {
        if !key.is_empty() {
            req = req.bearer_auth(key);
        }
    }
    if let Some(hdrs) = headers {
        for (k, v) in hdrs {
            req = req.header(k.as_str(), v.as_str());
        }
    }

    let response = req.send().await?.error_for_status()?;

    let body: serde_json::Value = response.json().await?;

    let mut models: Vec<String> = body
        .get("data")
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("id")?.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    models.sort();
    Ok(models)
}

pub use types::{
    gemini_cache_key_hash, BlockKind, CacheControl, CacheScope, CacheType, ContentBlockDelta,
    ContentBlockDeltaEvent, ContentBlockStartEvent, ContentBlockStopEvent, GeminiCachedContent,
    ImageSource, InputContentBlock, InputMessage, MessageDelta, MessageDeltaEvent, MessageRequest,
    MessageResponse, MessageStartEvent, MessageStopEvent, OutputContentBlock, StreamEvent,
    SystemBlock, ThinkingConfig, ThinkingMode, ToolChoice, ToolDefinition, ToolResultContentBlock,
    Usage,
};
