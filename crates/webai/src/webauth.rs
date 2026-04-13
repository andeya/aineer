use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Listener, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;

use crate::auth_store;
use crate::error::{WebAiError, WebAiResult};
use crate::provider::ProviderConfig;

/// Credentials captured from a webauth session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthCredentials {
    pub provider_id: String,
}

/// HTML page shown in the helper WebView window.
/// Provides a step-by-step guide and buttons for the system-browser login flow.
fn confirmation_html(provider_name: &str, login_url: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Login to {provider_name} — Aineer WebAuth</title>
<style>
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: #0f0f10; color: #e4e4e7;
    display: flex; align-items: center; justify-content: center;
    height: 100vh; padding: 2rem;
  }}
  .card {{
    max-width: 420px; width: 100%; text-align: center;
    background: #18181b; border: 1px solid #27272a; border-radius: 12px;
    padding: 2.5rem 2rem;
  }}
  h2 {{ font-size: 1.25rem; margin-bottom: 0.75rem; color: #fafafa; }}
  .url {{
    display: inline-block; font-size: 0.75rem; color: #71717a;
    background: #09090b; border: 1px solid #27272a; border-radius: 6px;
    padding: 0.4rem 0.8rem; margin-bottom: 1.5rem; word-break: break-all;
    font-family: 'SF Mono', Monaco, monospace;
  }}
  .btn {{
    display: inline-block; padding: 0.65rem 1.5rem; border-radius: 8px;
    font-size: 0.85rem; font-weight: 600; cursor: pointer; border: none;
    transition: all 0.15s; text-decoration: none;
  }}
  .btn-primary {{ background: #2563eb; color: #fff; }}
  .btn-primary:hover {{ background: #1d4ed8; }}
  .btn-success {{ background: #16a34a; color: #fff; }}
  .btn-success:hover {{ background: #15803d; }}
  .actions {{ display: flex; gap: 0.75rem; justify-content: center; flex-wrap: wrap; }}
  .step {{ display: flex; align-items: flex-start; gap: 0.75rem; text-align: left; margin-bottom: 0.75rem; }}
  .step-num {{
    flex-shrink: 0; width: 1.5rem; height: 1.5rem; border-radius: 50%;
    background: #27272a; color: #a1a1aa; font-size: 0.7rem; font-weight: 700;
    display: flex; align-items: center; justify-content: center;
  }}
  .step-text {{ font-size: 0.8rem; color: #a1a1aa; line-height: 1.5; }}
  .step-text b {{ color: #e4e4e7; }}
</style>
</head>
<body>
<div class="card">
  <h2>Login to {provider_name}</h2>
  <div style="margin-bottom:1.25rem">
    <div class="step">
      <span class="step-num">1</span>
      <span class="step-text">Click <b>"Open Browser"</b> to open the login page</span>
    </div>
    <div class="step">
      <span class="step-num">2</span>
      <span class="step-text">Complete the login in your browser</span>
    </div>
    <div class="step">
      <span class="step-num">3</span>
      <span class="step-text">Return here and click <b>"Login Complete"</b></span>
    </div>
  </div>
  <div class="url">{login_url}</div>
  <div class="actions">
    <button class="btn btn-primary" onclick="openBrowser()">Open Browser</button>
    <button class="btn btn-success" onclick="loginDone()">Login Complete ✓</button>
  </div>
</div>
<script>
  function openBrowser() {{
    if (window.__TAURI__ && window.__TAURI__.event) {{
      window.__TAURI__.event.emit('webauth-open-browser', {{}});
    }}
  }}
  function loginDone() {{
    if (window.__TAURI__ && window.__TAURI__.event) {{
      window.__TAURI__.event.emit('webauth-done', {{}});
    }}
  }}
</script>
</body>
</html>"#
    )
}

/// Launch the WebAuth flow.
///
/// Instead of loading the provider website directly in WKWebView (which fails
/// on older macOS due to JS compatibility), this shows a lightweight helper
/// window with instructions and opens the login URL in the system default
/// browser.  Modern browsers (Chrome, Firefox, Safari) handle all JS features
/// correctly.  After the user finishes logging in, they click "Login Complete"
/// or close the helper window.
pub async fn start_webauth(
    app_handle: &AppHandle,
    config: &ProviderConfig,
) -> WebAiResult<WebAuthCredentials> {
    let label = format!("webauth-{}", config.id);
    let html = confirmation_html(&config.name, &config.start_url);

    let window = WebviewWindowBuilder::new(
        app_handle,
        &label,
        WebviewUrl::App("index.html".into()),
    )
    .title(format!("Login to {} — Aineer WebAuth", config.name))
    .inner_size(520.0, 480.0)
    .resizable(false)
    .initialization_script(&format!(
        "document.addEventListener('DOMContentLoaded', function() {{ document.open(); document.write({}); document.close(); }});",
        serde_json::to_string(&html).unwrap_or_default()
    ))
    .build()
    .map_err(|e| WebAiError::WindowCreation(e.to_string()))?;

    let (tx, rx) = oneshot::channel::<()>();
    let tx = std::sync::Arc::new(std::sync::Mutex::new(Some(tx)));

    let login_url = config.start_url.clone();
    let tx_browser = tx.clone();
    let open_listener = app_handle.listen("webauth-open-browser", move |_event| {
        let _ = tx_browser; // prevent premature drop
        let _ = std::process::Command::new("open").arg(&login_url).spawn();
    });

    let tx_done = tx.clone();
    let done_listener = app_handle.listen("webauth-done", move |_event| {
        if let Some(sender) = tx_done.lock().unwrap().take() {
            let _ = sender.send(());
        }
    });

    let tx_close = tx;
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Destroyed = event {
            if let Some(sender) = tx_close.lock().unwrap().take() {
                let _ = sender.send(());
            }
        }
    });

    let _ = rx.await;

    app_handle.unlisten(open_listener);
    app_handle.unlisten(done_listener);

    let creds = WebAuthCredentials {
        provider_id: config.id.clone(),
    };

    auth_store::save_credentials(&config.id, &creds)?;
    tracing::info!(provider = %config.id, "WebAuth credentials saved");

    Ok(creds)
}

/// List all providers that have saved credentials.
pub fn list_authenticated() -> Vec<String> {
    auth_store::list_authorized_providers()
}

/// Remove saved credentials for a provider.
pub fn logout(provider_id: &str) -> WebAiResult<()> {
    auth_store::remove_credentials(provider_id)
}
