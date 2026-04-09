use std::path::{Path, PathBuf};

use egui::{RichText, ScrollArea, Ui};

use crate::icons;
use crate::theme::{self as t, font_size, spacing};
use crate::widgets::EmptyState;

/// A single entry in the file tree.
#[derive(Clone)]
struct FsEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    depth: usize,
    expanded: bool,
}

/// Action returned when the user interacts with the explorer.
#[derive(Debug, Clone)]
pub enum ExplorerAction {
    None,
    /// User requested to change CWD to this directory.
    ChangeDir(PathBuf),
    /// User right-clicked — caller can show a context menu.
    RightClicked(PathBuf),
}

/// A file-tree view rooted at `root_dir`.
pub struct ExplorerPanel {
    root: Option<PathBuf>,
    entries: Vec<FsEntry>,
    dirty: bool,
}

impl Default for ExplorerPanel {
    fn default() -> Self {
        Self {
            root: None,
            entries: Vec::new(),
            dirty: true,
        }
    }
}

impl ExplorerPanel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the root directory. Triggers a re-scan.
    pub fn set_root(&mut self, dir: impl Into<PathBuf>) {
        let new = dir.into();
        if self.root.as_deref() != Some(&new) {
            self.root = Some(new);
            self.dirty = true;
        }
    }

    /// Force a refresh of the entry list.
    pub fn refresh(&mut self) {
        self.dirty = true;
    }

    fn load_entries(&mut self) {
        self.entries.clear();
        if let Some(root) = &self.root.clone() {
            self.scan_dir(root, 0, 2);
        }
        self.dirty = false;
    }

    fn scan_dir(&mut self, dir: &Path, depth: usize, max_depth: usize) {
        let Ok(read) = std::fs::read_dir(dir) else {
            return;
        };

        let mut entries: Vec<_> = read.flatten().collect();
        entries.sort_by(|a, b| {
            let a_dir = a.path().is_dir();
            let b_dir = b.path().is_dir();
            // Directories first, then alphabetical
            b_dir.cmp(&a_dir).then_with(|| a.file_name().cmp(&b.file_name()))
        });

        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip hidden files and common noise directories
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            let is_dir = path.is_dir();
            self.entries.push(FsEntry {
                name,
                path: path.clone(),
                is_dir,
                depth,
                expanded: false,
            });
            // Eager-expand one level for immediate visual feedback
            if is_dir && depth < max_depth.saturating_sub(1) {
                self.scan_dir(&path, depth + 1, max_depth);
            }
        }
    }

    pub fn show(&mut self, ui: &mut Ui) -> ExplorerAction {
        if self.dirty {
            self.load_entries();
        }

        let mut action = ExplorerAction::None;

        // Header
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("EXPLORER")
                    .size(font_size::CAPTION)
                    .strong()
                    .color(t::FG_DIM()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("↺").size(font_size::SMALL).color(t::FG_DIM()))
                            .fill(egui::Color32::TRANSPARENT),
                    )
                    .on_hover_text("Refresh")
                    .clicked()
                {
                    self.dirty = true;
                }
            });
        });
        ui.add_space(spacing::XXS);
        ui.separator();

        if self.root.is_none() {
            EmptyState::new(icons::FOLDER, "No folder open")
                .subtitle("Open a terminal session to browse files")
                .show(ui);
            return action;
        }

        if self.entries.is_empty() {
            EmptyState::new(icons::FOLDER, "Empty folder").show(ui);
            return action;
        }

        ScrollArea::vertical()
            .id_salt("explorer_scroll")
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                let entries = self.entries.clone();
                for entry in &entries {
                    let indent = entry.depth as f32 * 14.0;
                    ui.horizontal(|ui| {
                        ui.add_space(indent);

                        let icon = if entry.is_dir {
                            if entry.expanded { "▼ 📂" } else { "▶ 📁" }
                        } else {
                            file_icon(&entry.name)
                        };

                        let label_resp = ui
                            .add(
                                egui::Label::new(
                                    RichText::new(format!("{} {}", icon, entry.name))
                                        .size(font_size::SMALL)
                                        .color(t::FG()),
                                )
                                .sense(egui::Sense::click()),
                            )
                            .on_hover_cursor(egui::CursorIcon::PointingHand);

                        if label_resp.clicked() && entry.is_dir {
                            action = ExplorerAction::ChangeDir(entry.path.clone());
                        }

                        label_resp.context_menu(|ui| {
                            if entry.is_dir
                                && ui.button("Open in Terminal").clicked() {
                                    action = ExplorerAction::ChangeDir(entry.path.clone());
                                    ui.close();
                                }
                            if ui.button("Copy Path").clicked() {
                                ui.ctx().copy_text(entry.path.to_string_lossy().to_string());
                                ui.close();
                            }
                        });
                    });
                }
            });

        action
    }
}

fn file_icon(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "🦀",
        "toml" | "yaml" | "yml" => "⚙",
        "md" => "📄",
        "json" => "{ }",
        "sh" | "zsh" | "bash" => "⬛",
        "ts" | "js" => "🟨",
        "py" => "🐍",
        "go" => "🔵",
        "lock" => "🔒",
        _ => "📄",
    }
}
