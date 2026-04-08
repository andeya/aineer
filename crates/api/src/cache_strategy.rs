//! Provider-level context caching strategies (e.g. Gemini `cachedContents`).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::ApiError;
use crate::providers::{is_retryable_status, parse_custom_provider_prefix};
use crate::types::{
    gemini_cache_key_hash, GeminiCachedContent, MessageRequest, SystemBlock, ToolDefinition,
};
use codineer_core::GeminiCacheConfig;

/// Strategy for provider-level context caching (e.g. Gemini's cachedContents API).
/// Providers that don't support explicit caching simply use [`NoCacheStrategy`].
#[async_trait]
pub trait ProviderCacheStrategy: Send + Sync + std::fmt::Debug {
    /// Ensure any remote/in-memory cache entries exist for this request (e.g. create via HTTP).
    async fn ensure_cached(&self, request: &MessageRequest);

    /// Optionally transform the request to use cached content.
    /// Returns the (possibly modified) request.
    fn apply_cache(&self, request: &MessageRequest) -> MessageRequest;
}

/// No-op cache strategy for providers without explicit caching APIs.
#[derive(Debug, Default, Clone)]
pub struct NoCacheStrategy;

#[async_trait]
impl ProviderCacheStrategy for NoCacheStrategy {
    async fn ensure_cached(&self, _request: &MessageRequest) {}

    fn apply_cache(&self, request: &MessageRequest) -> MessageRequest {
        request.clone()
    }
}

/// Gemini `cachedContents` strategy for Google Generative Language OpenAI-compatible endpoints.
pub struct GeminiCacheStrategy {
    config: GeminiCacheConfig,
    entries: Arc<Mutex<HashMap<u64, GeminiCachedContent>>>,
    http: reqwest::Client,
    api_key: String,
    base_url: String,
    custom_headers: std::collections::BTreeMap<String, String>,
}

impl std::fmt::Debug for GeminiCacheStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeminiCacheStrategy")
            .field("config", &self.config)
            .field("base_url", &self.base_url)
            .field("api_key", &"***")
            .finish()
    }
}

impl GeminiCacheStrategy {
    #[must_use]
    pub fn new(
        config: GeminiCacheConfig,
        http: reqwest::Client,
        api_key: String,
        base_url: String,
        custom_headers: std::collections::BTreeMap<String, String>,
    ) -> Self {
        Self {
            config,
            entries: Arc::new(Mutex::new(HashMap::new())),
            http,
            api_key,
            base_url,
            custom_headers,
        }
    }

    /// Create a cached content resource via `POST .../cachedContents` (Google Generative Language API).
    pub async fn create_cached_content(
        &self,
        model: &str,
        system_blocks: &[SystemBlock],
        tools: &[ToolDefinition],
    ) -> Result<GeminiCachedContent, ApiError> {
        let Some(url) = gemini_cached_contents_url_from_openai_base(&self.base_url) else {
            return Err(ApiError::Auth(
                "Gemini context caching requires a Google Generative Language API base URL (…/v1beta/openai/…)"
                    .to_string(),
            ));
        };
        let system_text = system_blocks
            .iter()
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let mut body = json!({
            "model": gemini_model_resource_name(model),
            "displayName": format!(
                "codineer-cache-{:x}",
                gemini_cache_key_hash(
                    if system_blocks.is_empty() {
                        None
                    } else {
                        Some(system_blocks)
                    },
                    if tools.is_empty() {
                        None
                    } else {
                        Some(tools)
                    },
                )
            ),
            "ttl": format!("{}s", self.config.ttl_seconds),
        });
        if !system_text.is_empty() {
            body["systemInstruction"] = json!({
                "parts": [{ "text": system_text }]
            });
        }
        if !tools.is_empty() {
            let decls: Vec<Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema,
                    })
                })
                .collect();
            body["tools"] = json!([{ "functionDeclarations": decls }]);
        }

        let mut req = self
            .http
            .post(&url)
            .header("content-type", "application/json")
            .header("x-goog-api-key", &self.api_key);
        for (name, value) in &self.custom_headers {
            req = req.header(name.as_str(), value.as_str());
        }
        let response = req.json(&body).send().await.map_err(ApiError::from)?;
        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(ApiError::Api {
                status,
                error_type: None,
                message: Some(format!("cachedContents error: {body_text}")),
                body: body_text,
                url: Some(url),
                retryable: is_retryable_status(status.as_u16()),
            });
        }
        let parsed: CreateCachedContentResponse = response.json().await.map_err(ApiError::from)?;
        Ok(GeminiCachedContent {
            name: parsed.name,
            expire_time: parsed.expire_time,
        })
    }
}

#[async_trait]
impl ProviderCacheStrategy for GeminiCacheStrategy {
    async fn ensure_cached(&self, request: &MessageRequest) {
        if !self.config.enabled || !is_gemini_model(&request.model) {
            return;
        }
        if gemini_cached_contents_url_from_openai_base(&self.base_url).is_none() {
            return;
        }
        let system = request.system.as_deref();
        let tools = request.tools.as_deref();
        let has_system = system.map(|s| !s.is_empty()).unwrap_or(false);
        let has_tools = tools.map(|t| !t.is_empty()).unwrap_or(false);
        if !has_system && !has_tools {
            return;
        }
        let key = gemini_cache_key_hash(system, tools);
        if let Ok(guard) = self.entries.lock() {
            if guard.contains_key(&key) {
                return;
            }
        }
        let system_vec = request.system.clone().unwrap_or_default();
        let tools_vec = request.tools.clone().unwrap_or_default();
        match self
            .create_cached_content(&request.model, &system_vec, &tools_vec)
            .await
        {
            Ok(cached) => {
                if let Ok(mut guard) = self.entries.lock() {
                    guard.insert(key, cached);
                }
            }
            Err(err) => {
                eprintln!("[warn] Gemini context cache create failed: {err}; sending full prompt");
            }
        }
    }

    fn apply_cache(&self, request: &MessageRequest) -> MessageRequest {
        if !self.config.enabled || !is_gemini_model(&request.model) {
            return request.clone();
        }
        if gemini_cached_contents_url_from_openai_base(&self.base_url).is_none() {
            return request.clone();
        }
        let system = request.system.as_deref();
        let tools = request.tools.as_deref();
        let has_system = system.map(|s| !s.is_empty()).unwrap_or(false);
        let has_tools = tools.map(|t| !t.is_empty()).unwrap_or(false);
        if !has_system && !has_tools {
            return request.clone();
        }
        let key = gemini_cache_key_hash(system, tools);
        if let Ok(guard) = self.entries.lock() {
            if let Some(cached) = guard.get(&key) {
                return use_cached_content(cached, request.clone());
            }
        }
        request.clone()
    }
}

/// Attach a Gemini cached content handle: drops inline system/tools (they live in the cache).
#[must_use]
fn use_cached_content(cached: &GeminiCachedContent, mut request: MessageRequest) -> MessageRequest {
    request.system = None;
    request.tools = None;
    request.tool_choice = None;
    request.gemini_cached_content = Some(cached.name.clone());
    request
}

fn upstream_openai_model(model: &str) -> String {
    parse_custom_provider_prefix(model)
        .map(|(_, rest)| rest.to_string())
        .unwrap_or_else(|| model.to_string())
}

/// `true` when the upstream model name refers to Gemini (OpenAI-compat or bare id).
#[must_use]
pub fn is_gemini_model(model: &str) -> bool {
    let rest = parse_custom_provider_prefix(model)
        .map(|(_, rest)| rest)
        .unwrap_or(model)
        .trim();
    let lower = rest.to_ascii_lowercase();
    lower.starts_with("gemini-")
        || lower.starts_with("models/gemini-")
        || lower
            .strip_prefix("models/")
            .is_some_and(|m| m.starts_with("gemini-"))
}

fn gemini_cached_contents_url_from_openai_base(base_url: &str) -> Option<String> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if !trimmed.contains("generativelanguage.googleapis.com") {
        return None;
    }
    let cut = trimmed.find("/openai")?;
    Some(format!("{}{}", &trimmed[..cut], "/cachedContents"))
}

fn gemini_model_resource_name(model: &str) -> String {
    let m = upstream_openai_model(model);
    let lower = m.to_ascii_lowercase();
    if lower.starts_with("models/") {
        m
    } else {
        format!("models/{m}")
    }
}

#[derive(Debug, Deserialize)]
struct CreateCachedContentResponse {
    name: String,
    #[serde(rename = "expireTime")]
    expire_time: Option<String>,
}
