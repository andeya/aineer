import { listen } from "@tauri-apps/api/event";
import {
  Check,
  ChevronDown,
  ChevronRight,
  Globe,
  Loader2,
  Pencil,
  Plus,
  RefreshCw,
  Trash2,
  X,
  Zap,
} from "lucide-react";
import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import { Select } from "@/components/ui/select";
import { useI18n } from "@/lib/i18n";
import { modelGroupsToSelectOptions, withCurrentModelOption } from "@/lib/model-options";
import {
  type CustomProviderConfig,
  fetchProviderModels,
  listModelGroups,
  type ModelGroupData,
  removeProvider,
  upsertProvider,
  type WebAiProviderInfo,
  webaiListAuthenticated,
  webaiListProviders,
  webaiStartAuth,
} from "@/lib/tauri";
import { cn } from "@/lib/utils";
import { PROTOCOL_LABELS, PROVIDER_PRESETS, type ProviderProtocol } from "./constants";
import { Field, NumberInput, Section, TextInput, Toggle } from "./shared";
import type { PageProps } from "./types";

/* ── Helpers ───────────────────────────────────────────────────── */

function nameToId(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 32);
}

function uniqueId(base: string, existing: Record<string, unknown>): string {
  if (!existing[base]) return base;
  for (let i = 2; i <= 99; i++) {
    const candidate = `${base}-${i}`;
    if (!existing[candidate]) return candidate;
  }
  return `${base}-${Date.now()}`;
}

const VALID_ID_RE = /^[a-z0-9]([a-z0-9_-]*[a-z0-9])?$/;

function urlToId(url: string): string {
  try {
    const host = new URL(url).hostname;
    return host
      .replace(/^(api|www)\./, "")
      .replace(/\.[a-z]{2,}$/i, "")
      .replace(/[^a-z0-9]+/gi, "-")
      .toLowerCase()
      .slice(0, 32);
  } catch {
    return "";
  }
}

function isLocalUrl(url: string): boolean {
  const lower = url.toLowerCase();
  return lower.includes("localhost") || lower.includes("127.0.0.1") || lower.includes("[::1]");
}

/* ── Model Tag Chips ───────────────────────────────────────────── */

function ModelChips({
  models,
  onChange,
  newModels,
  inputId,
}: {
  models: string[];
  onChange: (models: string[]) => void;
  newModels: Set<string>;
  inputId: string;
}) {
  const { t } = useI18n();
  const [input, setInput] = useState("");

  const handleAdd = useCallback(() => {
    const trimmed = input.trim();
    if (trimmed && !models.includes(trimmed)) {
      onChange([...models, trimmed]);
    }
    setInput("");
  }, [input, models, onChange]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleAdd();
      }
    },
    [handleAdd],
  );

  return (
    <div className="rounded border border-border bg-background px-2 py-1.5">
      {models.length > 0 && (
        <div className="mb-1.5 flex flex-wrap gap-1">
          {models.map((m) => (
            <span
              key={m}
              className={cn(
                "inline-flex items-center gap-0.5 rounded-md border px-1.5 py-0.5 font-mono text-[10px]",
                newModels.has(m)
                  ? "animate-pulse border-primary/40 bg-primary/10 text-primary"
                  : "border-border bg-muted/50 text-foreground",
              )}
            >
              {m}
              <button
                type="button"
                onClick={() => onChange(models.filter((x) => x !== m))}
                className="ml-0.5 rounded-sm text-muted-foreground hover:text-destructive"
              >
                <X className="h-2.5 w-2.5" />
              </button>
            </span>
          ))}
        </div>
      )}
      <input
        id={inputId}
        type="text"
        value={input}
        onChange={(e) => setInput(e.target.value)}
        onKeyDown={handleKeyDown}
        onBlur={() => {
          if (input.trim()) handleAdd();
        }}
        placeholder={t.settings.addModel}
        className="w-full bg-transparent text-xs font-mono text-foreground placeholder:text-muted-foreground focus:outline-none"
      />
    </div>
  );
}

/* ── Provider form (shared for add & edit) ─────────────────────── */

interface ProviderFormData {
  id: string;
  displayName: string;
  protocol: ProviderProtocol;
  baseUrl: string;
  apiKey: string;
  models: string[];
}

function emptyForm(): ProviderFormData {
  return {
    id: "",
    displayName: "",
    protocol: "openai-compatible",
    baseUrl: "",
    apiKey: "",
    models: [],
  };
}

function ProviderForm({
  form,
  onChange,
  onSave,
  onCancel,
  isNew,
  existingIds,
}: {
  form: ProviderFormData;
  onChange: (f: ProviderFormData) => void;
  onSave: () => void;
  onCancel: () => void;
  isNew: boolean;
  existingIds: Record<string, unknown>;
}) {
  const { t } = useI18n();
  const formDomId = useId();
  const presetTriggerId = `${formDomId}-preset`;
  const displayNameId = `${formDomId}-display`;
  const baseUrlId = `${formDomId}-base`;
  const apiKeyId = `${formDomId}-apikey`;
  const modelsInputId = `${formDomId}-models`;
  const providerIdInputId = `${formDomId}-provider-id`;
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [idManuallyEdited, setIdManuallyEdited] = useState(!isNew);
  const [fetching, setFetching] = useState(false);
  const [fetchError, setFetchError] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<{ ok: boolean; message: string } | null>(null);
  const [newModels, setNewModels] = useState<Set<string>>(new Set());
  const newModelsTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  useEffect(() => {
    return () => clearTimeout(newModelsTimerRef.current);
  }, []);

  const presetOptions = useMemo(
    () => PROVIDER_PRESETS.map((p) => ({ value: p.id, label: p.name })),
    [],
  );

  const applyPreset = useCallback(
    (presetId: string) => {
      const p = PROVIDER_PRESETS.find((x) => x.id === presetId);
      if (!p) return;
      const baseId = p.id !== "custom" ? p.id : "";
      onChange({
        ...form,
        id: baseId ? uniqueId(baseId, existingIds) : "",
        displayName: p.name !== "Custom" ? p.name : "",
        protocol: p.protocol,
        baseUrl: p.baseUrl,
        models: p.defaultModels,
      });
      setIdManuallyEdited(false);
    },
    [form, onChange, existingIds],
  );

  const handleDisplayNameChange = useCallback(
    (name: string) => {
      const next: ProviderFormData = { ...form, displayName: name };
      if (isNew && !idManuallyEdited) {
        const derived = nameToId(name);
        next.id = derived ? uniqueId(derived, existingIds) : "";
      }
      onChange(next);
    },
    [form, onChange, isNew, idManuallyEdited, existingIds],
  );

  const handleIdChange = useCallback(
    (raw: string) => {
      const sanitized = raw.toLowerCase().replace(/[^a-z0-9_-]/g, "");
      onChange({ ...form, id: sanitized });
      setIdManuallyEdited(true);
    },
    [form, onChange],
  );

  const idTrimmed = form.id.trim();
  const idFormatOk = VALID_ID_RE.test(idTrimmed);
  const idValidForUi = idTrimmed === "" || idFormatOk;
  const canSave =
    idFormatOk && form.baseUrl.trim() !== "" && (!isNew || form.displayName.trim() !== "");
  const canFetch = form.baseUrl.trim() !== "";

  const handleFetchModels = useCallback(
    async (mergeIntoForm: boolean) => {
      setFetching(true);
      setFetchError(null);
      setTestResult(null);
      try {
        const remote = await fetchProviderModels(
          form.baseUrl.trim(),
          form.apiKey.trim() || undefined,
        );
        if (remote.length > 0) {
          setTestResult({
            ok: true,
            message: t.settings.testSuccessWithCount.replace("{0}", String(remote.length)),
          });
          if (mergeIntoForm) {
            const existingSet = new Set(form.models);
            const added = new Set<string>();
            for (const m of remote) {
              if (!existingSet.has(m)) added.add(m);
            }
            onChange({ ...form, models: [...form.models, ...Array.from(added)] });
            if (added.size > 0) {
              setNewModels(added);
              clearTimeout(newModelsTimerRef.current);
              newModelsTimerRef.current = setTimeout(() => setNewModels(new Set()), 2000);
            }
          }
        } else {
          setFetchError(t.settings.fetchModelsEmpty);
          setTestResult({ ok: true, message: t.settings.testSuccess });
        }
      } catch (err) {
        setFetchError(String(err));
        setTestResult({ ok: false, message: String(err) });
      } finally {
        setFetching(false);
      }
    },
    [form, onChange, t],
  );

  const requiredMark = <span className="ml-0.5 text-destructive">*</span>;
  const modelCount = form.models.length;

  return (
    <div className="space-y-5 rounded-lg border border-primary/30 bg-card p-5">
      {/* ── Preset selector ── */}
      {isNew && (
        <div>
          <label
            htmlFor={presetTriggerId}
            className="mb-2 block text-[11px] font-medium text-muted-foreground"
          >
            {t.settings.providerPreset}
          </label>
          <Select
            fullWidth
            value=""
            options={presetOptions}
            onChange={applyPreset}
            placeholder={t.common.select}
            triggerId={presetTriggerId}
          />
        </div>
      )}

      {/* ── Display Name (drives Provider ID until Advanced override) ── */}
      <div>
        <label
          htmlFor={displayNameId}
          className="mb-2 block text-[11px] font-medium text-muted-foreground"
        >
          {t.settings.providerDisplayName}
          {isNew && requiredMark}
        </label>
        <input
          id={displayNameId}
          type="text"
          value={form.displayName}
          onChange={(e) => handleDisplayNameChange(e.target.value)}
          placeholder={form.id.trim() || t.settings.providerDisplayNamePlaceholder}
          className="w-full rounded-md border border-border bg-background px-3 py-2 text-xs focus:border-primary focus:outline-none"
        />
        {isNew && (
          <p className="mt-2 text-[10px] leading-relaxed text-muted-foreground/80">
            {t.settings.providerDisplayNameHint}
          </p>
        )}
      </div>

      {/* ── API connection (protocol + endpoint + credentials) ── */}
      <div className="space-y-4 rounded-md bg-muted/20 p-3.5">
        <div className="flex flex-wrap items-center gap-x-2 gap-y-1 border-b border-border/60 pb-3">
          <span className="text-[11px] font-medium text-muted-foreground">
            {t.settings.providerProtocol}
          </span>
          <span className="rounded-full border border-border bg-background px-2.5 py-0.5 text-[10px] font-mono text-foreground">
            {PROTOCOL_LABELS[form.protocol]}
          </span>
        </div>
        <div>
          <label
            htmlFor={baseUrlId}
            className="mb-2 block text-[11px] font-medium text-muted-foreground"
          >
            {t.settings.providerBaseUrl}
            {requiredMark}
          </label>
          <input
            id={baseUrlId}
            type="text"
            value={form.baseUrl}
            onChange={(e) => {
              onChange({ ...form, baseUrl: e.target.value });
              setTestResult(null);
            }}
            placeholder={t.settings.providerBaseUrlPlaceholder}
            className="w-full rounded-md border border-border bg-background px-3 py-2 text-xs font-mono focus:border-primary focus:outline-none"
          />
        </div>

        <div>
          <label
            htmlFor={apiKeyId}
            className="mb-2 block text-[11px] font-medium text-muted-foreground"
          >
            {t.settings.providerApiKey}
          </label>
          <input
            id={apiKeyId}
            type="password"
            value={form.apiKey}
            onChange={(e) => {
              onChange({ ...form, apiKey: e.target.value });
              setTestResult(null);
            }}
            placeholder={t.settings.providerApiKeyPlaceholder}
            className="w-full rounded-md border border-border bg-background px-3 py-2 text-xs font-mono focus:border-primary focus:outline-none"
          />
        </div>
      </div>

      {/* ── Models ── */}
      <div>
        <div className="mb-2 flex items-center justify-between">
          <label htmlFor={modelsInputId} className="text-[11px] font-medium text-muted-foreground">
            {t.settings.providerModels}
            {modelCount > 0 && (
              <span className="ml-1.5 font-normal text-muted-foreground/60">({modelCount})</span>
            )}
          </label>
          <button
            type="button"
            onClick={() => handleFetchModels(true)}
            disabled={!canFetch || fetching}
            className="flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1 text-[10px] text-muted-foreground hover:bg-accent hover:text-foreground disabled:opacity-40"
          >
            {fetching ? (
              <Loader2 className="h-3 w-3 animate-spin" />
            ) : (
              <RefreshCw className="h-3 w-3" />
            )}
            {modelCount === 0 ? t.settings.fetchModelsFromServer : t.settings.fetchModels}
          </button>
        </div>
        <ModelChips
          models={form.models}
          onChange={(models) => onChange({ ...form, models })}
          newModels={newModels}
          inputId={modelsInputId}
        />
        {fetchError && <p className="mt-1.5 text-[10px] text-destructive">{fetchError}</p>}
      </div>

      {/* ── Advanced (collapsible) ── */}
      <div>
        <button
          type="button"
          onClick={() => setShowAdvanced(!showAdvanced)}
          className="flex items-center gap-1 text-[10px] text-muted-foreground hover:text-foreground"
        >
          <ChevronDown
            className={cn("h-3 w-3 transition-transform", !showAdvanced && "-rotate-90")}
          />
          {t.settings.advancedOptions}
        </button>
        {showAdvanced && (
          <div className="mt-3">
            <label
              htmlFor={providerIdInputId}
              className="mb-2 block text-[11px] font-medium text-muted-foreground"
            >
              {t.settings.providerId}
              {requiredMark}
            </label>
            <input
              id={providerIdInputId}
              type="text"
              value={form.id}
              onChange={(e) => handleIdChange(e.target.value)}
              placeholder={t.settings.providerIdPlaceholder}
              disabled={!isNew}
              className={cn(
                "w-full rounded-md border border-border bg-background px-3 py-2 text-xs font-mono focus:border-primary focus:outline-none",
                !isNew && "opacity-60",
                isNew && form.id.trim() !== "" && !idValidForUi && "border-destructive",
              )}
            />
            <p className="mt-1.5 text-[9px] text-muted-foreground/70">
              {t.settings.providerIdHint}
            </p>
          </div>
        )}
      </div>

      {/* ── Action bar ── */}
      <div className="flex items-center gap-3 border-t border-border pt-4">
        <button
          type="button"
          onClick={() => handleFetchModels(false)}
          disabled={!canFetch || fetching}
          className="flex items-center gap-1.5 rounded-md border border-border px-3 py-1.5 text-[11px] text-muted-foreground hover:bg-accent hover:text-foreground disabled:opacity-40"
        >
          {fetching ? <Loader2 className="h-3 w-3 animate-spin" /> : <Zap className="h-3 w-3" />}
          {t.settings.testConnection}
        </button>

        {testResult && (
          <span
            className={cn(
              "flex items-center gap-1 text-[10px]",
              testResult.ok ? "text-success" : "text-destructive",
            )}
          >
            {testResult.ok ? <Check className="h-3 w-3" /> : <X className="h-3 w-3" />}
            {testResult.message}
          </span>
        )}

        <div className="ml-auto flex gap-2.5">
          <button
            type="button"
            onClick={onCancel}
            className="rounded-md px-4 py-1.5 text-[11px] text-muted-foreground hover:bg-accent hover:text-foreground"
          >
            {t.common.cancel}
          </button>
          <button
            type="button"
            onClick={onSave}
            disabled={!canSave}
            className="rounded-md bg-primary px-4 py-1.5 text-[11px] text-primary-foreground disabled:opacity-50"
          >
            {t.common.save}
          </button>
        </div>
      </div>
    </div>
  );
}

/* ── Provider row ──────────────────────────────────────────────── */

function ProviderRow({
  id,
  cfg,
  available,
  modelCount,
  onEdit,
  onRemove,
}: {
  id: string;
  cfg: CustomProviderConfig;
  available: boolean;
  modelCount: number;
  onEdit: () => void;
  onRemove: () => void;
}) {
  const { t } = useI18n();
  const label = cfg.displayName || id;

  const statusTooltip = available
    ? isLocalUrl(cfg.baseUrl)
      ? t.settings.statusAvailableLocal
      : t.settings.statusAvailableKey
    : t.settings.statusUnavailable;

  return (
    <div className="flex items-center gap-2 rounded-md border border-border px-2.5 py-2 text-xs">
      <span
        className={cn("h-2 w-2 shrink-0 rounded-full", available ? "bg-success" : "bg-muted")}
        title={statusTooltip}
      />
      <span className="w-28 truncate font-medium" title={label}>
        {label}
      </span>
      <span className="truncate text-[10px] font-mono text-muted-foreground" title={cfg.baseUrl}>
        {cfg.baseUrl}
      </span>
      <span className="ml-auto shrink-0 text-[10px] text-muted-foreground">
        {modelCount} {t.settings.modelsCount}
      </span>
      <button
        type="button"
        onClick={onEdit}
        className="shrink-0 rounded p-1 text-muted-foreground hover:bg-accent hover:text-foreground"
        title={t.settings.editProvider}
      >
        <Pencil className="h-3 w-3" />
      </button>
      <button
        type="button"
        onClick={onRemove}
        className="shrink-0 rounded p-1 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
        title={t.settings.removeProvider}
      >
        <Trash2 className="h-3 w-3" />
      </button>
    </div>
  );
}

/* ── Empty state - Preset Quick Cards ──────────────────────────── */

function PresetCards({ onSelect }: { onSelect: (presetId: string) => void }) {
  const { t } = useI18n();

  return (
    <div>
      <p className="mb-3 text-[11px] text-muted-foreground">{t.settings.noProvidersHint}</p>
      <div className="grid grid-cols-3 gap-2">
        {PROVIDER_PRESETS.map((p) => (
          <button
            key={p.id}
            type="button"
            onClick={() => onSelect(p.id)}
            className="flex flex-col items-start gap-1.5 rounded-lg border border-border p-3 text-left transition-colors hover:border-primary/40 hover:bg-accent/50"
          >
            <span className="text-xs font-medium text-foreground">{p.name}</span>
            <span className="text-[9px] leading-tight text-muted-foreground">
              {p.noKeyNeeded ? t.settings.localNoKey : p.description}
            </span>
            <span className="rounded-sm bg-muted/60 px-1.5 py-0.5 text-[8px] font-mono text-muted-foreground">
              {PROTOCOL_LABELS[p.protocol]}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}

/* ── Main page ─────────────────────────────────────────────────── */

export function ModelsPage({ settings, onSave, onRefresh }: PageProps) {
  const { t } = useI18n();
  const [newFallback, setNewFallback] = useState("");
  const [modelGroups, setModelGroups] = useState<ModelGroupData[]>([]);
  const [webaiProviders, setWebaiProviders] = useState<WebAiProviderInfo[]>([]);
  const [webaiAuthenticated, setWebaiAuthenticated] = useState<Set<string>>(new Set());

  const [addingNew, setAddingNew] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState<ProviderFormData>(emptyForm());
  const [confirmRemove, setConfirmRemove] = useState<string | null>(null);

  const fallbackModels = settings.fallbackModels ?? [];
  const aliases = settings.modelAliases ?? {};
  const providers = settings.providers ?? {};

  const refreshAll = useCallback(() => {
    listModelGroups()
      .then(setModelGroups)
      .catch(() => setModelGroups([]));
    Promise.all([webaiListProviders(), webaiListAuthenticated()])
      .then(([pList, aList]) => {
        setWebaiProviders(pList);
        setWebaiAuthenticated(new Set(aList));
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    refreshAll();
    let unlisten: (() => void) | undefined;
    listen("webai-auth-changed", () => refreshAll()).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [refreshAll]);

  const catalogModelOptions = useMemo(
    () => modelGroupsToSelectOptions(modelGroups, true),
    [modelGroups],
  );
  const modelSelectOptions = useMemo(
    () => withCurrentModelOption(catalogModelOptions, settings.model),
    [catalogModelOptions, settings.model],
  );

  const showModelSelect = catalogModelOptions.length > 0;

  /* ── provider CRUD handlers ── */

  const handleStartAdd = useCallback(() => {
    setEditingId(null);
    setForm(emptyForm());
    setAddingNew(true);
  }, []);

  const handleStartAddFromPreset = useCallback(
    (presetId: string) => {
      const p = PROVIDER_PRESETS.find((x) => x.id === presetId);
      if (!p) return;
      const baseId = p.id !== "custom" ? p.id : "";
      const newId = baseId ? uniqueId(baseId, providers) : "";
      setEditingId(null);
      setForm({
        id: newId,
        displayName: p.name !== "Custom" ? p.name : "",
        protocol: p.protocol,
        baseUrl: p.baseUrl,
        apiKey: "",
        models: p.defaultModels,
      });
      setAddingNew(true);
    },
    [providers],
  );

  const handleStartEdit = useCallback(
    (id: string) => {
      const cfg = providers[id];
      if (!cfg) return;
      const preset = PROVIDER_PRESETS.find((p) => p.id === id || p.baseUrl === cfg.baseUrl);
      setAddingNew(false);
      setEditingId(id);
      setForm({
        id,
        displayName: cfg.displayName ?? "",
        protocol: preset?.protocol ?? "openai-compatible",
        baseUrl: cfg.baseUrl,
        apiKey: cfg.apiKey ?? "",
        models: cfg.models ?? [],
      });
    },
    [providers],
  );

  const handleCancelForm = useCallback(() => {
    setAddingNew(false);
    setEditingId(null);
  }, []);

  const buildConfig = useCallback(
    (f: ProviderFormData): CustomProviderConfig => ({
      displayName: f.displayName.trim() || undefined,
      baseUrl: f.baseUrl.trim(),
      apiKey: f.apiKey.trim() || undefined,
      models: f.models.filter(Boolean),
    }),
    [],
  );

  const handleSaveProvider = useCallback(async () => {
    const cfg = buildConfig(form);
    let providerId = addingNew ? form.id.trim() : (editingId ?? form.id.trim());
    if (!providerId) {
      const base = urlToId(form.baseUrl) || "provider";
      providerId = uniqueId(base, providers);
    }
    try {
      await upsertProvider(providerId, cfg);
      setAddingNew(false);
      setEditingId(null);
      onRefresh?.();
      refreshAll();

      if (cfg.models.length === 0 && cfg.baseUrl) {
        fetchProviderModels(cfg.baseUrl, cfg.apiKey)
          .then((remote) => {
            if (remote.length > 0) {
              upsertProvider(providerId, { ...cfg, models: remote }).then(() => {
                onRefresh?.();
                refreshAll();
              });
            }
          })
          .catch(() => {});
      }
    } catch (err) {
      console.error("Failed to save provider:", err);
    }
  }, [form, addingNew, editingId, providers, buildConfig, refreshAll, onRefresh]);

  const handleRemoveProvider = useCallback(
    async (id: string) => {
      try {
        await removeProvider(id);
        setConfirmRemove(null);
        onRefresh?.();
        refreshAll();
      } catch (err) {
        console.error("Failed to remove provider:", err);
      }
    },
    [refreshAll, onRefresh],
  );

  const providerEntries = Object.entries(providers);

  return (
    <>
      <Section title={t.settings.defaultModel}>
        <Field label={t.settings.model} hint={t.settings.modelHint}>
          {showModelSelect ? (
            <Select
              fullWidth
              value={settings.model ?? ""}
              options={modelSelectOptions}
              onChange={(v) => onSave({ model: v || undefined })}
              placeholder={t.settings.modelPlaceholder}
            />
          ) : (
            <TextInput
              value={settings.model ?? ""}
              onChange={(v) => onSave({ model: v || undefined })}
              placeholder={t.settings.modelPlaceholder}
            />
          )}
        </Field>

        <Field label={t.settings.thinkingMode}>
          <div className="flex items-center gap-2">
            <Toggle
              checked={settings.thinkingMode ?? false}
              onChange={(v) => onSave({ thinkingMode: v })}
            />
            <span className="text-[10px] text-muted-foreground">{t.settings.thinkingModeHint}</span>
          </div>
        </Field>

        <Field label={t.settings.maxContextTokens}>
          <NumberInput
            value={settings.maxContextTokens ?? 200000}
            onChange={(v) => onSave({ maxContextTokens: v })}
            min={1000}
            max={2000000}
            step={1000}
          />
        </Field>

        <Field label={t.settings.streamTimeout} hint={t.settings.streamTimeoutHint}>
          <NumberInput
            value={settings.streamTimeout ?? 300}
            onChange={(v) => onSave({ streamTimeout: v })}
            min={30}
            max={600}
            step={30}
          />
        </Field>
      </Section>

      <Section title={t.settings.modelAliases}>
        <div className="space-y-1">
          {Object.entries(aliases).map(([alias, model]) => (
            <div key={alias} className="flex items-center gap-2 text-xs">
              <span className="font-mono text-primary">{alias}</span>
              <ChevronRight className="h-3 w-3 text-muted-foreground" />
              <span className="flex-1 truncate font-mono text-muted-foreground">{model}</span>
            </div>
          ))}
          {Object.keys(aliases).length === 0 && (
            <p className="text-[10px] text-muted-foreground">{t.settings.modelAliasesEmpty}</p>
          )}
        </div>
      </Section>

      <Section title={t.settings.fallbackModels}>
        <div className="space-y-1">
          {fallbackModels.map((m, i) => (
            <div key={m} className="flex items-center gap-2 text-xs">
              <span className="w-4 text-muted-foreground">{i + 1}.</span>
              <span className="flex-1 truncate font-mono">{m}</span>
              <button
                type="button"
                onClick={() => {
                  const next = fallbackModels.filter((_, j) => j !== i);
                  onSave({ fallbackModels: next });
                }}
                className="text-muted-foreground hover:text-destructive"
              >
                <X className="h-3 w-3" />
              </button>
            </div>
          ))}
          <div className="flex items-center gap-1">
            <TextInput
              value={newFallback}
              onChange={setNewFallback}
              placeholder={t.settings.fallbackPlaceholder}
            />
            <button
              type="button"
              disabled={!newFallback.trim()}
              onClick={() => {
                onSave({ fallbackModels: [...fallbackModels, newFallback.trim()] });
                setNewFallback("");
              }}
              className="rounded bg-primary px-2 py-1.5 text-[10px] text-primary-foreground disabled:opacity-50"
            >
              <Plus className="h-3 w-3" />
            </button>
          </div>
        </div>
      </Section>

      <Section
        title={t.settings.providers}
        action={
          !addingNew && !editingId ? (
            <button
              type="button"
              onClick={handleStartAdd}
              className="flex items-center gap-1 rounded px-2 py-0.5 text-[10px] text-muted-foreground hover:bg-accent hover:text-foreground"
            >
              <Plus className="h-3 w-3" />
              {t.settings.addProvider}
            </button>
          ) : undefined
        }
      >
        {addingNew && (
          <ProviderForm
            form={form}
            onChange={setForm}
            onSave={handleSaveProvider}
            onCancel={handleCancelForm}
            isNew
            existingIds={providers}
          />
        )}

        {providerEntries.length === 0 && !addingNew && (
          <PresetCards onSelect={handleStartAddFromPreset} />
        )}

        <div className="space-y-2">
          {providerEntries.map(([id, cfg]) => {
            if (editingId === id) {
              return (
                <ProviderForm
                  key={id}
                  form={form}
                  onChange={setForm}
                  onSave={handleSaveProvider}
                  onCancel={handleCancelForm}
                  isNew={false}
                  existingIds={providers}
                />
              );
            }
            if (confirmRemove === id) {
              return (
                <div
                  key={id}
                  className="flex items-center gap-2 rounded-md border border-destructive/30 bg-destructive/5 px-2.5 py-2 text-xs"
                >
                  <span className="flex-1">{t.settings.confirmRemoveProvider}</span>
                  <button
                    type="button"
                    onClick={() => handleRemoveProvider(id)}
                    className="rounded bg-destructive px-2 py-1 text-[10px] text-destructive-foreground"
                  >
                    {t.common.confirm}
                  </button>
                  <button
                    type="button"
                    onClick={() => setConfirmRemove(null)}
                    className="rounded px-2 py-1 text-[10px] text-muted-foreground hover:text-foreground"
                  >
                    {t.common.cancel}
                  </button>
                </div>
              );
            }
            const group = modelGroups.find((g) => g.provider === id);
            return (
              <ProviderRow
                key={id}
                id={id}
                cfg={cfg}
                available={group?.available ?? false}
                modelCount={group?.models.length ?? cfg.models?.length ?? 0}
                onEdit={() => handleStartEdit(id)}
                onRemove={() => setConfirmRemove(id)}
              />
            );
          })}
        </div>

        {webaiProviders.length > 0 && (
          <div className="mt-4">
            <h4 className="mb-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
              Web AI — {t.settings.free}
            </h4>
            <div className="space-y-1.5">
              {webaiProviders.map((wp) => {
                const authed = webaiAuthenticated.has(wp.id);
                return (
                  <div
                    key={wp.id}
                    className="flex items-center gap-2 rounded border border-border px-2.5 py-2 text-xs"
                  >
                    <span
                      className={cn("h-2 w-2 rounded-full", authed ? "bg-success" : "bg-muted")}
                    />
                    <span className="w-28 font-medium">{wp.name}</span>
                    <span className="flex-1 text-[10px] text-muted-foreground">
                      {wp.models.length} {t.settings.modelsCount}
                    </span>
                    {!authed && (
                      <button
                        type="button"
                        onClick={async () => {
                          try {
                            await webaiStartAuth(wp.id);
                            const aList = await webaiListAuthenticated();
                            setWebaiAuthenticated(new Set(aList));
                          } catch {
                            /* user may cancel */
                          }
                        }}
                        className="flex items-center gap-1 rounded px-2 py-0.5 text-[10px] text-muted-foreground hover:bg-accent hover:text-foreground"
                      >
                        <Globe className="h-3 w-3" />
                        {t.settings.login}
                      </button>
                    )}
                    {authed && (
                      <span className="text-[10px] text-success">{t.settings.loggedIn}</span>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </Section>
    </>
  );
}
