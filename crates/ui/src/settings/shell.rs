use egui::{RichText, Ui};

use crate::icons;
use crate::theme::{self as t, font_size, spacing};
use crate::widgets::SectionCard;

use super::SettingsDraft;

pub fn show(ui: &mut Ui, draft: &mut SettingsDraft) -> bool {
    let mut changed = false;

    SectionCard::new("Shell Configuration")
        .icon(icons::SHELL)
        .description("Configure the default shell and its arguments")
        .show(ui, |ui| {
            ui.label(
                RichText::new("Shell Path")
                    .size(font_size::BODY)
                    .color(t::FG()),
            );
            ui.add_space(spacing::XXS);
            let resp = ui.add(
                egui::TextEdit::singleline(&mut draft.shell_path)
                    .desired_width(ui.available_width())
                    .font(egui::FontId::monospace(font_size::BODY)),
            );
            changed |= resp.changed();
            ui.label(
                RichText::new("Path to the shell executable (e.g., /bin/zsh, /bin/bash)")
                    .size(font_size::CAPTION)
                    .color(t::FG_MUTED()),
            );

            ui.add_space(spacing::LG);

            ui.label(
                RichText::new("Shell Arguments")
                    .size(font_size::BODY)
                    .color(t::FG()),
            );
            ui.add_space(spacing::XXS);
            let resp = ui.add(
                egui::TextEdit::singleline(&mut draft.shell_args)
                    .desired_width(ui.available_width())
                    .font(egui::FontId::monospace(font_size::BODY))
                    .hint_text("--login"),
            );
            changed |= resp.changed();
            ui.label(
                RichText::new("Additional arguments passed to the shell on startup")
                    .size(font_size::CAPTION)
                    .color(t::FG_MUTED()),
            );
        });

    SectionCard::new("Environment Variables")
        .icon(icons::CLIPBOARD)
        .description("Custom environment variables injected into every shell session")
        .show(ui, |ui| {
            let mut remove_idx = None;
            for (i, (key, val)) in draft.env_vars.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    let key_resp = ui.add(
                        egui::TextEdit::singleline(key)
                            .desired_width(120.0)
                            .font(egui::FontId::monospace(font_size::SMALL))
                            .hint_text("KEY"),
                    );
                    ui.label(RichText::new("=").color(t::FG_DIM()));
                    let val_resp = ui.add(
                        egui::TextEdit::singleline(val)
                            .desired_width(ui.available_width() - 40.0)
                            .font(egui::FontId::monospace(font_size::SMALL))
                            .hint_text("value"),
                    );
                    changed |= key_resp.changed() || val_resp.changed();

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("✕").size(font_size::SMALL).color(t::FG_DIM()),
                            )
                            .fill(egui::Color32::TRANSPARENT),
                        )
                        .clicked()
                    {
                        remove_idx = Some(i);
                        changed = true;
                    }
                });
                ui.add_space(spacing::XXS);
            }
            if let Some(idx) = remove_idx {
                draft.env_vars.remove(idx);
            }

            ui.add_space(spacing::SM);
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("+ Add Variable")
                            .size(font_size::SMALL)
                            .color(t::ACCENT_LIGHT()),
                    )
                    .fill(t::alpha(t::ACCENT(), 15))
                    .corner_radius(crate::theme::radius::MD),
                )
                .clicked()
            {
                draft.env_vars.push((String::new(), String::new()));
                changed = true;
            }
        });

    changed
}
