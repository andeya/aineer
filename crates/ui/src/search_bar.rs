use egui::{Key, RichText, Ui, Vec2};

use crate::theme as t;

#[derive(Debug, Clone)]
pub enum SearchAction {
    None,
    QueryChanged(String),
    FindNext,
    FindPrev,
    Close,
}

pub struct SearchBar {
    pub open: bool,
    pub query: String,
    pub regex_valid: bool,
    request_focus: bool,
}

impl SearchBar {
    pub fn new() -> Self {
        Self {
            open: false,
            query: String::new(),
            regex_valid: true,
            request_focus: false,
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.request_focus = true;
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
    }

    pub fn show(&mut self, ui: &mut Ui) -> SearchAction {
        if !self.open {
            return SearchAction::None;
        }

        let mut action = SearchAction::None;

        egui::Frame::new()
            .fill(t::PANEL_BG)
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.set_min_width(ui.available_width());

                    ui.label(RichText::new("Find:").size(12.0).color(t::FG_SOFT));

                    let text_edit = egui::TextEdit::singleline(&mut self.query)
                        .desired_width((ui.available_width() - 140.0).max(80.0))
                        .font(egui::TextStyle::Monospace)
                        .text_color(if self.regex_valid { t::FG } else { t::ERROR });
                    let response = ui.add(text_edit);

                    if self.request_focus {
                        response.request_focus();
                        self.request_focus = false;
                    }

                    if response.changed() {
                        action = SearchAction::QueryChanged(self.query.clone());
                    }

                    // Handle Enter/Shift+Enter in the text field
                    if response.has_focus() {
                        let shift = ui.input(|i| i.modifiers.shift);
                        if ui.input(|i| i.key_pressed(Key::Enter)) {
                            action = if shift {
                                SearchAction::FindPrev
                            } else {
                                SearchAction::FindNext
                            };
                        }
                        if ui.input(|i| i.key_pressed(Key::Escape)) {
                            action = SearchAction::Close;
                        }
                    }

                    let btn_size = Vec2::new(24.0, 20.0);

                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(RichText::new("\u{25B2}").size(11.0)),
                        )
                        .on_hover_text("Previous (Shift+Enter)")
                        .clicked()
                    {
                        action = SearchAction::FindPrev;
                    }

                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(RichText::new("\u{25BC}").size(11.0)),
                        )
                        .on_hover_text("Next (Enter)")
                        .clicked()
                    {
                        action = SearchAction::FindNext;
                    }

                    if ui
                        .add_sized(
                            btn_size,
                            egui::Button::new(RichText::new("\u{2715}").size(11.0)),
                        )
                        .on_hover_text("Close (Esc)")
                        .clicked()
                    {
                        action = SearchAction::Close;
                    }
                });
            });

        action
    }
}

impl Default for SearchBar {
    fn default() -> Self {
        Self::new()
    }
}
