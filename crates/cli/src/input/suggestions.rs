use std::path::Path;

#[derive(Debug, Clone)]
pub(super) struct SuggestionItem {
    pub display: String,
    pub description: String,
    pub completion: String,
    pub execute_on_enter: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SuggestionTrigger {
    Slash,
    At {
        token_start: usize,
        token_len: usize,
    },
}

#[derive(Debug, Clone)]
pub(super) struct SuggestionState {
    pub items: Vec<SuggestionItem>,
    pub selected: usize,
    pub trigger: SuggestionTrigger,
}

#[derive(Debug, Clone)]
pub(crate) struct CommandEntry {
    pub name: String,
    pub description: String,
    pub has_args: bool,
}

/// Generate slash-command suggestions when input starts with `/` and has no whitespace.
pub(super) fn slash_suggestions(
    text: &str,
    cursor: usize,
    specs: &[CommandEntry],
) -> Option<SuggestionState> {
    if cursor != text.len() || !text.starts_with('/') || text.contains(char::is_whitespace) {
        return None;
    }

    let prefix = text.to_ascii_lowercase();
    let items: Vec<SuggestionItem> = specs
        .iter()
        .filter(|spec| spec.name.to_ascii_lowercase().starts_with(&prefix))
        .map(|spec| SuggestionItem {
            display: spec.name.clone(),
            description: spec.description.clone(),
            completion: format!("{} ", spec.name),
            execute_on_enter: !spec.has_args,
        })
        .collect();

    if items.is_empty() {
        return None;
    }
    if items.len() == 1 && items[0].display == text {
        return None;
    }

    Some(SuggestionState {
        items,
        selected: 0,
        trigger: SuggestionTrigger::Slash,
    })
}

/// Find the `@token` span ending at `cursor`. Returns `(token_start, token_len)` where
/// `token_start` is the byte offset of `@`.
pub(super) fn extract_at_token(text: &str, cursor: usize) -> Option<(usize, usize)> {
    let before = &text[..cursor];
    let at_pos = before.rfind('@')?;
    if at_pos > 0 && !text.as_bytes()[at_pos - 1].is_ascii_whitespace() {
        return None;
    }
    let token = &text[at_pos + 1..cursor];
    if token.contains(char::is_whitespace) {
        return None;
    }
    Some((at_pos, cursor - at_pos))
}

/// Generate file/directory suggestions for an `@` token.
pub(super) fn file_suggestions(text: &str, cursor: usize) -> Option<SuggestionState> {
    let (token_start, token_len) = extract_at_token(text, cursor)?;
    let token = &text[token_start + 1..token_start + token_len];

    let (dir, prefix) = if token.is_empty() {
        (Path::new("."), "")
    } else if token.ends_with('/') || token.ends_with(std::path::MAIN_SEPARATOR) {
        (Path::new(token), "")
    } else {
        match Path::new(token).parent() {
            Some(parent) if !parent.as_os_str().is_empty() => {
                let file_name = Path::new(token)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("");
                (parent, file_name)
            }
            _ => (Path::new("."), token),
        }
    };

    let entries = std::fs::read_dir(dir).ok()?;
    let prefix_lower = prefix.to_ascii_lowercase();

    let mut dir_items = Vec::new();
    let mut file_items = Vec::new();

    const MAX_SUGGESTIONS: usize = 200;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str.starts_with('.') && !prefix_lower.starts_with('.') {
            continue;
        }
        if !prefix.is_empty() && !name_str.to_ascii_lowercase().starts_with(&prefix_lower) {
            continue;
        }

        let is_dir = entry.file_type().is_ok_and(|ft| ft.is_dir());
        let display_path = if dir == Path::new(".") {
            if is_dir {
                format!("{name_str}/")
            } else {
                name_str.to_string()
            }
        } else {
            let full = dir.join(&*name_str);
            if is_dir {
                format!("{}/", full.display())
            } else {
                full.display().to_string()
            }
        };

        let completion = if is_dir {
            format!("@{display_path}")
        } else {
            format!("@{display_path} ")
        };

        let item = SuggestionItem {
            display: format!("+ {display_path}"),
            description: String::new(),
            completion,
            execute_on_enter: false,
        };

        if is_dir {
            dir_items.push(item);
        } else {
            file_items.push(item);
        }

        if dir_items.len() + file_items.len() >= MAX_SUGGESTIONS {
            break;
        }
    }

    dir_items.sort_by(|a, b| a.display.cmp(&b.display));
    file_items.sort_by(|a, b| a.display.cmp(&b.display));
    dir_items.append(&mut file_items);

    if dir_items.is_empty() {
        return None;
    }

    Some(SuggestionState {
        items: dir_items,
        selected: 0,
        trigger: SuggestionTrigger::At {
            token_start,
            token_len,
        },
    })
}

/// Extract `@path` references from submitted input for content injection.
pub(crate) fn extract_at_mentioned_files(input: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'@' && (i == 0 || bytes[i - 1].is_ascii_whitespace()) {
            let start = i + 1;
            if start < bytes.len() && bytes[start] == b'"' {
                if let Some(end) = input[start + 1..].find('"') {
                    let path = &input[start + 1..start + 1 + end];
                    if !path.is_empty() {
                        paths.push(path.to_string());
                    }
                    i = start + 1 + end + 1;
                    continue;
                }
            }
            let end = input[start..]
                .find(char::is_whitespace)
                .map_or(input.len(), |pos| start + pos);
            let path = &input[start..end];
            if !path.is_empty() && !path.starts_with('/') {
                paths.push(path.to_string());
            }
            i = end;
        } else {
            i += 1;
        }
    }

    paths.sort();
    paths.dedup();
    paths
}
