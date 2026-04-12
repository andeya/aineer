import { call } from "./call";

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number | null;
}

export interface SearchResult {
  path: string;
  is_dir: boolean;
  matches: ContentMatch[];
}

export interface ContentMatch {
  line_number: number;
  line: string;
}

export const getProjectRoot = () => call<string>("get_project_root");
export const listDir = (path: string) => call<FileEntry[]>("list_dir", { path });
export const readFile = (path: string) => call<string>("read_file", { path });
export const searchFiles = (dir: string, query: string, searchContent: boolean) =>
  call<SearchResult[]>("search_files", { dir, query, searchContent });
