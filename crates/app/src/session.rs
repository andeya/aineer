//! Session persistence (stub until wired into tabs / workspace).
#![allow(dead_code)]

use aineer_ui::blocks::Block;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionData {
    pub version: u32,
    pub tabs: Vec<TabSession>,
    pub active_tab_index: usize,
    pub sidebar_visible: bool,
    pub sidebar_width: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TabSession {
    pub id: u64,
    pub title: String,
    pub working_dir: PathBuf,
    pub blocks: Vec<Block>,
    pub scroll_position: f64,
}

impl SessionData {
    pub fn new() -> Self {
        Self {
            version: 1,
            tabs: Vec::new(),
            active_tab_index: 0,
            sidebar_visible: true,
            sidebar_width: 280.0,
        }
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, json)
    }

    pub fn load(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let json = std::fs::read_to_string(path)?;
        serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
}

impl Default for SessionData {
    fn default() -> Self {
        Self::new()
    }
}
