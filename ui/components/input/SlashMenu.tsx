"use client";

import { Folder, Terminal } from "lucide-react";
import { forwardRef } from "react";
import type { CompletionItem } from "@/lib/tauri";
import { cn } from "@/lib/utils";

export const CommandMenu = forwardRef<HTMLDivElement, { children: React.ReactNode }>(
  ({ children }, ref) => (
    <div
      ref={ref}
      className="absolute bottom-full left-4 right-4 mb-1 max-h-48 overflow-y-auto rounded-lg border bg-popover p-1 shadow-lg"
    >
      {children}
    </div>
  ),
);
CommandMenu.displayName = "CommandMenu";

export function CommandMenuItem({
  children,
  selected,
  onMouseEnter,
  onClick,
}: {
  children: React.ReactNode;
  selected: boolean;
  onMouseEnter: () => void;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onMouseEnter={onMouseEnter}
      onClick={onClick}
      className={cn(
        "flex w-full items-center gap-3 rounded-md px-3 py-2 text-left text-xs transition-colors",
        selected ? "bg-accent text-accent-foreground" : "text-foreground hover:bg-accent/50",
      )}
    >
      {children}
    </button>
  );
}

export interface SlashCommandItem {
  name: string;
  description: string;
}

export function SlashMenu({
  menuRef,
  items,
  selectedIdx,
  onHoverIndex,
  onSelect,
}: {
  menuRef: React.RefObject<HTMLDivElement | null>;
  items: SlashCommandItem[];
  selectedIdx: number;
  onHoverIndex: (i: number) => void;
  onSelect: (name: string) => void;
}) {
  if (items.length === 0) return null;

  return (
    <CommandMenu ref={menuRef}>
      {items.map((cmd, i) => (
        <CommandMenuItem
          key={cmd.name}
          selected={i === selectedIdx}
          onMouseEnter={() => onHoverIndex(i)}
          onClick={() => onSelect(cmd.name)}
        >
          <span className="font-mono text-primary">/{cmd.name}</span>
          <span className="text-muted-foreground">{cmd.description}</span>
        </CommandMenuItem>
      ))}
    </CommandMenu>
  );
}

export function ShellCompletionMenu({
  menuRef,
  completions,
  selectedIdx,
  onHoverIndex,
  onSelect,
}: {
  menuRef: React.RefObject<HTMLDivElement | null>;
  completions: CompletionItem[];
  selectedIdx: number;
  onHoverIndex: (i: number) => void;
  onSelect: (item: CompletionItem) => void;
}) {
  if (completions.length === 0) return null;

  return (
    <CommandMenu ref={menuRef}>
      {completions.map((comp, i) => {
        const Icon = comp.isDir ? Folder : Terminal;
        return (
          <CommandMenuItem
            key={comp.value}
            selected={i === selectedIdx}
            onMouseEnter={() => onHoverIndex(i)}
            onClick={() => onSelect(comp)}
          >
            <Icon className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
            <span className="font-mono">
              {comp.value}
              {comp.isDir ? "/" : ""}
            </span>
          </CommandMenuItem>
        );
      })}
    </CommandMenu>
  );
}
