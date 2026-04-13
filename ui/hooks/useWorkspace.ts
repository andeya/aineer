import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import {
  type AppSettings,
  gitBranch as fetchGitBranch,
  getProjectRoot,
  getSettings,
  getSlashCommands,
  gitCheckout,
  gitListBranches,
  listModelGroups,
  type ModelGroupData,
  type SlashCommandDef,
  tryInvoke,
  updateSettings,
} from "@/lib/tauri";

export function useWorkspace() {
  const [projectRoot, setProjectRoot] = useState("");
  const [gitBranchName, setGitBranchName] = useState("");
  const [modelName, setModelName] = useState("");
  const [streamTimeoutMs, setStreamTimeoutMs] = useState(0);
  const [modelGroups, setModelGroups] = useState<ModelGroupData[]>([]);
  const [slashCommands, setSlashCommands] = useState<SlashCommandDef[]>([]);

  const refreshModelGroups = useCallback(() => {
    tryInvoke(listModelGroups, []).then((groups) => {
      if (groups && groups.length > 0) setModelGroups(groups);
    });
  }, []);

  useEffect(() => {
    tryInvoke(getProjectRoot, "").then((root) => {
      setProjectRoot(root);
      if (root) {
        tryInvoke(() => fetchGitBranch(root), null).then((b) => setGitBranchName(b || ""));
      }
    });
    tryInvoke(getSlashCommands, []).then(setSlashCommands);
    tryInvoke(getSettings, {} as AppSettings).then((s) => {
      setModelName(s.model || "");
      if (s.streamTimeout) setStreamTimeoutMs(s.streamTimeout * 1000);
    });
    refreshModelGroups();

    let unlisten: (() => void) | undefined;
    listen("webai-auth-changed", () => refreshModelGroups()).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [refreshModelGroups]);

  const handleListBranches = useCallback(() => gitListBranches(projectRoot), [projectRoot]);

  const handleSwitchBranch = useCallback(
    (branch: string) => {
      gitCheckout(projectRoot, branch)
        .then(() => tryInvoke(() => fetchGitBranch(projectRoot), null))
        .then((b) => setGitBranchName(b || ""));
    },
    [projectRoot],
  );

  const handleSelectModel = useCallback((fullModel: string) => {
    setModelName(fullModel);
    updateSettings({ model: fullModel }).catch(() => {});
  }, []);

  return {
    projectRoot,
    gitBranchName,
    modelName,
    streamTimeoutMs,
    modelGroups,
    slashCommands,
    handleListBranches,
    handleSwitchBranch,
    handleSelectModel,
  };
}
