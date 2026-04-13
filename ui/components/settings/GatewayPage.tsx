import { Check, Copy, Loader2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Select } from "@/components/ui/select";
import { useI18n } from "@/lib/i18n";
import { modelGroupsToSelectOptions, withCurrentModelOption } from "@/lib/model-options";
import {
  type AppSettings,
  type GatewayStatusInfo,
  getGatewayStatus,
  listModelGroups,
  type ModelGroupData,
  startGateway,
  stopGateway,
} from "@/lib/tauri";
import { cn } from "@/lib/utils";
import { Field, Section, TextInput, Toggle } from "./shared";

type StatusKey = "running" | "starting" | "stopped" | "error";

function useGatewayStatus() {
  const [status, setStatus] = useState<GatewayStatusInfo | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const refresh = useCallback(async () => {
    try {
      const s = await getGatewayStatus();
      setStatus(s);
      return s;
    } catch {
      setStatus(null);
      return null;
    }
  }, []);

  const startPolling = useCallback(() => {
    if (pollRef.current) return;
    pollRef.current = setInterval(async () => {
      const s = await refresh();
      const key = s?.status as StatusKey | undefined;
      if (key && key !== "starting") {
        if (pollRef.current) {
          clearInterval(pollRef.current);
          pollRef.current = null;
        }
      }
    }, 800);
  }, [refresh]);

  const stopPolling = useCallback(() => {
    if (pollRef.current) {
      clearInterval(pollRef.current);
      pollRef.current = null;
    }
  }, []);

  useEffect(() => () => stopPolling(), [stopPolling]);

  return { status, refresh, startPolling, stopPolling };
}

const STATUS_STYLE: Record<StatusKey, { dot: string; badge: string }> = {
  running: {
    dot: "bg-emerald-500",
    badge: "border border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-400",
  },
  starting: {
    dot: "bg-amber-500 animate-pulse",
    badge: "border border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-400",
  },
  stopped: {
    dot: "bg-zinc-400 dark:bg-zinc-500",
    badge: "border border-zinc-300 dark:border-zinc-600 bg-zinc-100 dark:bg-zinc-800 text-zinc-600 dark:text-zinc-400",
  },
  error: {
    dot: "bg-red-500",
    badge: "border border-red-500/30 bg-red-500/10 text-red-700 dark:text-red-400",
  },
};

export function GatewayPage({
  settings,
  onSave,
}: {
  settings: AppSettings;
  onSave: (updates: Partial<AppSettings>) => void;
}) {
  const { t } = useI18n();
  const gw = settings.gateway ?? {};
  const enabled = gw.enabled ?? false;
  const listenAddr = gw.listenAddr ?? "127.0.0.1:8090";

  const { status, refresh, startPolling } = useGatewayStatus();
  const [copied, setCopied] = useState(false);
  const [modelGroups, setModelGroups] = useState<ModelGroupData[]>([]);

  const baseUrl = `http://${listenAddr}/v1`;

  useEffect(() => {
    refresh().then(async (s) => {
      const key = s?.status as StatusKey | undefined;
      if (key === "starting") {
        startPolling();
      } else if (enabled && (!key || key === "stopped")) {
        try {
          await startGateway();
          startPolling();
        } catch {
          await refresh();
        }
      }
    });
    listModelGroups()
      .then(setModelGroups)
      .catch(() => setModelGroups([]));
  }, [enabled, refresh, startPolling]);

  const catalogModelOptions = useMemo(
    () => modelGroupsToSelectOptions(modelGroups, true),
    [modelGroups],
  );
  const modelSelectOptions = useMemo(
    () => withCurrentModelOption(catalogModelOptions, gw.defaultModel),
    [catalogModelOptions, gw.defaultModel],
  );

  const handleToggle = useCallback(
    async (on: boolean) => {
      const next = { ...gw, enabled: on };
      await onSave({ gateway: next });
      try {
        if (on) {
          await startGateway();
          startPolling();
        } else {
          await stopGateway();
          await refresh();
        }
      } catch (err) {
        console.error("Gateway toggle failed:", err);
        await refresh();
      }
    },
    [gw, onSave, refresh, startPolling],
  );

  const handleAddrChange = useCallback(
    (addr: string) => {
      onSave({ gateway: { ...gw, listenAddr: addr } });
    },
    [gw, onSave],
  );

  const handleModelChange = useCallback(
    (model: string) => {
      onSave({ gateway: { ...gw, defaultModel: model || undefined } });
    },
    [gw, onSave],
  );

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(baseUrl);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }, [baseUrl]);

  const statusKey: StatusKey = (status?.status as StatusKey) ?? "stopped";
  const style = STATUS_STYLE[statusKey] ?? STATUS_STYLE.stopped;

  const statusLabel: Record<StatusKey, string> = {
    running: t.settings.gatewayRunning,
    starting: t.settings.gatewayStarting,
    stopped: t.settings.gatewayStopped,
    error: t.settings.gatewayError,
  };

  return (
    <Section title={t.settings.clawGateway}>
      <p className="mb-4 text-[10px] text-muted-foreground">{t.settings.gatewayDesc}</p>

      <Field label={t.settings.enableGateway}>
        <div className="flex items-center gap-3">
          <Toggle checked={enabled} onChange={handleToggle} />
          <span
            className={cn(
              "flex items-center gap-1.5 rounded-full px-2 py-0.5 text-[10px] font-medium",
              style.badge,
            )}
          >
            {statusKey === "starting" ? (
              <Loader2 className="h-3 w-3 animate-spin" />
            ) : (
              <span className={cn("h-1.5 w-1.5 rounded-full", style.dot)} />
            )}
            {statusLabel[statusKey]}
          </span>
        </div>
      </Field>

      <Field label={t.settings.listenAddress} hint={t.settings.listenAddressHint}>
        <TextInput value={listenAddr} onChange={handleAddrChange} placeholder="127.0.0.1:8090" />
      </Field>

      <Field label={t.settings.gatewayDefaultModel}>
        {catalogModelOptions.length > 0 ? (
          <Select
            fullWidth
            value={gw.defaultModel ?? ""}
            options={modelSelectOptions}
            onChange={handleModelChange}
            placeholder={t.settings.modelPlaceholder}
          />
        ) : (
          <TextInput
            value={gw.defaultModel ?? ""}
            onChange={handleModelChange}
            placeholder={t.settings.modelPlaceholder}
          />
        )}
      </Field>

      <Field label={t.settings.baseUrl}>
        <div className="flex items-center gap-2">
          <code className="flex-1 rounded-md border border-border bg-muted px-2.5 py-1.5 font-mono text-xs">
            {baseUrl}
          </code>
          <button
            type="button"
            onClick={handleCopy}
            className={cn(
              "flex items-center gap-1 rounded-md border border-border px-2.5 py-1.5 text-[10px] transition-colors",
              copied
                ? "border-success text-success"
                : "text-muted-foreground hover:bg-accent hover:text-foreground",
            )}
          >
            {copied ? (
              <>
                <Check className="h-3 w-3" />
                {t.settings.copied}
              </>
            ) : (
              <>
                <Copy className="h-3 w-3" />
                {t.settings.copyUrl}
              </>
            )}
          </button>
        </div>
        <p className="mt-1.5 text-[10px] text-muted-foreground">{t.settings.gatewayTip}</p>
      </Field>
    </Section>
  );
}
