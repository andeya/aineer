mod loader;
pub mod schema;

use schema::SettingsContent;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("Failed to read {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("Invalid JSON in {path}: {source}")]
    ParseJson {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("Failed to write {path}: {source}")]
    WriteFile {
        path: PathBuf,
        source: std::io::Error,
    },
}

pub struct SettingsStore {
    user: SettingsContent,
    #[allow(dead_code)]
    project: SettingsContent,
    #[allow(dead_code)]
    local: SettingsContent,
    merged: SettingsContent,
    user_path: PathBuf,
    #[allow(dead_code)]
    project_path: Option<PathBuf>,
    #[allow(dead_code)]
    local_path: Option<PathBuf>,
}

impl SettingsStore {
    pub fn load(user_path: PathBuf, project_dir: Option<PathBuf>) -> Result<Self, SettingsError> {
        let user = loader::load_json_or_default(&user_path)?;
        let project_path = project_dir.as_ref().map(|d| d.join("settings.json"));
        let local_path = project_dir.as_ref().map(|d| d.join("settings.local.json"));

        let project = project_path
            .as_ref()
            .map(|p| loader::load_json_or_default(p))
            .transpose()?
            .unwrap_or_default();
        let local = local_path
            .as_ref()
            .map(|p| loader::load_json_or_default(p))
            .transpose()?
            .unwrap_or_default();

        let merged = Self::deep_merge_all(&user, &project, &local);

        Ok(Self {
            user,
            project,
            local,
            merged,
            user_path,
            project_path,
            local_path,
        })
    }

    pub fn merged(&self) -> &SettingsContent {
        &self.merged
    }

    pub fn user(&self) -> &SettingsContent {
        &self.user
    }

    pub fn save_user(&self, updates: &serde_json::Value) -> Result<(), SettingsError> {
        let mut existing = loader::read_json_preserving(&self.user_path)?;
        deep_merge(&mut existing, updates);
        loader::write_json_pretty(&self.user_path, &existing)
    }

    fn deep_merge_all(
        user: &SettingsContent,
        project: &SettingsContent,
        local: &SettingsContent,
    ) -> SettingsContent {
        let user_val = serde_json::to_value(user).unwrap_or_default();
        let project_val = serde_json::to_value(project).unwrap_or_default();
        let local_val = serde_json::to_value(local).unwrap_or_default();

        let mut merged = user_val;
        deep_merge(&mut merged, &project_val);
        deep_merge(&mut merged, &local_val);

        serde_json::from_value(merged).unwrap_or_default()
    }
}

/// Recursively merge `source` into `target`. Source values overwrite target
/// values for the same key, but keys only in target are preserved.
pub fn deep_merge(target: &mut serde_json::Value, source: &serde_json::Value) {
    match (target, source) {
        (serde_json::Value::Object(target_map), serde_json::Value::Object(source_map)) => {
            for (key, source_val) in source_map {
                if source_val.is_null() {
                    continue;
                }
                let target_val = target_map
                    .entry(key.clone())
                    .or_insert(serde_json::Value::Null);
                deep_merge(target_val, source_val);
            }
        }
        (target, source) => {
            if !source.is_null() {
                *target = source.clone();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deep_merge_preserves_unknown_keys() {
        let mut target = serde_json::json!({
            "theme": "dark",
            "mcpServers": {"fs": {"type": "stdio"}},
            "hooks": {"preToolUse": ["hook1"]}
        });
        let source = serde_json::json!({
            "theme": "light"
        });
        deep_merge(&mut target, &source);
        assert_eq!(target["theme"], "light");
        assert!(target["mcpServers"]["fs"]["type"].is_string());
        assert!(target["hooks"]["preToolUse"].is_array());
    }

    #[test]
    fn deep_merge_nested_objects() {
        let mut target = serde_json::json!({
            "terminal": {"shellPath": "/bin/zsh", "fontSize": 14}
        });
        let source = serde_json::json!({
            "terminal": {"fontSize": 16}
        });
        deep_merge(&mut target, &source);
        assert_eq!(target["terminal"]["shellPath"], "/bin/zsh");
        assert_eq!(target["terminal"]["fontSize"], 16);
    }

    #[test]
    fn deep_merge_null_values_ignored() {
        let mut target = serde_json::json!({"theme": "dark"});
        let source = serde_json::json!({"theme": null});
        deep_merge(&mut target, &source);
        assert_eq!(target["theme"], "dark");
    }
}
