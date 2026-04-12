"use client";

import { FileText, X } from "lucide-react";
import { MENTION_ICON_MAP } from "./MentionMenu";
import type { InputChip } from "./types";

export function InlineChip({
  chip,
  onRemove,
}: {
  chip: InputChip;
  onRemove: (id: string) => void;
}) {
  const IconComp = chip.icon ? (MENTION_ICON_MAP[chip.icon] ?? FileText) : null;

  return (
    <span className="chip-tag hover-reveal">
      {IconComp && <IconComp className="h-3 w-3 shrink-0 opacity-60" />}
      <span className="truncate">{chip.label}</span>
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          onRemove(chip.id);
        }}
        className="hover-reveal-target ml-0.5 inline-flex h-4 w-4 shrink-0 items-center justify-center rounded-sm text-muted-foreground hover:bg-accent hover:text-foreground"
      >
        <X className="h-3 w-3" />
      </button>
    </span>
  );
}
