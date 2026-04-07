//! Language Server Protocol tool.
//!
//! The `Lsp` tool exposes LSP operations (hover, completion, go-to-definition,
//! references, symbols, rename, formatting, diagnostics) as a single tool with
//! an `operation` discriminator.  A dedicated multi-thread tokio runtime and
//! global [`LspManager`] singleton are lazily initialized from the
//! `CODINEER_LSP_SERVERS` env variable, or via [`initialize_lsp_manager`].

use std::future::Future;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use lsp::{
    diagnostic_severity_label, CompletionItem, DocumentSymbolInfo, HoverResult, LspManager,
    LspServerConfig, LspTextEdit, Position, SymbolLocation,
};
use serde::Serialize;
use serde_json::Value;

use crate::types::LspInput;

// ── Global LSP state ─────────────────────────────────────────────────────────

fn lsp_runtime() -> &'static tokio::runtime::Runtime {
    static LSP_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    LSP_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("codineer-lsp")
            .enable_all()
            .build()
            .expect("LSP tokio runtime should build")
    })
}

fn lsp_manager_slot() -> &'static Mutex<Option<Arc<LspManager>>> {
    static SLOT: OnceLock<Mutex<Option<Arc<LspManager>>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// Bridge an async future to a synchronous caller using the persistent LSP runtime.
/// When an ambient tokio context is detected, uses `block_in_place` to avoid blocking
/// the current scheduler thread.
fn lsp_block_on<F, R>(future: F) -> R
where
    F: Future<Output = R> + Send,
{
    let handle = lsp_runtime().handle().clone();
    if tokio::runtime::Handle::try_current().is_ok() {
        tokio::task::block_in_place(move || handle.block_on(future))
    } else {
        handle.block_on(future)
    }
}

fn get_or_init_manager() -> Result<Arc<LspManager>, String> {
    let mut guard = lsp_manager_slot()
        .lock()
        .map_err(|_| "LSP manager lock poisoned".to_string())?;

    if let Some(ref m) = *guard {
        return Ok(Arc::clone(m));
    }

    let servers_json = std::env::var("CODINEER_LSP_SERVERS").map_err(|_| {
        "LSP manager not configured. Set CODINEER_LSP_SERVERS to a JSON array of \
         LspServerConfig objects (see LspServerConfig type) to enable the Lsp tool."
            .to_string()
    })?;

    let configs: Vec<LspServerConfig> = serde_json::from_str(&servers_json)
        .map_err(|e| format!("invalid CODINEER_LSP_SERVERS JSON: {e}"))?;

    let manager = LspManager::new(configs).map_err(|e| e.to_string())?;
    let manager = Arc::new(manager);
    *guard = Some(Arc::clone(&manager));
    Ok(manager)
}

// ── Output types ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct LspHoverOutput {
    operation: &'static str,
    path: String,
    line: u32,
    character: u32,
    result: Option<HoverOutput>,
}

#[derive(Serialize)]
struct HoverOutput {
    contents: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<RangeOutput>,
}

#[derive(Serialize)]
struct LspCompletionOutput {
    operation: &'static str,
    path: String,
    line: u32,
    character: u32,
    items: Vec<CompletionOutput>,
}

#[derive(Serialize)]
struct CompletionOutput {
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    documentation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    insert_text: Option<String>,
}

#[derive(Serialize)]
struct LspLocationsOutput {
    operation: &'static str,
    path: String,
    locations: Vec<String>,
}

#[derive(Serialize)]
struct LspSymbolsOutput {
    operation: &'static str,
    path: String,
    symbols: Vec<SymbolOutput>,
}

#[derive(Serialize)]
struct SymbolOutput {
    name: String,
    kind: String,
    location: String,
}

#[derive(Serialize)]
struct LspEditsOutput {
    operation: &'static str,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_name: Option<String>,
    edits: Vec<FileEditOutput>,
}

#[derive(Serialize)]
struct FileEditOutput {
    file: String,
    changes: Vec<TextEditOutput>,
}

#[derive(Serialize)]
struct TextEditOutput {
    range: String,
    new_text: String,
}

#[derive(Serialize)]
struct LspDiagnosticsOutput {
    operation: &'static str,
    total: usize,
    diagnostics: Vec<DiagnosticOutput>,
}

#[derive(Serialize)]
struct DiagnosticOutput {
    file: String,
    line: u32,
    character: u32,
    severity: &'static str,
    message: String,
}

#[derive(Serialize)]
struct RangeOutput {
    start: String,
    end: String,
}

// ── Conversion helpers ────────────────────────────────────────────────────────

fn range_output(range: &lsp_types::Range) -> RangeOutput {
    RangeOutput {
        start: format!("{}:{}", range.start.line + 1, range.start.character + 1),
        end: format!("{}:{}", range.end.line + 1, range.end.character + 1),
    }
}

fn text_edit_output(edit: &LspTextEdit) -> TextEditOutput {
    TextEditOutput {
        range: format!(
            "{}:{}-{}:{}",
            edit.range.start.line + 1,
            edit.range.start.character + 1,
            edit.range.end.line + 1,
            edit.range.end.character + 1,
        ),
        new_text: edit.new_text.clone(),
    }
}

fn loc_string(loc: &SymbolLocation) -> String {
    format!(
        "{}:{}:{}",
        loc.path.display(),
        loc.range.start.line + 1,
        loc.range.start.character + 1,
    )
}

fn symbol_out(sym: &DocumentSymbolInfo) -> SymbolOutput {
    SymbolOutput {
        name: sym.name.clone(),
        kind: sym.kind.clone(),
        location: loc_string(&sym.location),
    }
}

fn hover_out(result: Option<HoverResult>) -> Option<HoverOutput> {
    result.map(|h| HoverOutput {
        contents: h.contents,
        range: h.range.as_ref().map(range_output),
    })
}

fn completion_out(items: Vec<CompletionItem>) -> Vec<CompletionOutput> {
    items
        .into_iter()
        .map(|item| CompletionOutput {
            label: item.label,
            kind: item.kind,
            detail: item.detail,
            documentation: item.documentation,
            insert_text: item.insert_text,
        })
        .collect()
}

// ── Main entry point ──────────────────────────────────────────────────────────

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn execute_lsp(input: LspInput) -> Result<String, String> {
    let manager = get_or_init_manager()?;
    let path = Path::new(&input.path).to_path_buf();
    let line = input.line.unwrap_or(0);
    let character = input.character.unwrap_or(0);
    let position = Position::new(line, character);

    let result_json: Result<Value, String> = match input.operation.as_str() {
        "hover" => {
            let r = lsp_block_on(async move { manager.hover(&path, position).await })
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(LspHoverOutput {
                operation: "hover",
                path: input.path,
                line,
                character,
                result: hover_out(r),
            })
            .expect("serialize should succeed"))
        }

        "completion" => {
            let items = lsp_block_on(async move { manager.completion(&path, position).await })
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(LspCompletionOutput {
                operation: "completion",
                path: input.path,
                line,
                character,
                items: completion_out(items),
            })
            .expect("serialize should succeed"))
        }

        "go_to_definition" => {
            let locs = lsp_block_on(async move { manager.go_to_definition(&path, position).await })
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(LspLocationsOutput {
                operation: "go_to_definition",
                path: input.path,
                locations: locs.iter().map(loc_string).collect(),
            })
            .expect("serialize should succeed"))
        }

        "find_references" => {
            let locs =
                lsp_block_on(async move { manager.find_references(&path, position, true).await })
                    .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(LspLocationsOutput {
                operation: "find_references",
                path: input.path,
                locations: locs.iter().map(loc_string).collect(),
            })
            .expect("serialize should succeed"))
        }

        "document_symbols" => {
            let syms = lsp_block_on(async move { manager.document_symbols(&path).await })
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(LspSymbolsOutput {
                operation: "document_symbols",
                path: input.path,
                symbols: syms.iter().map(symbol_out).collect(),
            })
            .expect("serialize should succeed"))
        }

        "workspace_symbols" => {
            let query = input.query.clone().unwrap_or_default();
            let locs = lsp_block_on(async move { manager.workspace_symbols(&query).await })
                .map_err(|e| e.to_string())?;
            Ok(serde_json::to_value(LspLocationsOutput {
                operation: "workspace_symbols",
                path: input.path,
                locations: locs.iter().map(loc_string).collect(),
            })
            .expect("serialize should succeed"))
        }

        "rename" => {
            let new_name = input
                .new_name
                .clone()
                .ok_or_else(|| "rename requires `new_name`".to_string())?;
            let new_name2 = new_name.clone();
            let edits =
                lsp_block_on(async move { manager.rename(&path, position, &new_name2).await })
                    .map_err(|e| e.to_string())?;
            let file_edits: Vec<FileEditOutput> = edits
                .into_iter()
                .map(|(fp, te)| FileEditOutput {
                    file: fp.display().to_string(),
                    changes: te.iter().map(text_edit_output).collect(),
                })
                .collect();
            Ok(serde_json::to_value(LspEditsOutput {
                operation: "rename",
                path: input.path,
                new_name: Some(new_name),
                edits: file_edits,
            })
            .expect("serialize should succeed"))
        }

        "formatting" => {
            let tab_size = input.tab_size.unwrap_or(4);
            let insert_spaces = input.insert_spaces.unwrap_or(true);
            let edits =
                lsp_block_on(
                    async move { manager.formatting(&path, tab_size, insert_spaces).await },
                )
                .map_err(|e| e.to_string())?;
            let file_edits = vec![FileEditOutput {
                file: input.path.clone(),
                changes: edits.iter().map(text_edit_output).collect(),
            }];
            Ok(serde_json::to_value(LspEditsOutput {
                operation: "formatting",
                path: input.path,
                new_name: None,
                edits: file_edits,
            })
            .expect("serialize should succeed"))
        }

        "diagnostics" => {
            let diags = lsp_block_on(async move { manager.collect_workspace_diagnostics().await })
                .map_err(|e| e.to_string())?;
            let items: Vec<DiagnosticOutput> = diags
                .files
                .iter()
                .flat_map(|f| {
                    f.diagnostics.iter().map(|d| DiagnosticOutput {
                        file: f.path.display().to_string(),
                        line: d.range.start.line + 1,
                        character: d.range.start.character + 1,
                        severity: diagnostic_severity_label(d.severity),
                        message: d.message.replace('\n', " "),
                    })
                })
                .collect();
            Ok(serde_json::to_value(LspDiagnosticsOutput {
                operation: "diagnostics",
                total: diags.total_diagnostics(),
                diagnostics: items,
            })
            .expect("serialize should succeed"))
        }

        other => Err(format!(
            "unknown Lsp operation: `{other}`. Valid values: hover, completion, \
             go_to_definition, find_references, document_symbols, workspace_symbols, \
             rename, formatting, diagnostics"
        )),
    };

    result_json.and_then(|v| serde_json::to_string_pretty(&v).map_err(|e| e.to_string()))
}

/// Initialize the global LSP manager from a JSON config array.
/// Useful for testing or programmatic setup; the `CODINEER_LSP_SERVERS` env
/// variable triggers lazy auto-init on first `Lsp` tool call.
pub fn initialize_lsp_manager(configs_json: &Value) -> Result<(), String> {
    let configs: Vec<LspServerConfig> = serde_json::from_value(configs_json.clone())
        .map_err(|e| format!("invalid LSP config JSON: {e}"))?;
    let manager = LspManager::new(configs).map_err(|e| e.to_string())?;
    let mut guard = lsp_manager_slot()
        .lock()
        .map_err(|_| "LSP manager lock poisoned".to_string())?;
    *guard = Some(Arc::new(manager));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::Range;

    #[test]
    fn range_output_is_one_based() {
        let range = Range {
            start: lsp_types::Position::new(0, 0),
            end: lsp_types::Position::new(9, 4),
        };
        let out = range_output(&range);
        assert_eq!(out.start, "1:1");
        assert_eq!(out.end, "10:5");
    }

    #[test]
    fn text_edit_output_formatting() {
        let edit = lsp_types::TextEdit {
            range: Range {
                start: lsp_types::Position::new(2, 3),
                end: lsp_types::Position::new(5, 0),
            },
            new_text: "replacement".to_string(),
        };
        let out = text_edit_output(&edit);
        assert_eq!(out.range, "3:4-6:1");
        assert_eq!(out.new_text, "replacement");
    }

    #[test]
    fn hover_out_none_gives_none() {
        assert!(hover_out(None).is_none());
    }

    #[test]
    fn hover_out_some_converts() {
        let result = HoverResult {
            contents: "fn test()".to_string(),
            range: Some(Range {
                start: lsp_types::Position::new(0, 0),
                end: lsp_types::Position::new(0, 8),
            }),
        };
        let out = hover_out(Some(result)).unwrap();
        assert_eq!(out.contents, "fn test()");
        assert_eq!(out.range.as_ref().unwrap().start, "1:1");
    }

    #[test]
    fn completion_out_maps_fields() {
        let items = vec![CompletionItem {
            label: "foo".to_string(),
            kind: Some("Function".to_string()),
            detail: Some("detail".to_string()),
            documentation: None,
            insert_text: Some("foo()".to_string()),
        }];
        let out = completion_out(items);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].label, "foo");
        assert_eq!(out[0].kind.as_deref(), Some("Function"));
        assert_eq!(out[0].insert_text.as_deref(), Some("foo()"));
    }

    #[test]
    fn execute_lsp_rejects_unknown_operation() {
        let input = LspInput {
            operation: "nonexistent".to_string(),
            path: "/dev/null".to_string(),
            line: None,
            character: None,
            query: None,
            new_name: None,
            tab_size: None,
            insert_spaces: None,
        };
        // This will either fail because manager not configured or because op is invalid.
        let result = execute_lsp(input);
        assert!(result.is_err());
    }

    #[test]
    fn initialize_lsp_manager_rejects_bad_json() {
        let bad = serde_json::json!("not an array");
        let err = initialize_lsp_manager(&bad).unwrap_err();
        assert!(err.contains("invalid"), "got: {err}");
    }
}
