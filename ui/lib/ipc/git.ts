import { call } from "./call";

export interface GitFileStatus {
  path: string;
  status: string;
}

export interface GitStatus {
  branch: string | null;
  changed_files: GitFileStatus[];
}

export interface GitBranchInfo {
  name: string;
  is_current: boolean;
}

export const gitStatus = (cwd: string) => call<GitStatus>("git_status", { cwd });
export const gitBranch = (cwd: string) => call<string | null>("git_branch", { cwd });
export const gitDiff = (cwd: string, path: string) => call<string>("git_diff", { cwd, path });
export const gitListBranches = (cwd: string) => call<GitBranchInfo[]>("git_list_branches", { cwd });
export const gitCheckout = (cwd: string, branch: string) =>
  call<void>("git_checkout", { cwd, branch });
