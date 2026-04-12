pub mod agent;
pub mod ai;
pub mod auto_update;
pub mod cache;
pub mod channels;
pub mod files;
pub mod gateway;
pub mod git;
pub mod lsp;
pub mod mcp;
pub mod memory;
pub mod plugins;
pub mod session;
pub mod settings;
pub mod shell;
pub mod slash_commands;
pub mod webai;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// Global block ID counter shared by AI and Agent commands to avoid ID collisions.
static NEXT_BLOCK_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn next_block_id() -> u64 {
    NEXT_BLOCK_ID.fetch_add(1, Ordering::Relaxed)
}

/// Resolve workspace cwd from an optional frontend-supplied path.
pub(crate) fn workspace_cwd_from(cwd: Option<&str>) -> PathBuf {
    if let Some(p) = cwd {
        let pb = PathBuf::from(p);
        if !pb.as_os_str().is_empty() {
            return pb;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
