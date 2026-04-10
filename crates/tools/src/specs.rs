use aineer_engine::PermissionMode;
use serde_json::json;

use crate::registry::{ToolSpec, ToolTier};

#[must_use]
#[allow(clippy::too_many_lines)]
pub fn mvp_tool_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "bash",
            description: "Execute a shell command in the current workspace.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string" },
                    "timeout": { "type": "integer", "minimum": 1 },
                    "description": { "type": "string" },
                    "run_in_background": { "type": "boolean" },
                    "dangerouslyDisableSandbox": { "type": "boolean" },
                    "namespaceRestrictions": { "type": "boolean" },
                    "isolateNetwork": { "type": "boolean" },
                    "filesystemMode": {
                        "type": "string",
                        "enum": ["none", "workspace-read-only", "workspace-full"]
                    },
                    "allowedMounts": {
                        "type": "array",
                        "items": { "type": "string" }
                    }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
            tier: ToolTier::Core,
        },
        ToolSpec {
            name: "read_file",
            description: "Read a text file from the workspace.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "offset": { "type": "integer", "minimum": 0 },
                    "limit": { "type": "integer", "minimum": 1 }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Core,
        },
        ToolSpec {
            name: "write_file",
            description: "Write a text file in the workspace.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            tier: ToolTier::Core,
        },
        ToolSpec {
            name: "edit_file",
            description: "Replace text in a workspace file.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "old_string": { "type": "string" },
                    "new_string": { "type": "string" },
                    "replace_all": { "type": "boolean" },
                    "last_modified_at": {
                        "type": "integer",
                        "description": "Nanoseconds since UNIX epoch returned by a prior read_file call. When set the edit is rejected if the file was modified since."
                    }
                },
                "required": ["path", "old_string", "new_string"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            tier: ToolTier::Core,
        },
        ToolSpec {
            name: "glob_search",
            description: "Find files by glob pattern.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string" },
                    "path": { "type": "string" }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Core,
        },
        ToolSpec {
            name: "grep_search",
            description: "Search file contents with a regex pattern. Returns matching file names, line content, or match counts depending on output_mode.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory or file path to search within (defaults to workspace root)"
                    },
                    "glob": {
                        "type": "string",
                        "description": "Glob pattern to filter files (e.g. \"*.ts\", \"**/*.rs\")"
                    },
                    "output_mode": {
                        "type": "string",
                        "enum": ["files_with_matches", "content", "count"],
                        "description": "files_with_matches (default): list matching file paths; content: show matching lines; count: show match counts per file"
                    },
                    "-B": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Lines of context before each match (requires output_mode: content)"
                    },
                    "-A": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Lines of context after each match (requires output_mode: content)"
                    },
                    "-C": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Lines of context before and after each match (requires output_mode: content)"
                    },
                    "context": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Alias for -C"
                    },
                    "-n": {
                        "type": "boolean",
                        "description": "Include line numbers in content output"
                    },
                    "-i": {
                        "type": "boolean",
                        "description": "Case-insensitive matching"
                    },
                    "type": {
                        "type": "string",
                        "description": "Filter by file type (e.g. \"rs\", \"ts\", \"py\")"
                    },
                    "head_limit": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Maximum number of results to return"
                    },
                    "offset": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "Skip the first N results"
                    },
                    "multiline": {
                        "type": "boolean",
                        "description": "Enable multiline mode where . matches newlines"
                    }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Core,
        },
        ToolSpec {
            name: "WebFetch",
            description:
                "Fetch a URL, convert it into readable text, and answer a prompt about it.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "format": "uri" },
                    "prompt": { "type": "string" }
                },
                "required": ["url", "prompt"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "WebSearch",
            description: "Search the web for current information and return cited results.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "minLength": 2 },
                    "allowed_domains": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "blocked_domains": {
                        "type": "array",
                        "items": { "type": "string" }
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "TodoWrite",
            description: "Update the structured task list for the current session.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "todos": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "content": { "type": "string" },
                                "activeForm": { "type": "string" },
                                "status": {
                                    "type": "string",
                                    "enum": ["pending", "in_progress", "completed"]
                                }
                            },
                            "required": ["content", "activeForm", "status"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["todos"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "Skill",
            description: "Load a local skill definition and its instructions.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "skill": { "type": "string" },
                    "args": { "type": "string" }
                },
                "required": ["skill"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "Agent",
            description: "Launch a specialized agent task and persist its handoff metadata.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "description": { "type": "string" },
                    "prompt": { "type": "string" },
                    "subagent_type": { "type": "string" },
                    "name": { "type": "string" },
                    "model": { "type": "string" }
                },
                "required": ["description", "prompt"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "ToolSearch",
            description: "Search for deferred or specialized tools by exact name or keywords. \
                Includes MCP tools when available.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "category": { "type": "string", "description": "Optional substring filter on tool name or description." },
                    "max_results": { "type": "integer", "minimum": 1 }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Core,
        },
        ToolSpec {
            name: "NotebookEdit",
            description: "Replace, insert, or delete a cell in a Jupyter notebook.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "notebook_path": { "type": "string" },
                    "cell_id": { "type": "string" },
                    "new_source": { "type": "string" },
                    "cell_type": { "type": "string", "enum": ["code", "markdown"] },
                    "edit_mode": { "type": "string", "enum": ["replace", "insert", "delete"] }
                },
                "required": ["notebook_path"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "Sleep",
            description: "Wait for a specified duration without holding a shell process.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "duration_ms": { "type": "integer", "minimum": 0 }
                },
                "required": ["duration_ms"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "SendUserMessage",
            description: "Send a message to the user.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" },
                    "attachments": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "status": {
                        "type": "string",
                        "enum": ["normal", "proactive"]
                    }
                },
                "required": ["message", "status"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "Config",
            description: "Get or set Aineer settings.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "setting": { "type": "string" },
                    "value": {
                        "type": ["string", "boolean", "number"]
                    }
                },
                "required": ["setting"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "StructuredOutput",
            description: "Return structured output in the requested format.",
            input_schema: json!({
                "type": "object",
                "additionalProperties": true
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "REPL",
            description: "Execute code in a REPL-like subprocess.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "code": { "type": "string" },
                    "language": { "type": "string" },
                    "timeout_ms": { "type": "integer", "minimum": 1 }
                },
                "required": ["code", "language"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "PowerShell",
            description: "Execute a PowerShell command with optional timeout.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string" },
                    "timeout": { "type": "integer", "minimum": 1 },
                    "description": { "type": "string" },
                    "run_in_background": { "type": "boolean" }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "MultiEdit",
            description: "Apply multiple find-and-replace edits to a single file in one operation.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "edits": {
                        "type": "array",
                        "minItems": 1,
                        "items": {
                            "type": "object",
                            "properties": {
                                "old_string": { "type": "string" },
                                "new_string": { "type": "string" },
                                "replace_all": { "type": "boolean" }
                            },
                            "required": ["old_string", "new_string"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["path", "edits"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "AskUserQuestion",
            description:
                "Present structured questions with options to the user and collect their answers.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "questions": {
                        "type": "array",
                        "minItems": 1,
                        "maxItems": 4,
                        "items": {
                            "type": "object",
                            "properties": {
                                "question": { "type": "string" },
                                "header": { "type": "string", "maxLength": 32 },
                                "multiSelect": { "type": "boolean" },
                                "options": {
                                    "type": "array",
                                    "minItems": 2,
                                    "maxItems": 26,
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "label": { "type": "string" },
                                            "description": { "type": "string" }
                                        },
                                        "required": ["label"],
                                        "additionalProperties": false
                                    }
                                }
                            },
                            "required": ["question", "options"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["questions"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "Lsp",
            description: "Query language intelligence via an LSP server: hover info, code \
                completions, go-to-definition, find references, document/workspace symbol \
                outlines, rename edits, formatting edits, and workspace diagnostics. \
                The LSP server must be configured via the AINEER_LSP_SERVERS environment \
                variable (JSON array of LspServerConfig objects).",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "description": "Which LSP operation to perform.",
                        "enum": [
                            "hover",
                            "completion",
                            "go_to_definition",
                            "find_references",
                            "document_symbols",
                            "workspace_symbols",
                            "rename",
                            "formatting",
                            "diagnostics"
                        ]
                    },
                    "path": {
                        "type": "string",
                        "description": "Absolute path of the source file to query."
                    },
                    "line": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "0-based line number (required for position-sensitive operations)."
                    },
                    "character": {
                        "type": "integer",
                        "minimum": 0,
                        "description": "0-based character offset (required for position-sensitive operations)."
                    },
                    "query": {
                        "type": "string",
                        "description": "Symbol search string (required for workspace_symbols)."
                    },
                    "new_name": {
                        "type": "string",
                        "description": "New symbol name (required for rename)."
                    },
                    "tab_size": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Tab width in spaces used for formatting (default 4)."
                    },
                    "insert_spaces": {
                        "type": "boolean",
                        "description": "Use spaces instead of tabs for formatting (default true)."
                    }
                },
                "required": ["operation", "path"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "TaskCreate",
            description: "Create a new task, optionally running a shell command in the background. \
                Returns a task_id for tracking. If `command` is provided, the command starts \
                immediately as a background process.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Short title for the task."
                    },
                    "description": {
                        "type": "string",
                        "description": "Optional longer description."
                    },
                    "command": {
                        "type": "string",
                        "description": "Optional shell command to run as a background process."
                    }
                },
                "required": ["title"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "TaskGet",
            description: "Retrieve the status and recent output of a task by its ID.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": {
                        "type": "string",
                        "description": "ID of the task to query."
                    },
                    "tail_lines": {
                        "type": "integer",
                        "minimum": 1,
                        "description": "Number of trailing output lines to return (default 50)."
                    }
                },
                "required": ["task_id"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "TaskList",
            description: "List all tracked tasks, optionally filtered by status.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "enum": ["pending", "running", "completed", "failed", "stopped"],
                        "description": "Filter tasks by status. Omit to return all tasks."
                    }
                },
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "TaskUpdate",
            description: "Update a task's title, description, or status.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" },
                    "title": { "type": "string" },
                    "description": { "type": "string" },
                    "status": {
                        "type": "string",
                        "enum": ["pending", "running", "completed", "failed", "stopped"]
                    }
                },
                "required": ["task_id"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "TaskStop",
            description: "Stop a running task (sends SIGTERM to the background process).",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" }
                },
                "required": ["task_id"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "EnterPlanMode",
            description: "Activate plan mode, signalling that the assistant is reasoning and \
                drafting a plan rather than executing changes. While plan mode is active, \
                write/execute tools should not be invoked. Call ExitPlanMode when the plan \
                is ready to execute.",
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "ExitPlanMode",
            description: "Deactivate plan mode and restore normal execution mode. Call this \
                after presenting the plan to the user and receiving approval.",
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "TeamCreate",
            description: "[Experimental] Create a named group of agent endpoints. \
                The team can be used as the recipient in SendMessage to broadcast \
                to all members.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Unique team name." },
                    "description": { "type": "string" },
                    "members": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Agent endpoint URLs."
                    }
                },
                "required": ["name"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "TeamDelete",
            description: "[Experimental] Delete a named agent team.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                },
                "required": ["name"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "SendMessage",
            description: "[Experimental] Send a message to an agent endpoint URL or a named \
                team. Messages are delivered via HTTP POST with a JSON body \
                `{role, content}`.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "recipient": {
                        "type": "string",
                        "description": "Agent endpoint URL or team name."
                    },
                    "content": {
                        "type": "string",
                        "description": "Message content."
                    }
                },
                "required": ["recipient", "content"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "SlashCommand",
            description: "[Experimental] Invoke a registered slash-command on the local agent \
                runtime. Commands must be registered programmatically via \
                `register_slash_command` before they can be invoked.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Command name, with or without a leading '/'."
                    },
                    "args": {
                        "description": "Optional command arguments (free-form JSON)."
                    }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "ListMcpResources",
            description: "List all MCP (Model Context Protocol) resources registered with this \
                agent. Resources are identified by URI and can be read with ReadMcpResource.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "server_filter": {
                        "type": "string",
                        "description": "Optional substring to filter resource URIs by server name."
                    }
                },
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "ReadMcpResource",
            description: "Read the content of a specific MCP resource by its URI.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "uri": {
                        "type": "string",
                        "description": "Full URI of the resource (e.g. mcp://server/path)."
                    }
                },
                "required": ["uri"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "MCPSearch",
            description: "Full-text search across all registered MCP resources (names, \
                descriptions, and content). Returns matching resources with context snippets.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Text to search for."
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "CronCreate",
            description: "Schedule a recurring shell command via cron (crontab). \
                Returns a cron_id that can be used with CronDelete.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "schedule": {
                        "type": "string",
                        "description": "Standard 5-field cron expression (min hour dom month dow). \
                            E.g. '0 9 * * 1-5' runs at 09:00 Mon-Fri."
                    },
                    "command": {
                        "type": "string",
                        "description": "Shell command to execute."
                    },
                    "label": {
                        "type": "string",
                        "description": "Optional human-readable label for the cron job."
                    }
                },
                "required": ["schedule", "command"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "CronDelete",
            description: "Remove a aineer-managed cron job by its ID.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "cron_id": {
                        "type": "string",
                        "description": "ID returned by CronCreate."
                    }
                },
                "required": ["cron_id"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "CronList",
            description: "List all aineer-managed cron jobs, optionally filtered by label.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "label_filter": {
                        "type": "string",
                        "description": "Optional substring to match against the job label."
                    }
                },
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "EnterWorktree",
            description: "Create and enter a git worktree for isolated development on a branch. \
                After this call all file operations target the new worktree directory. \
                Call ExitWorktree to return to the original workspace.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "branch": {
                        "type": "string",
                        "description": "Branch name. Created from HEAD if it does not already exist."
                    },
                    "path": {
                        "type": "string",
                        "description": "Filesystem path for the worktree. Defaults to \
                            `.worktrees/<branch>` under the repository root."
                    }
                },
                "required": ["branch"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            tier: ToolTier::Extended,
        },
        ToolSpec {
            name: "ExitWorktree",
            description: "Exit the current git worktree and restore the original workspace \
                directory. Optionally prune and remove the worktree directory.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "cleanup": {
                        "type": "boolean",
                        "description": "If true, prune the worktree from git and delete its \
                            directory on disk. Default: false."
                    }
                },
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            tier: ToolTier::Extended,
        },
    ]
}
