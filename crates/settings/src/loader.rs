use crate::SettingsError;
use crate::schema::SettingsContent;
use std::path::Path;

pub fn load_json_or_default(path: &Path) -> Result<SettingsContent, SettingsError> {
    if !path.exists() {
        return Ok(SettingsContent::default());
    }
    let contents = std::fs::read_to_string(path).map_err(|e| SettingsError::ReadFile {
        path: path.to_path_buf(),
        source: e,
    })?;
    if contents.trim().is_empty() {
        return Ok(SettingsContent::default());
    }
    serde_json::from_str(&contents).map_err(|e| SettingsError::ParseJson {
        path: path.to_path_buf(),
        source: e,
    })
}

pub fn read_json_preserving(path: &Path) -> Result<serde_json::Value, SettingsError> {
    if !path.exists() {
        return Ok(serde_json::Value::Object(serde_json::Map::new()));
    }
    let contents = std::fs::read_to_string(path).map_err(|e| SettingsError::ReadFile {
        path: path.to_path_buf(),
        source: e,
    })?;
    if contents.trim().is_empty() {
        return Ok(serde_json::Value::Object(serde_json::Map::new()));
    }
    serde_json::from_str(&contents).map_err(|e| SettingsError::ParseJson {
        path: path.to_path_buf(),
        source: e,
    })
}

pub fn write_json_pretty(path: &Path, value: &serde_json::Value) -> Result<(), SettingsError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SettingsError::WriteFile {
            path: path.to_path_buf(),
            source: e,
        })?;
    }
    let contents = serde_json::to_string_pretty(value).unwrap();
    std::fs::write(path, contents).map_err(|e| SettingsError::WriteFile {
        path: path.to_path_buf(),
        source: e,
    })
}
