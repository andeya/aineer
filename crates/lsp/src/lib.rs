mod client;
mod error;
mod manager;
mod types;

pub use error::LspError;
pub use lsp_types::{DiagnosticSeverity, Position};
pub use manager::LspManager;
pub use types::{
    diagnostic_severity_label, CompletionItem, DocumentSymbolInfo, FileDiagnostics, HoverResult,
    LspContextEnrichment, LspServerConfig, LspTextEdit, SymbolLocation, WorkspaceDiagnostics,
};

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use lsp_types::{DiagnosticSeverity, Position};

    use crate::{LspManager, LspServerConfig};

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("lsp-{label}-{nanos}"))
    }

    fn python3_path() -> Option<String> {
        let candidates = ["python3", "/usr/bin/python3"];
        candidates.iter().find_map(|candidate| {
            Command::new(candidate)
                .arg("--version")
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|_| (*candidate).to_string())
        })
    }

    #[allow(clippy::too_many_lines)]
    fn write_mock_server_script(root: &std::path::Path) -> PathBuf {
        let script_path = root.join("mock_lsp_server.py");
        fs::write(
            &script_path,
            r#"import json
import sys


def read_message():
    headers = {}
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            return None
        if line == b"\r\n":
            break
        key, value = line.decode("utf-8").split(":", 1)
        headers[key.lower()] = value.strip()
    length = int(headers["content-length"])
    body = sys.stdin.buffer.read(length)
    return json.loads(body)


def write_message(payload):
    raw = json.dumps(payload).encode("utf-8")
    sys.stdout.buffer.write(f"Content-Length: {len(raw)}\r\n\r\n".encode("utf-8"))
    sys.stdout.buffer.write(raw)
    sys.stdout.buffer.flush()


opened_uri = None

while True:
    message = read_message()
    if message is None:
        break

    method = message.get("method")
    if method == "initialize":
        write_message({
            "jsonrpc": "2.0",
            "id": message["id"],
            "result": {
                "capabilities": {
                    "definitionProvider": True,
                    "referencesProvider": True,
                    "hoverProvider": True,
                    "completionProvider": {"triggerCharacters": ["."]},
                    "documentSymbolProvider": True,
                    "workspaceSymbolProvider": True,
                    "renameProvider": True,
                    "documentFormattingProvider": True,
                    "textDocumentSync": 1,
                }
            },
        })
    elif method == "initialized":
        continue
    elif method == "textDocument/didOpen":
        document = message["params"]["textDocument"]
        opened_uri = document["uri"]
        write_message({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": document["uri"],
                "diagnostics": [
                    {
                        "range": {
                            "start": {"line": 0, "character": 0},
                            "end": {"line": 0, "character": 3},
                        },
                        "severity": 1,
                        "source": "mock-server",
                        "message": "mock error",
                    }
                ],
            },
        })
    elif method == "textDocument/didChange":
        continue
    elif method == "textDocument/didSave":
        continue
    elif method == "textDocument/definition":
        uri = message["params"]["textDocument"]["uri"]
        write_message({
            "jsonrpc": "2.0",
            "id": message["id"],
            "result": [
                {
                    "uri": uri,
                    "range": {
                        "start": {"line": 0, "character": 0},
                        "end": {"line": 0, "character": 3},
                    },
                }
            ],
        })
    elif method == "textDocument/references":
        uri = message["params"]["textDocument"]["uri"]
        write_message({
            "jsonrpc": "2.0",
            "id": message["id"],
            "result": [
                {
                    "uri": uri,
                    "range": {
                        "start": {"line": 0, "character": 0},
                        "end": {"line": 0, "character": 3},
                    },
                },
                {
                    "uri": uri,
                    "range": {
                        "start": {"line": 1, "character": 4},
                        "end": {"line": 1, "character": 7},
                    },
                },
            ],
        })
    elif method == "textDocument/hover":
        write_message({
            "jsonrpc": "2.0",
            "id": message["id"],
            "result": {
                "contents": {
                    "kind": "markdown",
                    "value": "**mock hover** documentation",
                },
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 0, "character": 3},
                },
            },
        })
    elif method == "textDocument/completion":
        write_message({
            "jsonrpc": "2.0",
            "id": message["id"],
            "result": [
                {"label": "foo", "kind": 3},
                {"label": "bar", "kind": 6, "detail": "i32"},
            ],
        })
    elif method == "textDocument/documentSymbol":
        uri = message["params"]["textDocument"]["uri"]
        write_message({
            "jsonrpc": "2.0",
            "id": message["id"],
            "result": [
                {
                    "name": "main",
                    "kind": 12,
                    "location": {
                        "uri": uri,
                        "range": {
                            "start": {"line": 0, "character": 0},
                            "end": {"line": 0, "character": 12},
                        },
                    },
                }
            ],
        })
    elif method == "workspace/symbol":
        ws_uri = opened_uri if opened_uri else "file:///mock/symbol.rs"
        write_message({
            "jsonrpc": "2.0",
            "id": message["id"],
            "result": [
                {
                    "name": "MockSymbol",
                    "kind": 5,
                    "location": {
                        "uri": ws_uri,
                        "range": {
                            "start": {"line": 2, "character": 0},
                            "end": {"line": 2, "character": 10},
                        },
                    },
                }
            ],
        })
    elif method == "textDocument/rename":
        uri = message["params"]["textDocument"]["uri"]
        new_name = message["params"]["newName"]
        write_message({
            "jsonrpc": "2.0",
            "id": message["id"],
            "result": {
                "changes": {
                    uri: [
                        {
                            "range": {
                                "start": {"line": 0, "character": 0},
                                "end": {"line": 0, "character": 3},
                            },
                            "newText": new_name,
                        }
                    ]
                }
            },
        })
    elif method == "textDocument/formatting":
        uri = message["params"]["textDocument"]["uri"]
        write_message({
            "jsonrpc": "2.0",
            "id": message["id"],
            "result": [
                {
                    "range": {
                        "start": {"line": 0, "character": 0},
                        "end": {"line": 0, "character": 0},
                    },
                    "newText": "// formatted\n",
                }
            ],
        })
    elif method == "shutdown":
        write_message({"jsonrpc": "2.0", "id": message["id"], "result": None})
    elif method == "exit":
        break
"#,
        )
        .expect("mock server should be written");
        script_path
    }

    async fn wait_for_diagnostics(manager: &LspManager) {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if manager
                    .collect_workspace_diagnostics()
                    .await
                    .expect("diagnostics snapshot should load")
                    .total_diagnostics()
                    > 0
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("diagnostics should arrive from mock server");
    }

    fn make_manager(python: String, root: &std::path::Path, script_path: PathBuf) -> LspManager {
        LspManager::new(vec![LspServerConfig {
            name: "rust-analyzer".to_string(),
            command: python,
            args: vec![script_path.display().to_string()],
            env: BTreeMap::new(),
            workspace_root: root.to_path_buf(),
            initialization_options: None,
            extension_to_language: BTreeMap::from([(".rs".to_string(), "rust".to_string())]),
        }])
        .expect("manager should build")
    }

    #[tokio::test(flavor = "current_thread")]
    async fn collects_diagnostics_and_symbol_navigation_from_mock_server() {
        let Some(python) = python3_path() else {
            return;
        };

        let root = temp_dir("manager");
        fs::create_dir_all(root.join("src")).expect("workspace root should exist");
        let script_path = write_mock_server_script(&root);
        let source_path = root.join("src").join("main.rs");
        fs::write(&source_path, "fn main() {}\nlet value = 1;\n")
            .expect("source file should exist");
        let manager = make_manager(python, &root, script_path);
        manager
            .open_document(
                &source_path,
                &fs::read_to_string(&source_path).expect("source read should succeed"),
            )
            .await
            .expect("document should open");
        wait_for_diagnostics(&manager).await;

        let diagnostics = manager
            .collect_workspace_diagnostics()
            .await
            .expect("diagnostics should be available");
        let definitions = manager
            .go_to_definition(&source_path, Position::new(0, 0))
            .await
            .expect("definition request should succeed");
        let references = manager
            .find_references(&source_path, Position::new(0, 0), true)
            .await
            .expect("references request should succeed");

        assert_eq!(diagnostics.files.len(), 1);
        assert_eq!(diagnostics.total_diagnostics(), 1);
        assert_eq!(
            diagnostics.files[0].diagnostics[0].severity,
            Some(DiagnosticSeverity::ERROR)
        );
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].start_line(), 1);
        assert_eq!(references.len(), 2);

        manager.shutdown().await.expect("shutdown should succeed");
        fs::remove_dir_all(root).expect("temp workspace should be removed");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn hover_completion_symbols_rename_formatting_from_mock_server() {
        let Some(python) = python3_path() else {
            return;
        };

        let root = temp_dir("new-methods");
        fs::create_dir_all(root.join("src")).expect("workspace root should exist");
        let script_path = write_mock_server_script(&root);
        let source_path = root.join("src").join("lib.rs");
        fs::write(&source_path, "fn main() {}\n").expect("source file should exist");
        let manager = make_manager(python, &root, script_path);
        manager
            .open_document(&source_path, "fn main() {}\n")
            .await
            .expect("document should open");
        wait_for_diagnostics(&manager).await;

        // hover
        let hover = manager
            .hover(&source_path, Position::new(0, 0))
            .await
            .expect("hover should succeed");
        assert!(hover.is_some());
        assert!(hover.unwrap().contents.contains("mock hover"));

        // completion
        let items = manager
            .completion(&source_path, Position::new(0, 0))
            .await
            .expect("completion should succeed");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].label, "foo");
        assert_eq!(items[0].kind.as_deref(), Some("Function"));
        assert_eq!(items[1].label, "bar");
        assert_eq!(items[1].kind.as_deref(), Some("Variable"));

        // document symbols
        let symbols = manager
            .document_symbols(&source_path)
            .await
            .expect("document symbols should succeed");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "main");
        assert_eq!(symbols[0].kind, "Function");

        // workspace symbols
        let ws_symbols = manager
            .workspace_symbols("Mock")
            .await
            .expect("workspace symbols should succeed");
        assert_eq!(ws_symbols.len(), 1);
        assert_eq!(ws_symbols[0].start_line(), 3);

        // rename
        let edits = manager
            .rename(&source_path, Position::new(0, 0), "new_main")
            .await
            .expect("rename should succeed");
        assert!(!edits.is_empty());
        let (_, file_edits) = edits.into_iter().next().unwrap();
        assert_eq!(file_edits[0].new_text, "new_main");

        // formatting
        let fmt_edits = manager
            .formatting(&source_path, 4, true)
            .await
            .expect("formatting should succeed");
        assert_eq!(fmt_edits.len(), 1);
        assert!(fmt_edits[0].new_text.contains("formatted"));

        manager.shutdown().await.expect("shutdown should succeed");
        fs::remove_dir_all(root).expect("temp workspace should be removed");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn renders_runtime_context_enrichment_for_prompt_usage() {
        let Some(python) = python3_path() else {
            return;
        };

        let root = temp_dir("prompt");
        fs::create_dir_all(root.join("src")).expect("workspace root should exist");
        let script_path = write_mock_server_script(&root);
        let source_path = root.join("src").join("lib.rs");
        fs::write(&source_path, "pub fn answer() -> i32 { 42 }\n")
            .expect("source file should exist");
        let manager = make_manager(python, &root, script_path);
        manager
            .open_document(
                &source_path,
                &fs::read_to_string(&source_path).expect("source read should succeed"),
            )
            .await
            .expect("document should open");
        wait_for_diagnostics(&manager).await;

        let enrichment = manager
            .context_enrichment(&source_path, Position::new(0, 0))
            .await
            .expect("context enrichment should succeed");
        let rendered = enrichment.render_prompt_section();

        assert!(rendered.contains("# LSP context"));
        assert!(rendered.contains("Workspace diagnostics: 1 across 1 file(s)"));
        assert!(rendered.contains("Definitions:"));
        assert!(rendered.contains("References:"));
        assert!(rendered.contains("mock error"));

        manager.shutdown().await.expect("shutdown should succeed");
        fs::remove_dir_all(root).expect("temp workspace should be removed");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn from_json_config_builds_manager() {
        let config_json = serde_json::json!([
            {
                "name": "rust-analyzer",
                "command": "echo",
                "args": [],
                "env": {},
                "workspace_root": "/workspace",
                "initialization_options": null,
                "extension_to_language": {".rs": "rust"},
            }
        ]);
        let manager =
            LspManager::from_json_config(&config_json).expect("manager should build from JSON");
        assert!(manager.supports_path(std::path::Path::new("main.rs")));
        assert!(!manager.supports_path(std::path::Path::new("main.py")));
    }
}
