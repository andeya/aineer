use runtime::ContentBlock;
use serde_json::json;

pub(crate) fn final_assistant_text(summary: &runtime::TurnSummary) -> String {
    summary
        .assistant_messages
        .last()
        .map(|message| {
            message
                .blocks
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

pub(crate) fn collect_tool_uses(summary: &runtime::TurnSummary) -> Vec<serde_json::Value> {
    summary
        .assistant_messages
        .iter()
        .flat_map(|message| message.blocks.iter())
        .filter_map(|block| match block {
            ContentBlock::ToolUse { id, name, input } => Some(json!({
                "id": id,
                "name": name,
                "input": input,
            })),
            _ => None,
        })
        .collect()
}

pub(crate) fn collect_tool_results(summary: &runtime::TurnSummary) -> Vec<serde_json::Value> {
    summary
        .tool_results
        .iter()
        .flat_map(|message| message.blocks.iter())
        .filter_map(|block| match block {
            ContentBlock::ToolResult {
                tool_use_id,
                tool_name,
                output,
                is_error,
            } => Some(json!({
                "tool_use_id": tool_use_id,
                "tool_name": tool_name,
                "output": output,
                "is_error": is_error,
            })),
            _ => None,
        })
        .collect()
}

pub(crate) fn process_at_mentioned_files(input: &str) -> String {
    let paths = crate::input::suggestions::extract_at_mentioned_files(input);
    if paths.is_empty() {
        return input.to_string();
    }

    const MAX_LINES: usize = 2000;
    const LINE_CONTEXT: usize = 50;
    let mut blocks = Vec::new();

    for path in &paths {
        let (file_path, line_ref) = parse_line_ref(path);
        let p = std::path::Path::new(file_path);
        if p.is_dir() {
            if let Ok(entries) = std::fs::read_dir(p) {
                let listing: Vec<String> = entries
                    .flatten()
                    .take(100)
                    .map(|e| {
                        let name = e.file_name().to_string_lossy().to_string();
                        if e.file_type().is_ok_and(|ft| ft.is_dir()) {
                            format!("{name}/")
                        } else {
                            name
                        }
                    })
                    .collect();
                blocks.push(format!(
                    "<attached_directory path=\"{file_path}\">\n{}\n</attached_directory>",
                    listing.join("\n")
                ));
            } else {
                eprintln!("[warn] @{path}: directory not readable, skipping");
                blocks.push(format!("<not_found path=\"{file_path}\"/>"));
            }
        } else if let Ok(content) = std::fs::read_to_string(p) {
            let all_lines: Vec<&str> = content.lines().collect();
            let total = all_lines.len();

            let (selected, start_line, end_line) = match line_ref {
                Some((start, Some(end))) => {
                    let s = start.saturating_sub(1).min(total);
                    let e = end.min(total);
                    (all_lines[s..e].to_vec(), s + 1, e)
                }
                Some((line, None)) => {
                    let s = line
                        .saturating_sub(1)
                        .saturating_sub(LINE_CONTEXT)
                        .min(total);
                    let e = (line + LINE_CONTEXT).min(total);
                    (all_lines[s..e].to_vec(), s + 1, e)
                }
                None => {
                    let e = total.min(MAX_LINES);
                    (all_lines[..e].to_vec(), 1, e)
                }
            };

            let lines_attr = if start_line != 1 || end_line != total {
                format!(" lines=\"{start_line}-{end_line}\"")
            } else {
                String::new()
            };
            blocks.push(format!(
                "<attached_file path=\"{file_path}\"{lines_attr}>\n{}\n</attached_file>",
                selected.join("\n")
            ));
        } else {
            eprintln!("[warn] @{path}: file not found or not readable, skipping");
            blocks.push(format!("<not_found path=\"{file_path}\"/>"));
        }
    }

    // Strip @path tokens from the user text — they are visual shorthand only.
    // The actual content is injected as XML blocks above, so the model receives
    // clean prose without stray "@…" tokens that might confuse it.
    let clean_input = strip_at_mentions(input, &paths);

    if blocks.is_empty() {
        return clean_input;
    }

    if clean_input.is_empty() {
        blocks.join("\n\n")
    } else {
        format!("{}\n\n{clean_input}", blocks.join("\n\n"))
    }
}

/// Parse an optional `:LINE` or `:START-END` suffix from a path reference.
/// Returns `(actual_path, Some((start, optional_end)))` or `(path, None)`.
fn parse_line_ref(path: &str) -> (&str, Option<(usize, Option<usize>)>) {
    if let Some(colon_pos) = path.rfind(':') {
        let suffix = &path[colon_pos + 1..];
        if let Some(dash) = suffix.find('-') {
            if let (Ok(start), Ok(end)) = (
                suffix[..dash].parse::<usize>(),
                suffix[dash + 1..].parse::<usize>(),
            ) {
                if start > 0 {
                    return (&path[..colon_pos], Some((start, Some(end))));
                }
            }
        } else if let Ok(line) = suffix.parse::<usize>() {
            if line > 0 {
                return (&path[..colon_pos], Some((line, None)));
            }
        }
    }
    (path, None)
}

/// Remove every `@path` (and `@"path"`) token from `input` and normalise the
/// resulting whitespace so the model receives clean prose.
fn strip_at_mentions(input: &str, paths: &[String]) -> String {
    let mut result = input.to_string();
    for path in paths {
        // Quoted form first (more specific) then plain form
        result = result.replace(&format!("@\"{path}\""), "");
        result = result.replace(&format!("@{path}"), "");
    }
    // Collapse runs of spaces/tabs within each line; preserve newlines.
    let mut cleaned = String::with_capacity(result.len());
    let mut prev_space = false;
    for ch in result.chars() {
        if ch == ' ' || ch == '\t' {
            if !prev_space {
                cleaned.push(' ');
            }
            prev_space = true;
        } else {
            prev_space = false;
            cleaned.push(ch);
        }
    }
    cleaned.trim().to_string()
}
