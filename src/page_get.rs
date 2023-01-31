use egui::{Align, Color32, DragValue, Layout, RichText, ScrollArea, TextEdit, TextStyle, Ui};
use egui_extras::{Column, TableBuilder};
use std::{
    collections::{BTreeMap, VecDeque},
    str::FromStr,
    time::Duration,
};
use zenoh::{
    prelude::{
        Encoding, KeyExpr, KnownEncoding, Locality, OwnedKeyExpr, QueryConsolidation, QueryTarget,
        Value,
    },
    query::{ConsolidationMode, Mode, Reply},
};

// query
pub struct Event {
    id: u64,
    key_expr: OwnedKeyExpr,
    target: QueryTarget,
    consolidation: QueryConsolidation,
    locality: Locality,
    timeout: Duration,
    value: Option<Value>,
}

pub struct Data {
    id: u64,
    name: String,
    input_key: String,
    selected_target: QueryTarget,
    selected_consolidation: QueryConsolidation,
    selected_locality: Locality,
    timeout: u64,
    edit_str: String,
    selected_encoding: KnownEncoding,
    replies: Vec<Reply>,
    pub info: Option<RichText>,
}

impl Default for Data {
    fn default() -> Self {
        Data {
            id: 0,
            name: "demo".to_string(),
            input_key: "demo/test".to_string(),
            selected_target: QueryTarget::default(),
            selected_consolidation: QueryConsolidation::default(),
            selected_locality: Locality::default(),
            timeout: 10000,
            edit_str: String::new(),
            selected_encoding: KnownEncoding::Empty,
            replies: Vec::new(),
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

            self.show_name_key(ui);
            self.show_options(ui);
            ui.separator();
            if let Some(rt) = &self.info {
                ui.label(rt.clone());
                return;
            };
            self.show_reply(ui);
        });
    }

    fn show_name_key(&mut self, ui: &mut Ui) {
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
            ui.label("target: ");
            egui::ComboBox::new("query target", "")
                .selected_text(format!("{:?}", self.selected_target))
                .show_ui(ui, |ui| {
                    let options = [
                        QueryTarget::BestMatching,
                        QueryTarget::All,
                        QueryTarget::AllComplete,
                    ];
                    for option in options {
                        ui.selectable_value(
                            &mut self.selected_target,
                            option,
                            format!("{:?}", option),
                        );
                    }
                });
            ui.end_row();

            let dc = |c: QueryConsolidation| match c.mode() {
                Mode::Auto => "Auto",
                Mode::Manual(m) => match m {
                    ConsolidationMode::None => "None",
                    ConsolidationMode::Monotonic => "Monotonic",
                    ConsolidationMode::Latest => "Latest",
                },
            };

            ui.label("consolidation: ");
            egui::ComboBox::new("consolidation", "")
                .selected_text(dc(self.selected_consolidation))
                .show_ui(ui, |ui| {
                    let options = [
                        QueryConsolidation::AUTO,
                        QueryConsolidation::from(ConsolidationMode::None),
                        QueryConsolidation::from(ConsolidationMode::Monotonic),
                        QueryConsolidation::from(ConsolidationMode::Latest),
                    ];
                    for option in options {
                        ui.selectable_value(&mut self.selected_consolidation, option, dc(option));
                    }
                });
            ui.end_row();

            ui.label("locality: ");
            egui::ComboBox::new("locality", "")
                .selected_text(format!("{:?}", self.selected_locality))
                .show_ui(ui, |ui| {
                    let options = [Locality::SessionLocal, Locality::Remote, Locality::Any];
                    for option in options {
                        ui.selectable_value(
                            &mut self.selected_locality,
                            option,
                            format!("{:?}", option),
                        );
                    }
                });
            ui.end_row();

            ui.label("timeout: ");
            let dv = DragValue::new(&mut self.timeout)
                .speed(10.0)
                .clamp_range(0..=10000);
            ui.add(dv);
            ui.end_row();

            ui.label("query payload: ");
            egui::ComboBox::new("query payload", "")
                .selected_text(format!("{}", Encoding::Exact(self.selected_encoding)))
                .show_ui(ui, |ui| {
                    let options = [
                        KnownEncoding::Empty,
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

        match self.selected_encoding {
            KnownEncoding::Empty => {}
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
        };
    }

    fn show_reply(&mut self, ui: &mut Ui) {
        if self.replies.is_empty() {
            return;
        }

        let mut table = TableBuilder::new(ui)
            .striped(true)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(
                Column::initial(40.0)
                    .range(40.0..=160.0)
                    .resizable(true)
                    .clip(true),
            )
            .column(Column::remainder())
            .resizable(true);

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label("value");
                });
                header.col(|ui| {
                    ui.label("key");
                });
            })
            .body(|mut body| {});
    }

    fn send(&mut self, events: &mut VecDeque<Event>) {
        self.replies.clear();
        self.info = None;
        let key_str = self.input_key.replace(&[' ', '\t', '\n', '\r'], "");
        let key: OwnedKeyExpr = match OwnedKeyExpr::from_str(key_str.as_str()) {
            Ok(o) => o,
            Err(e) => {
                let rt = RichText::new(format!("{}", e)).color(Color32::RED);
                self.info = Some(rt);
                return;
            }
        };
        let v = match self.selected_encoding {
            KnownEncoding::Empty => None,
            KnownEncoding::TextPlain => {
                let v = Value::from(self.edit_str.as_str());
                Some(v)
            }
            KnownEncoding::AppJson => {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(self.edit_str.as_str()) {
                    let rt = RichText::new(format!("{}", e)).color(Color32::RED);
                    self.info = Some(rt);
                    return;
                }
                let v = Value::from(self.edit_str.as_str()).encoding(KnownEncoding::AppJson.into());
                Some(v)
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
                Some(Value::from(i))
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
                Some(Value::from(f))
            }
            KnownEncoding::TextJson => {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(self.edit_str.as_str()) {
                    let rt = RichText::new(format!("{}", e)).color(Color32::RED);
                    self.info = Some(rt);
                    return;
                }
                let v =
                    Value::from(self.edit_str.as_str()).encoding(KnownEncoding::TextJson.into());
                Some(v)
            }
            _ => None,
        };
        let e = Event {
            id: self.id,
            key_expr: key,
            target: self.selected_target,
            consolidation: self.selected_consolidation,
            locality: self.selected_locality,
            timeout: Duration::from_millis(self.timeout),
            value: v,
        };
        events.push_back(e);
    }
}

pub struct PageGet {
    pub events: VecDeque<Event>,
    pub data_map: BTreeMap<u64, Data>,
    selected_data: u64,
    data_id_count: u64,
}

impl Default for PageGet {
    fn default() -> Self {
        let mut btm = BTreeMap::new();
        btm.insert(1, Data::default());
        PageGet {
            events: VecDeque::new(),
            data_map: btm,
            selected_data: 1,
            data_id_count: 1,
        }
    }
}

impl PageGet {
    pub fn show(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
            self.show_gets(ui);

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

    fn show_gets(&mut self, ui: &mut Ui) {
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
                .id_source("gets list")
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
}
