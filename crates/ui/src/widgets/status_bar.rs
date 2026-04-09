use egui::{Color32, RichText, Ui};

use crate::theme::{self as t, font_size, spacing};

pub struct StatusSegment {
    pub text: String,
    pub color: Color32,
    pub tooltip: Option<String>,
    pub url: Option<String>,
}

impl StatusSegment {
    pub fn new(text: impl Into<String>, color: Color32) -> Self {
        Self {
            text: text.into(),
            color,
            tooltip: None,
            url: None,
        }
    }

    pub fn tooltip(mut self, tip: impl Into<String>) -> Self {
        self.tooltip = Some(tip.into());
        self
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
}

pub struct StatusBar {
    left: Vec<StatusSegment>,
    right: Vec<StatusSegment>,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            left: Vec::new(),
            right: Vec::new(),
        }
    }

    pub fn left(mut self, segment: StatusSegment) -> Self {
        self.left.push(segment);
        self
    }

    pub fn right(mut self, segment: StatusSegment) -> Self {
        self.right.push(segment);
        self
    }

    pub fn show(self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.set_min_height(t::STATUS_BAR_HEIGHT);
            ui.spacing_mut().item_spacing.x = 0.0;

            for seg in &self.left {
                show_segment(ui, seg);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                for seg in self.right.iter().rev() {
                    show_segment(ui, seg);
                }
            });
        });
    }
}

fn show_segment(ui: &mut Ui, seg: &StatusSegment) {
    let pad = egui::Margin::symmetric(spacing::MD as i8, spacing::XXS as i8);

    let resp = egui::Frame::new()
        .inner_margin(pad)
        .show(ui, |ui| {
            ui.label(
                RichText::new(&seg.text)
                    .size(font_size::SMALL)
                    .color(seg.color),
            )
        })
        .response;

    let resp = if let Some(ref tip) = seg.tooltip {
        resp.on_hover_text(tip)
    } else {
        resp
    };

    if seg.url.is_some() {
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        if resp.clicked() {
            if let Some(ref url) = seg.url {
                ui.ctx().open_url(egui::OpenUrl::new_tab(url));
            }
        }
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}
