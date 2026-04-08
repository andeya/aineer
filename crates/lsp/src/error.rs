use std::path::PathBuf;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum LspError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("invalid LSP header: {header}")]
    InvalidHeader { header: String },
    #[error("missing LSP Content-Length header")]
    MissingContentLength,
    #[error("invalid LSP Content-Length value: {value}")]
    InvalidContentLength { value: String },
    #[error("no LSP server configured for {}", path.display())]
    UnsupportedDocument { path: PathBuf },
    #[error("unknown LSP server: {name}")]
    UnknownServer { name: String },
    #[error("duplicate LSP extension mapping for {extension}: {existing_server} and {new_server}")]
    DuplicateExtension {
        extension: String,
        existing_server: String,
        new_server: String,
    },
    #[error("failed to convert path to file URL: {}", path.display())]
    PathToUrl { path: PathBuf },
    #[error("LSP protocol error: {message}")]
    Protocol { message: String },
    #[error("LSP payload too large: Content-Length {content_length} exceeds {limit} byte limit")]
    PayloadTooLarge { content_length: usize, limit: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_formats_all_variants() {
        let io_err = LspError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "gone"));
        assert!(io_err.to_string().contains("gone"));

        let json_str = "not json";
        let json_err: serde_json::Error = serde_json::from_str::<bool>(json_str).unwrap_err();
        let json_display = LspError::Json(json_err);
        assert!(!json_display.to_string().is_empty());

        assert_eq!(
            LspError::InvalidHeader {
                header: "bad".into()
            }
            .to_string(),
            "invalid LSP header: bad"
        );
        assert_eq!(
            LspError::MissingContentLength.to_string(),
            "missing LSP Content-Length header"
        );
        assert_eq!(
            LspError::InvalidContentLength {
                value: "xyz".into()
            }
            .to_string(),
            "invalid LSP Content-Length value: xyz"
        );
        assert!(LspError::UnsupportedDocument {
            path: PathBuf::from("/foo.txt")
        }
        .to_string()
        .contains("/foo.txt"));
        assert_eq!(
            LspError::UnknownServer {
                name: "rust-analyzer".into()
            }
            .to_string(),
            "unknown LSP server: rust-analyzer"
        );
        assert!(LspError::DuplicateExtension {
            extension: ".rs".into(),
            existing_server: "a".into(),
            new_server: "b".into(),
        }
        .to_string()
        .contains(".rs"));
        assert!(LspError::PathToUrl {
            path: PathBuf::from("/bad")
        }
        .to_string()
        .contains("/bad"));
        assert_eq!(
            LspError::Protocol {
                message: "timeout".into()
            }
            .to_string(),
            "LSP protocol error: timeout"
        );
        assert!(LspError::PayloadTooLarge {
            content_length: 100_000_000,
            limit: 8_000_000,
        }
        .to_string()
        .contains("100000000"));
    }

    #[test]
    fn from_io_error_converts() {
        let err: LspError = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe").into();
        assert!(matches!(err, LspError::Io(_)));
    }

    #[test]
    fn from_json_error_converts() {
        let json_err: serde_json::Error = serde_json::from_str::<bool>("x").unwrap_err();
        let err: LspError = json_err.into();
        assert!(matches!(err, LspError::Json(_)));
    }
}
