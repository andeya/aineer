use egui::{RichText, Ui};

use crate::icons;
use crate::theme::{self as t, font_size, spacing};
use crate::widgets::{SectionCard, SettingsRow};

use super::SettingsDraft;

pub fn show(ui: &mut Ui, draft: &mut SettingsDraft) -> bool {
    let mut changed = false;

    SectionCard::new("Default Model")
        .icon(icons::MODEL)
        .description("Choose which AI model to use for conversations")
        .show(ui, |ui| {
            ui.label(
                RichText::new("Model Identifier")
                    .size(font_size::BODY)
                    .color(t::FG()),
            );
            ui.add_space(spacing::XXS);
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut draft.default_model)
                        .desired_width(ui.available_width())
                        .font(egui::FontId::monospace(font_size::BODY)),
                )
                .changed();
            ui.label(
                RichText::new(
                    "Use \"auto\" for automatic detection, or specify: \
                     claude-sonnet-4-6, gpt-4o, ollama/qwen2.5-coder",
                )
                .size(font_size::CAPTION)
                .color(t::FG_MUTED()),
            );
        });

    SectionCard::new("Fallback Chain")
        .icon(icons::LINK)
        .description("Models to try when the primary model fails")
        .show(ui, |ui| {
            ui.label(
                RichText::new("Fallback Models")
                    .size(font_size::BODY)
                    .color(t::FG()),
            );
            ui.add_space(spacing::XXS);
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut draft.fallback_models)
                        .desired_width(ui.available_width())
                        .font(egui::FontId::monospace(font_size::BODY))
                        .hint_text("gpt-4o, claude-sonnet-4-6"),
                )
                .changed();
            ui.label(
                RichText::new("Comma-separated list of fallback model identifiers")
                    .size(font_size::CAPTION)
                    .color(t::FG_MUTED()),
            );
        });

    SectionCard::new("Thinking Mode")
        .icon(icons::THINKING)
        .description("Extended reasoning capabilities")
        .show(ui, |ui| {
            changed |= SettingsRow::new("Extended Thinking")
                .description(
                    "Enable step-by-step reasoning for complex tasks (when model supports it)",
                )
                .show(ui, |ui| ui.checkbox(&mut draft.thinking_mode, "").changed());
        });

    changed
}
