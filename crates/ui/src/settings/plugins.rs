use egui::{RichText, Ui};

use crate::icons;
use crate::theme::{self as t, font_size, radius, spacing};
use crate::widgets::SectionCard;

pub fn show(ui: &mut Ui) -> bool {
    SectionCard::new("Installed Plugins")
        .icon(icons::PLUGIN)
        .description("Manage plugins that extend Aineer's capabilities")
        .show(ui, |ui| {
            ui.add_space(spacing::XL);
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(icons::PACKAGE)
                        .size(32.0)
                        .color(t::FG_MUTED()),
                );
                ui.add_space(spacing::MD);
                ui.label(
                    RichText::new("No plugins installed")
                        .size(font_size::SUBTITLE)
                        .color(t::FG_DIM()),
                );
                ui.add_space(spacing::XS);
                ui.label(
                    RichText::new(
                        "The plugin system is coming soon.\n\
                         Plugins will extend Aineer with custom tools,\n\
                         model providers, and UI panels.",
                    )
                    .size(font_size::SMALL)
                    .color(t::FG_MUTED()),
                );
                ui.add_space(spacing::XL);
                ui.add(
                    egui::Button::new(
                        RichText::new("Browse Plugin Registry")
                            .size(font_size::BODY)
                            .color(t::FG_DIM()),
                    )
                    .fill(t::PANEL_BG_ALT())
                    .corner_radius(radius::LG),
                )
                .on_hover_text("Coming soon");
            });
            ui.add_space(spacing::XL);
        });

    false
}
