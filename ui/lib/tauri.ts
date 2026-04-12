/**
 * Backward-compatible barrel — all IPC logic now lives in ipc/ submodules.
 * Prefer importing from "@/lib/ipc" or specific submodules for new code.
 */
export * from "./ipc";
