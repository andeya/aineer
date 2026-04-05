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
    let mut blocks = Vec::new();

    for path in &paths {
        let p = std::path::Path::new(path);
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
                    "<attached_directory path=\"{path}\">\n{}\n</attached_directory>",
                    listing.join("\n")
                ));
            } else {
                eprintln!("[warn] @{path}: directory not readable, skipping");
                blocks.push(format!("<not_found path=\"{path}\"/>"));
            }
        } else if let Ok(content) = std::fs::read_to_string(p) {
            let truncated: String = content
                .lines()
                .take(MAX_LINES)
                .collect::<Vec<_>>()
                .join("\n");
            blocks.push(format!(
                "<attached_file path=\"{path}\">\n{truncated}\n</attached_file>"
            ));
        } else {
            eprintln!("[warn] @{path}: file not found or not readable, skipping");
            blocks.push(format!("<not_found path=\"{path}\"/>"));
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
