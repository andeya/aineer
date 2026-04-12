"use client";

import type { LucideIcon } from "lucide-react";
import {
  BookOpen,
  Code,
  FileCode,
  FileText,
  Folder,
  GitBranch,
  Globe,
  Search,
  Terminal,
} from "lucide-react";
import type { AtMention } from "@/lib/types";
import { cn } from "@/lib/utils";

export const MENTION_ICON_MAP: Record<string, LucideIcon> = {
  file: FileCode,
  folder: Folder,
  code: Code,
  "book-open": BookOpen,
  terminal: Terminal,
  "git-branch": GitBranch,
  globe: Globe,
  search: Search,
};

export function MentionMenu({
  menuRef,
  items,
  selectedIdx,
  sectionTitle,
  onHoverIndex,
  onSelect,
}: {
  menuRef: React.RefObject<HTMLDivElement | null>;
  items: AtMention[];
  selectedIdx: number;
  sectionTitle: string;
  onHoverIndex: (i: number) => void;
  onSelect: (item: AtMention) => void;
}) {
  if (items.length === 0) return null;

  return (
    <div
      ref={menuRef}
      className="absolute bottom-full left-4 right-4 mb-1 max-h-[320px] overflow-y-auto rounded-lg border bg-popover p-1.5 shadow-lg"
    >
      <div className="mb-1 px-2 py-1 text-[10px] font-medium tracking-wide text-muted-foreground uppercase">
        {sectionTitle}
      </div>
      {items.map((m, i) => {
        const IconComp = MENTION_ICON_MAP[m.icon] ?? FileText;
        return (
          <button
            key={m.name}
            type="button"
            onMouseEnter={() => onHoverIndex(i)}
            onClick={() => onSelect(m)}
            className={cn(
              "flex w-full items-center gap-3 rounded-md px-2.5 py-2 text-left text-sm transition-colors",
              i === selectedIdx
                ? "bg-accent text-accent-foreground"
                : "text-foreground hover:bg-accent/50",
            )}
          >
            <IconComp className="h-4 w-4 shrink-0 text-muted-foreground" />
            <span className="flex-1 font-medium">{m.name}</span>
            {m.hasPicker && <span className="text-[10px] text-muted-foreground">›</span>}
          </button>
        );
      })}
    </div>
  );
}
