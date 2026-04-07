//! Plugin lifecycle management: discovery, installation, hooks, and updates.

mod bundled;
mod constants;
mod definition;
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
pub use error::{PluginError, PluginManifestValidationError};
pub use types::*;
