//! Background task management tools.
//!
//! Provides `TaskCreate`, `TaskGet`, `TaskList`, `TaskUpdate`, and `TaskStop`
//! for spawning, monitoring, and controlling long-running shell commands.
//! Task metadata is persisted in `.aineer/tasks.json`; command output and
//! exit codes are captured in `.aineer/task-outputs/`.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::builtin::BuiltinTool;
use crate::tool_output::{ToolError, ToolOutput};
use crate::types::{
    TaskCreateInput, TaskGetInput, TaskListInput, TaskStatus, TaskStopInput, TaskUpdateInput,
};

// ── Data model ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Task {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: TaskStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_file: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ── Persistence helpers ───────────────────────────────────────────────────────

fn task_store_path() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    Ok(engine::aineer_runtime_dir(&cwd).join("tasks.json"))
}

fn task_output_dir() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    Ok(engine::aineer_runtime_dir(&cwd).join("task-outputs"))
}

fn read_tasks() -> Result<Vec<Task>, String> {
    let path = task_store_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn write_tasks(tasks: &[Task]) -> Result<(), String> {
    let path = task_store_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(
        &path,
        serde_json::to_string_pretty(tasks).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn new_task_id() -> String {
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("task-{:x}{:08x}", d.as_secs(), d.subsec_nanos())
}

// ── Process helpers ───────────────────────────────────────────────────────────

/// Start command in background, redirecting output to `output_file`.
/// After the command finishes, its exit code is written to `exit_file`.
fn start_background_command(task_id: &str, command: &str) -> Result<(u32, String, String), String> {
    let out_dir = task_output_dir()?;
    std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;

    let output_file = out_dir.join(format!("{task_id}.log"));
    let exit_file = out_dir.join(format!("{task_id}.exit"));
    let output_file_str = output_file.display().to_string();
    let exit_file_str = exit_file.display().to_string();

    // Wrap command so exit code is written after it completes.
    let script = format!(
        "({}) > {} 2>&1; echo $? > {}",
        command, output_file_str, exit_file_str
    );

    let result = std::process::Command::new("bash")
        .arg("-c")
        // Run in background with &, echo the PID
        .arg(format!("{script} & echo $!"))
        .output()
        .map_err(|e| format!("failed to spawn command: {e}"))?;

    let pid_str = String::from_utf8_lossy(&result.stdout).trim().to_string();
    let pid = pid_str
        .parse::<u32>()
        .map_err(|_| format!("failed to parse PID from output: {pid_str:?}"))?;

    Ok((pid, output_file_str, exit_file_str))
}

/// Returns `true` if the process with `pid` is still alive.
fn is_process_running(pid: u32) -> bool {
    std::process::Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .output()
        .is_ok_and(|out| out.status.success())
}

/// Refresh status of a running task by checking process state and exit file.
fn sync_running_task(task: &mut Task) {
    if task.status != TaskStatus::Running {
        return;
    }
    let Some(pid) = task.pid else { return };

    if let Some(exit_file) = task.exit_file.as_deref() {
        if let Ok(code_str) = std::fs::read_to_string(exit_file) {
            let code: i32 = code_str.trim().parse().unwrap_or(-1);
            task.exit_code = Some(code);
            task.status = if code == 0 {
                TaskStatus::Completed
            } else {
                TaskStatus::Failed
            };
            task.updated_at = unix_now();
            return;
        }
    }

    // If exit file not yet written, check if process is still alive.
    if !is_process_running(pid) {
        // Process died without writing exit file (killed externally).
        task.status = TaskStatus::Stopped;
        task.updated_at = unix_now();
    }
}

/// Read the last `max_lines` lines from a file.
fn read_tail(path: &str, max_lines: usize) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .rev()
        .take(max_lines)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Tool implementations ──────────────────────────────────────────────────────

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn execute_task_create(input: TaskCreateInput) -> Result<String, String> {
    let mut tasks = read_tasks()?;
    let task_id = new_task_id();
    let now = unix_now();

    let mut task = Task {
        id: task_id.clone(),
        title: input.title.clone(),
        description: input.description.clone(),
        status: TaskStatus::Pending,
        command: input.command.clone(),
        pid: None,
        output_file: None,
        exit_file: None,
        created_at: now,
        updated_at: now,
        exit_code: None,
        error: None,
    };

    if let Some(ref cmd) = input.command {
        match start_background_command(&task_id, cmd) {
            Ok((pid, output_file, exit_file)) => {
                task.pid = Some(pid);
                task.output_file = Some(output_file);
                task.exit_file = Some(exit_file);
                task.status = TaskStatus::Running;
            }
            Err(e) => {
                task.status = TaskStatus::Failed;
                task.error = Some(e);
            }
        }
    }

    tasks.push(task.clone());
    write_tasks(&tasks)?;

    serde_json::to_string_pretty(&serde_json::json!({
        "task_id": task.id,
        "title": task.title,
        "status": task.status.as_str(),
        "pid": task.pid,
        "message": if task.command.is_some() {
            format!("Task created and started (PID {:?})", task.pid)
        } else {
            "Task created".to_string()
        }
    }))
    .map_err(|e| e.to_string())
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn execute_task_get(input: TaskGetInput) -> Result<String, String> {
    let mut tasks = read_tasks()?;
    let pos = tasks
        .iter()
        .position(|t| t.id == input.task_id)
        .ok_or_else(|| format!("task not found: {}", input.task_id))?;

    sync_running_task(&mut tasks[pos]);
    write_tasks(&tasks)?;

    let task = &tasks[pos];
    let recent_output = task
        .output_file
        .as_deref()
        .map(|f| read_tail(f, input.tail_lines.unwrap_or(50)));

    serde_json::to_string_pretty(&serde_json::json!({
        "id": task.id,
        "title": task.title,
        "description": task.description,
        "status": task.status.as_str(),
        "command": task.command,
        "pid": task.pid,
        "exit_code": task.exit_code,
        "error": task.error,
        "created_at": task.created_at,
        "updated_at": task.updated_at,
        "recent_output": recent_output,
    }))
    .map_err(|e| e.to_string())
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn execute_task_list(input: TaskListInput) -> Result<String, String> {
    let mut tasks = read_tasks()?;

    // Auto-sync running tasks.
    for task in &mut tasks {
        sync_running_task(task);
    }
    write_tasks(&tasks)?;

    // Filter by status if requested.
    let filtered: Vec<_> = tasks
        .iter()
        .filter(|t| input.status.is_none_or(|wanted| t.status == wanted))
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "title": t.title,
                "status": t.status.as_str(),
                "command": t.command,
                "pid": t.pid,
                "exit_code": t.exit_code,
                "created_at": t.created_at,
                "updated_at": t.updated_at,
            })
        })
        .collect();

    serde_json::to_string_pretty(&serde_json::json!({
        "total": filtered.len(),
        "tasks": filtered,
    }))
    .map_err(|e| e.to_string())
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn execute_task_update(input: TaskUpdateInput) -> Result<String, String> {
    let mut tasks = read_tasks()?;
    let task = tasks
        .iter_mut()
        .find(|t| t.id == input.task_id)
        .ok_or_else(|| format!("task not found: {}", input.task_id))?;

    if let Some(ref title) = input.title {
        task.title = title.clone();
    }
    if let Some(desc) = input.description {
        task.description = if desc.is_empty() { None } else { Some(desc) };
    }
    if let Some(status) = input.status {
        task.status = status;
    }
    task.updated_at = unix_now();

    write_tasks(&tasks)?;

    let task = tasks
        .iter()
        .find(|t| t.id == input.task_id)
        .ok_or_else(|| format!("task not found after write: {}", input.task_id))?;
    serde_json::to_string_pretty(&serde_json::json!({
        "id": task.id,
        "title": task.title,
        "status": task.status.as_str(),
        "message": "Task updated",
    }))
    .map_err(|e| e.to_string())
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn execute_task_stop(input: TaskStopInput) -> Result<String, String> {
    let mut tasks = read_tasks()?;
    let task = tasks
        .iter_mut()
        .find(|t| t.id == input.task_id)
        .ok_or_else(|| format!("task not found: {}", input.task_id))?;

    if task.status != TaskStatus::Running {
        return Err(format!(
            "task {} is not running (status: {})",
            task.id,
            task.status.as_str()
        ));
    }

    if let Some(pid) = task.pid {
        let _ = std::process::Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .output();
    }

    task.status = TaskStatus::Stopped;
    task.updated_at = unix_now();
    let id = task.id.clone();

    write_tasks(&tasks)?;

    serde_json::to_string_pretty(&serde_json::json!({
        "task_id": id,
        "status": "stopped",
        "message": "Task stopped",
    }))
    .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// BuiltinTool adapters
// ---------------------------------------------------------------------------

pub(crate) struct TaskCreateTool;

impl BuiltinTool for TaskCreateTool {
    const NAME: &'static str = "TaskCreate";
    type Input = TaskCreateInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        execute_task_create(input)
            .map(ToolOutput::ok)
            .map_err(ToolError::execution)
    }
}

pub(crate) struct TaskGetTool;

impl BuiltinTool for TaskGetTool {
    const NAME: &'static str = "TaskGet";
    type Input = TaskGetInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        execute_task_get(input)
            .map(ToolOutput::ok)
            .map_err(ToolError::execution)
    }

    fn is_concurrency_safe(_input: &Self::Input) -> bool {
        true
    }
}

pub(crate) struct TaskListTool;

impl BuiltinTool for TaskListTool {
    const NAME: &'static str = "TaskList";
    type Input = TaskListInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        execute_task_list(input)
            .map(ToolOutput::ok)
            .map_err(ToolError::execution)
    }

    fn is_concurrency_safe(_input: &Self::Input) -> bool {
        true
    }
}

pub(crate) struct TaskUpdateTool;

impl BuiltinTool for TaskUpdateTool {
    const NAME: &'static str = "TaskUpdate";
    type Input = TaskUpdateInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        execute_task_update(input)
            .map(ToolOutput::ok)
            .map_err(ToolError::execution)
    }
}

pub(crate) struct TaskStopTool;

impl BuiltinTool for TaskStopTool {
    const NAME: &'static str = "TaskStop";
    type Input = TaskStopInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        execute_task_stop(input)
            .map(ToolOutput::ok)
            .map_err(ToolError::execution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TaskCreateInput, TaskGetInput, TaskListInput, TaskStatus, TaskUpdateInput};
    use std::sync::{Mutex, OnceLock};

    fn task_test_lock() -> &'static Mutex<()> {
        static L: OnceLock<Mutex<()>> = OnceLock::new();
        L.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn create_get_list_update_lifecycle() {
        let _g = task_test_lock().lock().unwrap();
        let dir = std::env::temp_dir().join(format!("aineer-task-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let orig = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let created = execute_task_create(TaskCreateInput {
            title: "Unit test task".to_string(),
            description: Some("desc".to_string()),
            command: None,
        })
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&created).unwrap();
        let task_id = v["task_id"].as_str().unwrap().to_string();
        assert_eq!(v["status"], "pending");

        let got = execute_task_get(TaskGetInput {
            task_id: task_id.clone(),
            tail_lines: None,
        })
        .unwrap();
        let gv: serde_json::Value = serde_json::from_str(&got).unwrap();
        assert_eq!(gv["title"], "Unit test task");

        let listed = execute_task_list(TaskListInput { status: None }).unwrap();
        let lv: serde_json::Value = serde_json::from_str(&listed).unwrap();
        assert!(lv["total"].as_u64().unwrap() >= 1);

        let updated = execute_task_update(TaskUpdateInput {
            task_id: task_id.clone(),
            title: Some("Updated title".to_string()),
            description: None,
            status: Some(TaskStatus::Completed),
        })
        .unwrap();
        let uv: serde_json::Value = serde_json::from_str(&updated).unwrap();
        assert_eq!(uv["title"], "Updated title");
        assert_eq!(uv["status"], "completed");

        std::env::set_current_dir(&orig).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn get_missing_task_returns_error() {
        let _g = task_test_lock().lock().unwrap();
        let dir = std::env::temp_dir().join(format!("aineer-task-miss-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let orig = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let err = execute_task_get(TaskGetInput {
            task_id: "nonexistent".to_string(),
            tail_lines: None,
        })
        .unwrap_err();
        assert!(err.contains("not found"));

        std::env::set_current_dir(&orig).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn update_invalid_status_fails_deserialization() {
        let _g = task_test_lock().lock().unwrap();
        let dir = std::env::temp_dir().join(format!("aineer-task-stat-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let orig = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let created = execute_task_create(TaskCreateInput {
            title: "status test".to_string(),
            description: None,
            command: None,
        })
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&created).unwrap();
        let task_id = v["task_id"].as_str().unwrap().to_string();

        let err = crate::execute_tool(
            "TaskUpdate",
            serde_json::json!({
                "task_id": task_id,
                "status": "bogus",
            }),
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("unknown variant")
                || err.contains("invalid value")
                || err.contains("deserialize"),
            "unexpected error: {err}"
        );

        std::env::set_current_dir(&orig).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    #[cfg(unix)]
    fn create_with_command_starts_background_process() {
        let _g = task_test_lock().lock().unwrap();
        let dir = std::env::temp_dir().join(format!("aineer-task-bg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let orig = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let created = execute_task_create(TaskCreateInput {
            title: "bg task".to_string(),
            description: None,
            command: Some("echo hello".to_string()),
        })
        .unwrap();
        let v: serde_json::Value = serde_json::from_str(&created).unwrap();
        assert!(v["pid"].as_u64().is_some());

        // Wait briefly for the command to complete
        std::thread::sleep(std::time::Duration::from_millis(500));

        let task_id = v["task_id"].as_str().unwrap().to_string();
        let got = execute_task_get(TaskGetInput {
            task_id,
            tail_lines: Some(10),
        })
        .unwrap();
        let gv: serde_json::Value = serde_json::from_str(&got).unwrap();
        let status = gv["status"].as_str().unwrap();
        assert!(status == "completed" || status == "running");

        std::env::set_current_dir(&orig).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
