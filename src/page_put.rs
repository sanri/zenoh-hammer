use crate::{
    payload_editor::{ArchivePayloadEdit, PayloadEdit},
    task_zenoh::PutData,
    zenoh_data::{ZCongestionControl, ZPriority},
};
use eframe::{
    egui::{
        CentralPanel, CollapsingHeader, Color32, ComboBox, Context, Grid, Layout, RichText,
        ScrollArea, SidePanel, TextEdit, TextStyle, Ui, Widget,
    },
    emath::Align,
};
use egui_dnd::dnd;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    str::FromStr,
};
use strum::IntoEnumIterator;
use zenoh::key_expr::OwnedKeyExpr;

pub enum Event {
    Put(Box<PutData>),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ArchivePagePutData {
    name: String,
    key: String,
    congestion_control: ZCongestionControl,
    priority: ZPriority,
    archive_payload_edit: ArchivePayloadEdit,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ArchivePagePut {
    puts: Vec<ArchivePagePutData>,
}

pub struct PagePutData {
    id: u64,
    name: String,
    input_key: String,
    selected_congestion_control: ZCongestionControl,
    selected_priority: ZPriority,
    payload_edit: PayloadEdit,
    info: Option<Result<String, String>>,
}

impl Default for PagePutData {
    fn default() -> Self {
        PagePutData {
            id: 1,
            name: "demo".to_string(),
            input_key: "demo/example".to_string(),
            selected_congestion_control: ZCongestionControl::Block,
            selected_priority: ZPriority::RealTime,
            payload_edit: PayloadEdit::default(),
            info: None,
        }
    }
}

impl From<&PagePutData> for PagePutData {
    fn from(value: &PagePutData) -> Self {
        PagePutData {
            id: value.id,
            name: value.name.clone(),
            input_key: value.input_key.clone(),
            selected_congestion_control: value.selected_congestion_control,
            selected_priority: value.selected_priority,
            payload_edit: (&value.payload_edit).into(),
            info: None,
        }
    }
}

impl From<&PagePutData> for ArchivePagePutData {
    fn from(value: &PagePutData) -> Self {
        ArchivePagePutData {
            name: value.name.clone(),
            key: value.input_key.clone(),
            congestion_control: value.selected_congestion_control,
            priority: value.selected_priority,
            archive_payload_edit: (&value.payload_edit).into(),
        }
    }
}

impl TryFrom<&ArchivePagePutData> for PagePutData {
    type Error = String;

    fn try_from(value: &ArchivePagePutData) -> Result<Self, Self::Error> {
        Ok(PagePutData {
            id: 0,
            name: value.name.clone(),
            input_key: value.key.clone(),
            selected_congestion_control: value.congestion_control,
            selected_priority: value.priority,
            payload_edit: (&value.archive_payload_edit).try_into()?,
            info: None,
        })
    }
}

impl TryFrom<ArchivePagePutData> for PagePutData {
    type Error = String;

    fn try_from(value: ArchivePagePutData) -> Result<Self, Self::Error> {
        Ok(PagePutData {
            id: 0,
            name: value.name,
            input_key: value.key,
            selected_congestion_control: value.congestion_control,
            selected_priority: value.priority,
            payload_edit: value.archive_payload_edit.try_into()?,
            info: None,
        })
    }
}

impl PagePutData {
    fn show(&mut self, ui: &mut Ui, events: &mut VecDeque<Event>) {
        self.show_name_key(ui, events);

        if let Some(info) = &self.info {
            let text = match info {
                Ok(o) => RichText::new(o),
                Err(o) => RichText::new(o).color(Color32::RED),
            };
            ui.label(text.clone());
        }

        ScrollArea::horizontal()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.show_options(ui);
                self.show_payload_edit(ui);
            });
    }

    fn show_name_key(&mut self, ui: &mut Ui, events: &mut VecDeque<Event>) {
        let mut input_grid = |ui: &mut Ui| {
            ui.label("name:");
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                if ui.button("send").clicked() {
                    self.send(events);
                }
                TextEdit::singleline(&mut self.name)
                    .desired_width(3000.0)
                    .font(TextStyle::Monospace)
                    .ui(ui);
            });
            ui.end_row();

            ui.label("key:");
            TextEdit::multiline(&mut self.input_key)
                .desired_rows(1)
                .desired_width(3000.0)
                .font(TextStyle::Monospace)
                .ui(ui);
            ui.end_row();
        };

        Grid::new("input_grid")
            .num_columns(2)
            .striped(false)
            .show(ui, |ui| {
                input_grid(ui);
            });
    }

    fn show_options(&mut self, ui: &mut Ui) {
        let mut show_grid = |ui: &mut Ui| {
            ui.label("congestion control:");
            ComboBox::new("congestion control", "")
                .selected_text(self.selected_congestion_control.as_ref())
                .show_ui(ui, |ui| {
                    for option in ZCongestionControl::iter() {
                        ui.selectable_value(
                            &mut self.selected_congestion_control,
                            option,
                            option.as_ref(),
                        );
                    }
                });
            ui.end_row();

            ui.label("priority:");
            ComboBox::new("priority", "")
                .selected_text(self.selected_priority.as_ref())
                .show_ui(ui, |ui| {
                    for option in ZPriority::iter() {
                        ui.selectable_value(&mut self.selected_priority, option, option.as_ref());
                    }
                });
            ui.end_row();
        };

        CollapsingHeader::new("Options")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("options_grid")
                    .num_columns(2)
                    .striped(false)
                    .show(ui, |ui| {
                        show_grid(ui);
                    });
            });
    }

    fn show_payload_edit(&mut self, ui: &mut Ui) {
        CollapsingHeader::new("Payload")
            .default_open(true)
            .show(ui, |ui| {
                self.payload_edit.show(ui);
            });
    }

    fn send(&mut self, events: &mut VecDeque<Event>) {
        let key_str = self.input_key.replace(&[' ', '\t', '\n', '\r'], "");
        let key: OwnedKeyExpr = match OwnedKeyExpr::from_str(key_str.as_str()) {
            Ok(o) => o,
            Err(e) => {
                self.info = Some(Err(format!("{}", e)));
                return;
            }
        };

        let (encoding, payload) = match self.payload_edit.get_zenoh_value() {
            None => {
                return;
            }
            Some(o) => o,
        };

        let put_data = PutData {
            id: self.id,
            key,
            congestion_control: self.selected_congestion_control.into(),
            priority: self.selected_priority.into(),
            encoding,
            payload,
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
    dnd_items: Vec<DndItem>,
}

impl Default for PagePut {
    fn default() -> Self {
        let mut p = PagePut {
            events: VecDeque::new(),
            data_map: BTreeMap::new(),
            selected_data_id: 1,
            put_id_count: 0,
            dnd_items: Vec::new(),
        };
        p.add_put_data(PagePutData::default());
        p
    }
}

impl From<&PagePut> for ArchivePagePut {
    fn from(value: &PagePut) -> Self {
        ArchivePagePut {
            puts: value
                .dnd_items
                .iter()
                .filter_map(|k| value.data_map.get(&k.key_id))
                .map(|d| d.into())
                .collect(),
        }
    }
}

impl PagePut {
    pub fn load(&mut self, archive: ArchivePagePut) -> Result<(), String> {
        let mut data = Vec::with_capacity(archive.puts.len());
        for d in archive.puts {
            let page_put_data = PagePutData::try_from(d)?;
            data.push(page_put_data);
        }

        self.clean_all_put_data();
        for d in data {
            self.add_put_data(d);
        }
        Ok(())
    }

    pub fn show(&mut self, ctx: &Context) {
        SidePanel::left("page_put_panel_left")
            .resizable(true)
            .show(ctx, |ui| {
                self.show_puts_name(ui);
            });

        CentralPanel::default().show(ctx, |ui| {
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
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new(" + ").code())
                .on_hover_text("copy add")
                .clicked()
            {
                if let Some(d) = self.data_map.get(&self.selected_data_id) {
                    self.add_put_data(PagePutData::from(d));
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

        ui.add_space(10.0);

        ScrollArea::both()
            .max_width(200.0)
            .auto_shrink([true, false])
            .show(ui, |ui| {
                dnd(ui, "page_put_list").show_vec(
                    self.dnd_items.as_mut_slice(),
                    |ui, item, handle, _state| {
                        if let Some(d) = self.data_map.get(&item.key_id) {
                            handle.ui(ui, |ui| {
                                let text = RichText::new(d.name.as_str());
                                ui.selectable_value(&mut self.selected_data_id, item.key_id, text);
                            });
                        }
                    },
                )
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
            pd.info = if b { Some(Ok(s)) } else { Some(Err(s)) }
        }
    }
}

#[derive(Hash)]
struct DndItem {
    key_id: u64,
}

impl DndItem {
    fn new(k: u64) -> Self {
        DndItem { key_id: k }
    }
}
