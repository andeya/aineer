# codineer-lsp

Language Server Protocol (LSP) client integration for [Codineer](https://github.com/andeya/codineer).

[中文文档](README_CN.md)

This crate provides the LSP transport layer used by Codineer to communicate with language servers. It manages server lifecycle (spawn, initialize, shutdown), JSON-RPC message framing, and async diagnostics polling via `tokio::sync::watch`.

### Supported operations

| Operation         | Method                                             |
| ----------------- | -------------------------------------------------- |
| Hover             | `textDocument/hover`                               |
| Completion        | `textDocument/completion`                          |
| Go to definition  | `textDocument/definition`                          |
| Find references   | `textDocument/references`                          |
| Document symbols  | `textDocument/documentSymbol`                      |
| Workspace symbols | `workspace/symbol`                                 |
| Rename            | `textDocument/rename`                              |
| Formatting        | `textDocument/formatting`                          |
| Diagnostics       | `textDocument/publishDiagnostics` (push) + polling |

### Architecture

- `LspClient` — per-server subprocess manager with stdin/stdout JSON-RPC framing.
- `LspManager` — multiplexer that routes requests to the correct server based on file extension, with async I/O via `tokio::fs`.
- Server capabilities are captured during `initialize` and exposed for feature detection.

## Note

This is an internal crate of the Codineer project. It is published to crates.io as a dependency of `codineer-cli` and is not intended for standalone use. API stability is not guaranteed outside the Codineer workspace.

## License

MIT — see [LICENSE](https://github.com/andeya/codineer/blob/main/LICENSE) for details.
