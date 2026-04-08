use std::path::PathBuf;

fn empty_entry_field_message(
    kind: &'static str,
    field: &'static str,
    name: &Option<String>,
) -> String {
    match name {
        Some(name) if !name.is_empty() => {
            format!("plugin {kind} `{name}` {field} cannot be empty")
        }
        _ => format!("plugin {kind} {field} cannot be empty"),
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PluginManifestValidationError {
    #[error("plugin manifest {field} cannot be empty")]
    EmptyField { field: &'static str },
    #[error("{}", empty_entry_field_message(.kind, .field, .name))]
    EmptyEntryField {
        kind: &'static str,
        field: &'static str,
        name: Option<String>,
    },
    #[error("plugin manifest permission `{permission}` must be one of read, write, or execute")]
    InvalidPermission { permission: String },
    #[error("plugin manifest permission `{permission}` is duplicated")]
    DuplicatePermission { permission: String },
    #[error("plugin {kind} `{name}` is duplicated")]
    DuplicateEntry { kind: &'static str, name: String },
    #[error("{kind} path `{path}` does not exist")]
    MissingPath { kind: &'static str, path: PathBuf },
    #[error("plugin tool `{tool_name}` inputSchema must be a JSON object")]
    InvalidToolInputSchema { tool_name: String },
    #[error(
        "plugin tool `{tool_name}` requiredPermission `{permission}` must be read-only, workspace-write, or danger-full-access"
    )]
    InvalidToolRequiredPermission {
        tool_name: String,
        permission: String,
    },
}

fn format_manifest_validation_errors(errors: &[PluginManifestValidationError]) -> String {
    errors
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("; ")
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{}", format_manifest_validation_errors(.0))]
    ManifestValidation(Vec<PluginManifestValidationError>),
    #[error("{0}")]
    InvalidManifest(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    CommandFailed(String),
}
