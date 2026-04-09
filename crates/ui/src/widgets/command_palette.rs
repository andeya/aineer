use egui::{Key, RichText};

use crate::theme::{self as t, font_size, radius, spacing};

#[derive(Clone)]
pub struct PaletteItem {
    pub id: String,
    pub label: String,
    pub category: String,
    pub shortcut: Option<String>,
}

pub struct CommandPalette {
    pub open: bool,
    query: String,
    selection: usize,
    items: Vec<PaletteItem>,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            open: false,
            query: String::new(),
            selection: 0,
            items: Vec::new(),
        }
    }

    pub fn set_items(&mut self, items: Vec<PaletteItem>) {
        self.items = items;
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.query.clear();
            self.selection = 0;
        }
    }

    /// Returns the selected item ID if the user confirmed a choice.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<String> {
        if !self.open {
            return None;
        }

        let mut result = None;
        let mut still_open = true;

        egui::Area::new(egui::Id::new("command_palette_overlay"))
            .fixed_pos(egui::pos2(0.0, 0.0))
            .show(ctx, |ui| {
                let screen = ui.ctx().input(|i| i.viewport_rect());
                ui.painter()
                    .rect_filled(screen, 0.0, egui::Color32::from_black_alpha(100));
            });

        let content_rect = ctx.input(|i| i.viewport_rect());
        let palette_width = (content_rect.width() * 0.5).clamp(300.0, 600.0);
        let palette_x = (content_rect.width() - palette_width) / 2.0;

        egui::Window::new("Command Palette")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .fixed_pos(egui::pos2(palette_x, content_rect.height() * 0.15))
            .fixed_size(egui::vec2(palette_width, 0.0))
            .frame(
                egui::Frame::new()
                    .fill(t::PANEL_BG())
                    .corner_radius(radius::XL)
                    .stroke(egui::Stroke::new(1.0, t::BORDER_STRONG()))
                    .shadow(egui::Shadow {
                        offset: [0, 8],
                        blur: 32,
                        spread: 4,
                        color: egui::Color32::from_black_alpha(120),
                    })
                    .inner_margin(egui::Margin::same(spacing::MD as i8)),
            )
            .show(ctx, |ui| {
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .desired_width(ui.available_width())
                        .font(egui::FontId::proportional(font_size::SUBTITLE))
                        .hint_text("Type a command...")
                        .frame(false),
                );
                resp.request_focus();

                if ui.input(|i| i.key_pressed(Key::Escape)) {
                    still_open = false;
                }

                let filtered: Vec<&PaletteItem> = self
                    .items
                    .iter()
                    .filter(|item| {
                        self.query.is_empty()
                            || item
                                .label
                                .to_lowercase()
                                .contains(&self.query.to_lowercase())
                            || item
                                .category
                                .to_lowercase()
                                .contains(&self.query.to_lowercase())
                    })
                    .take(12)
                    .collect();

                let up = ui.input(|i| i.key_pressed(Key::ArrowUp));
                let down = ui.input(|i| i.key_pressed(Key::ArrowDown));
                let enter = ui.input(|i| i.key_pressed(Key::Enter));

                if up && self.selection > 0 {
                    self.selection -= 1;
                }
                if down && self.selection + 1 < filtered.len() {
                    self.selection += 1;
                }
                self.selection = self.selection.min(filtered.len().saturating_sub(1));

                if enter {
                    if let Some(item) = filtered.get(self.selection) {
                        result = Some(item.id.clone());
                        still_open = false;
                    }
                }

                ui.add_space(spacing::SM);
                ui.separator();
                ui.add_space(spacing::XS);

                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        for (i, item) in filtered.iter().enumerate() {
                            let is_sel = i == self.selection;
                            let bg = if is_sel {
                                t::alpha(t::ACCENT(), 25)
                            } else {
                                egui::Color32::TRANSPARENT
                            };

                            let resp = egui::Frame::new()
                                .fill(bg)
                                .corner_radius(radius::MD)
                                .inner_margin(egui::Margin::symmetric(
                                    spacing::MD as i8,
                                    spacing::XS as i8,
                                ))
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(&item.category)
                                                .size(font_size::SMALL)
                                                .color(t::FG_DIM()),
                                        );
                                        ui.label(
                                            RichText::new(&item.label).size(font_size::BODY).color(
                                                if is_sel { t::ACCENT_LIGHT() } else { t::FG() },
                                            ),
                                        );
                                        if let Some(ref sc) = item.shortcut {
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(
                                                        RichText::new(sc)
                                                            .size(font_size::CAPTION)
                                                            .monospace()
                                                            .color(t::FG_MUTED()),
                                                    );
                                                },
                                            );
                                        }
                                    });
                                })
                                .response;

                            if resp.clicked() {
                                result = Some(item.id.clone());
                                still_open = false;
                            }
                            if resp.hovered() {
                                self.selection = i;
                            }
                        }

                        if filtered.is_empty() {
                            ui.add_space(spacing::XL);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new("No matching commands")
                                        .size(font_size::BODY)
                                        .color(t::FG_MUTED()),
                                );
                            });
                            ui.add_space(spacing::XL);
                        }
                    });
            });

        if !still_open {
            self.open = false;
        }
        result
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}
