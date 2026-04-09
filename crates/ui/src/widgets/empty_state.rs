use egui::{RichText, Ui};

use crate::theme::{self as t, font_size, spacing};

/// A centered placeholder shown when a list or panel has no content.
pub struct EmptyState<'a> {
    icon: &'a str,
    title: &'a str,
    subtitle: Option<&'a str>,
    action_label: Option<&'a str>,
}

impl<'a> EmptyState<'a> {
    pub fn new(icon: &'a str, title: &'a str) -> Self {
        Self {
            icon,
            title,
            subtitle: None,
            action_label: None,
        }
    }

    pub fn subtitle(mut self, s: &'a str) -> Self {
        self.subtitle = Some(s);
        self
    }

    pub fn action(mut self, label: &'a str) -> Self {
        self.action_label = Some(label);
        self
    }

    /// Returns `true` if the action button was clicked.
    pub fn show(self, ui: &mut Ui) -> bool {
        let mut clicked = false;
        ui.vertical_centered(|ui| {
            let height = ui.available_height();
            // Push down roughly 1/3 of the available height so it looks centered.
            ui.add_space((height * 0.30).max(spacing::XXL));

            ui.label(RichText::new(self.icon).size(40.0).color(t::FG_DIM()));
            ui.add_space(spacing::MD);

            ui.label(
                RichText::new(self.title)
                    .size(font_size::SUBTITLE)
                    .strong()
                    .color(t::FG()),
            );

            if let Some(sub) = self.subtitle {
                ui.add_space(spacing::XXS);
                ui.label(RichText::new(sub).size(font_size::SMALL).color(t::FG_DIM()));
            }

            if let Some(label) = self.action_label {
                ui.add_space(spacing::LG);
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(label)
                                .size(font_size::BODY)
                                .color(t::ACCENT_LIGHT()),
                        )
                        .fill(t::alpha(t::ACCENT(), 20))
                        .corner_radius(crate::theme::BUTTON_CORNER_RADIUS),
                    )
                    .clicked()
                {
                    clicked = true;
                }
            }
        });
        clicked
    }
}
