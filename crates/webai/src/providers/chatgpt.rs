use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::error::WebAiResult;
use crate::page::{extract_stream_error, WebAiPage};
use crate::provider::{ModelInfo, ProviderConfig, WebProviderClient};
use crate::sse_parser::SseLineParser;

pub struct ChatGptProvider {
    config: ProviderConfig,
}

impl Default for ChatGptProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatGptProvider {
    pub fn new() -> Self {
        Self {
            config: ProviderConfig {
                id: "chatgpt-web".into(),
                name: "ChatGPT Web".into(),
                start_url: "https://chatgpt.com/".into(),
                host_key: "chatgpt.com".into(),
                models: vec![
                    ModelInfo {
                        id: "gpt-4".into(),
                        name: "GPT-4".into(),
                        default: true,
                    },
                    ModelInfo {
                        id: "gpt-4-turbo".into(),
                        name: "GPT-4 Turbo".into(),
                        default: false,
                    },
                    ModelInfo {
                        id: "gpt-3.5-turbo".into(),
                        name: "GPT-3.5 Turbo".into(),
                        default: false,
                    },
                ],
            },
        }
    }
}

#[async_trait]
impl WebProviderClient for ChatGptProvider {
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
        let model = if model.is_empty() { "gpt-4" } else { model };
        let js = build_send_js(message, model);
        let (rx, _handle) = page.evaluate_streaming(&js, 256)?;

        let (parsed_tx, parsed_rx) = mpsc::channel::<String>(256);
        tokio::spawn(async move {
            let mut sse = SseLineParser::new();
            let mut raw_rx = rx;
            let mut prev_len = 0usize;
            let mut got_sse = false;
            let mut raw_buf = String::new();
            while let Some(chunk) = raw_rx.recv().await {
                if let Some(err) = extract_stream_error(&chunk) {
                    let _ = parsed_tx.send(err).await;
                    return;
                }
                raw_buf.push_str(&chunk);
                sse.push(&chunk);
                for event_data in sse.drain_events() {
                    if let Some(full_text) = extract_accumulated_text(&event_data) {
                        got_sse = true;
                        if full_text.len() > prev_len {
                            let new_part = full_text[prev_len..].to_string();
                            prev_len = full_text.len();
                            if parsed_tx.send(new_part).await.is_err() {
                                return;
                            }
                        }
                    }
                }
            }
            for event_data in sse.flush() {
                if let Some(full_text) = extract_accumulated_text(&event_data) {
                    got_sse = true;
                    if full_text.len() > prev_len {
                        let new_part = full_text[prev_len..].to_string();
                        let _ = parsed_tx.send(new_part).await;
                    }
                }
            }
            // DOM fallback returns plain text, not SSE
            if !got_sse && !raw_buf.trim().is_empty() {
                let _ = parsed_tx.send(raw_buf).await;
            }
            drop(_handle);
        });
        Ok(parsed_rx)
    }

    async fn check_session(&self, page: &WebAiPage) -> WebAiResult<bool> {
        let js = r#"
const r = await fetch('https://chatgpt.com/api/auth/session', { credentials: 'include' });
if (!r.ok) return false;
const data = await r.json();
return !!data.accessToken;
"#;
        page.evaluate::<bool>(js, None).await
    }
}

fn build_send_js(message: &str, model: &str) -> String {
    let msg = serde_json::to_string(message).unwrap_or_else(|_| "\"\"".into());
    let mdl = serde_json::to_string(model).unwrap_or_else(|_| "\"gpt-4\"".into());
    format!(
        r#"
const message = {msg};
const model = {mdl};
const msgId = crypto.randomUUID();
const parentId = crypto.randomUUID();

const body = {{
    action: 'next',
    messages: [{{ id: msgId, author: {{ role: 'user' }}, content: {{ content_type: 'text', parts: [message] }} }}],
    parent_message_id: parentId,
    model: model,
    timezone_offset_min: new Date().getTimezoneOffset(),
    history_and_training_disabled: false,
    conversation_mode: {{ kind: 'primary_assistant', plugin_ids: null }},
    force_paragen: false,
    force_paragen_model_slug: '',
    force_rate_limit: false,
    reset_rate_limits: false,
    force_use_sse: true,
}};

/* ---- Session check (fail fast if not authenticated) ---- */
const session = await fetch('https://chatgpt.com/api/auth/session', {{ credentials: 'include' }})
    .then(r => r.ok ? r.json() : null).catch(() => null);
const accessToken = session?.accessToken;
if (!accessToken) {{
    throw new Error('[ChatGPT] Not authenticated. Please login via the WebAI settings page first.');
}}
const deviceId = session?.oaiDeviceId || crypto.randomUUID();
const refUrl = location.href || 'https://chatgpt.com/';

function baseHeaders() {{
    return {{
        'Content-Type': 'application/json',
        'Accept': 'text/event-stream',
        'oai-device-id': deviceId,
        'oai-language': 'en-US',
        'Referer': refUrl,
        'sec-ch-ua': '"Google Chrome";v="131", "Chromium";v="131", "Not_A Brand";v="24"',
        'sec-ch-ua-mobile': '?0',
        'sec-ch-ua-platform': '"macOS"',
        ...(accessToken ? {{ Authorization: 'Bearer ' + accessToken }} : {{}}),
    }};
}}

/* ---- Sentinel warmup ---- */
async function warmup() {{
    const h = baseHeaders();
    const endpoints = [
        'https://chatgpt.com/backend-api/conversation/init',
        'https://chatgpt.com/backend-api/sentinel/chat-requirements/prepare',
        'https://chatgpt.com/backend-api/sentinel/chat-requirements/finalize',
    ];
    for (const url of endpoints) {{
        await fetch(url, {{ method: 'POST', headers: h, body: '{{}}', credentials: 'include' }}).catch(() => {{}});
    }}
}}

/* ---- oaistatic dynamic sentinel headers ---- */
async function trySentinelFetch() {{
    await warmup();
    const scripts = Array.from(document.scripts);
    const assetSrc = scripts.map(s => s.src).find(s => s?.includes('oaistatic.com') && s.endsWith('.js'));
    const assetUrl = assetSrc || 'https://cdn.oaistatic.com/assets/i5bamk05qmvsi6c3.js';
    try {{
        const g = await import(assetUrl);
        if (typeof g.bk !== 'function' || typeof g.fX !== 'function') return null;
        const z = await g.bk();
        const turnstileKey = z?.turnstile?.bx ?? z?.turnstile?.dx;
        if (!turnstileKey) return null;
        const r = await g.bi(turnstileKey);
        let arkose = null; try {{ arkose = await g.bl?.getEnforcementToken?.(z); }} catch {{}}
        let proof = null;  try {{ proof  = await g.bm?.getEnforcementToken?.(z); }} catch {{}}
        const extra = await g.fX(z, arkose, r, proof, null);
        const headers = {{ ...baseHeaders(), ...(typeof extra === 'object' ? extra : {{}}) }};
        return await fetch('https://chatgpt.com/backend-api/conversation', {{
            method: 'POST', headers, body: JSON.stringify(body), credentials: 'include',
        }});
    }} catch {{ return null; }}
}}

/* ---- DOM fallback when API returns 403 ---- */
async function domFallback() {{
    const sels = ['#prompt-textarea', 'textarea[placeholder]', 'textarea', '[contenteditable="true"]'];
    let el = null;
    for (const s of sels) {{ el = document.querySelector(s); if (el) break; }}
    if (!el) throw new Error('[ChatGPT] DOM fallback: input not found');

    el.focus(); el.click();
    await new Promise(r => setTimeout(r, 300));

    if (el.tagName === 'TEXTAREA' || el.tagName === 'INPUT') {{
        const setter = Object.getOwnPropertyDescriptor(
            HTMLTextAreaElement.prototype, 'value')?.set
            || Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set;
        if (setter) setter.call(el, message); else el.value = message;
        el.dispatchEvent(new Event('input', {{ bubbles: true }}));
    }} else {{
        el.innerText = message;
        el.dispatchEvent(new Event('input', {{ bubbles: true }}));
    }}
    await new Promise(r => setTimeout(r, 300));
    el.dispatchEvent(new KeyboardEvent('keydown', {{ key: 'Enter', code: 'Enter', keyCode: 13, bubbles: true }}));
    el.dispatchEvent(new KeyboardEvent('keyup',   {{ key: 'Enter', code: 'Enter', keyCode: 13, bubbles: true }}));

    const maxWait = 90000, poll = 2000;
    let lastText = '', stable = 0;
    for (let t = 0; t < maxWait; t += poll) {{
        await new Promise(r => setTimeout(r, poll));
        const els = document.querySelectorAll(
            'div[data-message-author-role="assistant"], .agent-turn [data-message-author-role="assistant"], [class*="markdown"], [class*="assistant"]');
        const last = els.length ? els[els.length - 1] : null;
        const txt = (last?.textContent || '').replace(/[\u200B-\u200D\uFEFF]/g, '').trim();
        const stopBtn = document.querySelector('button.bg-black .icon-lg, [aria-label*="Stop"]');
        if (txt && txt !== lastText) {{ lastText = txt; stable = 0; }}
        else if (txt) {{ stable++; if (!stopBtn && stable >= 2) break; }}
    }}
    if (!lastText) throw new Error('[ChatGPT] DOM fallback: no reply detected');
    await __webai_stream(lastText);
}}

/* ---- Main flow ---- */
const sentinelRes = await Promise.race([
    trySentinelFetch(),
    new Promise(r => setTimeout(() => r(null), 15000)),
]);
const res = sentinelRes || await fetch('https://chatgpt.com/backend-api/conversation', {{
    method: 'POST', headers: baseHeaders(), body: JSON.stringify(body), credentials: 'include',
}});

if (!res.ok) {{
    if (res.status === 403) {{
        await domFallback();
    }} else {{
        const text = await res.text();
        throw new Error('[ChatGPT] ' + res.status + ' ' + text.slice(0, 500));
    }}
}} else {{
    const reader = res.body.getReader();
    const decoder = new TextDecoder();
    while (true) {{
        const {{ done, value }} = await reader.read();
        if (done) break;
        await __webai_stream(decoder.decode(value, {{ stream: true }}));
    }}
}}
"#
    )
}

/// ChatGPT SSE events carry the full accumulated text in `message.content.parts[-1]`.
/// We return the full text and let the caller compute the incremental diff.
fn extract_accumulated_text(json_str: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let msg = v.get("message")?;
    if msg.get("author")?.get("role")?.as_str()? != "assistant" {
        return None;
    }
    let parts = msg.get("content")?.get("parts")?.as_array()?;
    let text = parts.last()?.as_str()?;
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}
