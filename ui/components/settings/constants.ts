import {
  Braces,
  Database,
  Globe,
  Info,
  Network,
  Palette,
  Shield,
  Sparkles,
  Terminal,
} from "lucide-react";
import type { SettingsPage } from "./types";

export const NAV_ICONS: Record<SettingsPage, typeof Palette> = {
  appearance: Palette,
  models: Sparkles,
  capabilities: Shield,
  channels: Globe,
  terminal: Terminal,
  memory: Database,
  safety: Shield,
  cache: Database,
  json: Braces,
  about: Info,
  webai: Globe,
  gateway: Network,
};

export const NAV_PAGES: SettingsPage[] = [
  "appearance",
  "models",
  "webai",
  "gateway",
  "terminal",
  "safety",
  "cache",
  "json",
  "about",
];

export const THEME_LABEL: Record<string, string> = {
  "github-light": "themeGithubLight",
  "solarized-light": "themeSolarizedLight",
  "one-dark-pro": "themeOneDarkPro",
  dracula: "themeDracula",
};

export type ProviderProtocol = "openai-compatible" | "anthropic-native";

export interface ProviderPreset {
  id: string;
  name: string;
  description: string;
  protocol: ProviderProtocol;
  baseUrl: string;
  defaultModels: string[];
  noKeyNeeded?: boolean;
}

export const PROVIDER_PRESETS: ProviderPreset[] = [
  {
    id: "anthropic",
    name: "Anthropic",
    description: "Claude series models",
    protocol: "openai-compatible",
    baseUrl: "https://api.anthropic.com",
    defaultModels: [
      "claude-sonnet-4-20250514",
      "claude-opus-4-20250514",
      "claude-haiku-4-20250514",
    ],
  },
  {
    id: "openai",
    name: "OpenAI",
    description: "GPT-4o, o3, o4-mini",
    protocol: "openai-compatible",
    baseUrl: "https://api.openai.com/v1",
    defaultModels: ["gpt-4o", "gpt-4o-mini", "o3", "o4-mini"],
  },
  {
    id: "google",
    name: "Google Gemini",
    description: "Gemini 2.5 Pro / Flash",
    protocol: "openai-compatible",
    baseUrl: "https://generativelanguage.googleapis.com/v1beta/openai",
    defaultModels: ["gemini-2.5-pro", "gemini-2.5-flash"],
  },
  {
    id: "deepseek",
    name: "DeepSeek",
    description: "DeepSeek Chat / Reasoner",
    protocol: "openai-compatible",
    baseUrl: "https://api.deepseek.com/v1",
    defaultModels: ["deepseek-chat", "deepseek-reasoner"],
  },
  {
    id: "xai",
    name: "xAI Grok",
    description: "Grok-3 series",
    protocol: "openai-compatible",
    baseUrl: "https://api.x.ai/v1",
    defaultModels: ["grok-3", "grok-3-mini"],
  },
  {
    id: "ollama",
    name: "Ollama",
    description: "Local models, no API key",
    protocol: "openai-compatible",
    baseUrl: "http://localhost:11434/v1",
    defaultModels: ["qwen3-coder", "llama3.1", "codellama"],
    noKeyNeeded: true,
  },
  {
    id: "openrouter",
    name: "OpenRouter",
    description: "Multi-provider gateway",
    protocol: "openai-compatible",
    baseUrl: "https://openrouter.ai/api/v1",
    defaultModels: [],
  },
  {
    id: "groq",
    name: "Groq",
    description: "Ultra-fast inference",
    protocol: "openai-compatible",
    baseUrl: "https://api.groq.com/openai/v1",
    defaultModels: [],
  },
  {
    id: "custom",
    name: "Custom",
    description: "OpenAI-compatible endpoint",
    protocol: "openai-compatible",
    baseUrl: "",
    defaultModels: [],
  },
];

export const PROTOCOL_LABELS: Record<ProviderProtocol, string> = {
  "openai-compatible": "OpenAI Compatible",
  "anthropic-native": "Anthropic Native",
};
