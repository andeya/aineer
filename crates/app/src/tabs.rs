use std::collections::BTreeMap;
use std::sync::mpsc::Sender;

use terminal::{BackendSettings, PtyEvent, TerminalBackend};

pub struct TabManager {
    tabs: BTreeMap<u64, Tab>,
    active_tab_id: Option<u64>,
    next_id: u64,
}

pub struct Tab {
    pub backend: TerminalBackend,
    pub title: String,
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            tabs: BTreeMap::new(),
            active_tab_id: None,
            next_id: 0,
        }
    }

    pub fn create_tab(&mut self, ctx: egui::Context, pty_sender: Sender<(u64, PtyEvent)>) {
        let id = self.next_id;
        self.next_id += 1;

        let shell = detect_default_shell();

        let backend = TerminalBackend::new(
            id,
            ctx,
            pty_sender,
            BackendSettings {
                shell: shell.clone(),
                ..Default::default()
            },
        )
        .expect("failed to create terminal backend");

        let tab = Tab {
            backend,
            title: short_shell_name(&shell),
        };

        self.tabs.insert(id, tab);
        self.active_tab_id = Some(id);
    }

    pub fn create_tab_with_settings(
        &mut self,
        ctx: egui::Context,
        pty_sender: Sender<(u64, PtyEvent)>,
        settings: BackendSettings,
    ) -> Option<u64> {
        let id = self.next_id;
        self.next_id += 1;

        match TerminalBackend::new(id, ctx, pty_sender, settings) {
            Ok(backend) => {
                let tab = Tab {
                    backend,
                    title: format!("tab-{id}"),
                };
                self.tabs.insert(id, tab);
                self.active_tab_id = Some(id);
                Some(id)
            }
            Err(e) => {
                tracing::warn!("Failed to create tab with custom settings: {e}");
                None
            }
        }
    }

    pub fn remove_tab(&mut self, id: u64) {
        self.tabs.remove(&id);
        if self.active_tab_id == Some(id) {
            self.active_tab_id = self
                .tabs
                .keys()
                .rev()
                .find(|&&k| k <= id)
                .or_else(|| self.tabs.keys().next())
                .copied();
        }
    }

    pub fn set_title(&mut self, id: u64, title: String) {
        if let Some(tab) = self.tabs.get_mut(&id) {
            tab.title = title;
        }
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        let id = self.active_tab_id?;
        self.tabs.get(&id)
    }

    pub fn active_tab_mut(&mut self) -> Option<&mut Tab> {
        let id = self.active_tab_id?;
        self.tabs.get_mut(&id)
    }

    pub fn tab(&self, id: u64) -> Option<&Tab> {
        self.tabs.get(&id)
    }

    pub fn tab_mut(&mut self, id: u64) -> Option<&mut Tab> {
        self.tabs.get_mut(&id)
    }

    pub fn active_tab_id(&self) -> Option<u64> {
        self.active_tab_id
    }

    pub fn set_active(&mut self, id: u64) {
        if self.tabs.contains_key(&id) {
            self.active_tab_id = Some(id);
        }
    }

    pub fn tab_ids_and_titles(&self) -> Vec<(u64, String)> {
        self.tabs
            .iter()
            .map(|(&id, tab)| (id, tab.title.clone()))
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }
}

fn detect_default_shell() -> String {
    #[cfg(unix)]
    {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
    }
    #[cfg(windows)]
    {
        if which_exists("pwsh.exe") {
            "pwsh.exe".to_string()
        } else {
            "cmd.exe".to_string()
        }
    }
}

fn short_shell_name(shell: &str) -> String {
    std::path::Path::new(shell)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| shell.to_string())
}

#[cfg(windows)]
fn which_exists(cmd: &str) -> bool {
    std::process::Command::new("where")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
