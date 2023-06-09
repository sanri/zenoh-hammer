use eframe::{
    egui,
    egui::{Color32, Context, Id, Layout, RichText, ScrollArea, TextEdit, TextStyle, Ui},
    emath::Align,
};
use egui_dnd::{utils::shift_vec, DragDropItem, DragDropUi};
use serde::{Deserialize, Serialize};
use serde_json;
use std::{
    collections::{BTreeMap, VecDeque},
    str::FromStr,
};
use zenoh::prelude::{CongestionControl, Encoding, KnownEncoding, OwnedKeyExpr, Priority, Value};

use crate::{app::ZenohValue, zenoh::PutData};

pub enum Event {
    Put(Box<PutData>),
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ZCongestionControl {
    Block,
    Drop,
}

impl From<CongestionControl> for ZCongestionControl {
    fn from(value: CongestionControl) -> Self {
        match value {
            CongestionControl::Block => ZCongestionControl::Block,
            CongestionControl::Drop => ZCongestionControl::Drop,
        }
    }
}

impl Into<CongestionControl> for ZCongestionControl {
    fn into(self) -> CongestionControl {
        match self {
            ZCongestionControl::Block => CongestionControl::Block,
            ZCongestionControl::Drop => CongestionControl::Drop,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ZPriority {
    RealTime,
    InteractiveHigh,
    InteractiveLow,
    DataHigh,
    Data,
    DataLow,
    Background,
}

impl From<Priority> for ZPriority {
    fn from(value: Priority) -> Self {
        match value {
            Priority::RealTime => ZPriority::RealTime,
            Priority::InteractiveHigh => ZPriority::InteractiveHigh,
            Priority::InteractiveLow => ZPriority::InteractiveLow,
            Priority::DataHigh => ZPriority::DataHigh,
            Priority::Data => ZPriority::Data,
            Priority::DataLow => ZPriority::DataLow,
            Priority::Background => ZPriority::Background,
        }
    }
}

impl Into<Priority> for ZPriority {
    fn into(self) -> Priority {
        match self {
            ZPriority::RealTime => Priority::RealTime,
            ZPriority::InteractiveHigh => Priority::InteractiveHigh,
            ZPriority::InteractiveLow => Priority::InteractiveLow,
            ZPriority::DataHigh => Priority::DataHigh,
            ZPriority::Data => Priority::Data,
            ZPriority::DataLow => Priority::DataLow,
            ZPriority::Background => Priority::Background,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DataItem {
    name: String,
    key: String,
    congestion_control: ZCongestionControl,
    priority: ZPriority,
    value: ZenohValue,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Data {
    puts: Vec<DataItem>,
}

pub struct PagePutData {
    id: u64,
    name: String,
    input_key: String,
    selected_congestion_control: CongestionControl,
    selected_priority: Priority,
    selected_encoding: KnownEncoding,
    edit_str: String,
    pub info: Option<RichText>,
}

impl Default for PagePutData {
    fn default() -> Self {
        PagePutData {
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

impl PagePutData {
    fn from(data: &DataItem) -> PagePutData {
        let (encoding, s) = data.value.to();
        PagePutData {
            id: 0,
            name: data.name.clone(),
            input_key: data.key.clone(),
            selected_congestion_control: data.congestion_control.into(),
            selected_priority: data.priority.into(),
            selected_encoding: encoding,
            edit_str: s,
            info: None,
        }
    }

    fn to(&self) -> DataItem {
        let value = ZenohValue::from(self.selected_encoding, self.edit_str.clone());
        DataItem {
            name: self.name.clone(),
            key: self.input_key.clone(),
            congestion_control: self.selected_congestion_control.into(),
            priority: self.selected_priority.into(),
            value,
        }
    }

    fn new_from(ppd: &PagePutData) -> Self {
        PagePutData {
            id: 0,
            name: ppd.name.clone(),
            input_key: ppd.input_key.clone(),
            selected_congestion_control: ppd.selected_congestion_control,
            selected_priority: ppd.selected_priority,
            selected_encoding: ppd.selected_encoding,
            edit_str: ppd.edit_str.clone(),
            info: None,
        }
    }

    fn show(&mut self, ui: &mut Ui, events: &mut VecDeque<Event>) {
        ui.vertical(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                if ui.button("send").clicked() {
                    self.send(events);
                }
            });

            self.show_name_key(ui);
            self.show_options(ui);
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
    pub data_map: BTreeMap<u64, PagePutData>,
    selected_data_id: u64,
    put_id_count: u64,
    dnd: DragDropUi,
    dnd_items: Vec<DndItem>,
}

impl Default for PagePut {
    fn default() -> Self {
        let mut p = PagePut {
            events: VecDeque::new(),
            data_map: BTreeMap::new(),
            selected_data_id: 1,
            put_id_count: 0,
            dnd: DragDropUi::default(),
            dnd_items: Vec::new(),
        };
        p.add_put_data(PagePutData::default());
        p
    }
}

impl PagePut {
    pub fn load(&mut self, data: Data) {
        self.clean_all_put_data();

        for d in data.puts {
            let page_data = PagePutData::from(&d);
            self.add_put_data(page_data);
        }
    }

    pub fn create_store_data(&self) -> Data {
        let mut data = Vec::with_capacity(self.data_map.len());
        for (_, d) in &self.data_map {
            let data_item = d.to();
            data.push(data_item);
        }
        Data { puts: data }
    }

    pub fn show(&mut self, ctx: &Context) {
        egui::SidePanel::left("page_put_panel_left")
            .resizable(true)
            .show(ctx, |ui| {
                self.show_puts_name(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let data = match self.data_map.get_mut(&self.selected_data_id) {
                None => {
                    return;
                }
                Some(o) => o,
            };

            data.show(ui, &mut self.events);
        });
    }

    fn show_puts_name(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(RichText::new(" + ").code())
                    .on_hover_text("copy add")
                    .clicked()
                {
                    if let Some(d) = self.data_map.get(&self.selected_data_id) {
                        self.add_put_data(PagePutData::new_from(d));
                    } else {
                        self.add_put_data(PagePutData::default());
                    }
                };

                if ui
                    .button(RichText::new(" - ").code())
                    .on_hover_text("del")
                    .clicked()
                {
                    self.del_put_data(self.selected_data_id);
                };
            });

            ui.label(" ");

            ScrollArea::both()
                .max_width(200.0)
                .auto_shrink([true, false])
                .show(ui, |ui| {
                    let response = self.dnd.ui::<DndItem>(
                        ui,
                        self.dnd_items.iter_mut(),
                        |item, ui, handle| {
                            ui.horizontal(|ui| {
                                if let Some(d) = self.data_map.get(&item.key_id) {
                                    handle.ui(ui, item, |ui| {
                                        ui.label("·");
                                    });

                                    let text = RichText::new(d.name.as_str());
                                    ui.selectable_value(
                                        &mut self.selected_data_id,
                                        item.key_id,
                                        text,
                                    );
                                }
                            });
                        },
                    );

                    if let Some(response) = response.completed {
                        shift_vec(response.from, response.to, &mut self.dnd_items);
                    }
                });
        });
    }

    fn add_put_data(&mut self, mut data: PagePutData) {
        self.put_id_count += 1;
        data.id = self.put_id_count;
        self.data_map.insert(self.put_id_count, data);
        self.selected_data_id = self.put_id_count;
        self.dnd_items.push(DndItem::new(self.put_id_count));
    }

    fn del_put_data(&mut self, put_id: u64) {
        if self.data_map.len() < 2 {
            return;
        }

        let _ = self.data_map.remove(&put_id);
        let mut del_index = None;
        for (i, di) in self.dnd_items.iter().enumerate() {
            if di.key_id == put_id {
                del_index = Some(i);
                break;
            }
        }
        if let Some(i) = del_index {
            self.dnd_items.remove(i);
        }
    }

    fn clean_all_put_data(&mut self) {
        self.data_map.clear();
        self.dnd_items.clear();
        self.selected_data_id = 0;
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

struct DndItem {
    key_id: u64,
}

impl DndItem {
    fn new(k: u64) -> Self {
        DndItem { key_id: k }
    }
}

impl DragDropItem for DndItem {
    fn id(&self) -> Id {
        Id::new(format!("page_put dnd_item {}", self.key_id))
    }
}
