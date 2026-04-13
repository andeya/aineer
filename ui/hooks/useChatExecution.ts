import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import { isInteractiveCommand } from "@/lib/constants";
import type { ShellContextSnippet } from "@/lib/tauri";
import {
  executeCommand,
  executeSlashCommand,
  isTauri,
  sendAiMessage,
  startAgent,
  stopAgent,
  stopAiStream,
} from "@/lib/tauri";
import type { Attachment, ChatMessage, InputMode } from "@/lib/types";

interface AiStreamPayload {
  blockId: number;
  delta: string;
  /** `"text"` for formal output, `"thinking"` for model reasoning */
  kind: string;
  done: boolean;
}

interface StreamBinding {
  kind: "ai" | "agent";
  backendBlockId: number;
  assistantMsgId: number;
  safetyTimer: ReturnType<typeof setTimeout>;
}

interface UseChatExecutionOptions {
  projectRoot: string;
  modelName: string;
  dequeue: (channel: "chat" | "terminal") => { content: string; mode: InputMode } | undefined;
  enqueue: (channel: "chat" | "terminal", mode: InputMode, content: string) => void;
  tabs: { id: string; type: string }[];
  activeTab: { id: string; type: string } | undefined;
  markUnread: (id: string) => void;
  terminalRef: React.RefObject<{
    runCommand: (cmd: string) => void;
    resetShell: () => void;
  } | null>;
  termCommandActive: React.MutableRefObject<boolean>;
  setTerminalVisible: (v: boolean) => void;
  streamTimeoutMs: number;
}

const DEFAULT_STREAM_TIMEOUT_MS = 300_000;

/** Freeze wall-clock reasoning duration when the first answer token arrives or the stream ends. */
function freezeThinkingDuration(
  m: ChatMessage,
  now: number,
): Partial<Pick<ChatMessage, "thinkingDurationMs" | "thinkingStartedAt">> {
  if (m.thinkingStartedAt == null || !m.thinking?.trim() || m.thinkingDurationMs != null) {
    return {};
  }
  return {
    thinkingDurationMs: now - m.thinkingStartedAt,
    thinkingStartedAt: undefined,
  };
}

function recentShellContext(msgs: ChatMessage[], max: number): ShellContextSnippet[] {
  const out: ShellContextSnippet[] = [];
  for (let i = msgs.length - 1; i >= 0 && out.length < max; i--) {
    const m = msgs[i];
    if (m.mode === "shell" && m.shell) {
      out.push({ command: m.shell.command, output: m.shell.output });
    }
  }
  return out.reverse();
}

const MAX_CHAT_HISTORY_PAIRS = 24;

function assistantHistoryText(m: ChatMessage): string {
  const t = m.thinking?.trim();
  const c = m.content?.trim();
  if (t && c) return `${t}\n\n${c}`;
  return c || t || "";
}

/** Build OpenAI-style history from completed Chat or Agent turns (excludes the in-flight user message). */
function buildChatHistoryForApi(
  msgs: ChatMessage[],
  mode: "ai" | "agent",
): { role: "user" | "assistant"; content: string }[] {
  const out: { role: "user" | "assistant"; content: string }[] = [];
  for (let i = 0; i < msgs.length; i++) {
    const m = msgs[i];
    if (m.mode !== mode || m.role !== "user") continue;
    const next = msgs[i + 1];
    if (!next || next.role !== "assistant" || next.mode !== mode) continue;
    const u = m.content.trim();
    const a = assistantHistoryText(next);
    if (!u || !a) continue;
    out.push({ role: "user", content: u });
    out.push({ role: "assistant", content: a });
    i++;
  }
  const maxMsgs = MAX_CHAT_HISTORY_PAIRS * 2;
  return out.length > maxMsgs ? out.slice(-maxMsgs) : out;
}

export function useChatExecution({
  projectRoot,
  modelName,
  dequeue,
  enqueue,
  tabs,
  activeTab,
  markUnread,
  terminalRef,
  termCommandActive,
  setTerminalVisible,
  streamTimeoutMs,
}: UseChatExecutionOptions) {
  const safetyTimeoutMs = streamTimeoutMs > 0 ? streamTimeoutMs : DEFAULT_STREAM_TIMEOUT_MS;
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isStreaming, setIsStreaming] = useState(false);
  const nextIdRef = useRef(1);
  const abortRef = useRef<AbortController | null>(null);
  const pendingAttachmentsRef = useRef<Attachment[] | null>(null);
  const [inputMode, setInputMode] = useState<InputMode>("shell");

  // Session CWD persists across shell commands; initialized from projectRoot.
  const [sessionCwd, setSessionCwd] = useState(projectRoot);
  const sessionCwdRef = useRef(sessionCwd);
  sessionCwdRef.current = sessionCwd;

  // Sync sessionCwd when projectRoot first becomes available
  const projectRootInitialized = useRef(false);
  useEffect(() => {
    if (projectRoot && !projectRootInitialized.current) {
      projectRootInitialized.current = true;
      setSessionCwd(projectRoot);
    }
  }, [projectRoot]);

  const messagesRef = useRef(messages);
  messagesRef.current = messages;

  const modelNameRef = useRef(modelName);
  modelNameRef.current = modelName;

  const projectRootRef = useRef(projectRoot);
  projectRootRef.current = projectRoot;

  const assistantStreamRef = useRef<StreamBinding | null>(null);

  const clearAssistantStream = useCallback(() => {
    const cur = assistantStreamRef.current;
    if (cur) {
      clearTimeout(cur.safetyTimer);
      assistantStreamRef.current = null;
    }
  }, []);

  const processChatQueue = useCallback(() => {
    if (activeTab && activeTab.type !== "chat") {
      const chatTab = tabs.find((t) => t.type === "chat");
      if (chatTab) markUnread(chatTab.id);
    }
    const next = dequeue("chat");
    if (next) {
      executeChatTaskRef.current(next.content, next.mode);
    } else {
      setIsStreaming(false);
    }
  }, [activeTab, tabs, markUnread, dequeue]);

  const processChatQueueRef = useRef(processChatQueue);
  processChatQueueRef.current = processChatQueue;

  const allocId = useCallback(() => {
    const id = nextIdRef.current;
    nextIdRef.current += 1;
    return id;
  }, []);

  const executeChatTaskRef = useRef((_text: string, _mode: InputMode) => {
    /* set below */
  });

  const _executeShell = useCallback(
    async (text: string) => {
      const userId = allocId();
      setMessages((prev) => [
        ...prev,
        {
          id: userId,
          role: "user",
          mode: "shell",
          content: text,
          timestamp: Date.now(),
          attachments: pendingAttachmentsRef.current ?? undefined,
        },
      ]);
      pendingAttachmentsRef.current = null;
      setIsStreaming(true);

      const abort = new AbortController();
      abortRef.current = abort;

      let output: string;
      let exitCode: number;
      let durationMs: number;
      let timedOut = false;
      const cwd = sessionCwdRef.current || projectRootRef.current || undefined;
      const startMs = Date.now();

      if (isTauri()) {
        try {
          const resultPromise = executeCommand({ command: text, cwd, track_cwd: true });
          const abortPromise = new Promise<"aborted">((resolve) => {
            abort.signal.addEventListener("abort", () => resolve("aborted"), { once: true });
          });
          const race = await Promise.race([resultPromise, abortPromise]);

          if (race === "aborted") {
            output = "[Command stopped by user]";
            exitCode = 130;
            durationMs = Date.now() - startMs;
          } else {
            const result = race;
            const parts: string[] = [];
            if (result.stdout) parts.push(result.stdout);
            if (result.stderr) parts.push(result.stderr);
            const newCwd = result.final_cwd && result.final_cwd !== cwd ? result.final_cwd : null;
            if (newCwd) setSessionCwd(newCwd);
            output = parts.join("\n") || newCwd || "(no output)";
            exitCode = result.exit_code;
            durationMs = result.duration_ms;
            timedOut = result.timed_out;
          }
        } catch (err) {
          if (abort.signal.aborted) {
            output = "[Command stopped by user]";
            exitCode = 130;
            durationMs = Date.now() - startMs;
          } else {
            output = `Error: ${err}`;
            exitCode = 1;
            durationMs = 0;
          }
        }
      } else {
        await new Promise<void>((resolve) => {
          const timer = setTimeout(resolve, Math.floor(Math.random() * 500) + 50);
          abort.signal.addEventListener(
            "abort",
            () => {
              clearTimeout(timer);
              resolve();
            },
            { once: true },
          );
        });
        if (abort.signal.aborted) {
          output = "[Command stopped by user]";
          exitCode = 130;
          durationMs = Date.now() - startMs;
        } else {
          output = simulateShellOutput(text);
          exitCode = text.includes("fail") ? 1 : 0;
          durationMs = Date.now() - startMs;
        }
      }

      abortRef.current = null;

      const outId = allocId();
      const displayCwd = sessionCwdRef.current || cwd || "~";
      setMessages((prev) => [
        ...prev,
        {
          id: outId,
          role: "assistant",
          mode: "shell",
          content: text,
          timestamp: Date.now(),
          shell: {
            command: text,
            cwd: displayCwd,
            output: output.replace(/\n$/, ""),
            exitCode,
            durationMs,
            timedOut,
          },
        },
      ]);
      processChatQueueRef.current();
    },
    [allocId],
  );

  const _executeAi = useCallback(
    async (text: string) => {
      const userId = allocId();
      const atts = pendingAttachmentsRef.current ?? undefined;
      pendingAttachmentsRef.current = null;
      setMessages((prev) => [
        ...prev,
        {
          id: userId,
          role: "user",
          mode: "ai",
          content: text,
          timestamp: Date.now(),
          attachments: atts,
        },
      ]);
      setIsStreaming(true);

      if (isTauri()) {
        try {
          const shellContext = recentShellContext(messagesRef.current, 5);
          const chatHistory = buildChatHistoryForApi(messagesRef.current, "ai");
          const assistantId = allocId();
          const blockId = await sendAiMessage({
            message: text,
            model: modelNameRef.current || undefined,
            cwd: sessionCwdRef.current || projectRootRef.current || undefined,
            shell_context: shellContext.length > 0 ? shellContext : undefined,
            chat_history: chatHistory.length > 0 ? chatHistory : undefined,
          });
          const safetyTimer = setTimeout(() => {
            if (assistantStreamRef.current?.backendBlockId === blockId) {
              const msgId = assistantStreamRef.current.assistantMsgId;
              clearAssistantStream();
              const now = Date.now();
              setMessages((prev) =>
                prev.map((m) =>
                  m.id === msgId && !m.content
                    ? {
                        ...m,
                        content: "**Error:** Request timed out. Please try again.",
                        ...freezeThinkingDuration(m, now),
                      }
                    : m,
                ),
              );
              processChatQueueRef.current();
            }
          }, safetyTimeoutMs);
          assistantStreamRef.current = {
            kind: "ai",
            backendBlockId: blockId,
            assistantMsgId: assistantId,
            safetyTimer,
          };
          setMessages((prev) => [
            ...prev,
            {
              id: assistantId,
              role: "assistant",
              mode: "ai",
              content: "",
              timestamp: Date.now(),
              model: modelNameRef.current || undefined,
            },
          ]);
          return;
        } catch (err) {
          clearAssistantStream();
          setMessages((prev) => [
            ...prev,
            {
              id: allocId(),
              role: "assistant",
              mode: "ai",
              content: `**Error:** ${err}`,
              timestamp: Date.now(),
              model: modelNameRef.current || undefined,
            },
          ]);
          setIsStreaming(false);
          processChatQueueRef.current();
          return;
        }
      }

      setTimeout(() => {
        setMessages((prev) => [
          ...prev,
          {
            id: allocId(),
            role: "assistant",
            mode: "ai",
            content: simulateAIResponse(text),
            model: "claude-sonnet-4",
            timestamp: Date.now(),
          },
        ]);
        processChatQueueRef.current();
      }, 1200);
    },
    [allocId, clearAssistantStream, safetyTimeoutMs],
  );

  const _executeAgent = useCallback(
    async (text: string) => {
      const userId = allocId();
      const atts = pendingAttachmentsRef.current ?? undefined;
      pendingAttachmentsRef.current = null;
      setMessages((prev) => [
        ...prev,
        {
          id: userId,
          role: "user",
          mode: "agent",
          content: text,
          timestamp: Date.now(),
          attachments: atts,
        },
      ]);
      setIsStreaming(true);

      if (isTauri()) {
        try {
          const shellContext = recentShellContext(messagesRef.current, 5);
          const chatHistory = buildChatHistoryForApi(messagesRef.current, "agent");
          const assistantId = allocId();
          const blockId = await startAgent({
            goal: text,
            cwd: sessionCwdRef.current || projectRootRef.current || undefined,
            model: modelNameRef.current || undefined,
            shell_context: shellContext.length > 0 ? shellContext : undefined,
            chat_history: chatHistory.length > 0 ? chatHistory : undefined,
          });
          const safetyTimer = setTimeout(() => {
            if (assistantStreamRef.current?.backendBlockId === blockId) {
              const msgId = assistantStreamRef.current.assistantMsgId;
              clearAssistantStream();
              const now = Date.now();
              setMessages((prev) =>
                prev.map((m) =>
                  m.id === msgId && !m.content
                    ? {
                        ...m,
                        content: "**Error:** Request timed out. Please try again.",
                        ...freezeThinkingDuration(m, now),
                      }
                    : m,
                ),
              );
              processChatQueueRef.current();
            }
          }, safetyTimeoutMs);
          assistantStreamRef.current = {
            kind: "agent",
            backendBlockId: blockId,
            assistantMsgId: assistantId,
            safetyTimer,
          };
          setMessages((prev) => [
            ...prev,
            {
              id: assistantId,
              role: "assistant",
              mode: "agent",
              content: "",
              timestamp: Date.now(),
              model: modelNameRef.current || undefined,
            },
          ]);
          return;
        } catch (err) {
          clearAssistantStream();
          setMessages((prev) => [
            ...prev,
            {
              id: allocId(),
              role: "assistant",
              mode: "agent",
              content: `**Error:** ${err}`,
              timestamp: Date.now(),
              model: modelNameRef.current || undefined,
            },
          ]);
          setIsStreaming(false);
          processChatQueueRef.current();
          return;
        }
      }

      setTimeout(() => {
        setMessages((prev) => [
          ...prev,
          {
            id: allocId(),
            role: "assistant",
            mode: "agent",
            content: `I'll help you with: **${text}**`,
            model: "claude-sonnet-4",
            timestamp: Date.now(),
            thinking:
              "Analyzing the request...\nBreaking down into steps:\n1. Understand the goal\n2. Search relevant files\n3. Make changes\n4. Verify results",
            agentSteps: [
              { name: "Analyzing codebase", status: "completed" as const },
              { name: "Searching for relevant files", status: "completed" as const },
              { name: "Planning changes", status: "running" as const },
            ],
            toolCalls: [
              {
                type: "search_files",
                state: "output-available" as const,
                input: { query: text, path: "." },
                output: { matches: 3, files: ["src/main.rs", "lib.rs", "Cargo.toml"] },
              },
            ],
          },
        ]);
        processChatQueueRef.current();
      }, 1800);
    },
    [allocId, clearAssistantStream, safetyTimeoutMs],
  );

  const executeChatTask = useCallback(
    async (text: string, mode: InputMode) => {
      if (mode === "shell") {
        await _executeShell(text);
      } else if (mode === "ai") {
        await _executeAi(text);
      } else {
        await _executeAgent(text);
      }
    },
    [_executeAgent, _executeAi, _executeShell],
  );

  executeChatTaskRef.current = executeChatTask;

  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;
    let unlisten: UnlistenFn | undefined;
    const setup = async () => {
      /** Chat and Agent share the same payload and event (desktop protocol). */
      const u = await listen<AiStreamPayload>("ai_stream_delta", (event) => {
        const p = event.payload;
        const cur = assistantStreamRef.current;
        if (!cur || p.blockId !== cur.backendBlockId) return;
        if (p.delta) {
          if (p.kind === "thinking") {
            setMessages((prev) =>
              prev.map((m) => {
                if (m.id !== cur.assistantMsgId) return m;
                const nextThinking = (m.thinking || "") + p.delta;
                const thinkingStartedAt =
                  m.thinkingStartedAt ?? (nextThinking.trim().length > 0 ? Date.now() : undefined);
                return { ...m, thinking: nextThinking, thinkingStartedAt };
              }),
            );
          } else {
            const now = Date.now();
            setMessages((prev) =>
              prev.map((m) => {
                if (m.id !== cur.assistantMsgId) return m;
                return {
                  ...m,
                  content: m.content + p.delta,
                  ...freezeThinkingDuration(m, now),
                };
              }),
            );
          }
        }
        if (p.done) {
          const now = Date.now();
          setMessages((prev) =>
            prev.map((m) =>
              m.id === cur.assistantMsgId ? { ...m, ...freezeThinkingDuration(m, now) } : m,
            ),
          );
          clearTimeout(cur.safetyTimer);
          assistantStreamRef.current = null;
          processChatQueueRef.current();
        }
      });
      if (cancelled) {
        u();
        return;
      }
      unlisten = u;
    };
    setup().catch((err) => {
      console.error("Failed to register Tauri event listeners:", err);
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  const handleSubmit = useCallback(
    (text: string, mode: InputMode, attachments?: Attachment[]) => {
      const interactive = mode === "shell" && isInteractiveCommand(text);

      if (interactive) {
        if (termCommandActive.current) {
          enqueue("terminal", mode, text);
        } else {
          termCommandActive.current = true;
          setTerminalVisible(true);
          requestAnimationFrame(() => {
            terminalRef.current?.runCommand(text);
          });
        }
        return;
      }

      if (isStreaming) {
        enqueue("chat", mode, text);
        return;
      }
      pendingAttachmentsRef.current = attachments ?? null;
      executeChatTask(text, mode);
    },
    [isStreaming, enqueue, executeChatTask, termCommandActive, terminalRef, setTerminalVisible],
  );

  const handleStop = useCallback(() => {
    abortRef.current?.abort();
    const cur = assistantStreamRef.current;
    if (cur == null) return;
    if (cur.kind === "agent") {
      void stopAgent(cur.backendBlockId);
    } else {
      void stopAiStream(cur.backendBlockId);
    }
  }, []);

  const handleSlashCommand = useCallback(
    async (cmd: string) => {
      if (cmd === "clear") {
        setMessages([]);
        return;
      }

      const userMsg: ChatMessage = {
        id: nextIdRef.current++,
        role: "user",
        mode: inputMode,
        content: `/${cmd}`,
        timestamp: Date.now(),
      };
      setMessages((prev) => [...prev, userMsg]);

      let result: string;
      try {
        result = await executeSlashCommand(cmd);
      } catch (err) {
        result = `Error: ${err}`;
      }

      const sysMsg: ChatMessage = {
        id: nextIdRef.current++,
        role: "assistant",
        mode: inputMode,
        content: result,
        timestamp: Date.now(),
      };
      setMessages((prev) => [...prev, sysMsg]);
    },
    [inputMode],
  );

  const handleForceExecute = useCallback(
    (task: { channel: string; content: string; mode: InputMode }) => {
      if (task.channel === "terminal") {
        terminalRef.current?.runCommand(task.content);
        termCommandActive.current = true;
        setTerminalVisible(true);
      } else {
        abortRef.current?.abort();
        const cur = assistantStreamRef.current;
        if (cur != null) {
          const bid = cur.backendBlockId;
          clearAssistantStream();
          if (cur.kind === "agent") {
            void stopAgent(bid);
          } else {
            void stopAiStream(bid);
          }
        }
        setIsStreaming(false);
        executeChatTask(task.content, task.mode);
      }
    },
    [executeChatTask, clearAssistantStream, terminalRef, termCommandActive, setTerminalVisible],
  );

  return {
    messages,
    isStreaming,
    inputMode,
    setInputMode,
    sessionCwd,
    handleSubmit,
    handleStop,
    handleSlashCommand,
    handleForceExecute,
    executeChatTask,
  };
}

function simulateShellOutput(cmd: string): string {
  const c = cmd.trim().split(/\s+/)[0];
  const outputs: Record<string, string> = {
    ls: "Cargo.toml  Cargo.lock  app/  crates/  ui/  scripts/  package.json  README.md",
    pwd: "/Users/demo/projects/aineer",
    echo: cmd.replace(/^echo\s+/, ""),
    date: new Date().toString(),
    whoami: "developer",
    cargo:
      "   Compiling aineer v0.1.0\n    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.34s",
    git: "On branch main\nYour branch is up to date with 'origin/main'.\n\nnothing to commit, working tree clean",
    bun: "bun v1.3.11\n✓ 127 packages installed",
  };
  return outputs[c] ?? `$ ${cmd}\nCommand executed successfully.`;
}

function simulateAIResponse(query: string): string {
  if (query.toLowerCase().includes("explain")) {
    return "Here's an explanation:\n\nThis code uses a **modular architecture** with clear separation of concerns:\n\n1. `app/` — Tauri desktop entry point with IPC commands\n2. `crates/` — Reusable Rust business logic\n3. `ui/` — React frontend with shadcn components\n\n```rust\nfn main() {\n    aineer_lib::run_desktop();\n}\n```\n\nThe Tauri IPC bridge connects frontend to Rust backend via `invoke()`.";
  }
  return `I'd be happy to help with that.\n\nBased on your question about **"${query}"**, here's what I think:\n\nThe approach involves analyzing the current codebase structure and making targeted changes. Would you like me to elaborate on any specific aspect?`;
}
