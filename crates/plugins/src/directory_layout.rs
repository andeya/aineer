//! Claude Code-style plugin layouts: `commands/*.md`, `agents/*.md`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::types::{PluginAgent, PluginCommand};

fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

fn collect_markdown_files(subdir: &Path) -> Vec<PathBuf> {
    if !subdir.is_dir() {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(subdir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_markdown_file(path))
        .collect();
    paths.sort();
    paths
}

/// Parses optional YAML-style `description:` from a `---` ... `---` frontmatter block.
fn split_frontmatter(raw: &str) -> (Option<String>, String) {
    let s = raw.trim_start();
    if !s.starts_with("---") {
        return (None, raw.to_string());
    }
    let mut after_open = s.strip_prefix("---").unwrap_or(s);
    after_open = after_open
        .strip_prefix("\r\n")
        .or_else(|| after_open.strip_prefix('\n'))
        .unwrap_or(after_open);
    let Some(close_rel) = after_open.find("\n---") else {
        return (None, raw.to_string());
    };
    let frontmatter = &after_open[..close_rel];
    let rest = &after_open[close_rel + "\n---".len()..];
    let rest = rest
        .strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'))
        .unwrap_or(rest);
    let description = parse_description_from_frontmatter(frontmatter);
    (Some(description), rest.to_string())
}

fn parse_description_from_frontmatter(fm: &str) -> String {
    for line in fm.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("description:") else {
            continue;
        };
        return rest
            .trim()
            .trim_matches(|c| c == '"' || c == '\'')
            .to_string();
    }
    String::new()
}

fn fallback_description_from_body(body: &str) -> String {
    for line in body.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        return t.trim_start_matches('#').trim().to_string();
    }
    String::new()
}

/// Scan `commands/` for `.md` files; stem is the slash command name.
#[must_use]
pub fn scan_command_files(plugin_dir: &Path) -> Vec<PluginCommand> {
    let mut out = Vec::new();
    for path in collect_markdown_files(&plugin_dir.join("commands")) {
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Some(name) = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        let (fm_desc, body) = split_frontmatter(&raw);
        let description = fm_desc
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| fallback_description_from_body(&body));
        out.push(PluginCommand {
            name,
            description,
            content: raw,
            source_path: path,
        });
    }
    out
}

/// Scan `agents/` for `.md` files; stem is the agent id; body is the system prompt.
#[must_use]
pub fn scan_agent_files(plugin_dir: &Path) -> Vec<PluginAgent> {
    let mut out = Vec::new();
    for path in collect_markdown_files(&plugin_dir.join("agents")) {
        let Ok(raw) = fs::read_to_string(&path) else {
            continue;
        };
        let Some(name) = path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        let (fm_desc, body) = split_frontmatter(&raw);
        let description = fm_desc
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| fallback_description_from_body(&body));
        let system_prompt = if raw.trim_start().starts_with("---") {
            body
        } else {
            raw.clone()
        };
        out.push(PluginAgent {
            name,
            description,
            system_prompt,
            source_path: path,
        });
    }
    out
}
