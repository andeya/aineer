import {
  Bot,
  Check,
  CheckCircle2,
  Circle,
  ClipboardCopy,
  Code,
  FileText,
  Loader2,
  Sparkles,
  Terminal,
  XCircle,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { AttachmentLightbox } from "@/components/input/AttachmentStrip";
import { Logo } from "@/components/Logo";
import { Badge } from "@/components/ui/badge";
import {
  ChatContainerContent,
  ChatContainerRoot,
  ChatContainerScrollAnchor,
} from "@/components/ui/chat-container";
import { Loader } from "@/components/ui/loader";
import { Markdown } from "@/components/ui/markdown";
import { Reasoning, ReasoningContent, ReasoningTrigger } from "@/components/ui/reasoning";
import { Steps, StepsContent, StepsItem, StepsTrigger } from "@/components/ui/steps";
import { Tool } from "@/components/ui/tool";
import { useCopyAction } from "@/hooks/useCopyAction";
import { useI18n } from "@/lib/i18n";
import type { Attachment, ChatMessage } from "@/lib/types";
import { cn } from "@/lib/utils";

interface ChatViewProps {
  messages: ChatMessage[];
  isStreaming: boolean;
  streamingMode?: "shell" | "ai" | "agent";
  onModeChange?: (mode: "shell" | "ai" | "agent") => void;
}

function isLastAssistantEmpty(messages: ChatMessage[]): boolean {
  if (messages.length === 0) return false;
  const last = messages[messages.length - 1];
  return last.role === "assistant" && !last.content && !last.shell;
}

export function ChatView({ messages, isStreaming, streamingMode, onModeChange }: ChatViewProps) {
  if (messages.length === 0) {
    return <WelcomeScreen onModeChange={onModeChange} />;
  }

  return (
    <ChatViewMessages messages={messages} isStreaming={isStreaming} streamingMode={streamingMode} />
  );
}

function ChatViewMessages({ messages, isStreaming, streamingMode }: ChatViewProps) {
  const showInlineThinking =
    isStreaming && streamingMode !== "shell" && isLastAssistantEmpty(messages);

  return (
    <ChatContainerRoot className="flex-1">
      <ChatContainerContent className="gap-3 px-4 pt-4 pb-2">
        {renderGroupedMessages(messages, showInlineThinking)}
        <ChatContainerScrollAnchor />
      </ChatContainerContent>
    </ChatContainerRoot>
  );
}

function renderGroupedMessages(messages: ChatMessage[], lastIsThinking = false) {
  const cards: React.ReactNode[] = [];
  let i = 0;
  while (i < messages.length) {
    const msg = messages[i];
    const isLast = i >= messages.length - 2;
    if (msg.role === "user") {
      const response = messages[i + 1];
      if (response && response.role !== "user") {
        cards.push(
          <MessageCard
            key={msg.id}
            userMessage={msg}
            responseMessage={response}
            isThinking={lastIsThinking && i + 2 >= messages.length}
          />,
        );
        i += 2;
        continue;
      }
      cards.push(<MessageCard key={msg.id} userMessage={msg} />);
      i += 1;
    } else {
      cards.push(
        <MessageCard key={msg.id} responseMessage={msg} isThinking={lastIsThinking && isLast} />,
      );
      i += 1;
    }
  }
  return cards;
}

const modeStyle = {
  shell: {
    icon: Terminal,
    accent: "text-foreground",
    badge: "bg-muted text-muted-foreground",
    border: "border-border",
  },
  ai: {
    icon: Sparkles,
    accent: "text-ai",
    badge: "bg-ai-badge text-ai",
    border: "border-ai-subtle",
  },
  agent: {
    icon: Bot,
    accent: "text-agent",
    badge: "bg-agent-badge text-agent",
    border: "border-agent-subtle",
  },
} as const;

// ────────────────────────────────────────────────────────
// Copy helper
// ────────────────────────────────────────────────────────

function CopyButton({
  label,
  text,
  copied,
  onCopy,
}: {
  label: string;
  text: string;
  copied: string | null;
  onCopy: (text: string, label: string) => void;
}) {
  const { t } = useI18n();
  const isCopied = copied === label;
  return (
    <button
      type="button"
      onClick={() => onCopy(text, label)}
      className="flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
      title={`${t.common.copy} ${label}`}
    >
      {isCopied ? (
        <Check className="h-3 w-3 text-success" />
      ) : (
        <ClipboardCopy className="h-3 w-3" />
      )}
      {label}
    </button>
  );
}

// ────────────────────────────────────────────────────────
// Bubble toolbar — hover-to-reveal actions row
// ────────────────────────────────────────────────────────

function BubbleToolbar({ children }: { children: React.ReactNode }) {
  return (
    <div className="bubble-toolbar absolute top-1 right-1 flex items-center gap-0.5 rounded-md border border-border bg-card px-1 py-0.5 shadow-sm">
      {children}
    </div>
  );
}

/** Typewriter-style reveal while reasoning streams; show full text when done. */
function useReasoningTypewriter(fullText: string, streaming: boolean): string {
  const fullRef = useRef(fullText);
  fullRef.current = fullText;
  const [n, setN] = useState(0);

  useEffect(() => {
    if (!streaming) {
      setN(fullRef.current.length);
      return;
    }
    let cancelled = false;
    const loop = () => {
      if (cancelled) return;
      setN((prev) => {
        const cap = fullRef.current.length;
        if (prev >= cap) return prev;
        return Math.min(prev + 2, cap);
      });
      requestAnimationFrame(loop);
    };
    const id = requestAnimationFrame(loop);
    return () => {
      cancelled = true;
      cancelAnimationFrame(id);
    };
  }, [streaming]);

  if (!streaming) return fullText;
  return fullText.slice(0, n);
}

/** Collapsible model reasoning: expanded while only thinking streams; folds when answer/UI appears. */
function AssistantThinkingBlock({
  thinking,
  live,
  answerVisible,
  thinkingStartedAt,
  thinkingDurationMs,
}: {
  thinking: string;
  live: boolean;
  answerVisible: boolean;
  thinkingStartedAt?: number;
  thinkingDurationMs?: number;
}) {
  const { t } = useI18n();
  const displayThinking = useReasoningTypewriter(thinking, live);
  const [open, setOpen] = useState(() => !answerVisible);
  const [, setTick] = useState(0);

  useEffect(() => {
    if (!live || thinkingStartedAt == null) return;
    const id = setInterval(() => setTick((x) => x + 1), 250);
    return () => clearInterval(id);
  }, [live, thinkingStartedAt]);

  useEffect(() => {
    if (answerVisible) setOpen(false);
  }, [answerVisible]);

  useEffect(() => {
    if (!answerVisible && thinking.length > 0) setOpen(true);
  }, [answerVisible, thinking]);

  const headerLabel = (() => {
    if (thinkingDurationMs != null) {
      const s = Math.max(1, Math.round(thinkingDurationMs / 1000));
      return t.chat.reasoningDurationTpl.replace("{seconds}", String(s));
    }
    if (live && thinkingStartedAt != null) {
      const s = Math.max(0, Math.floor((Date.now() - thinkingStartedAt) / 1000));
      return t.chat.reasoningLiveForTpl.replace("{seconds}", String(s));
    }
    if (live) return t.chat.reasoningLive;
    return t.chat.reasoning;
  })();

  return (
    <Reasoning open={open} onOpenChange={setOpen} isStreaming={false}>
      <div className="rounded-md border border-border/50 bg-muted/25">
        <ReasoningTrigger className="w-full rounded-t-md px-3 py-2 text-left text-xs hover:bg-muted/40">
          <span className="flex min-w-0 items-center gap-2 font-medium text-muted-foreground">
            {live && thinkingStartedAt == null && (
              <Loader2 className="h-3.5 w-3.5 shrink-0 animate-spin" />
            )}
            <span className="truncate">{headerLabel}</span>
          </span>
        </ReasoningTrigger>
        <ReasoningContent
          className="border-t border-border/40"
          contentClassName="!max-w-none px-0 py-0"
          disableOuterProse
        >
          <div className="text-reasoning max-h-[min(40vh,320px)] overflow-y-auto px-3 py-2.5">
            <Markdown
              className={cn(
                "prose prose-sm max-w-none dark:prose-invert",
                "text-[13px] leading-relaxed",
                "prose-headings:mb-2 prose-headings:mt-3 prose-headings:font-semibold prose-headings:text-reasoning",
                "prose-p:my-2 prose-li:my-0.5",
                "prose-code:rounded prose-code:bg-primary/10 prose-code:px-1 prose-code:py-px prose-code:font-mono prose-code:text-[12px] prose-code:before:content-none prose-code:after:content-none",
                "prose-pre:bg-muted/40 prose-pre:border prose-pre:border-border/50",
              )}
            >
              {displayThinking}
            </Markdown>
            {live && displayThinking.length < thinking.length && (
              <span className="ml-px inline-block h-3.5 w-0.5 animate-pulse bg-primary/60 align-text-bottom" />
            )}
          </div>
        </ReasoningContent>
      </div>
    </Reasoning>
  );
}

function ChatAIBody({
  message,
  showRaw,
  thinkingOnlyPhase,
}: {
  message: ChatMessage;
  showRaw: boolean;
  thinkingOnlyPhase: boolean;
}) {
  const { t } = useI18n();
  const thinking = message.thinking ?? "";
  const hasThinking = thinking.length > 0;
  const hasContent = !!message.content?.trim();

  return (
    <div className="space-y-3 px-3 py-3">
      {message.model && <div className="text-[10px] text-muted-foreground">{message.model}</div>}
      {hasThinking && (
        <AssistantThinkingBlock
          thinking={thinking}
          live={thinkingOnlyPhase && !hasContent}
          answerVisible={hasContent}
          thinkingStartedAt={message.thinkingStartedAt}
          thinkingDurationMs={message.thinkingDurationMs}
        />
      )}
      {thinkingOnlyPhase && !hasThinking && (
        <div className="flex items-center gap-2 py-1">
          <Loader variant="typing" size="sm" />
          <span className="text-xs text-muted-foreground">{t.common.responding}</span>
        </div>
      )}
      {hasContent &&
        (showRaw ? (
          <div className={cn(hasThinking && "border-t border-border/40 pt-3")}>
            <pre className="whitespace-pre-wrap text-sm leading-relaxed text-foreground">
              {message.content}
            </pre>
          </div>
        ) : (
          <div className={cn(hasThinking && "border-t border-border/40 pt-3")}>
            <Markdown className="prose prose-sm prose-inherit max-w-none break-words text-sm leading-relaxed text-foreground">
              {message.content}
            </Markdown>
          </div>
        ))}
    </div>
  );
}

function RawToggle({ showRaw, onToggle }: { showRaw: boolean; onToggle: () => void }) {
  const { t } = useI18n();
  return (
    <button
      type="button"
      onClick={onToggle}
      className={cn(
        "flex items-center gap-1 rounded px-1.5 py-0.5 text-[10px] transition-colors",
        showRaw
          ? "bg-accent text-foreground"
          : "text-muted-foreground hover:bg-accent hover:text-foreground",
      )}
      title={showRaw ? t.chat.showMarkdown : t.chat.showRaw}
    >
      {showRaw ? <FileText className="h-3 w-3" /> : <Code className="h-3 w-3" />}
      {showRaw ? t.chat.markdownBtn : t.chat.rawBtn}
    </button>
  );
}

// ────────────────────────────────────────────────────────
// MessageCard — Q bubble + A bubble, each independent
// ────────────────────────────────────────────────────────

function MessageCard({
  userMessage,
  responseMessage,
  isThinking,
}: {
  userMessage?: ChatMessage;
  responseMessage?: ChatMessage;
  isThinking?: boolean;
}) {
  const { t } = useI18n();
  const mode = userMessage?.mode ?? responseMessage?.mode ?? "ai";
  const style = modeStyle[mode as keyof typeof modeStyle] ?? modeStyle.ai;
  const Icon = style.icon;
  const modeLabel = mode === "shell" ? t.mode.shell : mode === "ai" ? t.mode.chat : t.mode.agent;
  const shell = userMessage?.shell ?? responseMessage?.shell;
  const { copied: qCopied, copy: qCopy } = useCopyAction();
  const { copied: aCopied, copy: aCopy } = useCopyAction();
  const [qRaw, setQRaw] = useState(false);
  const [aRaw, setARaw] = useState(false);
  const questionText = userMessage?.content ?? "";
  const answerText = responseMessage?.content ?? responseMessage?.shell?.output ?? "";

  return (
    <div className={cn("overflow-hidden rounded-lg border bg-card", style.border)}>
      {/* ── Q: title area ── */}
      {userMessage && (
        <div className="bubble-zone relative border-b border-border/40 bg-secondary">
          <div className="flex items-center gap-2 px-3 pt-2 pb-1">
            <Icon className={cn("h-3.5 w-3.5 shrink-0", style.accent)} />
            <span className={cn("text-[10px] font-medium", style.accent)}>{modeLabel}</span>
            {mode === "shell" && shell && (
              <span className="ml-auto flex items-center gap-1.5 text-[10px] text-muted-foreground">
                <span className="truncate">{shell.cwd}</span>
                {shell.durationMs != null && <span>({shell.durationMs}ms)</span>}
                {shell.timedOut && (
                  <Badge variant="destructive" className="h-3.5 px-1 text-[9px]">
                    {t.chat.timeout}
                  </Badge>
                )}
                {shell.exitCode !== 0 && !shell.timedOut && (
                  <Badge variant="destructive" className="h-3.5 px-1 text-[9px]">
                    exit {shell.exitCode}
                  </Badge>
                )}
              </span>
            )}
          </div>
          <div className="px-3 pb-2 text-sm text-foreground">
            {mode === "shell" ? (
              <span className="font-mono">{userMessage.content}</span>
            ) : qRaw ? (
              <pre className="whitespace-pre-wrap text-sm">{userMessage.content}</pre>
            ) : (
              <Markdown className="prose prose-sm prose-inherit break-words text-sm">
                {userMessage.content}
              </Markdown>
            )}
          </div>
          {userMessage.attachments && userMessage.attachments.length > 0 && (
            <ChatAttachmentStrip attachments={userMessage.attachments} />
          )}
          <BubbleToolbar>
            {mode !== "shell" && <RawToggle showRaw={qRaw} onToggle={() => setQRaw((v) => !v)} />}
            <CopyButton label={t.chat.copyQ} text={questionText} copied={qCopied} onCopy={qCopy} />
          </BubbleToolbar>
        </div>
      )}

      {/* ── A: content area ── */}
      {(responseMessage || isThinking) && (
        <div className="bubble-zone relative">
          <div className="text-sm">
            {responseMessage && mode === "shell" && responseMessage.shell ? (
              <ShellOutput shell={responseMessage.shell} />
            ) : responseMessage && mode === "agent" ? (
              <AgentBody
                message={responseMessage}
                showRaw={aRaw}
                thinkingOnlyPhase={!!(isThinking && !responseMessage.content)}
              />
            ) : responseMessage ? (
              <ChatAIBody
                message={responseMessage}
                showRaw={aRaw}
                thinkingOnlyPhase={!!(isThinking && !responseMessage.content)}
              />
            ) : isThinking && !responseMessage ? (
              <div className="flex items-center gap-2 px-3 py-3">
                <Loader variant="typing" size="sm" />
                <span className="text-xs text-muted-foreground">{t.common.responding}</span>
              </div>
            ) : null}
          </div>
          <BubbleToolbar>
            {mode !== "shell" && <RawToggle showRaw={aRaw} onToggle={() => setARaw((v) => !v)} />}
            <CopyButton label={t.chat.copyA} text={answerText} copied={aCopied} onCopy={aCopy} />
          </BubbleToolbar>
        </div>
      )}
    </div>
  );
}

// ────────────────────────────────────────────────────────
// Attachment strip
// ────────────────────────────────────────────────────────

function ChatAttachmentStrip({ attachments }: { attachments: Attachment[] }) {
  const [lightbox, setLightbox] = useState<{ url: string; alt: string } | null>(null);

  const openPreview = useCallback((url: string, alt: string) => {
    setLightbox({ url, alt });
  }, []);

  return (
    <>
      <div className="flex flex-wrap gap-1.5 border-b border-border/50 bg-card/30 px-3 py-1.5">
        {attachments.map((att) =>
          att.type === "image" && att.previewUrl ? (
            <button
              key={att.id}
              type="button"
              onClick={() => openPreview(att.previewUrl ?? "", att.name)}
              className="cursor-zoom-in overflow-hidden rounded border border-border transition-opacity hover:opacity-80"
            >
              <img src={att.previewUrl} alt={att.name} className="h-12 w-12 object-cover" />
            </button>
          ) : (
            <div
              key={att.id}
              className="flex items-center gap-1 rounded border border-border bg-muted px-2 py-1 text-[10px] text-muted-foreground"
            >
              <FileText className="h-3 w-3" />
              <span className="max-w-[120px] truncate">{att.name}</span>
            </div>
          ),
        )}
      </div>
      {lightbox && (
        <AttachmentLightbox
          url={lightbox.url}
          alt={lightbox.alt}
          onClose={() => setLightbox(null)}
        />
      )}
    </>
  );
}

// ────────────────────────────────────────────────────────
// Shell components
// ────────────────────────────────────────────────────────

function isColumnable(output: string): boolean {
  const lines = output.split("\n");
  if (lines.length < 4) return false;
  return lines.every((l) => l.length < 60 && !l.includes("  ") && l.trim().length > 0);
}

function ShellOutput({ shell }: { shell: NonNullable<ChatMessage["shell"]> }) {
  const isError = shell.exitCode !== 0;
  if (!shell.output) return null;

  const columnar = !isError && isColumnable(shell.output);

  if (columnar) {
    const items = shell.output.split("\n").filter(Boolean);
    return (
      <div
        className="gap-x-6 gap-y-0.5 px-3 py-2.5 font-mono text-xs text-card-foreground"
        style={{ columnWidth: "10rem", columnCount: "auto" }}
      >
        {items.map((item, i) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: static command output never reorders
          <div key={i} className="break-inside-avoid truncate leading-relaxed">
            {item}
          </div>
        ))}
      </div>
    );
  }

  return (
    <pre
      className={cn(
        "overflow-x-auto whitespace-pre-wrap px-3 py-2.5 font-mono text-xs leading-relaxed",
        isError ? "text-error" : "text-card-foreground",
      )}
    >
      {shell.output}
    </pre>
  );
}

// ────────────────────────────────────────────────────────
// AI / Agent body with raw/md toggle
// ────────────────────────────────────────────────────────

function AgentBody({
  message,
  showRaw,
  thinkingOnlyPhase = false,
}: {
  message: ChatMessage;
  showRaw: boolean;
  thinkingOnlyPhase?: boolean;
}) {
  const { t } = useI18n();
  const thinking = message.thinking ?? "";
  const hasThinking = thinking.length > 0;
  const hasContent = !!message.content?.trim();
  const hasAgentChrome = Boolean(
    (message.agentSteps && message.agentSteps.length > 0) ||
      (message.toolCalls && message.toolCalls.length > 0),
  );
  const answerVisible = hasContent || hasAgentChrome;

  return (
    <div className="space-y-2 px-3 py-3">
      {message.model && <div className="text-[10px] text-muted-foreground">{message.model}</div>}

      {hasThinking && (
        <AssistantThinkingBlock
          thinking={thinking}
          live={Boolean(thinkingOnlyPhase && !answerVisible)}
          answerVisible={answerVisible}
          thinkingStartedAt={message.thinkingStartedAt}
          thinkingDurationMs={message.thinkingDurationMs}
        />
      )}

      {message.agentSteps && message.agentSteps.length > 0 && (
        <Steps defaultOpen>
          <StepsTrigger leftIcon={<Bot className="h-4 w-4 text-agent" />}>
            {message.agentSteps.filter((s) => s.status === "completed").length}/
            {message.agentSteps.length} {t.chat.steps}
          </StepsTrigger>
          <StepsContent>
            {message.agentSteps.map((step) => (
              <StepsItem key={step.name} className="flex items-center gap-2 py-0.5">
                <StepIcon status={step.status} />
                <span className={cn(step.status === "completed" && "text-foreground")}>
                  {step.name}
                </span>
              </StepsItem>
            ))}
          </StepsContent>
        </Steps>
      )}

      {message.toolCalls?.map((tc) => (
        <Tool key={tc.toolCallId ?? tc.type} toolPart={tc} defaultOpen />
      ))}

      {message.content &&
        (showRaw ? (
          <div className={cn(hasThinking && "border-t border-border/40 pt-3")}>
            <pre className="whitespace-pre-wrap text-sm leading-relaxed text-foreground">
              {message.content}
            </pre>
          </div>
        ) : (
          <div className={cn(hasThinking && "border-t border-border/40 pt-3")}>
            <Markdown className="prose prose-sm prose-inherit max-w-none break-words text-sm leading-relaxed text-foreground">
              {message.content}
            </Markdown>
          </div>
        ))}
    </div>
  );
}

function StepIcon({ status }: { status: string }) {
  switch (status) {
    case "completed":
      return <CheckCircle2 className="h-3.5 w-3.5 text-success" />;
    case "running":
      return <Loader2 className="h-3.5 w-3.5 animate-spin text-ai" />;
    case "failed":
      return <XCircle className="h-3.5 w-3.5 text-destructive" />;
    default:
      return <Circle className="h-3.5 w-3.5 text-muted-foreground" />;
  }
}

// ────────────────────────────────────────────────────────
// Welcome screen
// ────────────────────────────────────────────────────────

function WelcomeScreen({
  onModeChange,
}: {
  onModeChange?: (mode: "shell" | "ai" | "agent") => void;
}) {
  const { t } = useI18n();

  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-6 p-8 text-center">
      <Logo className="h-16 w-16" />
      <div>
        <h1 className="text-lg font-semibold text-foreground">{t.chat.welcome}</h1>
        <p className="mt-1 text-sm text-muted-foreground">{t.chat.subtitle}</p>
      </div>
      <div className="grid max-w-lg grid-cols-3 gap-3 text-xs">
        <button
          type="button"
          onClick={() => onModeChange?.("shell")}
          className="cursor-pointer rounded-xl border border-border bg-card p-4 transition-colors hover:bg-secondary"
        >
          <Terminal className="mx-auto mb-2 h-5 w-5 text-foreground" />
          <div className="mb-0.5 font-medium text-foreground">{t.mode.shell}</div>
          <div className="text-muted-foreground">{t.chat.shellDesc}</div>
        </button>
        <button
          type="button"
          onClick={() => onModeChange?.("ai")}
          className="cursor-pointer rounded-xl border border-ai-subtle bg-card p-4 transition-colors hover:bg-secondary"
        >
          <Sparkles className="mx-auto mb-2 h-5 w-5 text-ai" />
          <div className="mb-0.5 font-medium text-ai">{t.mode.chat}</div>
          <div className="text-muted-foreground">{t.chat.chatDesc}</div>
        </button>
        <button
          type="button"
          onClick={() => onModeChange?.("agent")}
          className="cursor-pointer rounded-xl border border-agent-subtle bg-card p-4 transition-colors hover:bg-secondary"
        >
          <Bot className="mx-auto mb-2 h-5 w-5 text-agent" />
          <div className="mb-0.5 font-medium text-agent">{t.mode.agent}</div>
          <div className="text-muted-foreground">{t.chat.agentDesc}</div>
        </button>
      </div>
      <div className="space-y-1 text-[11px] text-muted-foreground">
        <p>
          <kbd className="rounded border bg-muted px-1.5 py-0.5">Enter</kbd> {t.chat.hintSend} ·{" "}
          <kbd className="rounded border bg-muted px-1.5 py-0.5">Tab</kbd> {t.chat.hintSwitchMode} ·{" "}
          <kbd className="rounded border bg-muted px-1.5 py-0.5">/</kbd> {t.chat.hintCommands} ·{" "}
          <kbd className="rounded border bg-muted px-1.5 py-0.5">@</kbd> {t.chat.hintMentions}
        </p>
      </div>
    </div>
  );
}
