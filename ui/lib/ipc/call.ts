let _isTauri: boolean | undefined;
export function isTauri(): boolean {
  if (_isTauri === undefined) {
    _isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  }
  return _isTauri;
}

async function getInvoke(): Promise<
  (cmd: string, args?: Record<string, unknown>) => Promise<unknown>
> {
  const mod = await import("@tauri-apps/api/core");
  return mod.invoke;
}

export async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const invoke = await getInvoke();
  return invoke(cmd, args) as Promise<T>;
}

export async function tryInvoke<T>(fn: () => Promise<T>, fallback: T): Promise<T> {
  if (!isTauri()) return fallback;
  try {
    return await fn();
  } catch {
    return fallback;
  }
}
