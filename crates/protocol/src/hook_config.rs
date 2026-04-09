//! Hook configuration types shared between plugins and engine.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeHookConfig {
    commands: BTreeMap<String, Vec<String>>,
}

impl RuntimeHookConfig {
    /// Backward-compatible constructor for PreToolUse/PostToolUse commands.
    #[must_use]
    pub fn new(pre_tool_use: Vec<String>, post_tool_use: Vec<String>) -> Self {
        let mut commands = BTreeMap::new();
        if !pre_tool_use.is_empty() {
            commands.insert("PreToolUse".to_string(), pre_tool_use);
        }
        if !post_tool_use.is_empty() {
            commands.insert("PostToolUse".to_string(), post_tool_use);
        }
        Self { commands }
    }

    /// Build from an arbitrary map of event-name -> commands.
    #[must_use]
    pub fn from_map(commands: BTreeMap<String, Vec<String>>) -> Self {
        Self { commands }
    }

    /// All registered event-name -> commands.
    #[must_use]
    pub fn commands(&self) -> &BTreeMap<String, Vec<String>> {
        &self.commands
    }

    #[must_use]
    pub fn pre_tool_use(&self) -> &[String] {
        self.commands
            .get("PreToolUse")
            .map_or(&[] as &[String], Vec::as_slice)
    }

    #[must_use]
    pub fn post_tool_use(&self) -> &[String] {
        self.commands
            .get("PostToolUse")
            .map_or(&[] as &[String], Vec::as_slice)
    }

    #[must_use]
    pub fn merged(&self, other: &Self) -> Self {
        let mut merged = self.clone();
        merged.extend(other);
        merged
    }

    pub fn extend(&mut self, other: &Self) {
        for (event, cmds) in &other.commands {
            let entry = self.commands.entry(event.clone()).or_default();
            extend_unique(entry, cmds);
        }
    }
}

fn extend_unique(target: &mut Vec<String>, values: &[String]) {
    for value in values {
        push_unique(target, value.clone());
    }
}

fn push_unique(target: &mut Vec<String>, value: String) {
    if !target.iter().any(|existing| existing == &value) {
        target.push(value);
    }
}
