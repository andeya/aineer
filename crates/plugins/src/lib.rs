//! Plugin lifecycle management: discovery, installation, hooks, and updates.

mod bundled;
mod constants;
mod definition;
pub mod directory_layout;
mod error;
mod install;
mod lifecycle;
mod manager;
mod manifest;
mod resolve;
mod types;

#[cfg(test)]
mod tests;

pub use definition::builtin_plugins;
pub use directory_layout::{scan_agent_files, scan_command_files};
pub use error::{PluginError, PluginManifestValidationError};
pub use types::*;
