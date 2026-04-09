use egui::Ui;

use crate::theme as t;

pub struct Toolbar {
    pub tabs: Vec<TabInfo>,
    pub active_tab: usize,
    pub gateway_status: GatewayStatus,
}

pub struct TabInfo {
    pub title: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GatewayStatus {
    Running,
    Error,
    Disabled,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            tabs: vec![TabInfo {
                title: "~".to_string(),
            }],
            active_tab: 0,
            gateway_status: GatewayStatus::Disabled,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            for (i, tab) in self.tabs.iter().enumerate() {
                let label = if i == self.active_tab {
                    format!("[{}]", tab.title)
                } else {
                    tab.title.clone()
                };
                if ui.selectable_label(i == self.active_tab, label).clicked() {
                    self.active_tab = i;
                }
            }

            if ui.button("+").clicked() {
                self.tabs.push(TabInfo {
                    title: "~".to_string(),
                });
                self.active_tab = self.tabs.len() - 1;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⚙").clicked() {
                    // open settings
                }

                let (color, label) = match self.gateway_status {
                    GatewayStatus::Running => (t::SUCCESS, "GW: ON"),
                    GatewayStatus::Error => (t::ERROR, "GW: ERR"),
                    GatewayStatus::Disabled => (t::FG_DIM, "GW: OFF"),
                };
                ui.colored_label(color, label);
            });
        });
    }
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}
