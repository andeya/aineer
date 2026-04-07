use std::collections::BTreeMap;
use std::process::Stdio;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value as JsonValue;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use crate::client::{McpClientBootstrap, McpClientTransport, McpStdioTransport};

use super::types::{
    JsonRpcId, JsonRpcRequest, JsonRpcResponse, McpInitializeClientInfo, McpInitializeParams,
    McpInitializeResult, McpListResourcesParams, McpListResourcesResult, McpListToolsParams,
    McpListToolsResult, McpReadResourceParams, McpReadResourceResult, McpToolCallParams,
    McpToolCallResult, McpTransportError,
};

#[derive(Debug)]
pub struct McpStdioProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl McpStdioProcess {
    pub fn spawn(transport: &McpStdioTransport) -> Result<Self, McpTransportError> {
        let mut command = Command::new(&transport.command);
        command
            .args(&transport.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        apply_env(&mut command, &transport.env);

        let mut child = command.spawn()?;
        let stdin = child.stdin.take().ok_or_else(|| {
            McpTransportError::Io(std::io::Error::other(
                "stdio MCP process missing stdin pipe",
            ))
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            McpTransportError::Io(std::io::Error::other(
                "stdio MCP process missing stdout pipe",
            ))
        })?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    pub async fn write_all(&mut self, bytes: &[u8]) -> Result<(), McpTransportError> {
        self.stdin.write_all(bytes).await?;
        Ok(())
    }

    pub async fn flush(&mut self) -> Result<(), McpTransportError> {
        self.stdin.flush().await?;
        Ok(())
    }

    pub async fn write_line(&mut self, line: &str) -> Result<(), McpTransportError> {
        self.write_all(line.as_bytes()).await?;
        self.write_all(b"\n").await?;
        self.flush().await
    }

    pub async fn read_line(&mut self) -> Result<String, McpTransportError> {
        let mut line = String::new();
        let bytes_read = self.stdout.read_line(&mut line).await?;
        if bytes_read == 0 {
            return Err(McpTransportError::Io(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "MCP stdio stream closed while reading line",
            )));
        }
        Ok(line)
    }

    pub async fn read_available(&mut self) -> Result<Vec<u8>, McpTransportError> {
        let mut buffer = vec![0_u8; 4096];
        let read = self.stdout.read(&mut buffer).await?;
        buffer.truncate(read);
        Ok(buffer)
    }

    pub async fn write_frame(&mut self, payload: &[u8]) -> Result<(), McpTransportError> {
        let encoded = encode_frame(payload);
        self.write_all(&encoded).await?;
        self.flush().await
    }

    pub async fn read_frame(&mut self) -> Result<Vec<u8>, McpTransportError> {
        const MAX_FRAME_SIZE: usize = 50 * 1024 * 1024; // 50 MiB
        let mut content_length = None;
        loop {
            let mut line = String::new();
            let bytes_read = self.stdout.read_line(&mut line).await?;
            if bytes_read == 0 {
                return Err(McpTransportError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "MCP stdio stream closed while reading headers",
                )));
            }
            if line == "\r\n" {
                break;
            }
            if let Some(value) = line
                .strip_prefix("Content-Length:")
                .or_else(|| line.strip_prefix("content-length:"))
            {
                let parsed =
                    value
                        .trim()
                        .parse::<usize>()
                        .map_err(|error| McpTransportError::Protocol {
                            message: format!("invalid Content-Length: {error}"),
                        })?;
                content_length = Some(parsed);
            }
        }

        let content_length = content_length.ok_or_else(|| McpTransportError::Protocol {
            message: "missing Content-Length header".into(),
        })?;
        if content_length > MAX_FRAME_SIZE {
            return Err(McpTransportError::Protocol {
                message: format!(
                    "MCP frame too large: {content_length} bytes exceeds {MAX_FRAME_SIZE} limit"
                ),
            });
        }
        let mut payload = vec![0_u8; content_length];
        self.stdout.read_exact(&mut payload).await?;
        Ok(payload)
    }

    pub async fn write_jsonrpc_message<T: Serialize>(
        &mut self,
        message: &T,
    ) -> Result<(), McpTransportError> {
        let body = serde_json::to_vec(message)?;
        self.write_frame(&body).await
    }

    pub async fn read_jsonrpc_message<T: DeserializeOwned>(
        &mut self,
    ) -> Result<T, McpTransportError> {
        let payload = self.read_frame().await?;
        Ok(serde_json::from_slice(&payload)?)
    }

    pub async fn send_request<T: Serialize>(
        &mut self,
        request: &JsonRpcRequest<T>,
    ) -> Result<(), McpTransportError> {
        self.write_jsonrpc_message(request).await
    }

    pub async fn read_response<T: DeserializeOwned>(
        &mut self,
    ) -> Result<JsonRpcResponse<T>, McpTransportError> {
        self.read_jsonrpc_message().await
    }

    pub async fn request<TParams: Serialize, TResult: DeserializeOwned>(
        &mut self,
        id: JsonRpcId,
        method: impl Into<String>,
        params: Option<TParams>,
    ) -> Result<JsonRpcResponse<TResult>, McpTransportError> {
        let method = method.into();
        let request = JsonRpcRequest::new(id.clone(), method.clone(), params);
        self.send_request(&request).await?;
        let response: JsonRpcResponse<TResult> = self.read_response().await?;
        if response.id != id {
            return Err(match (&id, &response.id) {
                (JsonRpcId::Number(expected), JsonRpcId::Number(actual)) => {
                    McpTransportError::IdMismatch {
                        expected: *expected,
                        actual: *actual,
                    }
                }
                _ => McpTransportError::Protocol {
                    message: format!(
                        "JSON-RPC response id mismatch for {method}: expected {id:?}, got {:?}",
                        response.id
                    ),
                },
            });
        }
        Ok(response)
    }

    pub async fn initialize(
        &mut self,
        id: JsonRpcId,
        params: McpInitializeParams,
    ) -> Result<JsonRpcResponse<McpInitializeResult>, McpTransportError> {
        self.request(id, "initialize", Some(params)).await
    }

    pub async fn list_tools(
        &mut self,
        id: JsonRpcId,
        params: Option<McpListToolsParams>,
    ) -> Result<JsonRpcResponse<McpListToolsResult>, McpTransportError> {
        self.request(id, "tools/list", params).await
    }

    pub async fn call_tool(
        &mut self,
        id: JsonRpcId,
        params: McpToolCallParams,
    ) -> Result<JsonRpcResponse<McpToolCallResult>, McpTransportError> {
        self.request(id, "tools/call", Some(params)).await
    }

    pub async fn list_resources(
        &mut self,
        id: JsonRpcId,
        params: Option<McpListResourcesParams>,
    ) -> Result<JsonRpcResponse<McpListResourcesResult>, McpTransportError> {
        self.request(id, "resources/list", params).await
    }

    pub async fn read_resource(
        &mut self,
        id: JsonRpcId,
        params: McpReadResourceParams,
    ) -> Result<JsonRpcResponse<McpReadResourceResult>, McpTransportError> {
        self.request(id, "resources/read", Some(params)).await
    }

    pub async fn terminate(&mut self) -> Result<(), McpTransportError> {
        self.child.kill().await?;
        Ok(())
    }

    pub async fn wait(&mut self) -> Result<std::process::ExitStatus, McpTransportError> {
        Ok(self.child.wait().await?)
    }

    pub(crate) async fn shutdown(&mut self) -> Result<(), McpTransportError> {
        if self.child.try_wait()?.is_none() {
            self.child.kill().await?;
        }
        let _ = self.child.wait().await?;
        Ok(())
    }
}

pub fn spawn_mcp_stdio_process(
    bootstrap: &McpClientBootstrap,
) -> Result<McpStdioProcess, McpTransportError> {
    match &bootstrap.transport {
        McpClientTransport::Stdio(transport) => McpStdioProcess::spawn(transport),
        other => Err(McpTransportError::Protocol {
            message: format!(
                "MCP bootstrap transport for {} is not stdio: {other:?}",
                bootstrap.server_name
            ),
        }),
    }
}

fn apply_env(command: &mut Command, env: &BTreeMap<String, String>) {
    for (key, value) in env {
        command.env(key, value);
    }
}

fn encode_frame(payload: &[u8]) -> Vec<u8> {
    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
    let mut framed = header.into_bytes();
    framed.extend_from_slice(payload);
    framed
}

pub(crate) fn default_initialize_params() -> McpInitializeParams {
    McpInitializeParams {
        protocol_version: "2025-03-26".to_string(),
        capabilities: JsonValue::Object(serde_json::Map::new()),
        client_info: McpInitializeClientInfo {
            name: "runtime".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    }
}
