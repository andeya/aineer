use egui::{Color32, RichText, Ui};

use crate::theme::{self as t, font_size, radius, spacing};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BadgeKind {
    Running,
    Success,
    Error,
    Warning,
    Info,
    Neutral,
}

impl BadgeKind {
    fn colors(&self) -> (Color32, Color32) {
        match self {
            Self::Running => (t::alpha(t::ACCENT(), 30), t::ACCENT_LIGHT()),
            Self::Success => (t::alpha(t::SUCCESS(), 25), t::SUCCESS()),
            Self::Error => (t::alpha(t::ERROR(), 25), t::ERROR()),
            Self::Warning => (t::alpha(t::AMBER(), 25), t::AMBER()),
            Self::Info => (t::alpha(t::ACCENT(), 20), t::FG()),
            Self::Neutral => (t::alpha(t::FG_MUTED(), 15), t::FG_MUTED()),
        }
    }
}

/// A small coloured pill displaying a status label, optionally with a dot indicator.
pub struct StatusBadge<'a> {
    kind: BadgeKind,
    label: &'a str,
    dot: bool,
}

impl<'a> StatusBadge<'a> {
    pub fn new(kind: BadgeKind, label: &'a str) -> Self {
        Self {
            kind,
            label,
            dot: false,
        }
    }

    pub fn with_dot(mut self) -> Self {
        self.dot = true;
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let (bg, fg) = self.kind.colors();
        egui::Frame::new()
            .fill(bg)
            .corner_radius(radius::XL)
            .inner_margin(egui::Margin::symmetric(spacing::XS as i8, 2))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    if self.dot {
                        ui.label(RichText::new("●").size(7.0).color(fg));
                    }
                    ui.label(RichText::new(self.label).size(font_size::CAPTION).color(fg));
                });
            });
    }
}
