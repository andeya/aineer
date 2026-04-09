use egui::{Color32, Response, RichText, Ui};

use crate::theme::{self as t, font_size, radius, spacing};

/// A square button containing only an icon glyph, with hover highlight and tooltip.
pub struct IconButton<'a> {
    icon: &'a str,
    tooltip: Option<&'a str>,
    active: bool,
    size: f32,
    fg: Option<Color32>,
}

impl<'a> IconButton<'a> {
    pub fn new(icon: &'a str) -> Self {
        Self {
            icon,
            tooltip: None,
            active: false,
            size: font_size::TITLE,
            fg: None,
        }
    }

    pub fn tooltip(mut self, tip: &'a str) -> Self {
        self.tooltip = Some(tip);
        self
    }

    pub fn active(mut self, a: bool) -> Self {
        self.active = a;
        self
    }

    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }

    pub fn color(mut self, c: Color32) -> Self {
        self.fg = Some(c);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Response {
        let fg = self.fg.unwrap_or_else(|| {
            if self.active {
                t::ACCENT_LIGHT()
            } else {
                t::FG_DIM()
            }
        });

        let resp = egui::Frame::new()
            .fill(if self.active {
                t::alpha(t::ACCENT(), 20)
            } else {
                Color32::TRANSPARENT
            })
            .corner_radius(radius::MD)
            .inner_margin(egui::Margin::same(spacing::XS as i8))
            .show(ui, |ui| {
                ui.set_min_width(24.0);
                ui.set_min_height(24.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(self.icon).size(self.size).color(fg));
                });
            })
            .response;

        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        if let Some(tip) = self.tooltip {
            resp.on_hover_text(tip)
        } else {
            resp
        }
    }
}
