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
