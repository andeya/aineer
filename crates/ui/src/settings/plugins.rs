use egui::{RichText, Ui};

use crate::theme as t;

/// Plugins management tab — placeholder for future plugin system.
pub fn show(ui: &mut Ui) -> bool {
    ui.label(RichText::new("Plugins").strong().size(13.0).color(t::FG));
    ui.add_space(4.0);
    ui.label(
        RichText::new("Manage installed plugins and discover new ones.")
            .size(11.0)
            .color(t::FG_DIM),
    );
    ui.add_space(16.0);

    egui::Frame::new()
        .fill(t::PANEL_BG)
        .corner_radius(t::CARD_CORNER_RADIUS)
        .inner_margin(egui::Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new("No plugins installed")
                    .size(12.0)
                    .color(t::FG_MUTED),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "The plugin system is coming soon. Plugins will extend Aineer with \
                     custom tools, model providers, and UI panels.",
                )
                .size(11.0)
                .color(t::FG_DIM),
            );
        });

    ui.add_space(16.0);

    ui.horizontal(|ui| {
        let btn = egui::Button::new(
            RichText::new("Browse Plugin Registry")
                .size(12.0)
                .color(t::FG_DIM),
        )
        .fill(t::PANEL_BG);
        if ui.add(btn).on_hover_text("Coming soon").clicked() {
            // No-op for now
        }
    });

    false
}
