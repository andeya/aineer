use egui::{RichText, Ui};

use crate::icons;
use crate::theme::{self as t, font_size, radius, spacing};
use crate::widgets::{SectionCard, SettingsRow};

use super::SettingsDraft;

pub fn show(ui: &mut Ui, draft: &mut SettingsDraft) -> bool {
    let mut changed = false;

    SectionCard::new("Embedded Gateway")
        .icon(icons::GATEWAY)
        .description("Local OpenAI-compatible API gateway for model routing")
        .show(ui, |ui| {
            changed |= SettingsRow::new("Enable Gateway")
                .description("Start the gateway server automatically on launch")
                .show(ui, |ui| {
                    ui.checkbox(&mut draft.gateway_enabled, "").changed()
                });

            ui.add_space(spacing::SM);
            ui.label(
                RichText::new("Listen Address")
                    .size(font_size::BODY)
                    .color(t::FG()),
            );
            ui.add_space(spacing::XXS);
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut draft.gateway_addr)
                        .desired_width(200.0)
                        .font(egui::FontId::monospace(font_size::BODY)),
                )
                .changed();
            ui.label(
                RichText::new("IP address and port for the gateway (e.g., 127.0.0.1:8090)")
                    .size(font_size::CAPTION)
                    .color(t::FG_MUTED()),
            );
        });

    SectionCard::new("Providers")
        .icon(icons::KEY)
        .description("API keys are configured via environment variables or `aineer login`")
        .show(ui, |ui| {
            let providers = [
                ("Anthropic (Claude)", "ANTHROPIC_API_KEY", "aineer login"),
                ("OpenAI", "OPENAI_API_KEY", ""),
                ("xAI (Grok)", "XAI_API_KEY", ""),
                ("Ollama", "OLLAMA_HOST", "http://localhost:11434"),
                ("OpenRouter", "OPENROUTER_API_KEY", ""),
            ];

            for (name, env_key, hint) in &providers {
                let has_key = std::env::var(env_key).is_ok();
                egui::Frame::new()
                    .fill(if has_key {
                        t::alpha(t::SUCCESS(), 8)
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .corner_radius(radius::SM)
                    .inner_margin(egui::Margin::symmetric(
                        spacing::MD as i8,
                        spacing::XS as i8,
                    ))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let (dot, dot_color) = if has_key {
                                ("●", t::SUCCESS())
                            } else {
                                ("○", t::FG_DIM())
                            };
                            ui.label(RichText::new(dot).size(font_size::SMALL).color(dot_color));
                            ui.label(
                                RichText::new(*name)
                                    .size(font_size::BODY)
                                    .color(if has_key { t::FG() } else { t::FG_SOFT() }),
                            );

                            if has_key {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new("Configured")
                                                .size(font_size::CAPTION)
                                                .color(t::SUCCESS()),
                                        );
                                    },
                                );
                            } else if !hint.is_empty() {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(*hint)
                                                .size(font_size::CAPTION)
                                                .color(t::FG_MUTED()),
                                        );
                                    },
                                );
                            }
                        });
                    });
                ui.add_space(spacing::XXS);
            }

            ui.add_space(spacing::SM);
            ui.label(
                RichText::new("Use `aineer --cli login` to configure API keys securely.")
                    .size(font_size::SMALL)
                    .color(t::FG_DIM()),
            );
        });

    changed
}
