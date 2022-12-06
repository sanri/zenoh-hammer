use egui::{Align, Layout, ScrollArea};
use serde_json;
use zenoh::config::{Config, WhatAmI};

pub struct PageSession {
    zenoh_config: Config,
    config_json: String,
    connected: bool,
}

impl Default for PageSession {
    fn default() -> Self {
        let config = Config::default();
        let json: serde_json::Value = serde_json::to_value(&config).unwrap();
        PageSession {
            zenoh_config: config,
            config_json: format!("{:#}", json),
            connected: false,
        }
    }
}

impl PageSession {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.vertical(|ui| {
                if self.connected {
                    if ui.button("断开连接").clicked() {
                        self.connected = false;
                    }
                } else {
                    if ui.button("建立连接").clicked() {
                        self.connected = true;
                    }
                }

                ScrollArea::vertical().id_source("1").show(ui, |ui| {
                    self.show_config_edit(ui);
                });
            });

            ui.separator();

            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                ui.horizontal(|ui| {
                    if ui.button("标准格式").clicked() {
                        let json: serde_json::Value =
                            serde_json::to_value(&self.zenoh_config).unwrap();
                        self.config_json = format!("{:#}", json);
                    };
                    if ui.button("紧凑格式").clicked() {
                        let json: serde_json::Value =
                            serde_json::to_value(&self.zenoh_config).unwrap();
                        self.config_json = format!("{}", json);
                    };
                });

                ScrollArea::vertical().id_source("3").show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.config_json)
                            .desired_width(f32::INFINITY)
                            .code_editor(),
                    )
                });
            });
        });
    }

    fn show_config_edit(&mut self, ui: &mut egui::Ui) {
        let mut show_set_mode_ui = |ui: &mut egui::Ui| {
            ui.label("mode").on_hover_text("节点模式");
            let mode_select_str = match self.zenoh_config.mode() {
                None => "",
                Some(m) => m.to_str(),
            };
            egui::ComboBox::new("mode", "")
                .selected_text(mode_select_str)
                .show_ui(ui, |ui| {
                    let what_am_i = [WhatAmI::Client, WhatAmI::Peer];
                    for wai in what_am_i {
                        if ui
                            .selectable_label(self.zenoh_config.mode() == &Some(wai), wai.to_str())
                            .clicked()
                        {
                            let _ = self.zenoh_config.set_mode(Some(wai));
                        }
                    }
                });
            ui.end_row();
        };

        egui::Grid::new("config_grid")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                show_set_mode_ui(ui);
            });
    }
}
