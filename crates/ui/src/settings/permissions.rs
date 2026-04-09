use egui::{RichText, Ui};

use crate::icons;
use crate::theme::{self as t, font_size, radius, spacing};
use crate::widgets::{SectionCard, SettingsRow};

use super::SettingsDraft;

pub fn show(ui: &mut Ui, draft: &mut SettingsDraft) -> bool {
    let mut changed = false;

    SectionCard::new("Default Mode")
        .icon(icons::PERMISSIONS)
        .description("How Aineer handles tool execution permissions by default")
        .show(ui, |ui| {
            changed |= SettingsRow::new("Permission Mode")
                .description("Controls whether tools require approval before executing")
                .show(ui, |ui| {
                    let before = draft.default_permission_mode.clone();
                    egui::ComboBox::from_id_salt("perm_mode_sel")
                        .selected_text(match draft.default_permission_mode.as_str() {
                            "auto" => "Auto",
                            "yolo" => "YOLO",
                            _ => "Ask",
                        })
                        .width(180.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut draft.default_permission_mode,
                                "ask".to_string(),
                                "Ask — confirm each tool",
                            );
                            ui.selectable_value(
                                &mut draft.default_permission_mode,
                                "auto".to_string(),
                                "Auto — allow safe operations",
                            );
                            ui.selectable_value(
                                &mut draft.default_permission_mode,
                                "yolo".to_string(),
                                "YOLO — allow everything ⚠",
                            );
                        });
                    draft.default_permission_mode != before
                });
        });

    SectionCard::new("Custom Rules")
        .icon(icons::RULER)
        .description("Fine-grained pattern-based rules for tool execution")
        .show(ui, |ui| {
            let mut remove_idx = None;
            for (i, (pattern, decision)) in draft.permission_rules.iter_mut().enumerate() {
                egui::Frame::new()
                    .fill(t::PANEL_BG_ALT())
                    .corner_radius(radius::SM)
                    .inner_margin(egui::Margin::symmetric(
                        spacing::MD as i8,
                        spacing::XS as i8,
                    ))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            changed |= ui
                                .add(
                                    egui::TextEdit::singleline(pattern)
                                        .desired_width(140.0)
                                        .font(egui::FontId::monospace(font_size::SMALL))
                                        .hint_text("Pattern"),
                                )
                                .changed();
                            ui.label(RichText::new("→").size(font_size::BODY).color(t::FG_DIM()));
                            let before = decision.clone();
                            egui::ComboBox::from_id_salt(format!("rule_sel_{i}"))
                                .selected_text(decision.as_str())
                                .width(80.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(decision, "allow".to_string(), "Allow");
                                    ui.selectable_value(decision, "deny".to_string(), "Deny");
                                    ui.selectable_value(decision, "ask".to_string(), "Ask");
                                });
                            changed |= *decision != before;

                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("✕")
                                            .size(font_size::SMALL)
                                            .color(t::FG_DIM()),
                                    )
                                    .fill(egui::Color32::TRANSPARENT),
                                )
                                .clicked()
                            {
                                remove_idx = Some(i);
                                changed = true;
                            }
                        });
                    });
                ui.add_space(spacing::XXS);
            }
            if let Some(idx) = remove_idx {
                draft.permission_rules.remove(idx);
            }

            ui.add_space(spacing::SM);
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("+ Add Rule")
                            .size(font_size::SMALL)
                            .color(t::ACCENT_LIGHT()),
                    )
                    .fill(t::alpha(t::ACCENT(), 15))
                    .corner_radius(radius::MD),
                )
                .clicked()
            {
                draft
                    .permission_rules
                    .push(("*".to_string(), "ask".to_string()));
                changed = true;
            }
        });

    changed
}
