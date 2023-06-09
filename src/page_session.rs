use eframe::{
    egui,
    egui::{
        Align, CollapsingHeader, Color32, Context, Id, Layout, RichText, ScrollArea, TextEdit,
        TextStyle,
    },
};
use serde_json;
use std::{collections::VecDeque, str::FromStr};
use zenoh::config::{Config, EndPoint, WhatAmI};

pub enum Event {
    Connect(Box<Config>),
    Disconnect,
}

enum AEWKind {
    Connect,
    Listen,
}

struct AddEndpointWindow {
    kind: AEWKind,
    edit_str: String,
    err_str: String,
}

impl Default for AddEndpointWindow {
    fn default() -> Self {
        AddEndpointWindow {
            kind: AEWKind::Connect,
            edit_str: String::new(),
            err_str: String::new(),
        }
    }
}

impl AddEndpointWindow {
    fn show(&mut self, ctx: &Context, is_open: &mut bool, ve: &mut Vec<EndPoint>) {
        let window_name = match self.kind {
            AEWKind::Connect => "Connect",
            AEWKind::Listen => "Listen",
        };
        let window = egui::Window::new(window_name)
            .id(Id::new("add endpoint window"))
            .collapsible(false)
            .resizable(true)
            .default_width(200.0)
            .min_width(200.0);

        window.show(ctx, |ui| {
            let te = TextEdit::singleline(&mut self.edit_str).font(TextStyle::Monospace);
            ui.add(te);

            let rt = RichText::new(self.err_str.as_str()).color(Color32::RED);
            ui.label(rt);

            ui.horizontal(|ui| {
                ui.label("                 ");

                if ui.button("确定").clicked() {
                    match EndPoint::from_str(self.edit_str.as_str()) {
                        Ok(o) => {
                            ve.push(o);
                            *is_open = false;
                            self.edit_str.clear();
                        }
                        Err(e) => {
                            self.err_str = format!("{}", e);
                        }
                    };
                }

                ui.label("  ");

                if ui.button("取消").clicked() {
                    *is_open = false;
                    self.edit_str.clear();
                }
            });
        });
    }
}

pub struct PageSession {
    pub events: VecDeque<Event>,
    zenoh_config: Config,
    config_json: String,
    pub connected: bool,
    show_add_endpoint_window: bool,
    add_endpoint_window: AddEndpointWindow,
}

impl Default for PageSession {
    fn default() -> Self {
        let config = Config::default();
        let json: serde_json::Value = serde_json::to_value(&config).unwrap();
        PageSession {
            events: VecDeque::new(),
            zenoh_config: config,
            config_json: format!("{:#}", json),
            connected: false,
            show_add_endpoint_window: false,
            add_endpoint_window: AddEndpointWindow::default(),
        }
    }
}

impl PageSession {
    pub fn show(&mut self, ctx: &Context) {
        egui::SidePanel::left("page_session_panel_left")
            .resizable(true)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    if self.connected {
                        if ui.button("disconnect").clicked() {
                            self.events.push_back(Event::Disconnect);
                        }
                    } else {
                        if ui.button("connect").clicked() {
                            self.events
                                .push_back(Event::Connect(Box::new(self.zenoh_config.clone())));
                        }
                    }

                    self.show_config_edit(ctx, ui);
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                ui.horizontal(|ui| {
                    if ui.button("pretty").clicked() {
                        let json: serde_json::Value =
                            serde_json::to_value(&self.zenoh_config).unwrap();
                        self.config_json = format!("{:#}", json);
                    };
                    if ui.button("compact").clicked() {
                        let json: serde_json::Value =
                            serde_json::to_value(&self.zenoh_config).unwrap();
                        self.config_json = format!("{}", json);
                    };
                });

                ScrollArea::vertical().id_source("3").show(ui, |ui| {
                    ui.add(
                        TextEdit::multiline(&mut self.config_json)
                            .desired_width(f32::INFINITY)
                            .code_editor(),
                    )
                });
            });
        });

        if self.show_add_endpoint_window {
            let ve = match self.add_endpoint_window.kind {
                AEWKind::Connect => &mut self.zenoh_config.connect.endpoints,
                AEWKind::Listen => &mut self.zenoh_config.listen.endpoints,
            };
            self.add_endpoint_window
                .show(ctx, &mut self.show_add_endpoint_window, ve);
        }
    }

    fn show_config_edit(&mut self, _ctx: &Context, ui: &mut egui::Ui) {
        let rt_add = RichText::new("+").monospace();
        let rt_del = RichText::new("-").monospace();

        CollapsingHeader::new("mode")
            .id_source("config mode")
            .default_open(true)
            .show(ui, |ui| {
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
                                .selectable_label(
                                    self.zenoh_config.mode() == &Some(wai),
                                    wai.to_str(),
                                )
                                .clicked()
                            {
                                let _ = self.zenoh_config.set_mode(Some(wai));
                            }
                        }
                    });
            });

        CollapsingHeader::new("connect")
            .id_source("config connect")
            .default_open(true)
            .show(ui, |ui| {
                if ui.button(rt_add.clone()).clicked() {
                    self.show_add_endpoint_window = true;
                    self.add_endpoint_window.kind = AEWKind::Connect;
                }

                let mut remove_index: Option<usize> = None;
                let mut index = 0;
                for ed in &self.zenoh_config.connect.endpoints {
                    ui.horizontal(|ui| {
                        if ui.button(rt_del.clone()).clicked() {
                            remove_index = Some(index);
                        }
                        let rt = RichText::new(format!("{}", ed)).monospace();
                        ui.label(rt);
                    });
                    index += 1;
                }

                if let Some(i) = remove_index {
                    let _ = self.zenoh_config.connect.endpoints.remove(i);
                }
            });

        CollapsingHeader::new("listen")
            .id_source("config listen")
            .default_open(true)
            .show(ui, |ui| {
                if ui.button(rt_add.clone()).clicked() {
                    self.show_add_endpoint_window = true;
                    self.add_endpoint_window.kind = AEWKind::Listen;
                }

                let mut remove_index: Option<usize> = None;
                let mut index = 0;
                for ed in &self.zenoh_config.listen.endpoints {
                    ui.horizontal(|ui| {
                        if ui.button(rt_del.clone()).clicked() {
                            remove_index = Some(index);
                        }
                        let rt = RichText::new(format!("{}", ed)).monospace();
                        ui.label(rt);
                    });
                    index += 1;
                }
                if let Some(i) = remove_index {
                    let _ = self.zenoh_config.listen.endpoints.remove(i);
                }
            });
    }
}
