use egui::{RichText, Ui};

use crate::theme as t;

use super::SettingsDraft;

pub fn show(ui: &mut Ui, draft: &mut SettingsDraft) -> bool {
    let mut changed = false;

    ui.heading(RichText::new("Sandbox").size(14.0));
    ui.add_space(4.0);

    if ui
        .checkbox(
            &mut draft.sandbox_enabled,
            "Enable sandbox for tool execution",
        )
        .changed()
    {
        changed = true;
    }

    ui.label(
        RichText::new("Sandboxed tools run in an isolated environment (requires Docker)")
            .size(11.0)
            .color(t::FG_DIM),
    );

    ui.add_space(12.0);
    ui.heading(RichText::new("Context Management").size(14.0));
    ui.add_space(4.0);

    if ui
        .checkbox(&mut draft.auto_compact, "Auto-compact context when full")
        .changed()
    {
        changed = true;
    }

    ui.horizontal(|ui| {
        ui.label("Max context tokens");
        if ui
            .add(
                egui::DragValue::new(&mut draft.max_context_tokens)
                    .range(10_000..=1_000_000)
                    .speed(1000),
            )
            .changed()
        {
            changed = true;
        }
    });

    changed
}
