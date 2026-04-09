use egui::{RichText, Ui};

use crate::theme as t;

use super::SettingsDraft;

pub fn show(ui: &mut Ui, draft: &mut SettingsDraft) -> bool {
    let mut changed = false;

    ui.heading(RichText::new("Default Mode").size(14.0));
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label("Permission mode");
        if egui::ComboBox::from_id_salt("permission_mode")
            .selected_text(&draft.default_permission_mode)
            .show_ui(ui, |ui| {
                let mut c = false;
                c |= ui
                    .selectable_value(
                        &mut draft.default_permission_mode,
                        "ask".to_string(),
                        "Ask — confirm each tool execution",
                    )
                    .changed();
                c |= ui
                    .selectable_value(
                        &mut draft.default_permission_mode,
                        "auto".to_string(),
                        "Auto — allow all safe operations",
                    )
                    .changed();
                c |= ui
                    .selectable_value(
                        &mut draft.default_permission_mode,
                        "yolo".to_string(),
                        "YOLO — allow everything (dangerous)",
                    )
                    .changed();
                c
            })
            .inner
            .unwrap_or(false)
        {
            changed = true;
        }
    });

    ui.add_space(12.0);
    ui.heading(RichText::new("Custom Rules").size(14.0));
    ui.add_space(4.0);

    ui.label(
        RichText::new("Pattern → Decision rules for tool execution")
            .size(11.0)
            .color(t::FG_DIM),
    );

    let mut remove_idx = None;
    for (i, (pattern, decision)) in draft.permission_rules.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label("Pattern:");
            if ui.text_edit_singleline(pattern).changed() {
                changed = true;
            }
            ui.label("→");
            if egui::ComboBox::from_id_salt(format!("rule_{i}"))
                .selected_text(decision.as_str())
                .show_ui(ui, |ui| {
                    let mut c = false;
                    c |= ui
                        .selectable_value(decision, "allow".to_string(), "Allow")
                        .changed();
                    c |= ui
                        .selectable_value(decision, "deny".to_string(), "Deny")
                        .changed();
                    c |= ui
                        .selectable_value(decision, "ask".to_string(), "Ask")
                        .changed();
                    c
                })
                .inner
                .unwrap_or(false)
            {
                changed = true;
            }
            if ui.small_button("✕").clicked() {
                remove_idx = Some(i);
                changed = true;
            }
        });
    }
    if let Some(idx) = remove_idx {
        draft.permission_rules.remove(idx);
    }

    if ui.small_button("+ Add rule").clicked() {
        draft
            .permission_rules
            .push(("*".to_string(), "ask".to_string()));
        changed = true;
    }

    changed
}
