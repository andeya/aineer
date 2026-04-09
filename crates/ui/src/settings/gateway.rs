use egui::{RichText, Ui};

use crate::theme as t;

use super::SettingsDraft;

pub fn show(ui: &mut Ui, draft: &mut SettingsDraft) -> bool {
    let mut changed = false;

    ui.heading(RichText::new("Embedded Gateway").size(14.0));
    ui.add_space(4.0);

    if ui
        .checkbox(&mut draft.gateway_enabled, "Enable gateway on startup")
        .changed()
    {
        changed = true;
    }

    ui.horizontal(|ui| {
        ui.label("Listen address");
        if ui.text_edit_singleline(&mut draft.gateway_addr).changed() {
            changed = true;
        }
    });

    ui.add_space(12.0);
    ui.heading(RichText::new("Providers").size(14.0));
    ui.add_space(4.0);

    ui.label(
        RichText::new("Providers are configured through environment variables or aineer login.")
            .size(11.0)
            .color(t::FG_DIM),
    );

    ui.add_space(8.0);

    let providers = [
        ("Anthropic (Claude)", "ANTHROPIC_API_KEY", "aineer login"),
        ("OpenAI", "OPENAI_API_KEY", ""),
        ("xAI (Grok)", "XAI_API_KEY", ""),
        ("Ollama", "OLLAMA_HOST", "http://localhost:11434"),
        ("OpenRouter", "OPENROUTER_API_KEY", ""),
    ];

    for (name, env_key, hint) in &providers {
        ui.horizontal(|ui| {
            let has_key = std::env::var(env_key).is_ok();
            let indicator = if has_key {
                RichText::new("●").color(t::SUCCESS).size(10.0)
            } else {
                RichText::new("○").color(t::FG_DIM).size(10.0)
            };
            ui.label(indicator);
            ui.label(RichText::new(*name).size(12.0));

            if !has_key && !hint.is_empty() {
                ui.label(
                    RichText::new(format!("({hint})"))
                        .size(10.0)
                        .color(t::FG_DIM),
                );
            }
        });
    }

    ui.add_space(8.0);
    ui.label(
        RichText::new("Use `aineer --cli login` to configure API keys securely.")
            .size(11.0)
            .color(t::FG_DIM),
    );

    changed
}
