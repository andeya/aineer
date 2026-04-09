use egui::{RichText, Ui};

use crate::icons;
use crate::theme::{self as t, font_size, radius, spacing};

use super::SettingsDraft;

pub fn show(
    ui: &mut Ui,
    dirty: &mut bool,
    status_msg: &mut Option<(String, bool)>,
    draft: &mut SettingsDraft,
) {
    egui::Frame::new()
        .fill(t::PANEL_BG_ALT())
        .corner_radius(radius::LG)
        .inner_margin(egui::Margin::symmetric(
            spacing::LG as i8,
            spacing::MD as i8,
        ))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                let save_fill = if *dirty {
                    t::blend(t::ACCENT(), t::SUCCESS(), 0.3)
                } else {
                    t::BUTTON_BG()
                };

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(format!("{} Save", icons::SAVE))
                                .size(font_size::BODY)
                                .color(if *dirty { t::FG() } else { t::FG_DIM() }),
                        )
                        .fill(save_fill)
                        .corner_radius(radius::MD),
                    )
                    .clicked()
                    && *dirty
                {
                    match super::save_settings(draft) {
                        Ok(()) => {
                            *dirty = false;
                            *status_msg = Some(("Settings saved ✓".to_string(), true));
                            let mode = crate::theme::ThemeMode::parse(&draft.theme);
                            crate::theme::apply(ui.ctx(), mode);
                        }
                        Err(e) => {
                            *status_msg = Some((format!("Save failed: {e}"), false));
                        }
                    }
                }

                ui.add_space(spacing::XS);

                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(format!("{} Reset", icons::RESET))
                                .size(font_size::BODY)
                                .color(t::FG_SOFT()),
                        )
                        .fill(t::BUTTON_BG())
                        .corner_radius(radius::MD),
                    )
                    .clicked()
                {
                    *draft = SettingsDraft::default();
                    *dirty = true;
                    *status_msg = Some(("Reset to defaults".to_string(), true));
                }

                ui.add_space(spacing::MD);

                if let Some((msg, is_ok)) = status_msg {
                    let color = if *is_ok { t::SUCCESS() } else { t::ERROR() };
                    ui.label(
                        RichText::new(msg.as_str())
                            .size(font_size::SMALL)
                            .color(color),
                    );
                }

                if *dirty {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{} unsaved changes", icons::UNSAVED))
                                .size(font_size::CAPTION)
                                .color(t::WARNING()),
                        );
                    });
                }
            });
        });
}
