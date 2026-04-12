use crate::error::{AppError, AppResult};
use aineer_memory::{MemoryCategory, MemoryClient};
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub created_at: String,
}

impl From<aineer_memory::MemoryEntry> for MemoryEntry {
    fn from(e: aineer_memory::MemoryEntry) -> Self {
        Self {
            id: e.id,
            content: e.content,
            created_at: e.created_at.to_rfc3339(),
        }
    }
}

#[tauri::command]
pub async fn search_memory(query: String) -> AppResult<Vec<MemoryEntry>> {
    tracing::info!("search_memory: query={query}");
    let client = MemoryClient::new();
    let results = client.search(&query, 20);
    Ok(results.into_iter().map(MemoryEntry::from).collect())
}

#[tauri::command]
pub async fn remember(content: String) -> AppResult<String> {
    tracing::info!("remember: len={}", content.len());
    let mut client = MemoryClient::new();
    let id = format!("mem-{}", Utc::now().timestamp_millis());
    let entry = aineer_memory::MemoryEntry {
        id: id.clone(),
        content,
        category: MemoryCategory::ProjectFact,
        level: 2,
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 0,
    };
    client
        .save(entry)
        .map_err(|e| AppError::Memory(e.to_string()))?;
    Ok(id)
}

#[tauri::command]
pub async fn forget(id: String) -> AppResult<()> {
    tracing::info!("forget: id={id}");
    let mut client = MemoryClient::new();
    client
        .forget(&id)
        .map_err(|e| AppError::Memory(e.to_string()))
}
