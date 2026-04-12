import { getActivePreset } from "@/lib/theme";

let _highlighter: import("shiki").Highlighter | null = null;
let _loadingPromise: Promise<import("shiki").Highlighter> | null = null;

const BUNDLED_THEMES = ["github-light", "solarized-light", "one-dark-pro", "dracula"] as const;

const COMMON_LANGS = [
  "javascript",
  "typescript",
  "jsx",
  "tsx",
  "json",
  "html",
  "css",
  "python",
  "rust",
  "go",
  "bash",
  "shell",
  "yaml",
  "toml",
  "markdown",
  "sql",
  "c",
  "cpp",
  "java",
  "swift",
  "kotlin",
  "ruby",
  "php",
  "lua",
  "diff",
  "xml",
  "dockerfile",
  "plaintext",
] as const;

async function createHighlighter() {
  const { createHighlighter } = await import("shiki");
  return createHighlighter({
    themes: [...BUNDLED_THEMES],
    langs: [...COMMON_LANGS],
  });
}

export async function getHighlighter(): Promise<import("shiki").Highlighter> {
  if (_highlighter) return _highlighter;
  if (!_loadingPromise) {
    _loadingPromise = createHighlighter().then((h) => {
      _highlighter = h;
      return h;
    });
  }
  return _loadingPromise;
}

export function getShikiTheme(): string {
  return getActivePreset().shikiTheme;
}

export function langFromPath(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  const map: Record<string, string> = {
    js: "javascript",
    mjs: "javascript",
    cjs: "javascript",
    ts: "typescript",
    mts: "typescript",
    cts: "typescript",
    jsx: "jsx",
    tsx: "tsx",
    json: "json",
    html: "html",
    htm: "html",
    css: "css",
    scss: "css",
    py: "python",
    rs: "rust",
    go: "go",
    sh: "bash",
    bash: "bash",
    zsh: "bash",
    yml: "yaml",
    yaml: "yaml",
    toml: "toml",
    md: "markdown",
    mdx: "markdown",
    sql: "sql",
    c: "c",
    h: "c",
    cpp: "cpp",
    cc: "cpp",
    cxx: "cpp",
    hpp: "cpp",
    java: "java",
    swift: "swift",
    kt: "kotlin",
    kts: "kotlin",
    rb: "ruby",
    php: "php",
    lua: "lua",
    diff: "diff",
    patch: "diff",
    xml: "xml",
    svg: "xml",
    dockerfile: "dockerfile",
    Dockerfile: "dockerfile",
  };

  const base = path.split("/").pop() ?? "";
  if (base === "Dockerfile" || base.startsWith("Dockerfile.")) return "dockerfile";
  if (base === "Makefile" || base === "Justfile") return "bash";

  return map[ext] ?? "plaintext";
}
