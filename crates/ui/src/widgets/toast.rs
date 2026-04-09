use std::time::{Duration, Instant};

use egui::{Color32, Context, RichText};

use crate::theme::{self as t, font_size, radius, spacing};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

impl ToastKind {
    fn icon(&self) -> &'static str {
        match self {
            Self::Info => "ℹ",
            Self::Success => "✓",
            Self::Warning => "⚠",
            Self::Error => "✗",
        }
    }

    fn colors(&self) -> (Color32, Color32, Color32) {
        match self {
            Self::Info => (t::PANEL_BG(), t::ACCENT(), t::FG()),
            Self::Success => (t::PANEL_BG(), t::SUCCESS(), t::FG()),
            Self::Warning => (t::PANEL_BG(), t::AMBER(), t::FG()),
            Self::Error => (t::PANEL_BG(), t::ERROR(), t::FG()),
        }
    }
}

struct Toast {
    kind: ToastKind,
    message: String,
    created: Instant,
    duration: Duration,
}

impl Toast {
    fn is_expired(&self) -> bool {
        self.created.elapsed() > self.duration
    }

    fn opacity(&self) -> f32 {
        let elapsed = self.created.elapsed().as_secs_f32();
        let total = self.duration.as_secs_f32();
        // Fade in first 0.15 s, hold, then fade out last 0.4 s
        let fade_in = (elapsed / 0.15).min(1.0);
        let fade_out = ((total - elapsed) / 0.4).clamp(0.0, 1.0);
        fade_in.min(fade_out)
    }
}

/// Global toast notification queue.  Call `ToastManager::show` each frame
/// in `eframe::App::update` to render all active toasts.
#[derive(Default)]
pub struct ToastManager {
    toasts: Vec<Toast>,
}

impl ToastManager {
    pub fn push(&mut self, kind: ToastKind, message: impl Into<String>) {
        self.push_with_duration(kind, message, Duration::from_secs(3));
    }

    pub fn push_with_duration(
        &mut self,
        kind: ToastKind,
        message: impl Into<String>,
        duration: Duration,
    ) {
        self.toasts.push(Toast {
            kind,
            message: message.into(),
            created: Instant::now(),
            duration,
        });
    }

    pub fn info(&mut self, msg: impl Into<String>) {
        self.push(ToastKind::Info, msg);
    }
    pub fn success(&mut self, msg: impl Into<String>) {
        self.push(ToastKind::Success, msg);
    }
    pub fn warning(&mut self, msg: impl Into<String>) {
        self.push(ToastKind::Warning, msg);
    }
    pub fn error(&mut self, msg: impl Into<String>) {
        self.push(ToastKind::Error, msg);
    }

    /// Render all active toasts in the bottom-right corner of the screen.
    /// Must be called every frame inside `eframe::App::update`.
    pub fn show(&mut self, ctx: &Context) {
        self.toasts.retain(|t| !t.is_expired());

        if self.toasts.is_empty() {
            return;
        }

        // Request continuous repaint while toasts are animating
        ctx.request_repaint_after(std::time::Duration::from_millis(50));

        let screen = ctx
            .input(|i| i.viewport().inner_rect)
            .unwrap_or(egui::Rect::from_min_max(
                egui::pos2(0.0, 0.0),
                egui::pos2(800.0, 600.0),
            ));
        let margin = 16.0_f32;
        let toast_width = 320.0_f32;
        let toast_height = 52.0_f32;
        let gap = 8.0_f32;

        let n = self.toasts.len();
        for (i, toast) in self.toasts.iter().enumerate().rev() {
            let opacity = toast.opacity();
            if opacity <= 0.0 {
                continue;
            }

            let idx_from_bottom = n - 1 - i;
            let y = screen.max.y
                - margin
                - (idx_from_bottom as f32) * (toast_height + gap)
                - toast_height;
            let x = screen.max.x - margin - toast_width;

            let (bg, accent, fg) = toast.kind.colors();
            let bg = t::alpha_color(bg, (opacity * 240.0) as u8);
            let accent_alpha = t::alpha_color(accent, (opacity * 255.0) as u8);
            let fg_alpha = t::alpha_color(fg, (opacity * 230.0) as u8);

            egui::Area::new(egui::Id::new(("toast_area", i)))
                .fixed_pos(egui::pos2(x, y))
                .order(egui::Order::Tooltip)
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(bg)
                        .corner_radius(radius::LG)
                        .stroke(egui::Stroke::new(1.0, t::BORDER_SUBTLE()))
                        .inner_margin(egui::Margin::symmetric(
                            spacing::MD as i8,
                            spacing::SM as i8,
                        ))
                        .show(ui, |ui| {
                            ui.set_min_width(toast_width - spacing::MD * 2.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(toast.kind.icon())
                                        .size(font_size::SUBTITLE)
                                        .color(accent_alpha),
                                );
                                ui.add_space(spacing::XS);
                                ui.label(
                                    RichText::new(toast.message.as_str())
                                        .size(font_size::BODY)
                                        .color(fg_alpha),
                                );
                            });
                        });
                });
        }
    }
}
