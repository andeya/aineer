mod client;
mod error;
mod providers;
mod sse;
mod types;

pub use client::{
    oauth_token_is_expired, read_base_url, read_xai_base_url, resolve_saved_oauth_token,
    resolve_startup_auth_source, MessageStream, OAuthTokenSet, ProviderClient,
};
pub use error::ApiError;
pub use providers::codineer_provider::{
    AuthSource, CodineerApiClient, CodineerApiClient as ApiClient,
