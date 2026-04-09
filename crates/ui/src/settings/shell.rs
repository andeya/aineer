use egui::{RichText, Ui};

use super::SettingsDraft;

pub fn show(ui: &mut Ui, draft: &mut SettingsDraft) -> bool {
    let mut changed = false;

    ui.heading(RichText::new("Shell").size(14.0));
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label("Shell path");
        if ui.text_edit_singleline(&mut draft.shell_path).changed() {
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Shell args");
        if ui.text_edit_singleline(&mut draft.shell_args).changed() {
            changed = true;
        }
    });

    ui.add_space(12.0);
    ui.heading(RichText::new("Environment Variables").size(14.0));
    ui.add_space(4.0);

    let mut remove_idx = None;
    for (i, (key, val)) in draft.env_vars.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            if ui.text_edit_singleline(key).changed() {
                changed = true;
            }
            ui.label("=");
            if ui.text_edit_singleline(val).changed() {
                changed = true;
            }
            if ui.small_button("✕").clicked() {
                remove_idx = Some(i);
                changed = true;
            }
        });
    }
    if let Some(idx) = remove_idx {
        draft.env_vars.remove(idx);
    }

    if ui.small_button("+ Add variable").clicked() {
        draft.env_vars.push((String::new(), String::new()));
        changed = true;
    }

    changed
}
