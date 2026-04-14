use async_trait::async_trait;
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::error::WebAiResult;
use crate::page::{extract_stream_error, WebAiPage};
use crate::provider::{ModelInfo, ProviderConfig, WebProviderClient};
use crate::sse_parser::SseLineParser;

pub struct QwenProvider {
    config: ProviderConfig,
}

impl Default for QwenProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl QwenProvider {
    pub fn new() -> Self {
        Self {
            config: ProviderConfig {
                id: "qwen-web".into(),
                name: "Qwen Web".into(),
                start_url: "https://chat.qwen.ai/".into(),
                host_key: "qwen.ai".into(),
                models: vec![
                    ModelInfo {
                        id: "qwen3.5-plus".into(),
                        name: "Qwen 3.5 Plus".into(),
                        default: true,
                    },
                    ModelInfo {
                        id: "qwen3.5-turbo".into(),
                        name: "Qwen 3.5 Turbo".into(),
                        default: false,
                    },
                ],
                session_cookie_names: vec!["token".into(), "cna".into()],
            },
        }
    }
}

#[async_trait]
impl WebProviderClient for QwenProvider {
    fn provider_id(&self) -> &str {
        &self.config.id
    }
    fn config(&self) -> &ProviderConfig {
        &self.config
    }
    async fn init(&self, _page: &WebAiPage) -> WebAiResult<()> {
        Ok(())
    }

    async fn send_message(
        &self,
        page: &WebAiPage,
        message: &str,
        model: &str,
    ) -> WebAiResult<mpsc::Receiver<String>> {
        let model = if model.is_empty() {
            "qwen3.5-plus"
        } else {
            model
        };
        let js = build_send_js(message, model);
        let (rx, _handle) = page.evaluate_streaming(&js, 256)?;

        let (tx, parsed_rx) = mpsc::channel::<String>(256);
        tokio::spawn(async move {
            let mut sse = SseLineParser::new();
            let mut raw_rx = rx;
            while let Some(chunk) = raw_rx.recv().await {
                if let Some(err) = extract_stream_error(&chunk) {
                    let _ = tx.send(err).await;
                    return;
                }
                sse.push(&chunk);
                for ev in sse.drain_events() {
                    if let Some(d) = extract_delta(&ev) {
                        if tx.send(d).await.is_err() {
                            return;
                        }
                    }
                }
            }
            for ev in sse.flush() {
                if let Some(d) = extract_delta(&ev) {
                    let _ = tx.send(d).await;
                }
            }
            drop(_handle);
        });
        Ok(parsed_rx)
    }

    async fn fetch_models(&self, page: &WebAiPage) -> WebAiResult<Vec<ModelInfo>> {
        let js = r#"
const r = await fetch('https://chat.qwen.ai/api/models', { credentials: 'include' });
if (!r.ok) return null;
const data = await r.json();
return data;
"#;
        let result: Option<QwenModelsResponse> = page.evaluate(js, None).await.unwrap_or(None);
        match result {
            Some(resp) => {
                let models: Vec<ModelInfo> = resp
                    .data
                    .into_iter()
                    .filter(|m| m.status == "active")
                    .map(|m| ModelInfo {
                        id: m.id.clone(),
                        name: m.name.unwrap_or(m.id),
                        default: false,
                    })
                    .collect();
                if models.is_empty() {
                    Ok(self.list_models())
                } else {
                    Ok(models)
                }
            }
            None => Ok(self.list_models()),
        }
    }

    async fn check_session(&self, page: &WebAiPage) -> WebAiResult<bool> {
        page.check_session_via_fetch("https://chat.qwen.ai/api/v2/user/info")
            .await
    }
}

#[derive(Deserialize)]
struct QwenModelsResponse {
    #[serde(default)]
    data: Vec<QwenModelEntry>,
}

#[derive(Deserialize)]
struct QwenModelEntry {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    status: String,
}

fn build_send_js(message: &str, model: &str) -> String {
    let msg = serde_json::to_string(message).unwrap_or_else(|_| "\"\"".into());
    let mdl = serde_json::to_string(model).unwrap_or_else(|_| "\"\"".into());
    format!(
        r#"
const base = 'https://chat.qwen.ai';
const chatRes = await fetch(base + '/api/v2/chats/new', {{ method: 'POST', headers: {{ 'Content-Type': 'application/json' }}, body: '{{}}' }});
if (!chatRes.ok) throw new Error('Qwen create chat: ' + chatRes.status);
const chatData = await chatRes.json();
const chatId = chatData.data?.id || chatData.chat_id || chatData.id || chatData.chatId;

const fid = crypto.randomUUID();
const compRes = await fetch(base + '/api/v2/chat/completions?chat_id=' + chatId, {{
    method: 'POST',
    headers: {{ 'Content-Type': 'application/json', Accept: 'text/event-stream' }},
    body: JSON.stringify({{
        stream: true, version: '2.1', incremental_output: true, chat_id: chatId, chat_mode: 'normal', model: {mdl},
        messages: [{{ fid, role: 'user', content: {msg}, user_action: 'chat', files: [],
            timestamp: Math.floor(Date.now()/1000), models: [{mdl}], chat_type: 't2t',
            feature_config: {{ thinking_enabled: true, output_schema: 'phase' }} }}]
    }})
}});
if (!compRes.ok) {{ const t = await compRes.text(); throw new Error('[Qwen] ' + compRes.status + ' ' + t.slice(0, 500)); }}
const reader = compRes.body.getReader();
const decoder = new TextDecoder();
while (true) {{ const {{ done, value }} = await reader.read(); if (done) break; await __webai_stream(decoder.decode(value, {{ stream: true }})); }}
"#
    )
}

fn extract_delta(json_str: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json_str).ok()?;
    if let Some(choices) = v.get("choices").and_then(|c| c.as_array()) {
        if let Some(delta) = choices.first().and_then(|c| c.get("delta")) {
            if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                if !content.is_empty() {
                    return Some(content.to_string());
                }
            }
        }
    }
    if let Some(t) = v.get("text").and_then(|t| t.as_str()) {
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    None
}
