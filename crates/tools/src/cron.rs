//! Cron job management tools.
//!
//! `CronCreate`, `CronDelete`, and `CronList` manage the user's crontab.
//! Managed entries are tagged with `# aineer-managed id=<id> label=<label>`
//! so they can be listed and deleted without affecting non-aineer entries.

use std::process::Command;

use serde::Serialize;

use crate::builtin::BuiltinTool;
use crate::tool_output::{ToolError, ToolOutput};
use crate::types::{CronCreateInput, CronDeleteInput, CronListInput};

// ── Constants ─────────────────────────────────────────────────────────────────

const MARKER: &str = "# aineer-managed";

// ── Output types ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct CronEntry {
    id: String,
    schedule: String,
    command: String,
    label: String,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn read_crontab() -> Result<String, String> {
    let out = Command::new("crontab")
        .arg("-l")
        .output()
        .map_err(|e| format!("cannot run crontab: {e}"))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).into_owned());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    // "no crontab for ..." is not a real error
    if stderr.contains("no crontab") {
        return Ok(String::new());
    }
    Err(format!("crontab -l failed: {stderr}"))
}

fn write_crontab(content: &str) -> Result<(), String> {
    use std::io::Write as _;
    let mut child = Command::new("crontab")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot spawn crontab: {e}"))?;
    child
        .stdin
        .take()
        .ok_or_else(|| "crontab stdin unavailable".to_string())?
        .write_all(content.as_bytes())
        .map_err(|e| format!("write to crontab stdin: {e}"))?;
    let status = child.wait().map_err(|e| format!("crontab wait: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("crontab write failed".to_string())
    }
}

/// Parse a managed line: `<schedule> <command> # aineer-managed id=<id> label=<label>`
fn parse_managed_line(line: &str) -> Option<CronEntry> {
    let marker_pos = line.find(MARKER)?;
    let meta = line[marker_pos + MARKER.len()..].trim();
    let id = meta
        .split_whitespace()
        .find(|t| t.starts_with("id="))?
        .trim_start_matches("id=")
        .to_string();
    let label = meta
        .split_whitespace()
        .find(|t| t.starts_with("label="))
        .map(|t| t.trim_start_matches("label=").to_string())
        .unwrap_or_default();

    let cron_part = line[..marker_pos].trim();
    let mut parts = cron_part.splitn(6, ' ');
    let f1 = parts.next()?;
    let f2 = parts.next()?;
    let f3 = parts.next()?;
    let f4 = parts.next()?;
    let f5 = parts.next()?;
    let cmd = parts.next()?.to_string();
    let schedule = format!("{f1} {f2} {f3} {f4} {f5}");
    Some(CronEntry {
        id,
        schedule,
        command: cmd,
        label,
    })
}

fn build_managed_line(schedule: &str, command: &str, id: &str, label: &str) -> String {
    format!("{schedule} {command} {MARKER} id={id} label={label}")
}

fn new_cron_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("cron-{ts}")
}

// ── Tool implementations ──────────────────────────────────────────────────────

pub(crate) fn execute_cron_create(input: CronCreateInput) -> Result<String, String> {
    // Basic schedule validation: must have exactly 5 space-separated fields.
    let field_count = input.schedule.split_whitespace().count();
    if field_count != 5 {
        return Err(format!(
            "invalid cron schedule '{}': expected 5 fields (min hour dom month dow), got {field_count}",
            input.schedule
        ));
    }
    if input.command.trim().is_empty() {
        return Err("command must not be empty".to_string());
    }

    let id = new_cron_id();
    let label = input.label.as_deref().unwrap_or("").replace(' ', "_");
    let new_line = build_managed_line(&input.schedule, input.command.trim(), &id, &label);

    let existing = read_crontab()?;
    let updated = if existing.ends_with('\n') || existing.is_empty() {
        format!("{existing}{new_line}\n")
    } else {
        format!("{existing}\n{new_line}\n")
    };
    write_crontab(&updated)?;

    Ok(format!(
        "Cron job created. id={id} schedule='{}' command='{}'",
        input.schedule,
        input.command.trim()
    ))
}

pub(crate) fn execute_cron_delete(input: CronDeleteInput) -> Result<String, String> {
    let existing = read_crontab()?;
    let target_id = format!("id={}", input.cron_id.trim());
    let lines: Vec<&str> = existing.lines().collect();
    let before = lines.len();
    let filtered: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| !(line.contains(MARKER) && line.contains(&target_id)))
        .collect();
    let after = filtered.len();
    if before == after {
        return Err(format!(
            "no managed cron job found with id={}",
            input.cron_id
        ));
    }
    let updated = filtered.join("\n") + "\n";
    write_crontab(&updated)?;
    Ok(format!("Deleted cron job with id={}.", input.cron_id))
}

pub(crate) fn execute_cron_list(input: CronListInput) -> Result<String, String> {
    let existing = read_crontab()?;
    let entries: Vec<CronEntry> = existing
        .lines()
        .filter_map(parse_managed_line)
        .filter(|e| {
            input
                .label_filter
                .as_deref()
                .map(|f| e.label.contains(f))
                .unwrap_or(true)
        })
        .collect();
    let json =
        serde_json::to_string_pretty(&entries).map_err(|e| format!("serialization error: {e}"))?;
    Ok(json)
}

// ---------------------------------------------------------------------------
// BuiltinTool adapters
// ---------------------------------------------------------------------------

pub(crate) struct CronCreateTool;

impl BuiltinTool for CronCreateTool {
    const NAME: &'static str = "CronCreate";
    type Input = CronCreateInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        execute_cron_create(input)
            .map(ToolOutput::ok)
            .map_err(ToolError::execution)
    }
}

pub(crate) struct CronDeleteTool;

impl BuiltinTool for CronDeleteTool {
    const NAME: &'static str = "CronDelete";
    type Input = CronDeleteInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        execute_cron_delete(input)
            .map(ToolOutput::ok)
            .map_err(ToolError::execution)
    }
}

pub(crate) struct CronListTool;

impl BuiltinTool for CronListTool {
    const NAME: &'static str = "CronList";
    type Input = CronListInput;

    fn execute(input: Self::Input) -> Result<ToolOutput, ToolError> {
        execute_cron_list(input)
            .map(ToolOutput::ok)
            .map_err(ToolError::execution)
    }

    fn is_concurrency_safe(_input: &Self::Input) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_managed_line_extracts_fields() {
        let line = "*/5 * * * * /usr/bin/check # aineer-managed id=cron-123 label=health_check";
        let entry = parse_managed_line(line).expect("should parse");
        assert_eq!(entry.id, "cron-123");
        assert_eq!(entry.schedule, "*/5 * * * *");
        assert_eq!(entry.command, "/usr/bin/check");
        assert_eq!(entry.label, "health_check");
    }

    #[test]
    fn parse_managed_line_returns_none_for_unmanaged() {
        assert!(parse_managed_line("0 9 * * * /usr/bin/backup").is_none());
        assert!(parse_managed_line("# just a comment").is_none());
    }

    #[test]
    fn build_managed_line_roundtrips() {
        let line = build_managed_line("0 9 * * 1-5", "/usr/bin/deploy", "cron-42", "deploy");
        let entry = parse_managed_line(&line).expect("should parse built line");
        assert_eq!(entry.id, "cron-42");
        assert_eq!(entry.schedule, "0 9 * * 1-5");
        assert_eq!(entry.command, "/usr/bin/deploy");
        assert_eq!(entry.label, "deploy");
    }

    #[test]
    fn cron_create_rejects_invalid_schedule() {
        let err = execute_cron_create(CronCreateInput {
            schedule: "* *".to_string(),
            command: "echo hi".to_string(),
            label: None,
        })
        .unwrap_err();
        assert!(err.contains("expected 5 fields"));
    }

    #[test]
    fn cron_create_rejects_empty_command() {
        let err = execute_cron_create(CronCreateInput {
            schedule: "* * * * *".to_string(),
            command: "   ".to_string(),
            label: None,
        })
        .unwrap_err();
        assert!(err.contains("must not be empty"));
    }

    #[test]
    fn new_cron_id_starts_with_prefix() {
        let id = new_cron_id();
        assert!(id.starts_with("cron-"), "got: {id}");
    }
}
