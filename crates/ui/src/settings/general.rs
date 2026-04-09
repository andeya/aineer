use egui::Ui;

use crate::icons;
use crate::theme as t;
use crate::widgets::{SectionCard, SettingsRow};

use super::SettingsDraft;

pub fn show(ui: &mut Ui, draft: &mut SettingsDraft) -> bool {
    let mut changed = false;

    SectionCard::new("Appearance")
        .icon(icons::APPEARANCE)
        .description("Customize the look and feel of Aineer")
        .show(ui, |ui| {
            changed |= SettingsRow::new("Theme")
                .description("Switch between dark and light mode")
                .show(ui, |ui| {
                    let before = draft.theme.clone();
                    egui::ComboBox::from_id_salt("theme_sel")
                        .selected_text(match draft.theme.as_str() {
                            "light" => format!("{} Light", icons::THEME_LIGHT),
                            _ => format!("{} Dark", icons::THEME_DARK),
                        })
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut draft.theme,
                                "dark".to_string(),
                                format!("{} Dark", icons::THEME_DARK),
                            );
                            ui.selectable_value(
                                &mut draft.theme,
                                "light".to_string(),
                                format!("{} Light", icons::THEME_LIGHT),
                            );
                        });
                    let c = draft.theme != before;
                    if c {
                        let mode = t::ThemeMode::parse(&draft.theme);
                        t::apply(ui.ctx(), mode);
                    }
                    c
                });

            changed |= SettingsRow::new("Font Size")
                .description("UI font size in pixels")
                .show(ui, |ui| {
                    ui.add(
                        egui::Slider::new(&mut draft.font_size, 10.0..=24.0)
                            .step_by(1.0)
                            .suffix(" px"),
                    )
                    .changed()
                });

            changed |= SettingsRow::new("Language")
                .description("Display language for the interface")
                .show(ui, |ui| {
                    let before = draft.language.clone();
                    egui::ComboBox::from_id_salt("lang_sel")
                        .selected_text(match draft.language.as_str() {
                            "zh" => "中文",
                            _ => "English",
                        })
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut draft.language, "en".to_string(), "English");
                            ui.selectable_value(&mut draft.language, "zh".to_string(), "中文");
                        });
                    draft.language != before
                });
        });

    SectionCard::new("Behavior")
        .icon(icons::BEHAVIOR)
        .description("General application behavior")
        .show(ui, |ui| {
            changed |= SettingsRow::new("Session Restore")
                .description("Automatically restore tabs and their content on startup")
                .show(ui, |ui| {
                    ui.checkbox(&mut draft.session_restore, "").changed()
                });
        });

    changed
}
