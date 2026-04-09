use egui::{RichText, Ui};

use super::SettingsDraft;

pub fn show(ui: &mut Ui, draft: &mut SettingsDraft) -> bool {
    let mut changed = false;

    ui.heading(RichText::new("Appearance").size(14.0));
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.label("Theme");
        if egui::ComboBox::from_id_salt("theme")
            .selected_text(&draft.theme)
            .show_ui(ui, |ui| {
                let mut c = false;
                c |= ui
                    .selectable_value(&mut draft.theme, "dark".to_string(), "Dark")
                    .changed();
                c |= ui
                    .selectable_value(&mut draft.theme, "light".to_string(), "Light")
                    .changed();
                c
            })
            .inner
            .unwrap_or(false)
        {
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Font size");
        if ui
            .add(egui::Slider::new(&mut draft.font_size, 10.0..=24.0).step_by(1.0))
            .changed()
        {
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Language");
        if egui::ComboBox::from_id_salt("language")
            .selected_text(&draft.language)
            .show_ui(ui, |ui| {
                let mut c = false;
                c |= ui
                    .selectable_value(&mut draft.language, "en".to_string(), "English")
                    .changed();
                c |= ui
                    .selectable_value(&mut draft.language, "zh".to_string(), "中文")
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
    ui.heading(RichText::new("Behavior").size(14.0));
    ui.add_space(4.0);

    if ui
        .checkbox(&mut draft.session_restore, "Restore tabs on startup")
        .changed()
    {
        changed = true;
    }

    changed
}
