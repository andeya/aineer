import { call } from "./call";

export interface ShellContextSnippet {
  command: string;
  output: string;
}

export interface ChatHistoryTurn {
  role: "user" | "assistant";
  content: string;
}

export interface AiMessageRequest {
  message: string;
  model?: string;
  cwd?: string;
  shell_context?: ShellContextSnippet[];
  /** Completed prior turns for multi-turn chat (same tab). */
  chat_history?: ChatHistoryTurn[];
}

export interface AgentRequest {
  goal: string;
  cwd?: string;
  model?: string;
  shell_context?: ShellContextSnippet[];
  /** Completed prior turns for multi-turn agent (same tab, agent mode only). */
  chat_history?: ChatHistoryTurn[];
}

export const sendAiMessage = (req: AiMessageRequest) =>
  call<number>("send_ai_message", { request: req });
export const stopAiStream = (blockId: number) => call<void>("stop_ai_stream", { blockId });

export const startAgent = (req: AgentRequest) => call<number>("start_agent", { request: req });
export const approveTool = (blockId: number) => call<void>("approve_tool", { blockId });
export const denyTool = (blockId: number) => call<void>("deny_tool", { blockId });
export const stopAgent = (blockId: number) => call<void>("stop_agent", { blockId });
