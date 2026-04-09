use std::collections::BTreeSet;
use std::path::PathBuf;

pub struct Completer {
    history: Vec<String>,
    path_cache: Vec<String>,
    cache_valid: bool,
}

impl Completer {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            path_cache: Vec::new(),
            cache_valid: false,
        }
    }

    pub fn add_to_history(&mut self, cmd: &str) {
        let trimmed = cmd.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        self.history.retain(|h| h != &trimmed);
        self.history.push(trimmed);
        if self.history.len() > 500 {
            self.history.remove(0);
        }
    }

    pub fn complete(&mut self, input: &str) -> Vec<String> {
        if input.is_empty() {
            return Vec::new();
        }

        let mut results = BTreeSet::new();

        // History completions
        for h in self.history.iter().rev() {
            if h.starts_with(input) && h != input {
                results.insert(h.clone());
            }
        }

        // If the input looks like a path (contains / or starts with . or ~), do file completion
        let last_word = input.rsplit_once(' ').map_or(input, |(_, w)| w);

        if last_word.contains('/') || last_word.starts_with('.') || last_word.starts_with('~') {
            let file_completions = complete_path(last_word);
            let prefix = if let Some((before, _)) = input.rsplit_once(' ') {
                format!("{before} ")
            } else {
                String::new()
            };
            for fc in file_completions {
                results.insert(format!("{prefix}{fc}"));
            }
        } else if !last_word.is_empty() {
            // PATH executable completion (only for the first word or last word)
            if !self.cache_valid {
                self.path_cache = discover_path_executables();
                self.cache_valid = true;
            }
            let prefix = if let Some((before, _)) = input.rsplit_once(' ') {
                format!("{before} ")
            } else {
                String::new()
            };
            for exe in &self.path_cache {
                if exe.starts_with(last_word) && *exe != last_word {
                    results.insert(format!("{prefix}{exe}"));
                }
            }
        }

        results.into_iter().take(10).collect()
    }
}

impl Default for Completer {
    fn default() -> Self {
        Self::new()
    }
}

fn complete_path(partial: &str) -> Vec<String> {
    let expanded = if partial.starts_with('~') {
        if let Ok(home) = std::env::var("HOME") {
            partial.replacen('~', &home, 1)
        } else {
            partial.to_string()
        }
    } else {
        partial.to_string()
    };

    let path = PathBuf::from(&expanded);
    let (dir, prefix) = if expanded.ends_with('/') {
        (path.clone(), String::new())
    } else {
        let dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let prefix = path
            .file_name()
            .map_or(String::new(), |n| n.to_string_lossy().to_string());
        (dir.to_path_buf(), prefix)
    };

    let mut completions = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&prefix) {
                let full = if expanded.ends_with('/') {
                    format!("{partial}{name}")
                } else if let Some(parent) = partial.rsplit_once('/') {
                    format!("{}/{name}", parent.0)
                } else {
                    name.clone()
                };
                let suffix = if entry.path().is_dir() {
                    format!("{full}/")
                } else {
                    full
                };
                completions.push(suffix);
            }
        }
    }

    completions.sort();
    completions.truncate(10);
    completions
}

fn discover_path_executables() -> Vec<String> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    let mut executables = BTreeSet::new();

    for dir in std::env::split_paths(&path_var) {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        if meta.is_file() && meta.permissions().mode() & 0o111 != 0 {
                            if let Some(name) = entry.file_name().to_str() {
                                executables.insert(name.to_string());
                            }
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        if meta.is_file() {
                            if let Some(name) = entry.file_name().to_str() {
                                executables.insert(name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    executables.into_iter().collect()
}
