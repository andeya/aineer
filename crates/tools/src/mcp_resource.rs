//! MCP (Model Context Protocol) resource access tools.
//!
//! `ListMcpResources`, `ReadMcpResource`, and `MCPSearch` operate on an
//! in-process resource registry.  Resources can be seeded via the
//! `CODINEER_MCP_RESOURCES` environment variable (JSON array) or registered
//! programmatically with [`register_mcp_resource`].

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::builtin::BuiltinTool;
use crate::tool_output::{ToolError, ToolOutput};
use crate::types::{ListMcpResourcesInput, McpSearchInput, ReadMcpResourceInput};

// ── Registry ──────────────────────────────────────────────────────────────────

/// A single MCP resource entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    /// Unique URI identifying the resource (e.g. `mcp://server/resource/path`).
    pub uri: String,
    /// Display name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// MIME type of the content.
    pub mime_type: Option<String>,
    /// Resource content (text or base64 data URI for binary).
    pub content: String,
}

type Registry = BTreeMap<String, McpResource>;

static MCP_REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();

fn registry() -> &'static Mutex<Registry> {
    MCP_REGISTRY.get_or_init(|| {
        let mut map = Registry::new();
        // Seed from `CODINEER_MCP_RESOURCES` env var (JSON array of McpResource).
        if let Ok(raw) = std::env::var("CODINEER_MCP_RESOURCES") {
            if let Ok(resources) = serde_json::from_str::<Vec<McpResource>>(&raw) {
                for r in resources {
                    map.insert(r.uri.clone(), r);
                }
            }
        }
        Mutex::new(map)
    })
}

/// Register (or update) a resource programmatically.
pub fn register_mcp_resource(resource: McpResource) -> Result<(), String> {
    registry()
        .lock()
        .map_err(|_| "mcp tool registry poisoned".to_string())?
        .insert(resource.uri.clone(), resource);
    Ok(())
}

// ── Tool implementations ──────────────────────────────────────────────────────

pub(crate) fn execute_list_mcp_resources(input: ListMcpResourcesInput) -> Result<String, String> {
    let guard = registry()
        .lock()
        .map_err(|e| format!("mcp registry lock poisoned: {e}"))?;

    #[derive(Serialize)]
    struct ListItem {
        uri: String,
        name: String,
        description: Option<String>,
        mime_type: Option<String>,
    }

    let items: Vec<ListItem> = guard
        .values()
        .filter(|r| {
            input
                .server_filter
                .as_deref()
                .map(|f| r.uri.contains(f))
                .unwrap_or(true)
        })
        .map(|r| ListItem {
            uri: r.uri.clone(),
            name: r.name.clone(),
            description: r.description.clone(),
            mime_type: r.mime_type.clone(),
        })
        .collect();

    serde_json::to_string_pretty(&items).map_err(|e| format!("serialization error: {e}"))
}

pub(crate) fn execute_read_mcp_resource(input: ReadMcpResourceInput) -> Result<String, String> {
    let guard = registry()
        .lock()
        .map_err(|e| format!("mcp registry lock poisoned: {e}"))?;

    guard
        .get(&input.uri)
        .map(|r| r.content.clone())
        .ok_or_else(|| format!("MCP resource not found: {}", input.uri))
}

pub(crate) fn execute_mcp_search(input: McpSearchInput) -> Result<String, String> {
    let guard = registry()
        .lock()
        .map_err(|e| format!("mcp registry lock poisoned: {e}"))?;

    let query_lower = input.query.to_lowercase();

    #[derive(Serialize)]
    struct SearchResult {
        uri: String,
        name: String,
        description: Option<String>,
        snippet: String,
    }

    let results: Vec<SearchResult> = guard
        .values()
        .filter_map(|r| {
            let name_match = r.name.to_lowercase().contains(&query_lower);
            let desc_match = r
                .description
                .as_deref()
                .map(|d| d.to_lowercase().contains(&query_lower))
                .unwrap_or(false);
            let content_pos = r.content.to_lowercase().find(&query_lower);

            if name_match || desc_match || content_pos.is_some() {
                let snippet = if let Some(pos) = content_pos {
                    let lowered = r.content.to_lowercase();
                    let start = lowered[..pos]
                        .char_indices()
                        .rev()
                        .nth(59)
                        .map_or(0, |(i, _)| i);
                    let end_hint = pos + query_lower.len() + 60;
                    let end = lowered[..lowered.len().min(end_hint)]
                        .char_indices()
                        .last()
                        .map_or(lowered.len(), |(i, c)| i + c.len_utf8());
                    format!("...{}...", &r.content[start..end])
                } else {
                    String::new()
                };
                Some(SearchResult {
                    uri: r.uri.clone(),
                    name: r.name.clone(),
                    description: r.description.clone(),
                    snippet,
                })
            } else {
                None
            }
        })
        .collect();

    serde_json::to_string_pretty(&results).map_err(|e| format!("serialization error: {e}"))
}

// ---------------------------------------------------------------------------
// BuiltinTool adapters
// ---------------------------------------------------------------------------

pub(crate) struct ListMcpResourcesTool;

impl BuiltinTool for ListMcpResourcesTool {
    const NAME: &'static str = "ListMcpResources";
    type Input = ListMcpResourcesInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        execute_list_mcp_resources(input)
            .map(ToolOutput::ok)
            .map_err(ToolError::execution)
    }

    fn is_concurrency_safe(_input: &Self::Input) -> bool {
        true
    }
}

pub(crate) struct ReadMcpResourceTool;

impl BuiltinTool for ReadMcpResourceTool {
    const NAME: &'static str = "ReadMcpResource";
    type Input = ReadMcpResourceInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        execute_read_mcp_resource(input)
            .map(ToolOutput::ok)
            .map_err(ToolError::execution)
    }

    fn is_concurrency_safe(_input: &Self::Input) -> bool {
        true
    }
}

pub(crate) struct McpSearchTool;

impl BuiltinTool for McpSearchTool {
    const NAME: &'static str = "MCPSearch";
    type Input = McpSearchInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        execute_mcp_search(input)
            .map(ToolOutput::ok)
            .map_err(ToolError::execution)
    }

    fn is_concurrency_safe(_input: &Self::Input) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ListMcpResourcesInput, McpSearchInput, ReadMcpResourceInput};

    fn seed_resource(uri: &str, name: &str, content: &str) {
        register_mcp_resource(McpResource {
            uri: uri.to_string(),
            name: name.to_string(),
            description: Some(format!("desc for {name}")),
            mime_type: Some("text/plain".to_string()),
            content: content.to_string(),
        })
        .unwrap();
    }

    #[test]
    fn list_read_search_roundtrip() {
        seed_resource("mcp://test/alpha", "Alpha", "alpha content");
        seed_resource("mcp://test/beta", "Beta", "beta content");

        let list = execute_list_mcp_resources(ListMcpResourcesInput {
            server_filter: Some("test".to_string()),
        })
        .unwrap();
        assert!(list.contains("Alpha"));
        assert!(list.contains("Beta"));

        let content = execute_read_mcp_resource(ReadMcpResourceInput {
            uri: "mcp://test/alpha".to_string(),
        })
        .unwrap();
        assert_eq!(content, "alpha content");

        let search = execute_mcp_search(McpSearchInput {
            query: "beta".to_string(),
        })
        .unwrap();
        assert!(search.contains("Beta"));
        assert!(search.contains("beta content"));
    }

    #[test]
    fn read_missing_resource_returns_error() {
        let err = execute_read_mcp_resource(ReadMcpResourceInput {
            uri: "mcp://nonexistent/xyz".to_string(),
        })
        .unwrap_err();
        assert!(err.contains("not found"));
    }

    #[test]
    fn search_no_match_returns_empty() {
        let result = execute_mcp_search(McpSearchInput {
            query: "zzz_no_match_zzz".to_string(),
        })
        .unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
        assert!(parsed.is_empty());
    }
}
