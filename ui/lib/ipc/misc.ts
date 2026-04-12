import { call } from "./call";

// Slash commands
export interface SlashCommandDef {
  name: string;
  description: string;
  argument_hint?: string;
}

export const getSlashCommands = () => call<SlashCommandDef[]>("get_slash_commands");
export const executeSlashCommand = (name: string, args?: string) =>
  call<string>("execute_slash_command", { name, args: args ?? null });

// Auto-update
export interface UpdateCheckResult {
  available: boolean;
  version?: string;
  downloadUrl?: string;
  releaseNotes?: string;
}

export const checkForUpdate = () => call<UpdateCheckResult>("check_for_update");
export const getUpdateChannel = () => call<string>("get_update_channel");

// Channels
export interface ChannelAdapterInfo {
  name: string;
  source: string;
  connected: boolean;
}

export const listChannelAdapters = () => call<ChannelAdapterInfo[]>("list_channel_adapters");

// MCP
export interface McpServerInfo {
  name: string;
  transport: string;
  running: boolean;
}

export interface McpToolCallRequest {
  serverName: string;
  toolName: string;
  arguments: unknown;
}

export const listMcpServers = () => call<McpServerInfo[]>("list_mcp_servers");
export const startMcpServer = (name: string) => call<void>("start_mcp_server", { name });
export const stopMcpServer = (name: string) => call<void>("stop_mcp_server", { name });
export const callMcpTool = (request: McpToolCallRequest) =>
  call<unknown>("call_mcp_tool", { request });

// LSP
export interface LspDiagnosticItem {
  file: string;
  line: number;
  character: number;
  severity: string;
  message: string;
}

export interface LspHoverInfo {
  contents: string;
  rangeStartLine: number;
  rangeEndLine: number;
}

export interface LspCompletionItem {
  label: string;
  kind?: string;
  detail?: string;
}

export const lspDiagnostics = (path: string) =>
  call<LspDiagnosticItem[]>("lsp_diagnostics", { path });
export const lspHover = (path: string, line: number, character: number) =>
  call<LspHoverInfo | null>("lsp_hover", { path, line, character });
export const lspCompletions = (path: string, line: number, character: number) =>
  call<LspCompletionItem[]>("lsp_completions", { path, line, character });

// Plugins
export interface PluginInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  kind: string;
  enabled: boolean;
}

export const listPlugins = () => call<PluginInfo[]>("list_plugins");
export const installPlugin = (name: string) => call<void>("install_plugin", { name });
export const uninstallPlugin = (name: string) => call<void>("uninstall_plugin", { name });

// Gateway
export interface GatewayStatusInfo {
  running: boolean;
  listenAddr?: string;
  status: string;
}

export const startGateway = () => call<GatewayStatusInfo>("start_gateway");
export const stopGateway = () => call<void>("stop_gateway");
export const getGatewayStatus = () => call<GatewayStatusInfo>("get_gateway_status");

// Memory
export interface MemoryEntryInfo {
  id: string;
  content: string;
  createdAt: string;
}

export const searchMemory = (query: string) => call<MemoryEntryInfo[]>("search_memory", { query });
export const remember = (content: string) => call<string>("remember", { content });
export const forget = (id: string) => call<void>("forget", { id });

// Session
export interface SessionInfo {
  id: string;
  title: string;
  updatedAt: string;
}

export const saveSession = (data: unknown) => call<string>("save_session", { data });
export const loadSession = (id: string) => call<unknown>("load_session", { id });
export const listSessions = () => call<SessionInfo[]>("list_sessions");
