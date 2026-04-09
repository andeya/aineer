use egui::Ui;

use crate::icons;
use crate::widgets::{SectionCard, SettingsRow};

use super::SettingsDraft;

pub fn show(ui: &mut Ui, draft: &mut SettingsDraft) -> bool {
    let mut changed = false;

    SectionCard::new("Sandbox")
        .icon(icons::PACKAGE)
        .description("Isolated execution environment for untrusted tool operations")
        .show(ui, |ui| {
            changed |= SettingsRow::new("Enable Sandbox")
                .description("Run tools in a Docker container for safety (requires Docker)")
                .show(ui, |ui| {
                    ui.checkbox(&mut draft.sandbox_enabled, "").changed()
                });
        });

    SectionCard::new("Context Management")
        .icon(icons::CHART)
        .description("Control how conversation context is managed")
        .show(ui, |ui| {
            changed |= SettingsRow::new("Auto-Compact")
                .description(
                    "Automatically summarize and compact context when it approaches the limit",
                )
                .show(ui, |ui| ui.checkbox(&mut draft.auto_compact, "").changed());

            changed |= SettingsRow::new("Max Context Tokens")
                .description("Maximum number of tokens before context compaction triggers")
                .show(ui, |ui| {
                    ui.add(
                        egui::DragValue::new(&mut draft.max_context_tokens)
                            .range(10_000..=1_000_000)
                            .speed(1000)
                            .suffix(" tokens"),
                    )
                    .changed()
                });
        });

    changed
}
