use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("MCP communication error: {0}")]
    Mcp(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Memory not found: {0}")]
    NotFound(String),
}

/// Memory loading levels (borrowed from MemPalace L0-L3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemoryLevel {
    /// Core identity, always loaded (~200 tokens)
    L0Identity,
    /// Essential facts, loaded at session start (~2000 tokens)
    L1Essential,
    /// Context-relevant, loaded per request (~3000 tokens)
    L2Context,
    /// Deep search, loaded on demand (~5000 tokens)
    L3Deep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub category: MemoryCategory,
    pub level: u8,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryCategory {
    ProjectFact,
    UserPreference,
    Decision,
    Workflow,
    Custom(String),
}

/// Client interface for memory operations.
/// Delegates to MemPalace MCP server when available, falls back to local SQLite.
pub struct MemoryClient {
    _private: (),
}

impl MemoryClient {
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Load L0+L1 memories at session startup
    pub async fn wake_up(&self) -> Vec<MemoryEntry> {
        // TODO: Call mempalace_wake_up MCP tool
        Vec::new()
    }

    /// Search relevant memories for a query (L2)
    pub async fn search(&self, _query: &str, _limit: usize) -> Vec<MemoryEntry> {
        // TODO: Call mempalace_search MCP tool
        Vec::new()
    }

    /// Save a memory entry
    pub async fn save(&self, _entry: MemoryEntry) -> Result<(), MemoryError> {
        // TODO: Call mempalace_save MCP tool
        Ok(())
    }
}

impl Default for MemoryClient {
    fn default() -> Self {
        Self::new()
    }
}
