use egui::{RichText, Ui};

use super::SettingsDraft;
use crate::theme as t;

/// Raw JSON settings editor. Allows editing all settings as a single JSON document.
pub fn show(ui: &mut Ui, json_raw: &mut String, draft: &mut SettingsDraft) -> bool {
    let mut changed = false;

    ui.label(
        RichText::new("Raw JSON Settings")
            .strong()
            .size(13.0)
            .color(t::FG),
    );
    ui.add_space(4.0);
    ui.label(
        RichText::new("Edit the raw JSON and click Apply to update settings.")
            .size(11.0)
            .color(t::FG_DIM),
    );
    ui.add_space(8.0);

    let text_edit = egui::TextEdit::multiline(json_raw)
        .font(egui::TextStyle::Monospace)
        .desired_width(ui.available_width())
        .desired_rows(20)
        .code_editor();
    ui.add(text_edit);

    ui.add_space(8.0);

    ui.horizontal(|ui| {
        if ui
            .add(
                egui::Button::new(RichText::new("Apply JSON").size(12.0).color(t::FG))
                    .fill(t::ACCENT),
            )
            .clicked()
        {
            match serde_json::from_str::<SettingsDraft>(json_raw) {
                Ok(new_draft) => {
                    *draft = new_draft;
                    changed = true;
                }
                Err(e) => {
                    // Show parse error inline
                    ui.label(
                        RichText::new(format!("Parse error: {e}"))
                            .size(11.0)
                            .color(t::ERROR),
                    );
                }
            }
        }

        if ui
            .add(
                egui::Button::new(RichText::new("Reformat").size(12.0).color(t::FG_SOFT))
                    .fill(t::PANEL_BG),
            )
            .clicked()
        {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_raw) {
                if let Ok(pretty) = serde_json::to_string_pretty(&val) {
                    *json_raw = pretty;
                }
            }
        }
    });

    changed
}
