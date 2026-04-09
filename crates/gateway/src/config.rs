use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub enabled: bool,
    pub listen_addr: String,
    pub default_model: Option<String>,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            listen_addr: "127.0.0.1:8090".to_string(),
            default_model: None,
        }
    }
}
