use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;

use crate::auth_store;
use crate::error::{WebAiError, WebAiResult};
use crate::provider::ProviderConfig;

/// Credentials captured from a webauth session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthCredentials {
    pub provider_id: String,
}

/// JS injected into the webauth page that adds a floating banner telling
/// the user to close the window after login.  Uses only basic DOM APIs
/// compatible with all WKWebView versions.  The IIFE guard prevents double
/// injection on SPA navigations.
const BANNER_INIT_JS: &str = r#"
(function(){
  if(window.__aineer_banner) return;
  window.__aineer_banner=true;
  function inject(){
    if(!document.body){setTimeout(inject,200);return;}
    var b=document.createElement('div');
    b.style.cssText='position:fixed;bottom:0;left:0;right:0;z-index:2147483647;background:linear-gradient(135deg,#1e293b,#0f172a);color:#e2e8f0;padding:10px 20px;display:flex;align-items:center;justify-content:space-between;font-family:-apple-system,BlinkMacSystemFont,sans-serif;font-size:13px;box-shadow:0 -2px 12px rgba(0,0,0,0.4);border-top:1px solid rgba(255,255,255,0.08);gap:12px;';
    var t=document.createElement('span');
    t.style.cssText='flex:1;opacity:0.9;';
    t.textContent='\u2139\uFE0F  Log in to your account, then close this window or click Done.';
    var d=document.createElement('button');
    d.textContent='\u2714  Done';
    d.style.cssText='background:#16a34a;color:#fff;border:none;padding:6px 20px;border-radius:6px;font-size:13px;font-weight:600;cursor:pointer;white-space:nowrap;transition:background 0.15s;';
    d.onmouseenter=function(){d.style.background='#15803d';};
    d.onmouseleave=function(){d.style.background='#16a34a';};
    d.onclick=function(){window.close();};
    b.appendChild(t);b.appendChild(d);
    document.body.appendChild(b);
  }
  inject();
  var _pushState=history.pushState;
  history.pushState=function(){_pushState.apply(history,arguments);inject();};
})();
"#;

/// Launch the WebAuth flow.
///
/// Opens the provider login page **directly inside a visible WKWebView window**.
/// This is critical because the hidden `webai-*` pages created by
/// [`WebAiPageManager`] share the same `WKWebsiteDataStore` — cookies set
/// during login are automatically available to them.
///
/// Using the system browser would NOT work: Safari/Chrome have a separate
/// cookie jar from WKWebView.
pub async fn start_webauth(
    app_handle: &AppHandle,
    config: &ProviderConfig,
) -> WebAiResult<WebAuthCredentials> {
    let label = format!("webauth-{}", config.id);

    // If a webauth window for this provider already exists, bring it to front.
    if let Some(existing) = app_handle.get_webview_window(&label) {
        let _ = existing.set_focus();
        return Err(WebAiError::Other(anyhow::anyhow!(
            "Login window for {} is already open",
            config.name
        )));
    }

    let url: url::Url = config
        .start_url
        .parse()
        .map_err(|e| WebAiError::WindowCreation(format!("invalid URL: {e}")))?;

    let window = WebviewWindowBuilder::new(app_handle, &label, WebviewUrl::External(url))
        .title(format!("Login to {} — Aineer", config.name))
        .inner_size(1024.0, 768.0)
        .resizable(true)
        .center()
        .initialization_script(BANNER_INIT_JS)
        .build()
        .map_err(|e| WebAiError::WindowCreation(e.to_string()))?;

    // Wait for the user to close the window (signals login completion).
    let (tx, rx) = oneshot::channel::<()>();
    let tx = std::sync::Mutex::new(Some(tx));

    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Destroyed = event {
            if let Some(sender) = tx.lock().unwrap().take() {
                let _ = sender.send(());
            }
        }
    });

    let _ = rx.await;

    // Record that the user has authenticated with this provider.
    // The actual session cookies live in WKWebView's shared WKWebsiteDataStore
    // and are automatically available to hidden webai-* pages.
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
