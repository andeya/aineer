use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum WebAiError {
    #[error("WebView eval failed: {0}")]
    Eval(String),

    #[error("JavaScript execution error: {0}")]
    JsError(String),

    #[error("Timed out after {0:?} waiting for WebView response")]
    Timeout(Duration),

    #[error("Event listener channel closed unexpectedly")]
    ChannelClosed,

    #[error("Failed to create WebView window: {0}")]
    WindowCreation(String),

    #[error("Provider not authenticated: {provider_id}")]
    NotAuthenticated { provider_id: String },

    #[error("Session expired for provider {provider_id}. Please log in again.")]
    SessionExpired { provider_id: String },

    #[error("Provider {provider_id} rate limited. Please wait before retrying.")]
    RateLimited { provider_id: String },

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Stream ended unexpectedly")]
    StreamEnded,

    #[error("Deserialization failed: {0}")]
    Deserialize(#[from] serde_json::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl WebAiError {
    /// Classify a JS error message into a more specific error variant when possible.
    pub fn classify_js_error(provider_id: &str, msg: &str) -> Self {
        let lower = msg.to_ascii_lowercase();
        if lower.contains("not authenticated")
            || lower.contains("login")
            || lower.contains("unauthorized")
            || lower.contains("401")
        {
            return Self::NotAuthenticated {
                provider_id: provider_id.to_string(),
            };
        }
        if lower.contains("rate limit") || lower.contains("429") || lower.contains("too many") {
            return Self::RateLimited {
                provider_id: provider_id.to_string(),
            };
        }
        Self::JsError(msg.to_string())
    }
}

pub type WebAiResult<T> = Result<T, WebAiError>;
