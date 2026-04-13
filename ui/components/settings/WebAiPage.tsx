import { listen } from "@tauri-apps/api/event";
import { Loader2, LogIn, LogOut, PowerOff, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useI18n } from "@/lib/i18n";
import {
  type AppSettings,
  type WebAiPageStatus,
  type WebAiProviderInfo,
  webaiCloseAllPages,
  webaiClosePage,
  webaiListAuthenticated,
  webaiListPages,
  webaiListProviders,
  webaiLogout,
  webaiStartAuth,
} from "@/lib/tauri";
import { cn } from "@/lib/utils";
import { NumberInput, Section } from "./shared";

/* ── Compact provider row ─────────────────────────────────────────── */

function ProviderRow({
  provider,
  authenticated,
  loggingIn,
  pageActive,
  onLogin,
  onLogout,
  onClosePage,
  disabled,
  closingPage,
}: {
  provider: WebAiProviderInfo;
  authenticated: boolean;
  loggingIn: boolean;
  pageActive: boolean;
  onLogin: () => void;
  onLogout: () => void;
  onClosePage: () => void;
  disabled: boolean;
  closingPage: boolean;
}) {
  const { t } = useI18n();

  return (
    <div
      className={cn(
        "group flex items-center gap-2.5 rounded-md px-2 py-1.5 transition-colors",
        loggingIn && "bg-amber-500/5",
      )}
    >
      {/* Status dot: amber=logging-in, green=logged-in (pulse if webview active), gray=not-logged-in */}
      <span
        className={cn(
          "h-2 w-2 shrink-0 rounded-full",
          loggingIn
            ? "bg-amber-500 animate-pulse"
            : authenticated
              ? pageActive
                ? "bg-emerald-500 animate-pulse"
                : "bg-emerald-500"
              : "bg-muted-foreground/25",
        )}
      />

      {/* Name + model count */}
      <span className="flex-1 min-w-0 text-xs font-medium leading-tight">{provider.name}</span>
      <span className="shrink-0 rounded bg-muted px-1.5 py-0.5 text-[9px] tabular-nums text-muted-foreground">
        {provider.models.length}
      </span>

      {/* WebView close button — only when page is active */}
      {pageActive && (
        <button
          type="button"
          disabled={closingPage}
          onClick={onClosePage}
          className="shrink-0 rounded p-0.5 text-muted-foreground/60 hover:bg-destructive/10 hover:text-destructive disabled:opacity-40"
          title={t.settings.closePage}
        >
          <PowerOff className="h-3 w-3" />
        </button>
      )}

      {/* Auth action */}
      {loggingIn ? (
        <span className="shrink-0 flex items-center gap-1 text-[10px] text-amber-500">
          <Loader2 className="h-3 w-3 animate-spin" />
        </span>
      ) : authenticated ? (
        <button
          type="button"
          disabled={disabled}
          onClick={onLogout}
          className="shrink-0 rounded p-0.5 text-muted-foreground/60 opacity-0 transition-opacity group-hover:opacity-100 hover:bg-destructive/10 hover:text-destructive disabled:opacity-40"
          title={t.settings.logout}
        >
          <LogOut className="h-3 w-3" />
        </button>
      ) : (
        <button
          type="button"
          disabled={disabled}
          onClick={onLogin}
          className="shrink-0 flex items-center gap-1 rounded-md bg-primary/90 px-2 py-0.5 text-[10px] font-medium text-primary-foreground hover:bg-primary disabled:opacity-40"
        >
          <LogIn className="h-2.5 w-2.5" />
          {t.settings.login}
        </button>
      )}
    </div>
  );
}

/* ── Main page ────────────────────────────────────────────────────── */

export function WebAiPage({
  settings,
  onSave,
}: {
  settings: AppSettings;
  onSave: (patch: Partial<AppSettings>) => void;
}) {
  const { t } = useI18n();
  const [providers, setProviders] = useState<WebAiProviderInfo[]>([]);
  const [authenticated, setAuthenticated] = useState<Set<string>>(new Set());
  const [loggingInId, setLoggingInId] = useState<string | null>(null);
  const [pages, setPages] = useState<WebAiPageStatus[]>([]);
  const [closingPage, setClosingPage] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const [pList, aList, pgList] = await Promise.all([
        webaiListProviders(),
        webaiListAuthenticated(),
        webaiListPages(),
      ]);
      setProviders(pList);
      setAuthenticated(new Set(aList));
      setPages(pgList);
    } catch {
      /* backend may not be ready */
    }
  }, []);

  useEffect(() => {
    refresh();
    let unlisten: (() => void) | undefined;
    listen("webai-auth-changed", () => refresh()).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [refresh]);

  /* ── handlers ── */

  const handleLogin = useCallback(
    async (providerId: string) => {
      setLoggingInId(providerId);
      try {
        await webaiStartAuth(providerId);
      } catch (err) {
        console.error("WebAI login failed:", err);
      }
      await refresh();
      setLoggingInId(null);
    },
    [refresh],
  );

  const handleLogout = useCallback(
    async (providerId: string) => {
      setLoggingInId(providerId);
      try {
        await webaiLogout(providerId);
      } catch (err) {
        console.error("WebAI logout failed:", err);
      }
      await refresh();
      setLoggingInId(null);
    },
    [refresh],
  );

  const handleLoginAll = useCallback(async () => {
    for (const p of providers) {
      if (!authenticated.has(p.id)) {
        setLoggingInId(p.id);
        try {
          await webaiStartAuth(p.id);
        } catch {
          /* continue with next */
        }
        await refresh();
      }
    }
    setLoggingInId(null);
  }, [providers, authenticated, refresh]);

  const handleLogoutAll = useCallback(async () => {
    setLoggingInId("__all__");
    for (const id of authenticated) {
      try {
        await webaiLogout(id);
      } catch {
        /* continue */
      }
    }
    await refresh();
    setLoggingInId(null);
  }, [authenticated, refresh]);

  const handleClosePage = useCallback(
    async (providerId: string) => {
      setClosingPage(true);
      try {
        await webaiClosePage(providerId);
      } catch (err) {
        console.error("WebAI close page failed:", err);
      }
      await refresh();
      setClosingPage(false);
    },
    [refresh],
  );

  const handleCloseAllPages = useCallback(async () => {
    setClosingPage(true);
    try {
      await webaiCloseAllPages();
    } catch (err) {
      console.error("WebAI close all pages failed:", err);
    }
    await refresh();
    setClosingPage(false);
  }, [refresh]);

  /* ── derived data ── */

  const busy = loggingInId !== null;
  const pageMap = useMemo(() => new Map(pages.map((p) => [p.providerId, p])), [pages]);

  const loggedIn = useMemo(
    () => providers.filter((p) => authenticated.has(p.id)),
    [providers, authenticated],
  );
  const notLoggedIn = useMemo(
    () => providers.filter((p) => !authenticated.has(p.id)),
    [providers, authenticated],
  );
  const activePageCount = pages.filter((p) => p.active).length;

  /* ── render helper ── */

  const renderRow = (p: WebAiProviderInfo) => (
    <ProviderRow
      key={p.id}
      provider={p}
      authenticated={authenticated.has(p.id)}
      loggingIn={loggingInId === p.id}
      pageActive={pageMap.get(p.id)?.active ?? false}
      onLogin={() => handleLogin(p.id)}
      onLogout={() => handleLogout(p.id)}
      onClosePage={() => handleClosePage(p.id)}
      disabled={busy}
      closingPage={closingPage}
    />
  );

  return (
    <Section title={t.settings.webAi}>
      <p className="mb-3 text-[10px] leading-relaxed text-muted-foreground">
        {t.settings.webAiDesc}
      </p>

      {/* ── Logged-in group ── */}
      {loggedIn.length > 0 && (
        <div className="mb-3">
          <div className="mb-1 flex items-center justify-between">
            <span className="text-[10px] font-semibold uppercase tracking-wider text-emerald-600 dark:text-emerald-400">
              {t.settings.loggedIn} ({loggedIn.length})
            </span>
            <div className="flex items-center gap-1.5">
              {activePageCount > 0 && (
                <button
                  type="button"
                  disabled={closingPage}
                  onClick={handleCloseAllPages}
                  className="flex items-center gap-1 rounded px-1.5 py-0.5 text-[9px] text-muted-foreground hover:bg-destructive/10 hover:text-destructive disabled:opacity-40"
                >
                  <X className="h-2.5 w-2.5" />
                  {t.settings.closeAll}
                </button>
              )}
              <button
                type="button"
                disabled={busy || loggedIn.length === 0}
                onClick={handleLogoutAll}
                className="flex items-center gap-1 rounded px-1.5 py-0.5 text-[9px] text-muted-foreground hover:bg-destructive/10 hover:text-destructive disabled:opacity-40"
              >
                <LogOut className="h-2.5 w-2.5" />
                {t.settings.logoutAll}
              </button>
            </div>
          </div>
          <div className="rounded-lg border border-border bg-card/50">
            {loggedIn.map(renderRow)}
          </div>
        </div>
      )}

      {/* ── Not-logged-in group ── */}
      {notLoggedIn.length > 0 && (
        <div className="mb-3">
          <div className="mb-1 flex items-center justify-between">
            <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
              {t.settings.notLoggedIn} ({notLoggedIn.length})
            </span>
            <button
              type="button"
              disabled={busy || notLoggedIn.length === 0}
              onClick={handleLoginAll}
              className="flex items-center gap-1 rounded px-1.5 py-0.5 text-[9px] text-muted-foreground hover:bg-accent disabled:opacity-40"
            >
              <LogIn className="h-2.5 w-2.5" />
              {t.settings.loginAll}
            </button>
          </div>
          <div className="rounded-lg border border-border bg-card/50">
            {notLoggedIn.map(renderRow)}
          </div>
        </div>
      )}

      {providers.length === 0 && (
        <p className="py-6 text-center text-xs text-muted-foreground">{t.common.loading}</p>
      )}

      {/* ── Settings ── */}
      <div className="mt-1 space-y-px rounded-lg border border-border bg-card/50">
        <div className="flex items-center justify-between px-3 py-2">
          <div className="min-w-0 pr-3">
            <div className="text-[10px] font-medium text-foreground/80">
              {t.settings.webAiIdleTimeout}
            </div>
            <div className="text-[9px] text-muted-foreground">
              {t.settings.webAiIdleTimeoutHint}
            </div>
          </div>
          <NumberInput
            value={settings.webaiIdleTimeout ?? 300}
            onChange={(v) => onSave({ webaiIdleTimeout: v })}
            min={60}
            max={1800}
            step={60}
          />
        </div>
        <div className="border-t border-border" />
        <div className="flex items-center justify-between px-3 py-2">
          <div className="min-w-0 pr-3">
            <div className="text-[10px] font-medium text-foreground/80">
              {t.settings.webAiPageLoadTimeout}
            </div>
            <div className="text-[9px] text-muted-foreground">
              {t.settings.webAiPageLoadTimeoutHint}
            </div>
          </div>
          <NumberInput
            value={settings.webaiPageLoadTimeout ?? 60}
            onChange={(v) => onSave({ webaiPageLoadTimeout: v })}
            min={10}
            max={300}
            step={10}
          />
        </div>
      </div>
    </Section>
  );
}
