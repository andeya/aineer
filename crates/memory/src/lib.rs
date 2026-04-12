use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MemoryLevel {
    L0Identity,
    L1Essential,
    L2Context,
    L3Deep,
}

impl MemoryLevel {
    fn ordinal(self) -> u8 {
        match self {
            MemoryLevel::L0Identity => 0,
            MemoryLevel::L1Essential => 1,
            MemoryLevel::L2Context => 2,
            MemoryLevel::L3Deep => 3,
        }
    }
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

/// Cross-platform home directory: `HOME` (Unix/WSL), `USERPROFILE` (Windows).
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn memory_store_path() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".aineer")
        .join("memory.json")
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct MemoryStore {
    entries: Vec<MemoryEntry>,
}

impl MemoryStore {
    fn load() -> Self {
        let path = memory_store_path();
        if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn save(&self) -> Result<(), MemoryError> {
        let path = memory_store_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| MemoryError::Storage(e.to_string()))?;
        }
        let json =
            serde_json::to_string_pretty(self).map_err(|e| MemoryError::Storage(e.to_string()))?;
        std::fs::write(&path, json).map_err(|e| MemoryError::Storage(e.to_string()))
    }
}

/// Client interface for memory operations.
/// Falls back to local JSON storage; will delegate to MemPalace MCP when available.
pub struct MemoryClient {
    store: MemoryStore,
}

impl MemoryClient {
    pub fn new() -> Self {
        Self {
            store: MemoryStore::load(),
        }
    }

    /// Load L0+L1 memories at session startup
    pub fn wake_up(&self) -> Vec<MemoryEntry> {
        self.store
            .entries
            .iter()
            .filter(|e| e.level <= MemoryLevel::L1Essential.ordinal())
            .cloned()
            .collect()
    }

    /// Search relevant memories for a query (L2)
    pub fn search(&self, query: &str, limit: usize) -> Vec<MemoryEntry> {
        let q = query.to_lowercase();
        let mut results: Vec<_> = self
            .store
            .entries
            .iter()
            .filter(|e| e.content.to_lowercase().contains(&q))
            .cloned()
            .collect();
        results.sort_by(|a, b| b.access_count.cmp(&a.access_count));
        results.truncate(limit);
        results
    }

    /// Remove a memory entry by id.
    pub fn forget(&mut self, id: &str) -> Result<(), MemoryError> {
        let before = self.store.entries.len();
        self.store.entries.retain(|e| e.id != id);
        if self.store.entries.len() == before {
            return Err(MemoryError::NotFound(id.to_string()));
        }
        self.store.save()
    }

    /// Save a memory entry
    pub fn save(&mut self, entry: MemoryEntry) -> Result<(), MemoryError> {
        if let Some(existing) = self.store.entries.iter_mut().find(|e| e.id == entry.id) {
            existing.content = entry.content;
            existing.category = entry.category;
            existing.level = entry.level;
            existing.last_accessed = Utc::now();
            existing.access_count += 1;
        } else {
            self.store.entries.push(entry);
        }
        self.store.save()
    }

    /// Format loaded memories as context for AI prompts
    pub fn format_context(&self, level: MemoryLevel) -> String {
        let entries: Vec<_> = self
            .store
            .entries
            .iter()
            .filter(|e| e.level <= level.ordinal())
            .collect();

        if entries.is_empty() {
            return String::new();
        }

        let mut ctx = String::from("<memory>\n");
        for entry in &entries {
            ctx.push_str(&format!("- [{}] {}\n", entry.id, entry.content));
        }
        ctx.push_str("</memory>");
        ctx
    }

    pub fn entry_count(&self) -> usize {
        self.store.entries.len()
    }
}

impl Default for MemoryClient {
    fn default() -> Self {
        Self::new()
    }
}
