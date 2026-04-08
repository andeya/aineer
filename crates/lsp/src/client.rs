use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use lsp_types::{
    CompletionResponse, Diagnostic, DocumentSymbolResponse, GotoDefinitionResponse,
    InitializeResult, Location, LocationLink, Position, PublishDiagnosticsParams,
    ServerCapabilities, TextEdit,
};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{oneshot, watch, Mutex};

use crate::error::LspError;
use crate::types::{
    CompletionItem, DocumentSymbolInfo, HoverResult, LspServerConfig, SymbolLocation,
};

type PendingMap = BTreeMap<i64, oneshot::Sender<Result<Value, LspError>>>;

pub(crate) struct LspClient {
    config: LspServerConfig,
    writer: Mutex<BufWriter<ChildStdin>>,
    child: Mutex<Child>,
    pending_requests: Arc<Mutex<PendingMap>>,
    diagnostics: Arc<Mutex<BTreeMap<String, Vec<Diagnostic>>>>,
    /// Monotonically increasing version; incremented on every `publishDiagnostics`.
    diag_version_tx: Arc<watch::Sender<u64>>,
    open_documents: Mutex<BTreeMap<PathBuf, i32>>,
    next_request_id: AtomicI64,
    server_capabilities: Arc<Mutex<ServerCapabilities>>,
}

impl LspClient {
    pub(crate) async fn connect(config: LspServerConfig) -> Result<Self, LspError> {
        let mut command = Command::new(&config.command);
        command
            .args(&config.args)
            .current_dir(&config.workspace_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(config.env.clone());

        let mut child = command.spawn()?;
        let stdin = child.stdin.take().ok_or_else(|| LspError::Protocol {
            message: "missing LSP stdin pipe".to_string(),
        })?;
        let stdout = child.stdout.take().ok_or_else(|| LspError::Protocol {
            message: "missing LSP stdout pipe".to_string(),
        })?;
        let stderr = child.stderr.take();

        let (diag_version_tx, _diag_version_rx) = watch::channel(0u64);
        let client = Self {
            config,
            writer: Mutex::new(BufWriter::new(stdin)),
            child: Mutex::new(child),
            pending_requests: Arc::new(Mutex::new(BTreeMap::new())),
            diagnostics: Arc::new(Mutex::new(BTreeMap::new())),
            diag_version_tx: Arc::new(diag_version_tx),
            open_documents: Mutex::new(BTreeMap::new()),
            next_request_id: AtomicI64::new(1),
            server_capabilities: Arc::new(Mutex::new(ServerCapabilities::default())),
        };

        client.spawn_reader(stdout);
        if let Some(stderr) = stderr {
            Self::spawn_stderr_drain(stderr);
        }
        if let Err(err) = client.initialize().await {
            let _ = client.child.lock().await.kill().await;
            return Err(err);
        }
        Ok(client)
    }

    pub(crate) async fn ensure_document_open(&self, path: &Path) -> Result<(), LspError> {
        if self.is_document_open(path).await {
            return Ok(());
        }
        let contents = tokio::fs::read_to_string(path).await?;
        self.open_document(path, &contents).await
    }

    pub(crate) async fn open_document(&self, path: &Path, text: &str) -> Result<(), LspError> {
        let uri = file_url(path)?;
        let language_id =
            self.config
                .language_id_for(path)
                .ok_or_else(|| LspError::UnsupportedDocument {
                    path: path.to_path_buf(),
                })?;

        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": 1,
                    "text": text,
                }
            }),
        )
        .await?;

        self.open_documents
            .lock()
            .await
            .insert(path.to_path_buf(), 1);
        Ok(())
    }

    pub(crate) async fn change_document(&self, path: &Path, text: &str) -> Result<(), LspError> {
        if !self.is_document_open(path).await {
            return self.open_document(path, text).await;
        }

        let uri = file_url(path)?;
        let next_version = {
            let mut open_documents = self.open_documents.lock().await;
            let version = open_documents
                .entry(path.to_path_buf())
                .and_modify(|value| *value += 1)
                .or_insert(1);
            *version
        };

        self.notify(
            "textDocument/didChange",
            json!({
                "textDocument": {
                    "uri": uri,
                    "version": next_version,
                },
                "contentChanges": [{
                    "text": text,
                }],
            }),
        )
        .await
    }

    pub(crate) async fn save_document(&self, path: &Path) -> Result<(), LspError> {
        if !self.is_document_open(path).await {
            return Ok(());
        }

        self.notify(
            "textDocument/didSave",
            json!({
                "textDocument": {
                    "uri": file_url(path)?,
                }
            }),
        )
        .await
    }

    pub(crate) async fn close_document(&self, path: &Path) -> Result<(), LspError> {
        if !self.is_document_open(path).await {
            return Ok(());
        }

        self.notify(
            "textDocument/didClose",
            json!({
                "textDocument": {
                    "uri": file_url(path)?,
                }
            }),
        )
        .await?;

        self.open_documents.lock().await.remove(path);
        Ok(())
    }

    pub(crate) async fn is_document_open(&self, path: &Path) -> bool {
        self.open_documents.lock().await.contains_key(path)
    }

    pub(crate) async fn go_to_definition(
        &self,
        path: &Path,
        position: Position,
    ) -> Result<Vec<SymbolLocation>, LspError> {
        self.ensure_document_open(path).await?;
        let response = self
            .request::<Option<GotoDefinitionResponse>>(
                "textDocument/definition",
                json!({
                    "textDocument": { "uri": file_url(path)? },
                    "position": position,
                }),
            )
            .await?;

        Ok(match response {
            Some(GotoDefinitionResponse::Scalar(location)) => {
                location_to_symbol_locations(vec![location])
            }
            Some(GotoDefinitionResponse::Array(locations)) => {
                location_to_symbol_locations(locations)
            }
            Some(GotoDefinitionResponse::Link(links)) => location_links_to_symbol_locations(links),
            None => Vec::new(),
        })
    }

    pub(crate) async fn find_references(
        &self,
        path: &Path,
        position: Position,
        include_declaration: bool,
    ) -> Result<Vec<SymbolLocation>, LspError> {
        self.ensure_document_open(path).await?;
        let response = self
            .request::<Option<Vec<Location>>>(
                "textDocument/references",
                json!({
                    "textDocument": { "uri": file_url(path)? },
                    "position": position,
                    "context": {
                        "includeDeclaration": include_declaration,
                    },
                }),
            )
            .await?;

        Ok(location_to_symbol_locations(response.unwrap_or_default()))
    }

    /// Request hover information at the given position.
    pub(crate) async fn hover(
        &self,
        path: &Path,
        position: Position,
    ) -> Result<Option<HoverResult>, LspError> {
        if self
            .server_capabilities
            .lock()
            .await
            .hover_provider
            .is_none()
        {
            return Ok(None);
        }
        self.ensure_document_open(path).await?;
        let response = self
            .request::<Option<lsp_types::Hover>>(
                "textDocument/hover",
                json!({
                    "textDocument": { "uri": file_url(path)? },
                    "position": position,
                }),
            )
            .await?;
        Ok(response.map(|h| HoverResult {
            contents: hover_contents_to_string(&h.contents),
            range: h.range,
        }))
    }

    /// Request code completion at the given position.
    pub(crate) async fn completion(
        &self,
        path: &Path,
        position: Position,
    ) -> Result<Vec<CompletionItem>, LspError> {
        if self
            .server_capabilities
            .lock()
            .await
            .completion_provider
            .is_none()
        {
            return Ok(Vec::new());
        }
        self.ensure_document_open(path).await?;
        let response = self
            .request::<Option<CompletionResponse>>(
                "textDocument/completion",
                json!({
                    "textDocument": { "uri": file_url(path)? },
                    "position": position,
                    "context": { "triggerKind": 1 },
                }),
            )
            .await?;

        let items = match response {
            None => return Ok(Vec::new()),
            Some(CompletionResponse::Array(items)) => items,
            Some(CompletionResponse::List(list)) => list.items,
        };
        Ok(items.into_iter().map(lsp_item_to_completion_item).collect())
    }

    /// Request document symbol outline.
    pub(crate) async fn document_symbols(
        &self,
        path: &Path,
    ) -> Result<Vec<DocumentSymbolInfo>, LspError> {
        if self
            .server_capabilities
            .lock()
            .await
            .document_symbol_provider
            .is_none()
        {
            return Ok(Vec::new());
        }
        self.ensure_document_open(path).await?;
        let response = self
            .request::<Option<DocumentSymbolResponse>>(
                "textDocument/documentSymbol",
                json!({
                    "textDocument": { "uri": file_url(path)? },
                }),
            )
            .await?;

        Ok(match response {
            None => Vec::new(),
            Some(DocumentSymbolResponse::Flat(items)) => items
                .into_iter()
                .filter_map(|info| {
                    uri_to_path(&info.location.uri.to_string()).map(|p| DocumentSymbolInfo {
                        name: info.name,
                        kind: symbol_kind_label(info.kind).to_string(),
                        location: SymbolLocation {
                            path: p,
                            range: info.location.range,
                        },
                    })
                })
                .collect(),
            Some(DocumentSymbolResponse::Nested(items)) => {
                let mut result = Vec::new();
                flatten_document_symbols(items, path, &mut result);
                result
            }
        })
    }

    /// Search workspace symbols matching `query`.
    pub(crate) async fn workspace_symbols(
        &self,
        query: &str,
    ) -> Result<Vec<SymbolLocation>, LspError> {
        if self
            .server_capabilities
            .lock()
            .await
            .workspace_symbol_provider
            .is_none()
        {
            return Ok(Vec::new());
        }
        let response = self
            .request::<Option<Value>>("workspace/symbol", json!({ "query": query }))
            .await?;

        let arr = match response {
            Some(Value::Array(arr)) => arr,
            _ => return Ok(Vec::new()),
        };

        let mut locations = Vec::new();
        for item in arr {
            if let Some(loc) = item.get("location") {
                let uri_str = loc.get("uri").and_then(Value::as_str);
                let range_val = loc.get("range");
                if let (Some(uri), Some(rv)) = (uri_str, range_val) {
                    if let (Some(path), Ok(range)) = (
                        uri_to_path(uri),
                        serde_json::from_value::<lsp_types::Range>(rv.clone()),
                    ) {
                        locations.push(SymbolLocation { path, range });
                    }
                }
            }
        }
        Ok(locations)
    }

    /// Rename a symbol at the given position.
    pub(crate) async fn rename(
        &self,
        path: &Path,
        position: Position,
        new_name: &str,
    ) -> Result<BTreeMap<PathBuf, Vec<TextEdit>>, LspError> {
        if self
            .server_capabilities
            .lock()
            .await
            .rename_provider
            .is_none()
        {
            return Ok(BTreeMap::new());
        }
        self.ensure_document_open(path).await?;
        let response = self
            .request::<Option<lsp_types::WorkspaceEdit>>(
                "textDocument/rename",
                json!({
                    "textDocument": { "uri": file_url(path)? },
                    "position": position,
                    "newName": new_name,
                }),
            )
            .await?;
        Ok(response.map(workspace_edit_to_changes).unwrap_or_default())
    }

    /// Format a document using the server's formatter.
    pub(crate) async fn formatting(
        &self,
        path: &Path,
        tab_size: u32,
        insert_spaces: bool,
    ) -> Result<Vec<TextEdit>, LspError> {
        if self
            .server_capabilities
            .lock()
            .await
            .document_formatting_provider
            .is_none()
        {
            return Ok(Vec::new());
        }
        self.ensure_document_open(path).await?;
        let response = self
            .request::<Option<Vec<TextEdit>>>(
                "textDocument/formatting",
                json!({
                    "textDocument": { "uri": file_url(path)? },
                    "options": {
                        "tabSize": tab_size,
                        "insertSpaces": insert_spaces,
                    },
                }),
            )
            .await?;
        Ok(response.unwrap_or_default())
    }

    /// Returns a snapshot of the server's declared capabilities.
    pub(crate) async fn server_capabilities(&self) -> ServerCapabilities {
        self.server_capabilities.lock().await.clone()
    }

    /// Returns the current diagnostics-update version counter.
    /// The counter increments every time the server sends `publishDiagnostics`.
    pub(crate) fn diagnostics_version(&self) -> u64 {
        *self.diag_version_tx.borrow()
    }

    /// Block until the diagnostics version reaches at least `min_version`, or until `timeout`
    /// elapses.  Returns a snapshot of all current diagnostics.
    pub(crate) async fn wait_for_diagnostics_update(
        &self,
        min_version: u64,
        timeout: Duration,
    ) -> BTreeMap<String, Vec<Diagnostic>> {
        let mut rx = self.diag_version_tx.subscribe();
        let _ = tokio::time::timeout(timeout, async {
            loop {
                if *rx.borrow_and_update() >= min_version {
                    break;
                }
                if rx.changed().await.is_err() {
                    break;
                }
            }
        })
        .await;
        self.diagnostics_snapshot().await
    }

    pub(crate) async fn diagnostics_snapshot(&self) -> BTreeMap<String, Vec<Diagnostic>> {
        self.diagnostics.lock().await.clone()
    }

    pub(crate) async fn shutdown(&self) -> Result<(), LspError> {
        let _ = self.request::<Value>("shutdown", json!({})).await;
        let _ = self.notify("exit", Value::Null).await;

        let mut child = self.child.lock().await;
        if child.kill().await.is_err() {
            let _ = child.wait().await;
            return Ok(());
        }
        let _ = child.wait().await;
        Ok(())
    }

    fn spawn_reader(&self, stdout: ChildStdout) {
        let diagnostics = self.diagnostics.clone();
        let pending_requests = self.pending_requests.clone();
        let diag_version_tx = self.diag_version_tx.clone();

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let result = async {
                while let Some(message) = read_message(&mut reader).await? {
                    if let Some(id) = message.get("id").and_then(Value::as_i64) {
                        let response = if let Some(error) = message.get("error") {
                            Err(LspError::Protocol {
                                message: error.to_string(),
                            })
                        } else {
                            Ok(message.get("result").cloned().unwrap_or(Value::Null))
                        };

                        if let Some(sender) = pending_requests.lock().await.remove(&id) {
                            let _ = sender.send(response);
                        }
                        continue;
                    }

                    let Some(method) = message.get("method").and_then(Value::as_str) else {
                        continue;
                    };
                    if method != "textDocument/publishDiagnostics" {
                        continue;
                    }

                    let params = message.get("params").cloned().unwrap_or(Value::Null);
                    let notification = serde_json::from_value::<PublishDiagnosticsParams>(params)?;
                    let mut diagnostics_map = diagnostics.lock().await;
                    if notification.diagnostics.is_empty() {
                        diagnostics_map.remove(&notification.uri.to_string());
                    } else {
                        diagnostics_map
                            .insert(notification.uri.to_string(), notification.diagnostics);
                    }
                    drop(diagnostics_map);
                    diag_version_tx.send_modify(|v| *v += 1);
                }
                Ok::<(), LspError>(())
            }
            .await;

            if let Err(error) = result {
                let mut pending = pending_requests.lock().await;
                let drained = pending.keys().copied().collect::<Vec<_>>();
                for id in drained {
                    if let Some(sender) = pending.remove(&id) {
                        let _ = sender.send(Err(LspError::Protocol {
                            message: error.to_string(),
                        }));
                    }
                }
            }
        });
    }

    fn spawn_stderr_drain<R>(stderr: R)
    where
        R: AsyncRead + Unpin + Send + 'static,
    {
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut buf = [0_u8; 4096];
            loop {
                match reader.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
            }
        });
    }

    async fn initialize(&self) -> Result<(), LspError> {
        let workspace_uri = file_url(&self.config.workspace_root)?;
        let result = self
            .request::<InitializeResult>(
                "initialize",
                json!({
                    "processId": std::process::id(),
                    "rootUri": workspace_uri,
                    "rootPath": self.config.workspace_root,
                    "workspaceFolders": [{
                        "uri": workspace_uri,
                        "name": self.config.name,
                    }],
                    "initializationOptions": self.config.initialization_options.clone().unwrap_or(Value::Null),
                    "capabilities": {
                        "textDocument": {
                            "publishDiagnostics": {
                                "relatedInformation": true,
                            },
                            "definition": {
                                "linkSupport": true,
                            },
                            "references": {},
                            "hover": {
                                "contentFormat": ["markdown", "plaintext"],
                            },
                            "completion": {
                                "completionItem": {
                                    "snippetSupport": false,
                                    "documentationFormat": ["markdown", "plaintext"],
                                },
                            },
                            "documentSymbol": {
                                "hierarchicalDocumentSymbolSupport": true,
                            },
                            "rename": {
                                "prepareSupport": false,
                            },
                            "formatting": {},
                        },
                        "workspace": {
                            "configuration": false,
                            "workspaceFolders": true,
                            "symbol": {},
                        },
                        "general": {
                            "positionEncodings": ["utf-16"],
                        }
                    }
                }),
            )
            .await?;
        *self.server_capabilities.lock().await = result.capabilities;
        self.notify("initialized", json!({})).await
    }

    async fn request<T>(&self, method: &str, params: Value) -> Result<T, LspError>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        self.pending_requests.lock().await.insert(id, sender);

        if let Err(error) = self
            .send_message(&json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params,
            }))
            .await
        {
            self.pending_requests.lock().await.remove(&id);
            return Err(error);
        }

        let response = tokio::time::timeout(std::time::Duration::from_secs(30), receiver)
            .await
            .map_err(|_| LspError::Protocol {
                message: format!("{method} request timed out after 30s"),
            })?
            .map_err(|_| LspError::Protocol {
                message: format!("request channel closed for {method}"),
            })??;
        Ok(serde_json::from_value(response)?)
    }

    async fn notify(&self, method: &str, params: Value) -> Result<(), LspError> {
        self.send_message(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }))
        .await
    }

    async fn send_message(&self, payload: &Value) -> Result<(), LspError> {
        let body = serde_json::to_vec(payload)?;
        let mut writer = self.writer.lock().await;
        writer
            .write_all(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes())
            .await?;
        writer.write_all(&body).await?;
        writer.flush().await?;
        Ok(())
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn hover_contents_to_string(contents: &lsp_types::HoverContents) -> String {
    match contents {
        lsp_types::HoverContents::Scalar(item) => match item {
            lsp_types::MarkedString::String(s) => s.clone(),
            lsp_types::MarkedString::LanguageString(ls) => ls.value.clone(),
        },
        lsp_types::HoverContents::Array(items) => items
            .iter()
            .map(|item| match item {
                lsp_types::MarkedString::String(s) => s.as_str(),
                lsp_types::MarkedString::LanguageString(ls) => ls.value.as_str(),
            })
            .collect::<Vec<_>>()
            .join("\n\n"),
        lsp_types::HoverContents::Markup(markup) => markup.value.clone(),
    }
}

fn lsp_item_to_completion_item(item: lsp_types::CompletionItem) -> CompletionItem {
    let documentation = item.documentation.map(|doc| match doc {
        lsp_types::Documentation::String(s) => s,
        lsp_types::Documentation::MarkupContent(mc) => mc.value,
    });
    CompletionItem {
        label: item.label,
        kind: completion_kind_label(item.kind),
        detail: item.detail,
        documentation,
        insert_text: item.insert_text,
    }
}

fn completion_kind_label(kind: Option<lsp_types::CompletionItemKind>) -> Option<String> {
    use lsp_types::CompletionItemKind as K;
    let k = kind?;
    let pairs: &[(lsp_types::CompletionItemKind, &str)] = &[
        (K::TEXT, "Text"),
        (K::METHOD, "Method"),
        (K::FUNCTION, "Function"),
        (K::CONSTRUCTOR, "Constructor"),
        (K::FIELD, "Field"),
        (K::VARIABLE, "Variable"),
        (K::CLASS, "Class"),
        (K::INTERFACE, "Interface"),
        (K::MODULE, "Module"),
        (K::PROPERTY, "Property"),
        (K::UNIT, "Unit"),
        (K::VALUE, "Value"),
        (K::ENUM, "Enum"),
        (K::KEYWORD, "Keyword"),
        (K::SNIPPET, "Snippet"),
        (K::COLOR, "Color"),
        (K::FILE, "File"),
        (K::REFERENCE, "Reference"),
        (K::FOLDER, "Folder"),
        (K::ENUM_MEMBER, "EnumMember"),
        (K::CONSTANT, "Constant"),
        (K::STRUCT, "Struct"),
        (K::EVENT, "Event"),
        (K::OPERATOR, "Operator"),
        (K::TYPE_PARAMETER, "TypeParameter"),
    ];
    pairs.iter().find_map(|(kv, label)| {
        if k == *kv {
            Some((*label).to_string())
        } else {
            None
        }
    })
}

fn symbol_kind_label(kind: lsp_types::SymbolKind) -> &'static str {
    use lsp_types::SymbolKind as K;
    let pairs: &[(lsp_types::SymbolKind, &str)] = &[
        (K::FILE, "File"),
        (K::MODULE, "Module"),
        (K::NAMESPACE, "Namespace"),
        (K::PACKAGE, "Package"),
        (K::CLASS, "Class"),
        (K::METHOD, "Method"),
        (K::PROPERTY, "Property"),
        (K::FIELD, "Field"),
        (K::CONSTRUCTOR, "Constructor"),
        (K::ENUM, "Enum"),
        (K::INTERFACE, "Interface"),
        (K::FUNCTION, "Function"),
        (K::VARIABLE, "Variable"),
        (K::CONSTANT, "Constant"),
        (K::STRING, "String"),
        (K::NUMBER, "Number"),
        (K::BOOLEAN, "Boolean"),
        (K::ARRAY, "Array"),
        (K::OBJECT, "Object"),
        (K::KEY, "Key"),
        (K::NULL, "Null"),
        (K::ENUM_MEMBER, "EnumMember"),
        (K::STRUCT, "Struct"),
        (K::EVENT, "Event"),
        (K::OPERATOR, "Operator"),
        (K::TYPE_PARAMETER, "TypeParameter"),
    ];
    pairs
        .iter()
        .find_map(|(kv, label)| if kind == *kv { Some(*label) } else { None })
        .unwrap_or("Unknown")
}

fn flatten_document_symbols(
    items: Vec<lsp_types::DocumentSymbol>,
    path: &Path,
    result: &mut Vec<DocumentSymbolInfo>,
) {
    for symbol in items {
        result.push(DocumentSymbolInfo {
            name: symbol.name,
            kind: symbol_kind_label(symbol.kind).to_string(),
            location: SymbolLocation {
                path: path.to_path_buf(),
                range: symbol.selection_range,
            },
        });
        flatten_document_symbols(symbol.children.unwrap_or_default(), path, result);
    }
}

fn workspace_edit_to_changes(edit: lsp_types::WorkspaceEdit) -> BTreeMap<PathBuf, Vec<TextEdit>> {
    let mut result: BTreeMap<PathBuf, Vec<TextEdit>> = BTreeMap::new();

    if let Some(changes) = edit.changes {
        for (uri, edits) in changes {
            if let Some(path) = uri_to_path(uri.as_str()) {
                result.entry(path).or_default().extend(edits);
            }
        }
    }

    if let Some(doc_changes) = edit.document_changes {
        match doc_changes {
            lsp_types::DocumentChanges::Edits(text_doc_edits) => {
                for doc_edit in text_doc_edits {
                    if let Some(path) = uri_to_path(doc_edit.text_document.uri.as_str()) {
                        let edits: Vec<TextEdit> = doc_edit
                            .edits
                            .into_iter()
                            .filter_map(|e| match e {
                                lsp_types::OneOf::Left(text_edit) => Some(text_edit),
                                lsp_types::OneOf::Right(_) => None,
                            })
                            .collect();
                        result.entry(path).or_default().extend(edits);
                    }
                }
            }
            lsp_types::DocumentChanges::Operations(_) => {}
        }
    }

    result
}

async fn read_message<R>(reader: &mut BufReader<R>) -> Result<Option<Value>, LspError>
where
    R: AsyncRead + Unpin,
{
    let mut content_length = None;

    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line).await?;
        if read == 0 {
            return Ok(None);
        }

        if line == "\r\n" {
            break;
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if let Some((name, value)) = trimmed.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                let value = value.trim().to_string();
                content_length =
                    Some(
                        value
                            .parse::<usize>()
                            .map_err(|_| LspError::InvalidContentLength {
                                value: value.clone(),
                            })?,
                    );
            }
        } else {
            return Err(LspError::InvalidHeader {
                header: trimmed.to_string(),
            });
        }
    }

    let content_length = content_length.ok_or(LspError::MissingContentLength)?;
    const MAX_BODY_SIZE: usize = 8 * 1024 * 1024;
    if content_length > MAX_BODY_SIZE {
        return Err(LspError::PayloadTooLarge {
            content_length,
            limit: MAX_BODY_SIZE,
        });
    }
    let mut body = vec![0_u8; content_length];
    reader.read_exact(&mut body).await?;
    Ok(Some(serde_json::from_slice(&body)?))
}

fn file_url(path: &Path) -> Result<String, LspError> {
    url::Url::from_file_path(path)
        .map(|url| url.to_string())
        .map_err(|()| LspError::PathToUrl {
            path: path.to_path_buf(),
        })
}

fn location_to_symbol_locations(locations: Vec<Location>) -> Vec<SymbolLocation> {
    locations
        .into_iter()
        .filter_map(|location| {
            uri_to_path(&location.uri.to_string()).map(|path| SymbolLocation {
                path,
                range: location.range,
            })
        })
        .collect()
}

fn location_links_to_symbol_locations(links: Vec<LocationLink>) -> Vec<SymbolLocation> {
    links
        .into_iter()
        .filter_map(|link| {
            uri_to_path(&link.target_uri.to_string()).map(|path| SymbolLocation {
                path,
                range: link.target_selection_range,
            })
        })
        .collect()
}

fn uri_to_path(uri: &str) -> Option<PathBuf> {
    url::Url::parse(uri).ok()?.to_file_path().ok()
}
