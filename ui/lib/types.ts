export type InputMode = "shell" | "ai" | "agent";

export type ToolPart = {
  type: string;
  state: "input-streaming" | "input-available" | "output-available" | "output-error";
  input?: Record<string, unknown>;
  output?: Record<string, unknown>;
  toolCallId?: string;
  errorText?: string;
};

export interface ShellResult {
  command: string;
  cwd: string;
  output: string;
  exitCode: number;
  durationMs: number;
  timedOut?: boolean;
}

export interface AgentStep {
  name: string;
  status: "pending" | "running" | "completed" | "failed";
}

export interface Attachment {
  id: string;
  type: "image" | "file";
  name: string;
  size: number;
  /** Data URL for frontend preview (ephemeral, not persisted) */
  previewUrl?: string;
  /** Absolute path after saved to cache dir via backend */
  cachePath?: string;
}

export interface ChatMessage {
  id: number;
  role: "user" | "assistant" | "system";
  mode: InputMode;
  content: string;
  timestamp: number;
  model?: string;
  shell?: ShellResult;
  thinking?: string;
  /** Wall time when first reasoning token arrived (cleared when duration is frozen). */
  thinkingStartedAt?: number;
  /** Milliseconds from first reasoning token to first answer token or stream end. */
  thinkingDurationMs?: number;
  agentSteps?: AgentStep[];
  toolCalls?: ToolPart[];
  attachments?: Attachment[];
}

export interface SlashCommand {
  name: string;
  description: string;
  icon?: string;
}

export const SLASH_COMMANDS: SlashCommand[] = [
  { name: "clear", description: "Clear conversation" },
  { name: "model", description: "Switch AI model" },
  { name: "context", description: "Manage project context" },
  { name: "memory", description: "Search project memory" },
  { name: "help", description: "Show available commands" },
  { name: "compact", description: "Summarize conversation" },
];

export type PreviewTab =
  | { type: "file"; path: string; content: string }
  | { type: "diff"; path: string; diff: string };

export interface AtMention {
  name: string;
  description: string;
  icon: string;
  /** When true, selecting this item opens a sub-picker (e.g. file browser) */
  hasPicker?: boolean;
}

export const AT_MENTIONS: AtMention[] = [
  {
    name: "Files & Folders",
    description: "Reference a file or folder",
    icon: "file",
    hasPicker: true,
  },
  { name: "Code", description: "Reference a code symbol", icon: "code", hasPicker: true },
  { name: "Docs", description: "Search documentation", icon: "book-open" },
  { name: "Terminals", description: "Reference terminal output", icon: "terminal" },
  { name: "Git", description: "Reference git changes", icon: "git-branch" },
  { name: "Web", description: "Search the web", icon: "globe" },
  { name: "Codebase", description: "Search entire codebase", icon: "search" },
];
