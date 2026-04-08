//! Structured runtime error types.
//!
//! [`RuntimeError`] replaces the previous string-wrapper error with a rich
//! enum whose variants encode specific failure modes. This enables:
//! - Exhaustive `match` for error handling
//! - Typed recovery strategies (see `recovery.rs`)
//! - `#[from]` automatic conversion from downstream errors

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuntimeError {
    #[error("max iterations exceeded ({iterations})")]
    MaxIterations { iterations: usize },
    #[error("max turns exceeded ({turns})")]
    MaxTurns { turns: usize },
    #[error("empty model reply")]
    EmptyReply,
    #[error("stream ended without stop event")]
    IncompleteStream,
    #[error("context overflow: {message}")]
    ContextOverflow { message: String },
    #[error("hook prevented continuation: {reason}")]
    HookPrevented { reason: String },
    #[error("cancelled")]
    Cancelled,
    #[error("compaction failed: {0}")]
    Compaction(String),
    #[error("api error: {message}")]
    Api {
        status_code: u16,
        error_type: Option<String>,
        message: String,
    },
    #[error("tool error: {0}")]
    Tool(String),
    #[error("recovery exhausted ({kind}): {reason}")]
    RecoveryExhausted { kind: String, reason: String },
    #[error("{0}")]
    Other(String),
}

impl RuntimeError {
    /// Backward-compatible constructor mapping to `Other`.
    /// Prefer using specific variants when the error category is known.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }

    /// Create a structured API error with status code and error classification.
    #[must_use]
    pub fn api(status_code: u16, error_type: Option<String>, message: impl Into<String>) -> Self {
        Self::Api {
            status_code,
            error_type,
            message: message.into(),
        }
    }

    /// Check if this error represents a context overflow.
    #[must_use]
    pub fn is_context_overflow(&self) -> bool {
        matches!(self, Self::ContextOverflow { .. })
    }

    /// Check if this error was caused by cancellation.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }

    /// Check if this error is an API error.
    #[must_use]
    pub fn is_api(&self) -> bool {
        matches!(self, Self::Api { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_format() {
        let err = RuntimeError::MaxIterations { iterations: 100 };
        assert_eq!(err.to_string(), "max iterations exceeded (100)");

        let err = RuntimeError::ContextOverflow {
            message: "too long".into(),
        };
        assert_eq!(err.to_string(), "context overflow: too long");

        let err = RuntimeError::Cancelled;
        assert_eq!(err.to_string(), "cancelled");
    }

    #[test]
    fn error_classification() {
        assert!(RuntimeError::ContextOverflow {
            message: "x".into()
        }
        .is_context_overflow());
        assert!(!RuntimeError::Cancelled.is_context_overflow());
        assert!(RuntimeError::Cancelled.is_cancelled());
        assert!(RuntimeError::api(429, Some("rate_limit_error".into()), "slow down").is_api());
        assert!(!RuntimeError::Cancelled.is_api());
    }

    #[test]
    fn api_error_constructor() {
        let err = RuntimeError::api(413, Some("request_too_large".into()), "prompt is too long");
        assert_eq!(err.to_string(), "api error: prompt is too long");
        match err {
            RuntimeError::Api {
                status_code,
                error_type,
                message,
            } => {
                assert_eq!(status_code, 413);
                assert_eq!(error_type.as_deref(), Some("request_too_large"));
                assert_eq!(message, "prompt is too long");
            }
            _ => panic!("expected Api variant"),
        }
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RuntimeError>();
    }
}
