//! Agent protocol definitions shared across all Aineer crates:
//! events, observer, cancellation, errors, hook config, OAuth types.

pub mod cancel;
pub mod config;
pub mod credentials_types;
pub mod elicitation;
pub mod error;
pub mod events;
pub mod gemini_cache;
pub mod hook_config;
pub mod loop_state;
pub mod oauth;
pub mod observer;
pub mod prompt_types;
pub mod telemetry;

pub use credentials_types::{CredentialStatus, ResolvedCredential};
pub use gemini_cache::GeminiCacheConfig;
pub use hook_config::RuntimeHookConfig;
pub use oauth::{
    clear_oauth_credentials, credentials_path, generate_state, load_oauth_credentials,
    loopback_redirect_uri, save_oauth_credentials, OAuthAuthorizationRequest, OAuthCallbackParams,
    OAuthConfig, OAuthRefreshRequest, OAuthTokenExchangeRequest, OAuthTokenSet,
    PkceChallengeMethod, PkceCodePair,
};
