use crate::error::ApiError;
use crate::providers::codineer_provider::{self, AuthSource, CodineerApiClient};
use crate::providers::openai_compat::{self, OpenAiCompatClient, OpenAiCompatConfig};
use crate::providers::{self, Provider, ProviderKind};
use crate::types::{MessageRequest, MessageResponse, StreamEvent};

async fn send_via_provider<P: Provider>(
    provider: &P,
    request: &MessageRequest,
) -> Result<MessageResponse, ApiError> {
    provider.send_message(request).await
}

async fn stream_via_provider<P: Provider>(
    provider: &P,
    request: &MessageRequest,
) -> Result<P::Stream, ApiError> {
    provider.stream_message(request).await
}

#[derive(Debug, Clone)]
pub enum ProviderClient {
    CodineerApi(CodineerApiClient),
    Xai(OpenAiCompatClient),
    OpenAi(OpenAiCompatClient),
}

impl ProviderClient {
    pub fn from_model(model: &str) -> Result<Self, ApiError> {
        Self::from_model_with_default_auth(model, None)
    }

    pub fn from_model_with_default_auth(
        model: &str,
        default_auth: Option<AuthSource>,
    ) -> Result<Self, ApiError> {
        let resolved_model = providers::resolve_model_alias(model);
        match providers::detect_provider_kind(&resolved_model) {
            ProviderKind::CodineerApi => Ok(Self::CodineerApi(match default_auth {
                Some(auth) => CodineerApiClient::from_auth(auth),
                None => CodineerApiClient::from_env()?,
            })),
            ProviderKind::Xai => Ok(Self::Xai(OpenAiCompatClient::from_env(
                OpenAiCompatConfig::xai(),
            )?)),
            ProviderKind::OpenAi => Ok(Self::OpenAi(OpenAiCompatClient::from_env(
                OpenAiCompatConfig::openai(),
            )?)),
        }
    }

    #[must_use]
    pub const fn provider_kind(&self) -> ProviderKind {
        match self {
            Self::CodineerApi(_) => ProviderKind::CodineerApi,
            Self::Xai(_) => ProviderKind::Xai,
            Self::OpenAi(_) => ProviderKind::OpenAi,
        }
    }

    pub async fn send_message(
        &self,
        request: &MessageRequest,
    ) -> Result<MessageResponse, ApiError> {
        match self {
            Self::CodineerApi(client) => send_via_provider(client, request).await,
            Self::Xai(client) | Self::OpenAi(client) => send_via_provider(client, request).await,
        }
    }

    pub async fn stream_message(
        &self,
        request: &MessageRequest,
    ) -> Result<MessageStream, ApiError> {
        match self {
            Self::CodineerApi(client) => stream_via_provider(client, request)
                .await
                .map(MessageStream::CodineerApi),
            Self::Xai(client) | Self::OpenAi(client) => stream_via_provider(client, request)
                .await
                .map(MessageStream::OpenAiCompat),
        }
    }
}

#[derive(Debug)]
pub enum MessageStream {
    CodineerApi(codineer_provider::MessageStream),
    OpenAiCompat(openai_compat::MessageStream),
}

impl MessageStream {
    #[must_use]
    pub fn request_id(&self) -> Option<&str> {
        match self {
            Self::CodineerApi(stream) => stream.request_id(),
            Self::OpenAiCompat(stream) => stream.request_id(),
        }
    }

    pub async fn next_event(&mut self) -> Result<Option<StreamEvent>, ApiError> {
        match self {
            Self::CodineerApi(stream) => stream.next_event().await,
            Self::OpenAiCompat(stream) => stream.next_event().await,
        }
    }
