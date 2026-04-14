//! Hidden `WebviewWindow` pool for WebAI providers.
//!
//! ## Why the app can “hang” on `waiting for Tauri IPC bridge readiness`
//!
//! Automation expects `window.__TAURI__.event` inside the **remote** webview. Tauri only injects
//! that API on origins allowed by the `webai-remote-ipc` capability (`remote.urls`). If the site
//! redirects to a host **not** in that list, or the page never reaches a document where injection
//! runs, our poll will never see `__TAURI__` and will hit the load timeout.
//!
//! ## Why the whole process can panic on macOS (wry)
//!
//! On WKWebView, `URL()` / navigation `request.URL()` can be **nil** during some delegate callbacks.
//! Dependency **wry** (used by Tauri) still uses `.unwrap()` in places such as `url_from_webview`
//! (`wkwebview/mod.rs`) and `navigation_policy` (`request.URL().unwrap()`). A nil URL then aborts
//! the process with `called Option::unwrap() on a None value`. Upstream fix is to treat those
//! Objective‑C optionals as Rust `Option` and return an error or empty string instead of unwrapping.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Listener, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;

use crate::error::{WebAiError, WebAiResult};
use crate::page::WebAiPage;

const DEFAULT_PAGE_LOAD_TIMEOUT: Duration = Duration::from_secs(60);

struct PageEntry {
    page: WebAiPage,
    last_used: Instant,
}

/// Manages a pool of hidden WebView pages, one per provider.
///
/// Each provider gets a dedicated `WebviewWindow` loaded on its domain
/// so that in-page `fetch` and DOM operations carry the right cookies.
///
/// ## Cookie sharing with `webauth-*` windows
///
/// Both `webai-*` (hidden) and `webauth-*` (visible login) windows are
/// created **without** an explicit `data_directory`.  This means the Tauri
/// runtime maps them to the **same default `WebContext`**, which in turn
/// gives them the same underlying cookie / website-data store:
///
/// - **macOS**: `WKWebsiteDataStore::defaultDataStore`
/// - **Windows**: shared `ICoreWebView2Environment` (same user-data folder)
/// - **Linux**: shared `WebContext` → shared `WebsiteDataManager` + cookie file
///
/// If we ever set `data_directory` on one but not the other, or use
/// `data_store_identifier` / `with_environment`, the stores will diverge
/// and login cookies will **not** be visible to automation pages.
pub struct WebAiPageManager {
    app_handle: AppHandle,
    pages: HashMap<String, PageEntry>,
    page_load_timeout: Duration,
}

impl WebAiPageManager {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            pages: HashMap::new(),
            page_load_timeout: DEFAULT_PAGE_LOAD_TIMEOUT,
        }
    }

    pub fn set_page_load_timeout(&mut self, timeout: Duration) {
        self.page_load_timeout = timeout;
    }

    /// Get or lazily create the hidden WebView page for a provider.
    pub async fn get_or_create(
        &mut self,
        provider_id: &str,
        start_url: &str,
    ) -> WebAiResult<&WebAiPage> {
        if !self.pages.contains_key(provider_id) {
            let page = self.create_page(provider_id, start_url).await?;
            self.pages.insert(
                provider_id.to_string(),
                PageEntry {
                    page,
                    last_used: Instant::now(),
                },
            );
        } else {
            self.pages.get_mut(provider_id).unwrap().last_used = Instant::now();
        }
        self.pages
            .get(provider_id)
            .map(|e| &e.page)
            .ok_or_else(|| WebAiError::WindowCreation("page vanished unexpectedly".into()))
    }

    pub fn close(&mut self, provider_id: &str) {
        if let Some(entry) = self.pages.remove(provider_id) {
            let _ = entry.page.window().close();
        }
    }

    pub fn close_all(&mut self) {
        for (_, entry) in self.pages.drain() {
            let _ = entry.page.window().close();
        }
    }

    pub fn has_page(&self, provider_id: &str) -> bool {
        self.pages.contains_key(provider_id)
    }

    /// Get a reference to the underlying `WebviewWindow` for a provider (if it exists).
    pub fn get_window(&self, provider_id: &str) -> Option<&tauri::WebviewWindow> {
        self.pages.get(provider_id).map(|e| e.page.window())
    }

    pub fn list_pages(&self) -> Vec<String> {
        self.pages.keys().cloned().collect()
    }

    /// Close pages that have been idle longer than `timeout`.
    pub fn cleanup_idle(&mut self, timeout: Duration) {
        let now = Instant::now();
        let stale: Vec<String> = self
            .pages
            .iter()
            .filter(|(_, e)| now.duration_since(e.last_used) > timeout)
            .map(|(id, _)| id.clone())
            .collect();
        for id in stale {
            tracing::info!(provider = %id, "closing idle WebAI page");
            self.close(&id);
        }
    }

    async fn create_page(&self, provider_id: &str, start_url: &str) -> WebAiResult<WebAiPage> {
        let label = format!("webai-{provider_id}");
        tracing::info!(provider = %provider_id, %start_url, "creating WebAI page");
        let url: url::Url = start_url
            .parse()
            .map_err(|e| WebAiError::WindowCreation(format!("invalid URL: {e}")))?;

        let window = if let Some(existing) = self.app_handle.get_webview_window(&label) {
            tracing::info!(provider = %provider_id, "reusing existing WebView window");
            existing
        } else {
            tracing::info!(provider = %provider_id, %label, "building new WebView window");
            let mut wb =
                WebviewWindowBuilder::new(&self.app_handle, &label, WebviewUrl::External(url))
                    .title(format!("WebAI - {provider_id}"))
                    .visible(false);
            if let Some(ua) = crate::browser_user_agent() {
                wb = wb.user_agent(ua);
            }
            let w = wb
                .build()
                .map_err(|e| WebAiError::WindowCreation(e.to_string()))?;
            tracing::info!(provider = %provider_id, "WebView window built successfully");
            w
        };

        tracing::info!(provider = %provider_id, "waiting for Tauri IPC bridge readiness");
        let page = WebAiPage::new(window, provider_id.to_string());
        self.wait_for_tauri_ready(&page).await?;
        Ok(page)
    }

    /// Poll until `__TAURI__.event` is available in the remote WebView.
    ///
    /// Repeatedly injects a lightweight JS check via `eval()`. Once the
    /// Tauri IPC bridge is ready the script emits an event back to Rust,
    /// resolving the oneshot channel.
    async fn wait_for_tauri_ready(&self, page: &WebAiPage) -> WebAiResult<()> {
        let timeout = self.page_load_timeout;
        let (tx, rx) = oneshot::channel::<()>();
        let tx = Arc::new(std::sync::Mutex::new(Some(tx)));

        let ready_event = format!("webai-ready-{}", page.provider_id());
        let tx_ref = tx.clone();
        let unlisten_id = page.window().listen(&ready_event, move |_| {
            if let Some(sender) = tx_ref.lock().unwrap().take() {
                let _ = sender.send(());
            }
        });

        let check_js = format!(
            r#"(function c(){{if(window.__TAURI__&&window.__TAURI__.event){{window.__TAURI__.event.emit('{ready_event}',{{}})}}else{{setTimeout(c,100)}}}})()"#
        );

        let window = page.window().clone();
        let provider_for_log = page.provider_id().to_string();
        let poll_handle = tokio::spawn(async move {
            let mut tick = 0u32;
            loop {
                let _ = window.eval(&check_js);
                tokio::time::sleep(Duration::from_millis(500)).await;
                tick += 1;
                if tick == 10 {
                    // Log the actual URL after ~5s to detect redirects
                    let url_js = "window.__TAURI__?.event?.emit?.('__debug_url',{url:location.href}); void 0";
                    let _ = window.eval(url_js);
                    tracing::info!(
                        provider = %provider_for_log,
                        "IPC readiness: still waiting after 5s, __TAURI__ may not be available"
                    );
                }
            }
        });

        let result = tokio::time::timeout(timeout, rx).await;
        poll_handle.abort();
        page.window().unlisten(unlisten_id);

        match result {
            Ok(Ok(())) => {
                tracing::debug!(provider = %page.provider_id(), "WebAI page ready");
                Ok(())
            }
            Ok(Err(_)) => Err(WebAiError::ChannelClosed),
            Err(_) => {
                // Do not call `WebviewWindow::url()` here: on macOS, wry may `unwrap()` a nil
                // `WKWebView.URL()` and panic the whole app (see module docs above).
                tracing::warn!(
                    provider = %page.provider_id(),
                    ?timeout,
                    "WebAI page load timed out — __TAURI__ not available (verify remote.urls cover final redirect host)"
                );
                Err(WebAiError::Timeout(timeout))
            }
        }
    }
}

impl Drop for WebAiPageManager {
    fn drop(&mut self) {
        for (id, entry) in self.pages.drain() {
            tracing::debug!(provider = %id, "dropping WebAI page");
            let _ = entry.page.window().close();
        }
    }
}
