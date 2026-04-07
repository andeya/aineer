use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Lifecycle state for background shell tasks (`TaskCreate` / `tasks.json`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Stopped,
}

impl TaskStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Stopped => "stopped",
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ReadFileInput {
    pub(crate) path: String,
    pub(crate) offset: Option<usize>,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WriteFileInput {
    pub(crate) path: String,
    pub(crate) content: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EditFileInput {
    pub(crate) path: String,
    pub(crate) old_string: String,
    pub(crate) new_string: String,
    pub(crate) replace_all: Option<bool>,
    /// Nanoseconds since UNIX epoch returned by a prior `read_file` call.
    /// When set, the edit is rejected if the file has been modified since.
    pub(crate) last_modified_at: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GlobSearchInputValue {
    pub(crate) pattern: String,
    pub(crate) path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WebFetchInput {
    pub(crate) url: String,
    pub(crate) prompt: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WebSearchInput {
    pub(crate) query: String,
    pub(crate) allowed_domains: Option<Vec<String>>,
    pub(crate) blocked_domains: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TodoWriteInput {
    pub(crate) todos: Vec<TodoItem>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub(crate) struct TodoItem {
    pub(crate) content: String,
    #[serde(rename = "activeForm")]
    pub(crate) active_form: String,
    pub(crate) status: TodoStatus,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SkillInput {
    pub(crate) skill: String,
    pub(crate) args: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AgentInput {
    pub(crate) description: String,
    pub(crate) prompt: String,
    pub(crate) subagent_type: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ToolSearchInput {
    pub query: String,
    pub max_results: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NotebookEditInput {
    pub(crate) notebook_path: String,
    pub(crate) cell_id: Option<String>,
    pub(crate) new_source: Option<String>,
    pub(crate) cell_type: Option<NotebookCellType>,
    pub(crate) edit_mode: Option<NotebookEditMode>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum NotebookCellType {
    Code,
    Markdown,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum NotebookEditMode {
    Replace,
    Insert,
    Delete,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SleepInput {
    pub(crate) duration_ms: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BriefInput {
    pub(crate) message: String,
    pub(crate) attachments: Option<Vec<String>>,
    pub(crate) status: BriefStatus,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum BriefStatus {
    Normal,
    Proactive,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConfigInput {
    pub(crate) setting: String,
    pub(crate) value: Option<ConfigValue>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum ConfigValue {
    String(String),
    Bool(bool),
    Number(f64),
}
#[derive(Clone, Copy)]
pub(crate) enum ConfigScope {
    Global,
    Settings,
}

#[derive(Clone, Copy)]
pub(crate) struct ConfigSettingSpec {
    pub(crate) scope: ConfigScope,
    pub(crate) kind: ConfigKind,
    pub(crate) path: &'static [&'static str],
    pub(crate) options: Option<&'static [&'static str]>,
}

#[derive(Clone, Copy)]
pub(crate) enum ConfigKind {
    Boolean,
    String,
}
#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub(crate) struct StructuredOutputInput(pub(crate) BTreeMap<String, Value>);

#[derive(Debug, Deserialize)]
pub(crate) struct ReplInput {
    pub(crate) code: String,
    pub(crate) language: String,
    pub(crate) timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PowerShellInput {
    pub(crate) command: String,
    pub(crate) timeout: Option<u64>,
    pub(crate) description: Option<String>,
    pub(crate) run_in_background: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct WebFetchOutput {
    pub(crate) bytes: usize,
    pub(crate) code: u16,
    #[serde(rename = "codeText")]
    pub(crate) code_text: String,
    pub(crate) result: String,
    #[serde(rename = "durationMs")]
    pub(crate) duration_ms: u128,
    pub(crate) url: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct WebSearchOutput {
    pub(crate) query: String,
    pub(crate) results: Vec<WebSearchResultItem>,
    #[serde(rename = "durationSeconds")]
    pub(crate) duration_seconds: f64,
}

#[derive(Debug, Serialize)]
pub(crate) struct TodoWriteOutput {
    #[serde(rename = "oldTodos")]
    pub(crate) old_todos: Vec<TodoItem>,
    #[serde(rename = "newTodos")]
    pub(crate) new_todos: Vec<TodoItem>,
    #[serde(rename = "verificationNudgeNeeded")]
    pub(crate) verification_nudge_needed: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct SkillOutput {
    pub(crate) skill: String,
    pub(crate) path: String,
    pub(crate) args: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) prompt: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentRunStatus {
    Running,
    Completed,
    Failed,
}

impl fmt::Display for AgentRunStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Running => f.write_str("running"),
            Self::Completed => f.write_str("completed"),
            Self::Failed => f.write_str("failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AgentOutput {
    #[serde(rename = "agentId")]
    pub(crate) agent_id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    #[serde(rename = "subagentType")]
    pub(crate) subagent_type: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) status: AgentRunStatus,
    #[serde(rename = "outputFile")]
    pub(crate) output_file: String,
    #[serde(rename = "manifestFile")]
    pub(crate) manifest_file: String,
    #[serde(rename = "createdAt")]
    pub(crate) created_at: String,
    #[serde(rename = "startedAt", skip_serializing_if = "Option::is_none")]
    pub(crate) started_at: Option<String>,
    #[serde(rename = "completedAt", skip_serializing_if = "Option::is_none")]
    pub(crate) completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentJob {
    pub(crate) manifest: AgentOutput,
    pub(crate) prompt: String,
    pub(crate) system_prompt: Vec<api::SystemBlock>,
    pub(crate) allowed_tools: BTreeSet<String>,
}

#[derive(Debug, Serialize)]
pub struct ToolSearchOutput {
    pub matches: Vec<String>,
    pub query: String,
    pub normalized_query: String,
    #[serde(rename = "total_deferred_tools")]
    pub total_deferred_tools: usize,
    #[serde(rename = "pending_mcp_servers")]
    pub pending_mcp_servers: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub(crate) struct NotebookEditOutput {
    pub(crate) new_source: String,
    pub(crate) cell_id: Option<String>,
    pub(crate) cell_type: Option<NotebookCellType>,
    pub(crate) language: String,
    pub(crate) edit_mode: String,
    pub(crate) error: Option<String>,
    pub(crate) notebook_path: String,
    pub(crate) original_file: String,
    pub(crate) updated_file: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct SleepOutput {
    pub(crate) duration_ms: u64,
    pub(crate) message: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct BriefOutput {
    pub(crate) message: String,
    pub(crate) attachments: Option<Vec<ResolvedAttachment>>,
    #[serde(rename = "sentAt")]
    pub(crate) sent_at: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ResolvedAttachment {
    pub(crate) path: String,
    pub(crate) size: u64,
    #[serde(rename = "isImage")]
    pub(crate) is_image: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct ConfigOutput {
    pub(crate) success: bool,
    pub(crate) operation: Option<String>,
    pub(crate) setting: Option<String>,
    pub(crate) value: Option<Value>,
    #[serde(rename = "previousValue")]
    pub(crate) previous_value: Option<Value>,
    #[serde(rename = "newValue")]
    pub(crate) new_value: Option<Value>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct StructuredOutputResult {
    pub(crate) data: String,
    pub(crate) structured_output: BTreeMap<String, Value>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReplOutput {
    pub(crate) language: String,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    #[serde(rename = "exitCode")]
    pub(crate) exit_code: i32,
    #[serde(rename = "durationMs")]
    pub(crate) duration_ms: u128,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MultiEditInput {
    pub(crate) path: String,
    pub(crate) edits: Vec<EditOperation>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EditOperation {
    pub(crate) old_string: String,
    pub(crate) new_string: String,
    pub(crate) replace_all: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct MultiEditOutput {
    pub(crate) path: String,
    #[serde(rename = "editsApplied")]
    pub(crate) edits_applied: usize,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AskUserQuestionInput {
    pub(crate) questions: Vec<UserQuestion>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct UserQuestion {
    pub(crate) question: String,
    pub(crate) header: Option<String>,
    #[serde(rename = "multiSelect", default)]
    pub(crate) multi_select: bool,
    pub(crate) options: Vec<QuestionOption>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct QuestionOption {
    pub(crate) label: String,
    pub(crate) description: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AskUserQuestionOutput {
    pub(crate) questions: Vec<UserQuestion>,
    #[serde(rename = "formattedMessage")]
    pub(crate) formatted_message: String,
    #[serde(rename = "pendingUserResponse")]
    pub(crate) pending_user_response: bool,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum WebSearchResultItem {
    SearchResult {
        tool_use_id: String,
        content: Vec<SearchHit>,
    },
    Commentary(String),
}

#[derive(Debug, Serialize)]
pub(crate) struct SearchHit {
    pub(crate) title: String,
    pub(crate) url: String,
}

/// Input for `TeamCreate`.
#[derive(Debug, Deserialize)]
pub(crate) struct TeamCreateInput {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    /// Agent endpoint URLs to add as initial members.
    pub(crate) members: Option<Vec<String>>,
}

/// Input for `TeamDelete`.
#[derive(Debug, Deserialize)]
pub(crate) struct TeamDeleteInput {
    pub(crate) name: String,
}

/// Input for `SendMessage`.
#[derive(Debug, Deserialize)]
pub(crate) struct SendMessageInput {
    /// Team name or direct agent endpoint URL.
    pub(crate) recipient: String,
    /// Message content (text or JSON).
    pub(crate) content: String,
}

/// Input for `SlashCommand`.
#[derive(Debug, Deserialize)]
pub(crate) struct SlashCommandInput {
    /// Command name with or without leading `/`.
    pub(crate) command: String,
    /// Optional arguments (free-form JSON).
    pub(crate) args: Option<serde_json::Value>,
}

/// Input for `ListMcpResources`.
#[derive(Debug, Deserialize)]
pub(crate) struct ListMcpResourcesInput {
    /// Optional substring to filter URIs by server name.
    pub(crate) server_filter: Option<String>,
}

/// Input for `ReadMcpResource`.
#[derive(Debug, Deserialize)]
pub(crate) struct ReadMcpResourceInput {
    /// Full URI of the resource to read.
    pub(crate) uri: String,
}

/// Input for `MCPSearch`.
#[derive(Debug, Deserialize)]
pub(crate) struct McpSearchInput {
    /// Full-text query to match against resource names, descriptions, and content.
    pub(crate) query: String,
}

/// Input for `CronCreate`.
#[derive(Debug, Deserialize)]
pub(crate) struct CronCreateInput {
    /// Cron schedule expression (5 fields: min hour dom month dow).
    pub(crate) schedule: String,
    /// Shell command to execute.
    pub(crate) command: String,
    /// Optional human-readable label for the job.
    pub(crate) label: Option<String>,
}

/// Input for `CronDelete`.
#[derive(Debug, Deserialize)]
pub(crate) struct CronDeleteInput {
    /// ID returned by CronCreate.
    pub(crate) cron_id: String,
}

/// Input for `CronList`.
#[derive(Debug, Deserialize)]
pub(crate) struct CronListInput {
    /// Optional substring to filter jobs by label.
    pub(crate) label_filter: Option<String>,
}

/// Input for `EnterWorktree`.
#[derive(Debug, Deserialize)]
pub(crate) struct EnterWorktreeInput {
    /// Branch name to check out in the new worktree. Created from HEAD if it does not exist.
    pub(crate) branch: String,
    /// Optional filesystem path for the worktree directory. Defaults to `.worktrees/<branch>`.
    pub(crate) path: Option<String>,
}

/// Input for `ExitWorktree`.
#[derive(Debug, Deserialize)]
pub(crate) struct ExitWorktreeInput {
    /// If true, prune and remove the worktree directory on exit.
    pub(crate) cleanup: Option<bool>,
}

/// Input for `EnterPlanMode` (no fields; empty object required by schema).
#[derive(Debug, Deserialize)]
pub(crate) struct EnterPlanModeInput {}

/// Input for `ExitPlanMode` (no fields; empty object required by schema).
#[derive(Debug, Deserialize)]
pub(crate) struct ExitPlanModeInput {}

/// Input for `TaskCreate`.
#[derive(Debug, Deserialize)]
pub(crate) struct TaskCreateInput {
    pub(crate) title: String,
    pub(crate) description: Option<String>,
    /// Optional shell command to run as a background process.
    pub(crate) command: Option<String>,
}

/// Input for `TaskGet`.
#[derive(Debug, Deserialize)]
pub(crate) struct TaskGetInput {
    pub(crate) task_id: String,
    /// Number of trailing output lines to return (default 50).
    pub(crate) tail_lines: Option<usize>,
}

/// Input for `TaskList`.
#[derive(Debug, Deserialize)]
pub(crate) struct TaskListInput {
    /// Filter by status: pending, running, completed, failed, stopped.
    pub(crate) status: Option<TaskStatus>,
}

/// Input for `TaskUpdate`.
#[derive(Debug, Deserialize)]
pub(crate) struct TaskUpdateInput {
    pub(crate) task_id: String,
    pub(crate) title: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) status: Option<TaskStatus>,
}

/// Input for `TaskStop`.
#[derive(Debug, Deserialize)]
pub(crate) struct TaskStopInput {
    pub(crate) task_id: String,
}

/// Input for the `Lsp` tool.
#[derive(Debug, Deserialize)]
pub(crate) struct LspInput {
    /// Operation to perform: hover, completion, go_to_definition, find_references,
    /// document_symbols, workspace_symbols, rename, formatting, diagnostics.
    pub(crate) operation: String,
    /// Absolute or workspace-relative path of the file to query.
    pub(crate) path: String,
    /// 0-based line number (required for position-sensitive operations).
    pub(crate) line: Option<u32>,
    /// 0-based character offset (required for position-sensitive operations).
    pub(crate) character: Option<u32>,
    /// Search query string (required for workspace_symbols).
    pub(crate) query: Option<String>,
    /// New symbol name (required for rename).
    pub(crate) new_name: Option<String>,
    /// Tab size in spaces (for formatting; default 4).
    pub(crate) tab_size: Option<u32>,
    /// Whether to use spaces instead of tabs (for formatting; default true).
    pub(crate) insert_spaces: Option<bool>,
}
