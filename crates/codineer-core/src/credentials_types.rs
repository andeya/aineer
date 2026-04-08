//! Credential value types shared across crates (API clients, runtime resolvers).
//! Resolver traits and chains remain in `codineer-runtime`.

use std::fmt;

/// A successfully resolved credential ready for use in API requests.
#[non_exhaustive]
#[derive(Clone, PartialEq, Eq)]
pub enum ResolvedCredential {
    ApiKey(String),
    BearerToken(String),
    ApiKeyAndBearer {
        api_key: String,
        bearer_token: String,
    },
}

impl fmt::Debug for ResolvedCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ApiKey(_) => write!(f, "ResolvedCredential::ApiKey(***)"),
            Self::BearerToken(_) => write!(f, "ResolvedCredential::BearerToken(***)"),
            Self::ApiKeyAndBearer { .. } => {
                write!(f, "ResolvedCredential::ApiKeyAndBearer(***)")
            }
        }
    }
}

/// Status snapshot of a single resolver in the chain.
#[derive(Debug, Clone)]
pub struct CredentialStatus {
    pub id: String,
    pub display_name: String,
    pub available: bool,
    pub supports_login: bool,
}
