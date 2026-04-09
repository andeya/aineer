use std::collections::{BTreeMap, BTreeSet, HashSet};

use api::ToolDefinition;
use engine::PermissionMode;
use plugins::PluginTool;
use serde_json::Value;

use crate::execute_tool;
use crate::specs::mvp_tool_specs;

/// Whether a tool is always included in the model prompt or loaded after [`ToolSearch`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolTier {
    /// Always included in prompt (bash, read_file, write_file, edit_file, glob_search, grep_search, [`ToolSearch`], etc.).
    Core,
    /// Included only after [`ToolSearch`] discovers and activates the tool name for the session.
    Extended,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolManifestEntry {
    pub name: String,
    pub source: ToolSource,
    /// Catalog tier when this entry is surfaced in the tool manifest.
    pub tier: ToolTier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSource {
    Base,
    Conditional,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolRegistry {
    entries: Vec<ToolManifestEntry>,
}

impl ToolRegistry {
    #[must_use]
    pub fn new(entries: Vec<ToolManifestEntry>) -> Self {
        Self { entries }
    }

    #[must_use]
    pub fn entries(&self) -> &[ToolManifestEntry] {
        &self.entries
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,
    pub required_permission: PermissionMode,
    pub tier: ToolTier,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalToolRegistry {
    plugin_tools: Vec<PluginTool>,
}

impl GlobalToolRegistry {
    #[must_use]
    pub fn builtin() -> Self {
        Self {
            plugin_tools: Vec::new(),
        }
    }

    pub fn with_plugin_tools(plugin_tools: Vec<PluginTool>) -> Result<Self, String> {
        let builtin_names = mvp_tool_specs()
            .into_iter()
            .map(|spec| spec.name.to_string())
            .collect::<BTreeSet<_>>();
        let mut seen_plugin_names = BTreeSet::new();

        for tool in &plugin_tools {
            let name = tool.definition().name.clone();
            if builtin_names.contains(&name) {
                return Err(format!(
                    "plugin tool `{name}` conflicts with a built-in tool name"
                ));
            }
            if !seen_plugin_names.insert(name.clone()) {
                return Err(format!("duplicate plugin tool name `{name}`"));
            }
        }

        Ok(Self { plugin_tools })
    }

    pub fn normalize_allowed_tools(
        &self,
        values: &[String],
    ) -> Result<Option<BTreeSet<String>>, String> {
        if values.is_empty() {
            return Ok(None);
        }

        let builtin_specs = mvp_tool_specs();
        let canonical_names = builtin_specs
            .iter()
            .map(|spec| spec.name.to_string())
            .chain(
                self.plugin_tools
                    .iter()
                    .map(|tool| tool.definition().name.clone()),
            )
            .collect::<Vec<_>>();
        let mut name_map = canonical_names
            .iter()
            .map(|name| (normalize_tool_name(name), name.clone()))
            .collect::<BTreeMap<_, _>>();

        for (alias, canonical) in [
            ("read", "read_file"),
            ("write", "write_file"),
            ("edit", "edit_file"),
            ("glob", "glob_search"),
            ("grep", "grep_search"),
        ] {
            name_map.insert(alias.to_string(), canonical.to_string());
        }

        let mut allowed = BTreeSet::new();
        for value in values {
            for token in value
                .split(|ch: char| ch == ',' || ch.is_whitespace())
                .filter(|token| !token.is_empty())
            {
                let normalized = normalize_tool_name(token);
                let canonical = name_map.get(&normalized).ok_or_else(|| {
                    format!(
                        "unsupported tool in --allowedTools: {token} (expected one of: {})",
                        canonical_names.join(", ")
                    )
                })?;
                allowed.insert(canonical.clone());
            }
        }

        Ok(Some(allowed))
    }

    #[must_use]
    pub fn definitions(&self, allowed_tools: Option<&BTreeSet<String>>) -> Vec<ToolDefinition> {
        self.definitions_for_lazy_prompt(allowed_tools, None)
    }

    /// Built-in and plugin tool definitions for the API, with optional lazy-load filtering.
    ///
    /// When `activated_extended` is `None`, every allowed tool is included (legacy “send all”
    /// behavior). When `Some(set)`, only [`ToolTier::Core`] tools plus entries whose names appear
    /// in the set are included. Plugin tools are always [`ToolTier::Extended`].
    #[must_use]
    pub fn definitions_for_lazy_prompt(
        &self,
        allowed_tools: Option<&BTreeSet<String>>,
        activated_extended: Option<&HashSet<String>>,
    ) -> Vec<ToolDefinition> {
        let lazy = activated_extended.is_some();
        let builtin = mvp_tool_specs().into_iter().filter(|spec| {
            if !allowed_tools.is_none_or(|allowed| allowed.contains(spec.name)) {
                return false;
            }
            if !lazy {
                return true;
            }
            match spec.tier {
                ToolTier::Core => true,
                ToolTier::Extended => activated_extended.is_some_and(|a| a.contains(spec.name)),
            }
        });
        let builtin = builtin.map(|spec| ToolDefinition {
            name: spec.name.to_string(),
            description: Some(spec.description.to_string()),
            input_schema: spec.input_schema,
            cache_control: None,
        });
        let plugin = self.plugin_tools.iter().filter(|tool| {
            let name = tool.definition().name.as_str();
            if !allowed_tools.is_none_or(|allowed| allowed.contains(name)) {
                return false;
            }
            if !lazy {
                return true;
            }
            activated_extended.is_some_and(|a| a.contains(name))
        });
        let plugin = plugin.map(|tool| ToolDefinition {
            name: tool.definition().name.clone(),
            description: tool.definition().description.clone(),
            input_schema: tool.definition().input_schema.clone(),
            cache_control: None,
        });
        builtin.chain(plugin).collect()
    }

    /// [`ToolDefinition`] values for plugin tools (for search and display).
    #[must_use]
    pub fn plugin_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.plugin_tools
            .iter()
            .map(|tool| ToolDefinition {
                name: tool.definition().name.clone(),
                description: tool.definition().description.clone(),
                input_schema: tool.definition().input_schema.clone(),
                cache_control: None,
            })
            .collect()
    }

    #[must_use]
    pub fn permission_specs(
        &self,
        allowed_tools: Option<&BTreeSet<String>>,
    ) -> Vec<(String, PermissionMode)> {
        let builtin = mvp_tool_specs()
            .into_iter()
            .filter(|spec| allowed_tools.is_none_or(|allowed| allowed.contains(spec.name)))
            .map(|spec| (spec.name.to_string(), spec.required_permission));
        let plugin = self
            .plugin_tools
            .iter()
            .filter(|tool| {
                allowed_tools
                    .is_none_or(|allowed| allowed.contains(tool.definition().name.as_str()))
            })
            .map(|tool| {
                (
                    tool.definition().name.clone(),
                    permission_mode_from_plugin(tool.required_permission()),
                )
            });
        builtin.chain(plugin).collect()
    }

    pub fn execute(&self, name: &str, input: Value) -> Result<String, String> {
        if mvp_tool_specs().iter().any(|spec| spec.name == name) {
            return execute_tool(name, input)
                .map(|o| o.content)
                .map_err(|e| e.to_string());
        }
        self.plugin_tools
            .iter()
            .find(|tool| tool.definition().name == name)
            .ok_or_else(|| format!("unsupported tool: {name}"))?
            .execute(&input)
            .map_err(|error| error.to_string())
    }

    /// Returns `true` for built-in tools that are read-only and safe to run
    /// concurrently with other tools. Plugin tools are conservatively excluded.
    #[must_use]
    pub fn is_concurrency_safe(&self, name: &str) -> bool {
        // Only pure read-only built-ins with no interactive side effects.
        matches!(
            name,
            "read_file"
                | "glob_search"
                | "grep_search"
                | "WebFetch"
                | "WebSearch"
                | "Skill"
                | "TaskGet"
                | "TaskList"
                | "ListMcpResources"
                | "ReadMcpResource"
                | "MCPSearch"
                | "CronList"
                | "Lsp"
                | "ToolSearch"
        )
    }

    /// Execute a batch of concurrency-safe tools in parallel using scoped threads.
    ///
    /// Each element of `calls` is `(tool_name, json_input_str)`.
    /// Returns results in the same order as `calls`.
    pub fn execute_batch(&self, calls: &[(&str, &str)]) -> Vec<Result<String, String>> {
        std::thread::scope(|scope| {
            let handles: Vec<_> = calls
                .iter()
                .map(|(name, input_str)| {
                    scope.spawn(move || {
                        let value: Value =
                            serde_json::from_str(input_str).map_err(|e| e.to_string())?;
                        self.execute(name, value)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("concurrent tool thread panicked"))
                .collect()
        })
    }
}

fn normalize_tool_name(value: &str) -> String {
    value.trim().replace('-', "_").to_ascii_lowercase()
}

fn permission_mode_from_plugin(value: &str) -> PermissionMode {
    match value {
        "danger-full-access" => PermissionMode::DangerFullAccess,
        "workspace-write" => PermissionMode::WorkspaceWrite,
        _ => PermissionMode::ReadOnly,
    }
}

#[cfg(test)]
mod lazy_prompt_tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn definitions_for_lazy_prompt_empty_activation_is_core_plus_tool_search_only() {
        let reg = GlobalToolRegistry::builtin();
        let activated = HashSet::new();
        let defs = reg.definitions_for_lazy_prompt(None, Some(&activated));
        let names: Vec<_> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"bash"));
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"ToolSearch"));
        assert!(
            !names.contains(&"WebFetch") && !names.contains(&"Agent"),
            "extended-tier tools should be omitted until activated: {names:?}"
        );
    }

    #[test]
    fn definitions_for_lazy_prompt_includes_activated_extended() {
        let reg = GlobalToolRegistry::builtin();
        let mut activated = HashSet::new();
        activated.insert("WebFetch".to_string());
        activated.insert("Skill".to_string());
        let defs = reg.definitions_for_lazy_prompt(None, Some(&activated));
        let names: Vec<_> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"WebFetch"));
        assert!(names.contains(&"Skill"));
        assert!(!names.contains(&"Agent"));
    }
}
