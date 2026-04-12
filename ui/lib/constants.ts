export const INTERACTIVE_COMMANDS = new Set([
  "top",
  "htop",
  "btop",
  "vim",
  "nvim",
  "vi",
  "nano",
  "emacs",
  "less",
  "more",
  "man",
  "ssh",
  "telnet",
  "ftp",
  "sftp",
  "python",
  "python3",
  "node",
  "irb",
  "ghci",
  "lua",
  "mysql",
  "psql",
  "sqlite3",
  "redis-cli",
  "mongosh",
  "watch",
  "tail",
]);

export function isInteractiveCommand(input: string): boolean {
  const cmd = input.trim().split(/\s+/)[0];
  return INTERACTIVE_COMMANDS.has(cmd);
}
