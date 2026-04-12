import type { SlashCommandDef } from "@/lib/tauri";
import type { Attachment, InputMode } from "@/lib/types";

export interface InputChip {
  id: string;
  type: "command" | "mention";
  /** Display text shown in the tag */
  label: string;
  /** Raw value sent to AI (e.g. the path without @) */
  value: string;
  icon?: string;
}

export interface PendingAttachment {
  id: string;
  type: "image" | "file";
  name: string;
  size: number;
  previewUrl?: string;
  file: File;
}

export interface InputBarProps {
  mode: InputMode;
  onModeChange: (mode: InputMode) => void;
  onSubmit: (text: string, mode: InputMode, attachments?: Attachment[]) => void;
  onSlashCommand: (cmd: string) => void;
  onStop: () => void;
  isStreaming: boolean;
  slashCommands?: SlashCommandDef[];
  queueSize?: number;
}

export interface ModeDraft {
  value: string;
  chips: InputChip[];
  attachments: PendingAttachment[];
}
