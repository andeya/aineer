# codineer-tools

AI-callable tool definitions and execution for [Codineer](https://github.com/andeya/codineer).

This crate implements the built-in tools available to the AI agent:

| Category            | Tools                                                                                             |
| ------------------- | ------------------------------------------------------------------------------------------------- |
| **File I/O**        | `read_file`, `write_file`, `edit_file`, `glob_search`, `grep_search`                              |
| **Shell**           | `bash`, `PowerShell`, `REPL`                                                                      |
| **Web**             | `WebFetch`, `WebSearch`                                                                           |
| **Notebook**        | `NotebookEdit`                                                                                    |
| **Agent**           | `Agent` (sub-agent orchestration), `SendUserMessage`                                              |
| **LSP**             | `Lsp` (hover, completion, go-to-definition, references, symbols, rename, formatting, diagnostics) |
| **Task management** | `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, `TaskStop`                                     |
| **Plan mode**       | `EnterPlanMode`, `ExitPlanMode`                                                                   |
| **Git worktree**    | `EnterWorktree`, `ExitWorktree`                                                                   |
| **Cron**            | `CronCreate`, `CronDelete`, `CronList`                                                            |
| **MCP resources**   | `ListMcpResources`, `ReadMcpResource`, `MCPSearch`                                                |
| **Collaboration**   | `TeamCreate`, `TeamDelete`, `SendMessage`, `SlashCommand`                                         |
| **Misc**            | `TodoWrite`, `Skill`, `ToolSearch`, `Config`, `StructuredOutput`, `Sleep`                         |

## Note

This is an internal crate of the Codineer project. It is published to crates.io as a dependency of `codineer-cli` and is not intended for standalone use. API stability is not guaranteed outside the Codineer workspace.

## License

MIT — see [LICENSE](https://github.com/andeya/codineer/blob/main/LICENSE) for details.
