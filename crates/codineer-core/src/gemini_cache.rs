/// Controls Gemini Context Caching (Google Generative Language API) for OpenAI-compat clients.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeminiCacheConfig {
    pub enabled: bool,
    pub ttl_seconds: u64,
}

impl Default for GeminiCacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ttl_seconds: 3600,
        }
    }
}
