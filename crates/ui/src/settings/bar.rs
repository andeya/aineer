use egui::{RichText, Ui};

use crate::theme as t;

use super::SettingsDraft;

pub fn show(
    ui: &mut Ui,
    dirty: &mut bool,
    status_msg: &mut Option<(String, bool)>,
    draft: &mut SettingsDraft,
) {
    ui.horizontal(|ui| {
        let save_btn = egui::Button::new(RichText::new("Save").size(12.0)).fill(if *dirty {
            t::blend(t::BUTTON_BG, t::SUCCESS, 0.4)
        } else {
            t::BUTTON_BG
        });

        if ui.add(save_btn).clicked() && *dirty {
            match super::save_settings(draft) {
                Ok(()) => {
                    *dirty = false;
                    *status_msg = Some(("Settings saved".to_string(), true));
                }
                Err(e) => {
                    *status_msg = Some((format!("Save failed: {e}"), false));
                }
            }
        }

        if ui
            .add(egui::Button::new(RichText::new("Reset").size(12.0)))
            .clicked()
        {
            *draft = SettingsDraft::default();
            *dirty = true;
            *status_msg = Some(("Reset to defaults".to_string(), true));
        }

        if let Some((msg, is_ok)) = status_msg {
            let color = if *is_ok { t::SUCCESS } else { t::ERROR };
            ui.label(RichText::new(msg.as_str()).size(11.0).color(color));
        }

        if *dirty {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new("● unsaved changes")
                        .size(10.0)
                        .color(t::WARNING),
                );
            });
        }
    });
}
