import type { Translations } from "@/lib/i18n";

export function formatBytes(bytes: number, u: Translations["units"]): string {
  if (bytes < 1024) return `${bytes} ${u.b}`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} ${u.kb}`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} ${u.mb}`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} ${u.gb}`;
}
