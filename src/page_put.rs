use crate::zenoh::PutData;
use eframe::emath::Align;
use egui::{Color32, Layout, RichText, ScrollArea, TextEdit, TextStyle, Ui};
use serde_json;
use std::{
    collections::{BTreeMap, VecDeque},
    str::FromStr,
};
use zenoh::prelude::{
    CongestionControl, Encoding, KeyExpr, KnownEncoding, OwnedKeyExpr, Priority, SampleKind, Value,
};

pub enum Event {
    Put(Box<PutData>),
}

pub struct Data {
    id: u64,
    name: String,
    input_key: String,
    selected_congestion_control: CongestionControl,
    selected_priority: Priority,
    selected_encoding: KnownEncoding,
    edit_str: String,
    pub info: Option<RichText>,
}

impl Default for Data {
    fn default() -> Self {
        Data {
            id: 1,
            name: "demo".to_string(),
            input_key: "demo/example".to_string(),
            selected_congestion_control: CongestionControl::Block,
            selected_priority: Priority::RealTime,
            selected_encoding: KnownEncoding::TextPlain,
            edit_str: String::new(),
            info: None,
        }
    }
}

impl Data {
    fn show(&mut self, ui: &mut Ui, events: &mut VecDeque<Event>) {
        ui.vertical(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                if ui.button("发送").clicked() {
                    self.send(events);
                }
            });

            self.show_name_key(events, ui);
            self.show_options(ui);
        });
    }
    fn show_name_key(&mut self, events: &mut VecDeque<Event>, ui: &mut Ui) {
        let mut input_grid = |ui: &mut Ui| {
            ui.label("name: ");
            let te = TextEdit::singleline(&mut self.name)
                .desired_width(600.0)
                .font(TextStyle::Monospace);
            ui.add(te);
            ui.end_row();

            ui.label("key: ");
            let te = TextEdit::multiline(&mut self.input_key)
                .desired_rows(2)
                .desired_width(600.0)
                .font(TextStyle::Monospace);
            ui.add(te);

            ui.end_row();
        };
        egui::Grid::new("input_grid")
            .num_columns(2)
            .striped(false)
            .show(ui, |ui| {
                input_grid(ui);
            });
    }

    fn show_options(&mut self, ui: &mut Ui) {
        let mut show_grid = |ui: &mut Ui| {
            ui.label("congestion control: ");
            egui::ComboBox::new("congestion control", "")
                .selected_text(format!("{:?}", self.selected_congestion_control))
                .show_ui(ui, |ui| {
                    let options = [CongestionControl::Block, CongestionControl::Drop];
                    for option in options {
                        ui.selectable_value(
                            &mut self.selected_congestion_control,
                            option,
                            format!("{:?}", option),
                        );
                    }
                });
            ui.end_row();

            ui.label("priority: ");
            egui::ComboBox::new("priority", "")
                .selected_text(format!("{:?}", self.selected_priority))
                .show_ui(ui, |ui| {
                    let options = [
                        Priority::RealTime,
                        Priority::InteractiveHigh,
                        Priority::InteractiveLow,
                        Priority::DataHigh,
                        Priority::Data,
                        Priority::DataLow,
                        Priority::Background,
                    ];
                    for option in options {
                        ui.selectable_value(
                            &mut self.selected_priority,
                            option,
                            format!("{:?}", option),
                        );
                    }
                });
            ui.end_row();

            ui.label("encoding: ");
            egui::ComboBox::new("encoding", "")
                .selected_text(format!("{}", Encoding::Exact(self.selected_encoding)))
                .show_ui(ui, |ui| {
                    let options = [
                        KnownEncoding::TextPlain,
                        KnownEncoding::TextJson,
                        KnownEncoding::AppJson,
                        KnownEncoding::AppInteger,
                        KnownEncoding::AppFloat,
                    ];
                    for option in options {
                        ui.selectable_value(
                            &mut self.selected_encoding,
                            option,
                            format!("{}", Encoding::Exact(option)),
                        );
                    }
                });
            ui.end_row();
        };

        egui::Grid::new("options_grid")
            .num_columns(2)
            .striped(false)
            .show(ui, |ui| {
                show_grid(ui);
            });

        ui.label("value: ");
        ScrollArea::vertical()
            .id_source("value scroll area")
            .show(ui, |ui| {
                match self.selected_encoding {
                    KnownEncoding::TextPlain => {
                        ui.add(
                            TextEdit::multiline(&mut self.edit_str)
                                .desired_width(f32::INFINITY)
                                .desired_rows(3)
                                .code_editor(),
                        );
                    }
                    KnownEncoding::AppJson => {
                        ui.add(
                            TextEdit::multiline(&mut self.edit_str)
                                .desired_width(f32::INFINITY)
                                .desired_rows(3)
                                .code_editor(),
                        );
                    }
                    KnownEncoding::AppInteger => {
                        ui.add(TextEdit::singleline(&mut self.edit_str));
                    }
                    KnownEncoding::AppFloat => {
                        ui.add(TextEdit::singleline(&mut self.edit_str));
                    }
                    KnownEncoding::TextJson => {
                        ui.add(
                            TextEdit::multiline(&mut self.edit_str)
                                .desired_width(f32::INFINITY)
                                .desired_rows(3)
                                .code_editor(),
                        );
                    }
                    _ => {}
                }

                if let Some(rt) = &self.info {
                    ui.label(rt.clone());
                };
            });
    }

    fn send(&mut self, events: &mut VecDeque<Event>) {
        let key_str = self.input_key.replace(&[' ', '\t', '\n', '\r'], "");
        let key: OwnedKeyExpr = match OwnedKeyExpr::from_str(key_str.as_str()) {
            Ok(o) => o,
            Err(e) => {
                let rt = RichText::new(format!("{}", e)).color(Color32::RED);
                self.info = Some(rt);
                return;
            }
        };
        let value = match self.selected_encoding {
            KnownEncoding::TextPlain => Value::from(self.edit_str.as_str()),
            KnownEncoding::AppJson => {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(self.edit_str.as_str()) {
                    let rt = RichText::new(format!("{}", e)).color(Color32::RED);
                    self.info = Some(rt);
                    return;
                }
                Value::from(self.edit_str.as_str()).encoding(KnownEncoding::AppJson.into())
            }
            KnownEncoding::AppInteger => {
                let i: i64 = match self.edit_str.parse::<i64>() {
                    Ok(i) => i,
                    Err(e) => {
                        let rt = RichText::new(format!("{}", e)).color(Color32::RED);
                        self.info = Some(rt);
                        return;
                    }
                };
                Value::from(i)
            }
            KnownEncoding::AppFloat => {
                let f: f64 = match self.edit_str.parse::<f64>() {
                    Ok(f) => f,
                    Err(e) => {
                        let rt = RichText::new(format!("{}", e)).color(Color32::RED);
                        self.info = Some(rt);
                        return;
                    }
                };
                Value::from(f)
            }
            KnownEncoding::TextJson => {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(self.edit_str.as_str()) {
                    let rt = RichText::new(format!("{}", e)).color(Color32::RED);
                    self.info = Some(rt);
                    return;
                }
                Value::from(self.edit_str.as_str()).encoding(KnownEncoding::TextJson.into())
            }
            _ => {
                return;
            }
        };
        let put_data = PutData {
            id: self.id,
            key,
            congestion_control: self.selected_congestion_control,
            priority: self.selected_priority,
            value,
        };
        events.push_back(Event::Put(Box::new(put_data)));
        self.info = None;
    }
}

pub struct PagePut {
    pub events: VecDeque<Event>,
    pub data_map: BTreeMap<u64, Data>,
    selected_data: u64,
    data_id_count: u64,
}

impl Default for PagePut {
    fn default() -> Self {
        let mut btm = BTreeMap::new();
        btm.insert(1, Data::default());
        PagePut {
            events: VecDeque::new(),
            data_map: btm,
            selected_data: 1,
            data_id_count: 1,
        }
    }
}

impl PagePut {
    pub fn show(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
            self.show_puts(ui);

            ui.separator();

            let mut data = match self.data_map.get_mut(&self.selected_data) {
                None => {
                    return;
                }
                Some(o) => o,
            };

            data.show(ui, &mut self.events);
        });
    }

    fn show_puts(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if ui.button(RichText::new(" + ").code()).clicked() {
                    self.data_id_count += 1;
                    let data = Data {
                        id: self.data_id_count,
                        ..Data::default()
                    };
                    self.data_map.insert(self.data_id_count, data);
                };

                if ui.button(RichText::new(" - ").code()).clicked() {
                    if self.data_map.len() < 2 {
                        return;
                    }

                    let _ = self.data_map.remove(&self.selected_data);
                    for (k, _) in &self.data_map {
                        self.selected_data = *k;
                        break;
                    }
                };
            });

            ui.label("");

            ScrollArea::both()
                .max_width(160.0)
                .auto_shrink([true, false])
                .show(ui, |ui| {
                    for (i, d) in &self.data_map {
                        let text = RichText::new(d.name.clone()).monospace();
                        ui.selectable_value(&mut self.selected_data, *i, text);
                    }
                });
        });
    }

    pub fn processing_put_res(&mut self, r: Box<(u64, bool, String)>) {
        let (id, b, s) = *r;
        if let Some(pd) = self.data_map.get_mut(&id) {
            pd.info = if b {
                Some(RichText::new(s))
            } else {
                Some(RichText::new(s).color(Color32::RED))
            }
        }
    }
}
