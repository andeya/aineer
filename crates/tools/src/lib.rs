//! Built-in tool implementations for the Aineer agent runtime.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use api::ToolDefinition;
use serde_json::Value;

use crate::builtin::BuiltinTool;

use engine::{
    edit_file, execute_bash, glob_search, grep_search, read_file, write_file, BashCommandInput,
    GrepSearchInput,
};

mod agent;
pub mod builtin;
mod collab;
mod config_tool;
mod cron;
mod lsp_tool;
mod mcp_resource;
mod notebook;
mod plan_mode;
mod powershell;
mod registry;
mod specs;
mod task;
pub mod tool_output;
mod types;
mod web;
mod worktree;

pub use collab::{register_slash_command, SlashCommandHandler};
pub use lsp_tool::initialize_lsp_manager;
pub use mcp_resource::{register_mcp_resource, McpResource};
pub use plan_mode::is_plan_mode;
pub use registry::{
    GlobalToolRegistry, ToolManifestEntry, ToolRegistry, ToolSource, ToolSpec, ToolTier,
};
pub use specs::mvp_tool_specs;
pub use tool_output::{ToolError, ToolOutput};
pub use types::{AgentResult, ToolSearchHit, ToolSearchInput};

#[cfg(test)]
pub(crate) use agent::{
    agent_permission_policy, allowed_tools_for_subagent, execute_agent_with_spawn,
    final_assistant_text, persist_agent_terminal_state, push_output_block, SubagentToolExecutor,
};
pub(crate) use types::AgentInput;
#[cfg(test)]
pub(crate) use types::AgentJob;

use crate::types::{
    AskUserQuestionInput, AskUserQuestionOutput, BriefInput, BriefOutput, BriefStatus, ConfigInput,
    EditFileInput, GlobSearchInputValue, MultiEditInput, MultiEditOutput, QuestionOption,
    ReadFileInput, ReplInput, ReplOutput, ResolvedAttachment, SkillInput, SkillOutput, SleepInput,
    SleepOutput, StructuredOutputInput, StructuredOutputResult, TodoItem, TodoStatus,
    TodoWriteInput, TodoWriteOutput, ToolSearchOutput, UserQuestion, WriteFileInput,
};

#[must_use]
#[allow(clippy::double_must_use)]
pub fn execute_tool(name: &str, input: Value) -> Result<ToolOutput, ToolError> {
    if let Some(tool) = builtin::find_builtin(name) {
        return tool.dispatch(input);
    }
    Err(ToolError::Unsupported {
        name: name.to_string(),
    })
}

pub(crate) fn to_pretty_json<T: serde::Serialize>(value: T) -> Result<ToolOutput, ToolError> {
    serde_json::to_string_pretty(&value)
        .map(ToolOutput::ok)
        .map_err(Into::into)
}

fn execute_todo_write(input: TodoWriteInput) -> Result<TodoWriteOutput, String> {
    validate_todos(&input.todos)?;
    let store_path = todo_store_path()?;
    let old_todos = if store_path.exists() {
        serde_json::from_str::<Vec<TodoItem>>(
            &std::fs::read_to_string(&store_path).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?
    } else {
        Vec::new()
    };

    let all_done = input
        .todos
        .iter()
        .all(|todo| matches!(todo.status, TodoStatus::Completed));
    let persisted = if all_done {
        Vec::new()
    } else {
        input.todos.clone()
    };

    if let Some(parent) = store_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(
        &store_path,
        serde_json::to_string_pretty(&persisted).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;

    let verification_nudge_needed = (all_done
        && input.todos.len() >= 3
        && !input
            .todos
            .iter()
            .any(|todo| todo.content.to_lowercase().contains("verif")))
    .then_some(true);

    Ok(TodoWriteOutput {
        old_todos,
        new_todos: input.todos,
        verification_nudge_needed,
    })
}

fn execute_skill(input: SkillInput) -> Result<SkillOutput, String> {
    let skill_path = resolve_skill_path(&input.skill)?;
    let prompt = std::fs::read_to_string(&skill_path).map_err(|error| error.to_string())?;
    let description = parse_skill_description(&prompt);

    Ok(SkillOutput {
        skill: input.skill,
        path: skill_path.display().to_string(),
        args: input.args,
        description,
        prompt,
    })
}

fn validate_todos(todos: &[TodoItem]) -> Result<(), String> {
    if todos.is_empty() {
        return Err(String::from("todos must not be empty"));
    }
    // Allow multiple in_progress items for parallel workflows
    if todos.iter().any(|todo| todo.content.trim().is_empty()) {
        return Err(String::from("todo content must not be empty"));
    }
    if todos.iter().any(|todo| todo.active_form.trim().is_empty()) {
        return Err(String::from("todo activeForm must not be empty"));
    }
    Ok(())
}

fn todo_store_path() -> Result<std::path::PathBuf, String> {
    if let Ok(path) = std::env::var("AINEER_TODO_STORE") {
        return Ok(std::path::PathBuf::from(path));
    }
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    Ok(engine::aineer_runtime_dir(&cwd).join("todos.json"))
}

fn resolve_skill_path(skill: &str) -> Result<std::path::PathBuf, String> {
    let requested = skill.trim().trim_start_matches('/').trim_start_matches('$');
    if requested.is_empty() {
        return Err(String::from("skill must not be empty"));
    }

    if requested.contains("..") || requested.contains('/') || requested.contains('\\') {
        return Err(format!(
            "invalid skill name '{requested}': must not contain path separators or '..'"
        ));
    }

    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(engine::aineer_runtime_dir(&cwd).join("skills"));
    }
    if let Ok(aineer_home) = std::env::var("AINEER_CONFIG_HOME") {
        candidates.push(std::path::PathBuf::from(aineer_home).join("skills"));
    }
    if let Some(home) = engine::home_dir() {
        candidates.push(home.join(".aineer").join("skills"));
    }

    for root in candidates {
        let direct = root.join(requested).join("SKILL.md");
        if direct.exists() {
            return Ok(direct);
        }

        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path().join("SKILL.md");
                if !path.exists() {
                    continue;
                }
                if entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(requested)
                {
                    return Ok(path);
                }
            }
        }
    }

    Err(format!("unknown skill: {requested}"))
}

fn searchable_tool_hits(
    extra_definitions: &[ToolDefinition],
    allowed_tools: Option<&BTreeSet<String>>,
) -> Vec<ToolSearchHit> {
    let mut out: Vec<ToolSearchHit> = mvp_tool_specs()
        .into_iter()
        .filter(|spec| spec.tier == ToolTier::Extended)
        .filter(|spec| allowed_tools.is_none_or(|allowed| allowed.contains(spec.name)))
        .map(|spec| ToolSearchHit {
            name: spec.name.to_string(),
            description: spec.description.to_string(),
        })
        .collect();
    for def in extra_definitions {
        if !allowed_tools.is_none_or(|allowed| allowed.contains(&def.name)) {
            continue;
        }
        out.push(ToolSearchHit {
            name: def.name.clone(),
            description: def.description.clone().unwrap_or_default(),
        });
    }
    out
}

fn apply_category_filter(hits: &[ToolSearchHit], category: Option<&str>) -> Vec<ToolSearchHit> {
    let Some(cat) = category.filter(|c| !c.trim().is_empty()) else {
        return hits.to_vec();
    };
    let c = cat.to_ascii_lowercase();
    hits.iter()
        .filter(|h| {
            h.name.to_ascii_lowercase().contains(&c)
                || h.description.to_ascii_lowercase().contains(&c)
        })
        .cloned()
        .collect()
}

fn search_tool_hits(query: &str, max_results: usize, hits: &[ToolSearchHit]) -> Vec<ToolSearchHit> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    let lowered = query.to_lowercase();
    if let Some(selection) = lowered.strip_prefix("select:") {
        return selection
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .filter_map(|wanted| {
                let wanted = canonical_tool_token(wanted);
                hits.iter()
                    .find(|h| canonical_tool_token(&h.name) == wanted)
                    .cloned()
            })
            .take(max_results)
            .collect();
    }

    let mut required = Vec::new();
    let mut optional = Vec::new();
    for term in lowered.split_whitespace() {
        if let Some(rest) = term.strip_prefix('+') {
            if !rest.is_empty() {
                required.push(rest);
            }
        } else {
            optional.push(term);
        }
    }
    let terms: Vec<&str> = if required.is_empty() {
        optional.clone()
    } else {
        required.iter().chain(optional.iter()).copied().collect()
    };

    let mut scored = hits
        .iter()
        .filter_map(|hit| {
            let name = hit.name.to_lowercase();
            let canonical_name = canonical_tool_token(&hit.name);
            let normalized_description = normalize_tool_search_query(&hit.description);
            let haystack = format!("{name} {} {canonical_name}", hit.description.to_lowercase());
            let normalized_haystack = format!("{canonical_name} {normalized_description}");
            if required.iter().any(|term| !haystack.contains(term)) {
                return None;
            }

            let mut score = 0_i32;
            for term in &terms {
                let canonical_term = canonical_tool_token(term);
                if haystack.contains(term) {
                    score += 2;
                }
                if name == *term {
                    score += 8;
                }
                if name.contains(term) {
                    score += 4;
                }
                if canonical_name == canonical_term {
                    score += 12;
                }
                if normalized_haystack.contains(&canonical_term) {
                    score += 3;
                }
            }

            if score == 0 && !lowered.is_empty() {
                return None;
            }
            Some((score, hit.clone()))
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.name.cmp(&right.1.name))
    });
    scored
        .into_iter()
        .map(|(_, hit)| hit)
        .take(max_results)
        .collect()
}

/// Search built-ins (extended tier), plugins, and MCP tools. Used by [`crate::builtin::ToolSearchTool`]
/// with empty `extra_definitions` when no registry context exists.
pub fn execute_tool_search_with_context(
    input: ToolSearchInput,
    pending_mcp_servers: Option<Vec<String>>,
    extra_definitions: &[ToolDefinition],
    allowed_tools: Option<&BTreeSet<String>>,
) -> ToolSearchOutput {
    let corpus = searchable_tool_hits(extra_definitions, allowed_tools);
    let max_results = input.max_results.unwrap_or(5).max(1);
    let query = input.query.trim().to_string();
    let normalized_query = normalize_tool_search_query(&query);
    let category = input
        .category
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let filtered = apply_category_filter(&corpus, category);
    let hits = search_tool_hits(&query, max_results, &filtered);
    let matches: Vec<String> = hits.iter().map(|h| h.name.clone()).collect();

    ToolSearchOutput {
        matches,
        hits,
        query,
        normalized_query,
        total_deferred_tools: corpus.len(),
        pending_mcp_servers: pending_mcp_servers.filter(|servers| !servers.is_empty()),
    }
}

fn normalize_tool_search_query(query: &str) -> String {
    query
        .trim()
        .split(|ch: char| ch.is_whitespace() || ch == ',')
        .filter(|term| !term.is_empty())
        .map(canonical_tool_token)
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn canonical_tool_token(value: &str) -> String {
    let mut canonical = value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect::<String>();
    if let Some(stripped) = canonical.strip_suffix("tool") {
        canonical = stripped.to_string();
    }
    canonical
}

#[cfg(test)]
pub(crate) const MAX_SLEEP_MS: u64 = 5 * 60 * 1000;
#[cfg(not(test))]
const MAX_SLEEP_MS: u64 = 5 * 60 * 1000;

#[cfg(test)]
pub(crate) fn clamp_sleep(requested_ms: u64) -> (u64, String) {
    clamp_sleep_inner(requested_ms)
}

fn clamp_sleep_inner(requested_ms: u64) -> (u64, String) {
    let clamped = requested_ms.min(MAX_SLEEP_MS);
    let message = if clamped < requested_ms {
        format!("Slept for {clamped}ms (clamped from {requested_ms}ms)")
    } else {
        format!("Slept for {clamped}ms")
    };
    (clamped, message)
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn execute_sleep(input: SleepInput) -> SleepOutput {
    let (duration_ms, message) = clamp_sleep_inner(input.duration_ms);
    std::thread::sleep(Duration::from_millis(duration_ms));
    SleepOutput {
        duration_ms,
        message,
    }
}

fn execute_brief(input: BriefInput) -> Result<BriefOutput, String> {
    if input.message.trim().is_empty() {
        return Err(String::from("message must not be empty"));
    }

    let attachments = input
        .attachments
        .as_ref()
        .map(|paths| {
            paths
                .iter()
                .map(|path| resolve_attachment(path))
                .collect::<Result<Vec<_>, String>>()
        })
        .transpose()?;

    let message = match input.status {
        BriefStatus::Normal | BriefStatus::Proactive => input.message,
    };

    Ok(BriefOutput {
        message,
        attachments,
        sent_at: crate::config_tool::iso8601_timestamp(),
    })
}

fn resolve_attachment(path: &str) -> Result<ResolvedAttachment, String> {
    let resolved = std::fs::canonicalize(path).map_err(|error| error.to_string())?;
    let metadata = std::fs::metadata(&resolved).map_err(|error| error.to_string())?;
    Ok(ResolvedAttachment {
        path: resolved.display().to_string(),
        size: metadata.len(),
        is_image: is_image_path(&resolved),
    })
}

fn is_image_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg")
    )
}

pub(crate) fn execute_structured_output(input: StructuredOutputInput) -> StructuredOutputResult {
    StructuredOutputResult {
        data: String::from("Structured output provided successfully"),
        structured_output: input.0,
    }
}

fn execute_repl(input: ReplInput) -> Result<ReplOutput, String> {
    if input.code.trim().is_empty() {
        return Err(String::from("code must not be empty"));
    }
    let timeout_ms = input.timeout_ms.unwrap_or(30_000).max(1_000);
    let runtime = resolve_repl_runtime(&input.language)?;
    let started = Instant::now();
    let child = Command::new(runtime.program)
        .args(runtime.args)
        .arg(&input.code)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;

    let pid = child.id();
    let timeout = Duration::from_millis(timeout_ms);
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => Ok(ReplOutput {
            language: input.language,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(1),
            duration_ms: started.elapsed().as_millis(),
        }),
        Ok(Err(error)) => Err(error.to_string()),
        Err(_) => {
            kill_process(pid);
            Ok(ReplOutput {
                language: input.language,
                stdout: String::new(),
                stderr: format!("REPL execution timed out after {timeout_ms}ms"),
                exit_code: 124,
                duration_ms: started.elapsed().as_millis(),
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplLanguage {
    Python,
    JavaScript,
    Shell,
}

impl ReplLanguage {
    fn parse(input: &str) -> Result<Self, String> {
        match input.trim().to_ascii_lowercase().as_str() {
            "python" | "py" => Ok(Self::Python),
            "javascript" | "js" | "node" => Ok(Self::JavaScript),
            "sh" | "shell" | "bash" => Ok(Self::Shell),
            other => Err(format!("unsupported REPL language: {other}")),
        }
    }

    fn command_candidates(self) -> &'static [&'static str] {
        match self {
            Self::Python => &["python3", "python"],
            Self::JavaScript => &["node"],
            Self::Shell => &["bash", "sh"],
        }
    }

    fn eval_args(self) -> &'static [&'static str] {
        match self {
            Self::Python => &["-c"],
            Self::JavaScript => &["-e"],
            Self::Shell => &["-lc"],
        }
    }
}

struct ReplRuntime {
    program: &'static str,
    args: &'static [&'static str],
}

fn resolve_repl_runtime(language: &str) -> Result<ReplRuntime, String> {
    let lang = ReplLanguage::parse(language)?;
    let program = detect_first_command(lang.command_candidates())
        .ok_or_else(|| format!("{language} runtime not found"))?;
    Ok(ReplRuntime {
        program,
        args: lang.eval_args(),
    })
}

fn detect_first_command(commands: &[&'static str]) -> Option<&'static str> {
    commands
        .iter()
        .copied()
        .find(|command| crate::powershell::command_exists(command))
}

fn parse_skill_description(contents: &str) -> Option<String> {
    for line in contents.lines() {
        if let Some(value) = line.strip_prefix("description:") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn execute_multi_edit(input: MultiEditInput) -> Result<MultiEditOutput, String> {
    if input.edits.is_empty() {
        return Err(String::from("edits must not be empty"));
    }
    for (i, op) in input.edits.iter().enumerate() {
        edit_file(
            &input.path,
            &op.old_string,
            &op.new_string,
            op.replace_all.unwrap_or(false),
            None, // MultiEdit does not track per-op mtime
        )
        .map_err(|error| format!("edit[{i}] failed: {error}"))?;
    }
    Ok(MultiEditOutput {
        path: input.path,
        edits_applied: input.edits.len(),
    })
}

fn execute_ask_user_question(input: AskUserQuestionInput) -> Result<AskUserQuestionOutput, String> {
    if input.questions.is_empty() {
        return Err(String::from("questions must not be empty"));
    }
    if input.questions.len() > 4 {
        return Err(String::from("at most 4 questions are allowed per call"));
    }
    for (qi, q) in input.questions.iter().enumerate() {
        if q.question.trim().is_empty() {
            return Err(format!("questions[{qi}].question must not be empty"));
        }
        if q.options.len() < 2 {
            return Err(format!(
                "questions[{qi}] must have at least 2 options, got {}",
                q.options.len()
            ));
        }
        if q.options.len() > 26 {
            return Err(format!(
                "questions[{qi}] must have at most 26 options, got {}",
                q.options.len()
            ));
        }
    }

    let formatted_message = format_questions(&input.questions);
    Ok(AskUserQuestionOutput {
        questions: input.questions,
        formatted_message,
        pending_user_response: true,
    })
}

fn format_questions(questions: &[UserQuestion]) -> String {
    let mut out = String::from("Please answer the following question(s):\n\n");
    for (i, q) in questions.iter().enumerate() {
        if let Some(header) = &q.header {
            out.push_str(&format!("**{}**\n", header));
        }
        let select_hint = if q.multi_select {
            " (select one or more)"
        } else {
            " (select one)"
        };
        out.push_str(&format!("{}. {}{}\n", i + 1, q.question, select_hint));
        for (oi, opt) in q.options.iter().enumerate() {
            out.push_str(&format_option(oi, opt));
        }
        out.push('\n');
    }
    out.trim_end().to_string()
}

fn format_option(index: usize, opt: &QuestionOption) -> String {
    // index is validated to be 0..=25 by execute_ask_user_question
    let letter = char::from(b'a' + index as u8);
    match &opt.description {
        Some(desc) if !desc.trim().is_empty() => {
            format!("  {letter}) {} — {}\n", opt.label, desc)
        }
        _ => format!("  {letter}) {}\n", opt.label),
    }
}

pub(crate) fn kill_process(pid: u32) {
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

// ---------------------------------------------------------------------------
// BuiltinTool adapters (registered in `builtin::BUILTIN_TOOLS`)
// ---------------------------------------------------------------------------

pub(crate) struct BashTool;

impl BuiltinTool for BashTool {
    const NAME: &'static str = "bash";
    type Input = BashCommandInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        execute_bash(input)
            .map(|o| ToolOutput::ok(serde_json::to_string_pretty(&o).unwrap_or_default()))
            .map_err(ToolError::Io)
    }
}

pub(crate) struct ReadFileTool;

impl BuiltinTool for ReadFileTool {
    const NAME: &'static str = "read_file";
    type Input = ReadFileInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        to_pretty_json(read_file(&input.path, input.offset, input.limit).map_err(ToolError::Io)?)
    }

    fn is_concurrency_safe(_input: &Self::Input) -> bool {
        true
    }
}

pub(crate) struct WriteFileTool;

impl BuiltinTool for WriteFileTool {
    const NAME: &'static str = "write_file";
    type Input = WriteFileInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        to_pretty_json(write_file(&input.path, &input.content).map_err(ToolError::Io)?)
    }
}

pub(crate) struct EditFileTool;

impl BuiltinTool for EditFileTool {
    const NAME: &'static str = "edit_file";
    type Input = EditFileInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        to_pretty_json(
            edit_file(
                &input.path,
                &input.old_string,
                &input.new_string,
                input.replace_all.unwrap_or(false),
                input.last_modified_at,
            )
            .map_err(ToolError::Io)?,
        )
    }
}

pub(crate) struct GlobSearchTool;

impl BuiltinTool for GlobSearchTool {
    const NAME: &'static str = "glob_search";
    type Input = GlobSearchInputValue;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        to_pretty_json(glob_search(&input.pattern, input.path.as_deref()).map_err(ToolError::Io)?)
    }

    fn is_concurrency_safe(_input: &Self::Input) -> bool {
        true
    }
}

pub(crate) struct GrepSearchTool;

impl BuiltinTool for GrepSearchTool {
    const NAME: &'static str = "grep_search";
    type Input = GrepSearchInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        to_pretty_json(grep_search(&input).map_err(ToolError::Io)?)
    }

    fn is_concurrency_safe(_input: &Self::Input) -> bool {
        true
    }
}

pub(crate) struct TodoWriteTool;

impl BuiltinTool for TodoWriteTool {
    const NAME: &'static str = "TodoWrite";
    type Input = TodoWriteInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        to_pretty_json(execute_todo_write(input).map_err(ToolError::execution)?)
    }
}

pub(crate) struct SkillTool;

impl BuiltinTool for SkillTool {
    const NAME: &'static str = "Skill";
    type Input = SkillInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        to_pretty_json(execute_skill(input).map_err(ToolError::execution)?)
    }

    fn is_concurrency_safe(_input: &Self::Input) -> bool {
        true
    }
}

pub(crate) struct AgentTool;

impl BuiltinTool for AgentTool {
    const NAME: &'static str = "Agent";
    type Input = AgentInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        to_pretty_json(crate::agent::execute_agent(input).map_err(ToolError::execution)?)
    }
}

pub(crate) struct BriefTool;

impl BuiltinTool for BriefTool {
    const NAME: &'static str = "Brief";
    type Input = BriefInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        to_pretty_json(execute_brief(input).map_err(ToolError::execution)?)
    }
}

pub(crate) struct SendUserMessageTool;

impl BuiltinTool for SendUserMessageTool {
    const NAME: &'static str = "SendUserMessage";
    type Input = BriefInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        to_pretty_json(execute_brief(input).map_err(ToolError::execution)?)
    }
}

pub(crate) struct ConfigTool;

impl BuiltinTool for ConfigTool {
    const NAME: &'static str = "Config";
    type Input = ConfigInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        to_pretty_json(crate::config_tool::execute_config(input).map_err(ToolError::execution)?)
    }
}

pub(crate) struct ReplTool;

impl BuiltinTool for ReplTool {
    const NAME: &'static str = "REPL";
    type Input = ReplInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        to_pretty_json(execute_repl(input).map_err(ToolError::execution)?)
    }
}

pub(crate) struct MultiEditTool;

impl BuiltinTool for MultiEditTool {
    const NAME: &'static str = "MultiEdit";
    type Input = MultiEditInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        to_pretty_json(execute_multi_edit(input).map_err(ToolError::execution)?)
    }
}

pub(crate) struct AskUserQuestionTool;

impl BuiltinTool for AskUserQuestionTool {
    const NAME: &'static str = "AskUserQuestion";
    type Input = AskUserQuestionInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        to_pretty_json(execute_ask_user_question(input).map_err(ToolError::execution)?)
    }
}

#[cfg(test)]
mod tests;
