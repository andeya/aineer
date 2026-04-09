use egui::{RichText, Ui};

use crate::theme as t;

use super::SettingsDraft;

pub fn show(ui: &mut Ui, draft: &mut SettingsDraft) -> bool {
    let mut changed = false;

    ui.heading(RichText::new("Default Model").size(14.0));
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label("Model");
        if ui.text_edit_singleline(&mut draft.default_model).changed() {
            changed = true;
        }
    });

    ui.label(
        RichText::new("Use \"auto\" for automatic detection, or specify a model like \"claude-sonnet-4-6\", \"gpt-4o\", \"ollama/qwen2.5-coder\"")
            .size(11.0)
            .color(t::FG_DIM),
    );

    ui.add_space(12.0);
    ui.heading(RichText::new("Fallback Chain").size(14.0));
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label("Fallback models (comma-separated)");
    });
    if ui
        .text_edit_singleline(&mut draft.fallback_models)
        .changed()
    {
        changed = true;
    }

    ui.add_space(12.0);
    ui.heading(RichText::new("Thinking Mode").size(14.0));
    ui.add_space(4.0);

    if ui
        .checkbox(
            &mut draft.thinking_mode,
            "Enable extended thinking (when supported)",
        )
        .changed()
    {
        changed = true;
    }

    changed
}
