use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::AppHandle;
use tokio::sync::{mpsc, Mutex};

use crate::error::{WebAiError, WebAiResult};
use crate::page_manager::WebAiPageManager;
use crate::provider::{ModelInfo, ProviderConfig, WebProviderClient};
use crate::providers::{
    chatgpt::ChatGptProvider, claude::ClaudeProvider, deepseek::DeepSeekProvider,
    doubao::DoubaoProvider, gemini::GeminiWebProvider, glm::GlmProvider,
    glm_intl::GlmIntlWebProvider, grok::GrokProvider, kimi::KimiProvider,
    perplexity::PerplexityWebProvider, qwen::QwenProvider, qwen_cn::QwenCnProvider,
    xiaomimo::XiaomimoProvider,
};
use crate::tool_calling::converter::{
    build_prompt_from_messages, parse_tool_response, ChatMessage, ParsedResponse, ToolChoice,
    ToolDefinition,
};

const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 300;
const DEFAULT_PAGE_LOAD_TIMEOUT_SECS: u64 = 60;

/// High-level facade that owns the WebView page manager, provider registry,
/// and tool-calling layer.  Thread-safe and cheaply cloneable via `Arc`.
#[derive(Clone)]
pub struct WebAiEngine {
    inner: Arc<Inner>,
}

struct Inner {
    page_manager: Mutex<WebAiPageManager>,
    providers: HashMap<String, Arc<dyn WebProviderClient>>,
    idle_timeout_secs: AtomicU64,
    page_load_timeout_secs: AtomicU64,
}

impl WebAiEngine {
    pub fn new(app_handle: AppHandle) -> Self {
        let mut p: HashMap<String, Arc<dyn WebProviderClient>> = HashMap::new();
        p.insert("chatgpt-web".into(), Arc::new(ChatGptProvider::new()));
        p.insert("claude-web".into(), Arc::new(ClaudeProvider::new()));
        p.insert("deepseek-web".into(), Arc::new(DeepSeekProvider::new()));
        p.insert("doubao-web".into(), Arc::new(DoubaoProvider::new()));
        p.insert("gemini-web".into(), Arc::new(GeminiWebProvider::new()));
        p.insert("glm-web".into(), Arc::new(GlmProvider::new()));
        p.insert("glm-intl-web".into(), Arc::new(GlmIntlWebProvider::new()));
        p.insert("grok-web".into(), Arc::new(GrokProvider::new()));
        p.insert("kimi-web".into(), Arc::new(KimiProvider::new()));
        p.insert(
            "perplexity-web".into(),
            Arc::new(PerplexityWebProvider::new()),
        );
        p.insert("qwen-web".into(), Arc::new(QwenProvider::new()));
        p.insert("qwen-cn-web".into(), Arc::new(QwenCnProvider::new()));
        p.insert("xiaomimo-web".into(), Arc::new(XiaomimoProvider::new()));

        Self {
            inner: Arc::new(Inner {
                page_manager: Mutex::new(WebAiPageManager::new(app_handle)),
                providers: p,
                idle_timeout_secs: AtomicU64::new(DEFAULT_IDLE_TIMEOUT_SECS),
                page_load_timeout_secs: AtomicU64::new(DEFAULT_PAGE_LOAD_TIMEOUT_SECS),
            }),
        }
    }

    /// List all registered WebAI providers.
    pub fn list_providers(&self) -> Vec<ProviderConfig> {
        self.inner
            .providers
            .values()
            .map(|p| p.config().clone())
            .collect()
    }

    /// List models for a specific provider.
    pub fn list_models(&self, provider_id: &str) -> Vec<ModelInfo> {
        self.inner
            .providers
            .get(provider_id)
            .map(|p| p.list_models())
            .unwrap_or_default()
    }

    pub fn set_idle_timeout_secs(&self, secs: u64) {
        self.inner.idle_timeout_secs.store(secs, Ordering::Relaxed);
    }

    pub fn set_page_load_timeout_secs(&self, secs: u64) {
        self.inner
            .page_load_timeout_secs
            .store(secs, Ordering::Relaxed);
    }

    /// Resolve `webai/<provider>` or `webai/<provider>/<model>` into (provider_id, model).
    pub fn parse_webai_model(model: &str) -> Option<(&str, Option<&str>)> {
        let stripped = model.strip_prefix("webai/")?;
        if stripped.is_empty() {
            return None;
        }
        if let Some(pos) = stripped.find('/') {
            let provider = &stripped[..pos];
            let model_part = &stripped[pos + 1..];
            if model_part.is_empty() {
                Some((provider, None))
            } else {
                Some((provider, Some(model_part)))
            }
        } else {
            Some((stripped, None))
        }
    }

    /// Map a short provider name to the registered provider ID.
    fn resolve_provider_id(&self, name: &str) -> Option<String> {
        let candidate = match name {
            "chatgpt" | "openai" => "chatgpt-web",
            "claude" => "claude-web",
            "deepseek" => "deepseek-web",
            "doubao" => "doubao-web",
            "gemini" | "google" => "gemini-web",
            "glm" | "chatglm" => "glm-web",
            "glm-intl" | "glm_intl" => "glm-intl-web",
            "grok" => "grok-web",
            "kimi" | "moonshot" => "kimi-web",
            "perplexity" => "perplexity-web",
            "qwen" => "qwen-web",
            "qwen-cn" | "qwen_cn" | "qianwen" => "qwen-cn-web",
            "xiaomimo" | "mimo" => "xiaomimo-web",
            other => other,
        };
        if self.inner.providers.contains_key(candidate) {
            Some(candidate.to_string())
        } else {
            None
        }
    }

    /// Return the list of provider IDs that currently have an active WebView page.
    pub async fn list_active_pages(&self) -> Vec<String> {
        let mgr = self.inner.page_manager.lock().await;
        mgr.list_pages()
    }

    /// Close the WebView page for a specific provider.
    pub async fn close_page(&self, provider_id: &str) {
        let mut mgr = self.inner.page_manager.lock().await;
        mgr.close(provider_id);
    }

    /// Close all active WebView pages.
    pub async fn close_all_pages(&self) {
        let mut mgr = self.inner.page_manager.lock().await;
        mgr.close_all();
    }

    /// Send a plain-text message (no tool handling) and get streaming response.
    pub async fn send_raw(
        &self,
        provider_name: &str,
        model: &str,
        message: &str,
    ) -> WebAiResult<mpsc::Receiver<String>> {
        tracing::info!(%provider_name, %model, "send_raw: resolving provider");
        let provider_id = self
            .resolve_provider_id(provider_name)
            .ok_or_else(|| WebAiError::Provider(format!("unknown provider: {provider_name}")))?;

        let provider = self.inner.providers[&provider_id].clone();
        let start_url = provider.config().start_url.clone();

        tracing::info!(%provider_id, %start_url, "send_raw: acquiring page manager lock");
        let mut mgr = self.inner.page_manager.lock().await;
        let idle_secs = self.inner.idle_timeout_secs.load(Ordering::Relaxed);
        let load_secs = self.inner.page_load_timeout_secs.load(Ordering::Relaxed);
        mgr.cleanup_idle(Duration::from_secs(idle_secs));
        mgr.set_page_load_timeout(Duration::from_secs(load_secs));
        let needs_init = !mgr.has_page(&provider_id);
        tracing::info!(%provider_id, needs_init, "send_raw: get_or_create page");
        let page = mgr.get_or_create(&provider_id, &start_url).await?.clone();
        drop(mgr);

        if needs_init {
            tracing::info!(%provider_id, "send_raw: running provider init");
            provider.init(&page).await?;
        }

        tracing::info!(%provider_id, %model, "send_raw: sending message");
        provider.send_message(&page, message, model).await
    }

    /// Send an OpenAI-format request with automatic tool-calling adaptation.
    ///
    /// - If `tools` is `None`/empty: direct pass-through (chat mode)
    /// - If `tools` is present: inject tool prompt, buffer response, extract tool calls (agent mode)
    pub async fn send_openai(
        &self,
        provider_name: &str,
        model: &str,
        messages: &[ChatMessage],
        tools: Option<&[ToolDefinition]>,
        tool_choice: Option<&ToolChoice>,
    ) -> WebAiResult<OpenAiStreamResult> {
        let converted = build_prompt_from_messages(messages, tools, tool_choice);

        let rx = self
            .send_raw(provider_name, model, &converted.prompt)
            .await?;

        if !converted.has_tools {
            return Ok(OpenAiStreamResult::Streaming(rx));
        }

        let full_text = collect_stream(rx).await;
        let parsed = parse_tool_response(&full_text, tools);
        Ok(OpenAiStreamResult::Completed(parsed))
    }
}

/// Result from `send_openai`: either a live stream (chat) or a fully-parsed response (tools).
pub enum OpenAiStreamResult {
    /// Chat mode: stream text chunks directly.
    Streaming(mpsc::Receiver<String>),
    /// Agent mode: buffered and parsed for tool calls.
    Completed(ParsedResponse),
}

async fn collect_stream(mut rx: mpsc::Receiver<String>) -> String {
    let mut buf = String::new();
    while let Some(chunk) = rx.recv().await {
        buf.push_str(&chunk);
    }
    buf
}
