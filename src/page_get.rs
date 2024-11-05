use eframe::egui::{
    Align, CentralPanel, CollapsingHeader, Color32, ComboBox, Context, DragValue, Grid, Id, Layout,
    RichText, ScrollArea, SidePanel, TextEdit, TextStyle, Ui, Widget, Window,
};
use egui_dnd::dnd;
use egui_extras::{Column, TableBody, TableBuilder, TableRow};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    str::FromStr,
    time::Duration,
};
use strum::IntoEnumIterator;
use zenoh::{bytes::ZBytes, internal::Value, key_expr::OwnedKeyExpr, query::Reply};

use crate::{
    payload_editor::{ArchivePayloadEdit, PayloadEdit},
    reply_viewer::ReplyViewer,
    task_zenoh::QueryData,
    zenoh_data::{zenoh_value_abstract, ZConsolidation, ZLocality, ZQueryTarget},
};

// query
pub enum Event {
    Get(Box<QueryData>),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ArchivePageGetData {
    name: String,
    key: String,
    attachment: String,
    target: ZQueryTarget,
    consolidation: ZConsolidation,
    locality: ZLocality,
    timeout: u64,
    payload: bool,
    archive_payload_edit: ArchivePayloadEdit,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ArchivePageGet {
    gets: Vec<ArchivePageGetData>,
}

pub struct PageGetData {
    id: u64,
    name: String,
    input_key: String,
    input_attachment: String,
    selected_target: ZQueryTarget,
    selected_consolidation: ZConsolidation,
    selected_locality: ZLocality,
    timeout: u64,
    payload: bool,
    payload_edit: PayloadEdit,
    replies: Vec<Reply>,
    error_info: Option<RichText>,
}

impl Default for PageGetData {
    fn default() -> Self {
        PageGetData {
            id: 1,
            name: "demo".to_string(),
            input_key: "demo/test".to_string(),
            input_attachment: String::new(),
            selected_target: ZQueryTarget::BestMatching,
            selected_consolidation: ZConsolidation::Auto,
            selected_locality: ZLocality::Any,
            timeout: 10000,
            payload: false,
            payload_edit: PayloadEdit::default(),
            replies: Vec::new(),
            error_info: None,
        }
    }
}

impl From<&PageGetData> for PageGetData {
    fn from(value: &PageGetData) -> Self {
        PageGetData {
            id: 0,
            name: value.name.clone(),
            input_key: value.input_key.clone(),
            input_attachment: value.input_attachment.clone(),
            selected_target: value.selected_target,
            selected_consolidation: value.selected_consolidation,
            selected_locality: value.selected_locality,
            timeout: value.timeout,
            payload: value.payload,
            payload_edit: (&value.payload_edit).into(),
            replies: Vec::new(),
            error_info: None,
        }
    }
}

impl From<&PageGetData> for ArchivePageGetData {
    fn from(value: &PageGetData) -> Self {
        ArchivePageGetData {
            name: value.name.clone(),
            key: value.input_key.clone(),
            attachment: value.input_attachment.clone(),
            target: value.selected_target,
            consolidation: value.selected_consolidation,
            locality: value.selected_locality,
            timeout: value.timeout,
            payload: value.payload,
            archive_payload_edit: (&value.payload_edit).into(),
        }
    }
}

impl TryFrom<&ArchivePageGetData> for PageGetData {
    type Error = String;

    fn try_from(value: &ArchivePageGetData) -> Result<Self, Self::Error> {
        Ok(PageGetData {
            id: 0,
            name: value.name.clone(),
            input_key: value.key.clone(),
            input_attachment: value.attachment.clone(),
            selected_target: value.target,
            selected_consolidation: value.consolidation,
            selected_locality: value.locality,
            timeout: value.timeout,
            payload: value.payload,
            payload_edit: (&value.archive_payload_edit).try_into()?,
            replies: Vec::new(),
            error_info: None,
        })
    }
}

impl TryFrom<ArchivePageGetData> for PageGetData {
    type Error = String;

    fn try_from(value: ArchivePageGetData) -> Result<Self, Self::Error> {
        Ok(PageGetData {
            id: 0,
            name: value.name,
            input_key: value.key,
            input_attachment: value.attachment,
            selected_target: value.target,
            selected_consolidation: value.consolidation,
            selected_locality: value.locality,
            timeout: value.timeout,
            payload: value.payload,
            payload_edit: (&value.archive_payload_edit).try_into()?,
            replies: Vec::new(),
            error_info: None,
        })
    }
}

impl PageGetData {
    fn show(
        &mut self,
        ui: &mut Ui,
        events: &mut VecDeque<Event>,
        show_window: &mut bool,
        reply_window: &mut ReplyViewer,
    ) {
        self.show_name_key_attachment(ui, events);
        ScrollArea::horizontal()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.show_options(ui);
                self.show_payload_edit(ui);
                ui.separator();
                self.show_reply_table(ui, show_window, reply_window);
            });
    }

    fn show_name_key_attachment(&mut self, ui: &mut Ui, events: &mut VecDeque<Event>) {
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

            ui.label("attachment:");
            TextEdit::singleline(&mut self.input_attachment)
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
            ui.label("target:");
            ComboBox::new("query target", "")
                .selected_text(self.selected_target.as_ref())
                .show_ui(ui, |ui| {
                    for option in ZQueryTarget::iter() {
                        ui.selectable_value(&mut self.selected_target, option, option.as_ref());
                    }
                });
            ui.end_row();

            ui.label("consolidation:");
            ComboBox::new("consolidation", "")
                .selected_text(self.selected_consolidation.as_ref())
                .show_ui(ui, |ui| {
                    for option in ZConsolidation::iter() {
                        ui.selectable_value(
                            &mut self.selected_consolidation,
                            option,
                            option.as_ref(),
                        );
                    }
                });
            ui.end_row();

            ui.label("locality:");
            ComboBox::new("locality", "")
                .selected_text(self.selected_locality.as_ref())
                .show_ui(ui, |ui| {
                    for option in ZLocality::iter() {
                        ui.selectable_value(&mut self.selected_locality, option, option.as_ref());
                    }
                });
            ui.end_row();

            ui.label("timeout:");
            let dv = DragValue::new(&mut self.timeout)
                .suffix("ms")
                .speed(10.0)
                .range(0..=10000);
            ui.add(dv);
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
                ui.checkbox(&mut self.payload, "payload");

                if self.payload {
                    self.payload_edit.show(ui);
                }
            });
    }

    fn show_reply_table(
        &mut self,
        ui: &mut Ui,
        show_window: &mut bool,
        reply_window: &mut ReplyViewer,
    ) {
        if self.replies.is_empty() {
            return;
        }

        let table_header = |mut table_row: TableRow| {
            table_row.col(|ui| {
                ui.label("key");
            });
            table_row.col(|ui| {
                ui.label("value");
            });
            table_row.col(|ui| {
                ui.label("type");
            });
            table_row.col(|ui| {
                ui.label("timestamp");
            });
        };

        let table_body = |mut body: TableBody| {
            for reply in &self.replies {
                body.row(20.0, |mut row| {
                    match reply.result() {
                        Ok(sample) => {
                            row.col(|ui| {
                                ui.label(sample.key_expr().as_str());
                            });
                            row.col(|ui| {
                                let text =
                                    zenoh_value_abstract(sample.encoding(), sample.payload());
                                let rich_text = match text {
                                    Ok(o) => RichText::new(o),
                                    Err(e) => RichText::new(e).color(Color32::RED),
                                };

                                if ui.button(rich_text).clicked() {
                                    *reply_window = ReplyViewer::new_from_reply(reply);
                                    *show_window = true;
                                }
                            });
                            row.col(|ui| {
                                ui.label(sample.encoding().to_string());
                            });
                            row.col(|ui| {
                                let rich_text = if let Some(timestamp) = sample.timestamp() {
                                    RichText::new(format!("{}", timestamp.get_time())).size(12.0)
                                } else {
                                    RichText::new("-")
                                };
                                ui.label(rich_text);
                            });
                        }
                        Err(e) => {
                            let text = zenoh_value_abstract(e.encoding(), e.payload())
                                .unwrap_or_else(|s| s);
                            let text = RichText::new(text).size(12.0).color(Color32::RED);
                            row.col(|ui| {
                                if ui.button("...").clicked() {
                                    *reply_window = ReplyViewer::new_from_reply(reply);
                                    *show_window = true;
                                }
                            });
                            row.col(|ui| {
                                ui.label(text);
                            });
                            row.col(|ui| {
                                ui.label("-");
                            });
                            row.col(|ui| {
                                ui.label("-");
                            });
                        }
                    };
                })
            }
        };

        let table = TableBuilder::new(ui)
            .striped(true)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::initial(100.0).resizable(true).clip(true))
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::remainder())
            .resizable(true);

        table.header(20.0, table_header).body(table_body);
    }

    fn send(&mut self, events: &mut VecDeque<Event>) {
        self.replies.clear();
        let key_str = self.input_key.replace(&[' ', '\t', '\n', '\r'], "");
        let key: OwnedKeyExpr = match OwnedKeyExpr::from_str(key_str.as_str()) {
            Ok(o) => o,
            Err(e) => {
                let rt = RichText::new(format!("{}", e)).color(Color32::RED);
                self.error_info = Some(rt);
                return;
            }
        };

        let value: Option<Value> = if self.payload {
            self.payload_edit.get_zenoh_value()
        } else {
            None
        };

        let attachment: Option<ZBytes> = if self.input_attachment.is_empty() {
            None
        } else {
            Some(ZBytes::from(self.input_attachment.as_str()))
        };

        let d = QueryData {
            id: self.id,
            key_expr: key,
            attachment,
            target: self.selected_target.into(),
            consolidation: self.selected_consolidation.into(),
            locality: self.selected_locality.into(),
            timeout: Duration::from_millis(self.timeout),
            value,
        };
        events.push_back(Event::Get(Box::new(d)));
    }
}

pub struct PageGet {
    pub events: VecDeque<Event>,
    pub data_map: BTreeMap<u64, PageGetData>,
    selected_data_id: u64,
    get_id_count: u64,
    show_reply_viewer_window: bool,
    reply_viewer_window: ReplyViewer,
    // dnd: DragDropUi,
    dnd_items: Vec<DndItem>,
}

impl Default for PageGet {
    fn default() -> Self {
        let mut p = PageGet {
            events: VecDeque::new(),
            data_map: BTreeMap::new(),
            selected_data_id: 1,
            get_id_count: 0,
            show_reply_viewer_window: false,
            reply_viewer_window: ReplyViewer::default(),
            // dnd: DragDropUi::default(),
            dnd_items: Vec::new(),
        };
        p.add_get_data(PageGetData::default());
        p
    }
}

impl From<&PageGet> for ArchivePageGet {
    fn from(value: &PageGet) -> Self {
        ArchivePageGet {
            gets: value
                .dnd_items
                .iter()
                .filter_map(|k| value.data_map.get(&k.key_id))
                .map(|d| d.into())
                .collect(),
        }
    }
}

impl PageGet {
    pub fn load(&mut self, archive: ArchivePageGet) -> Result<(), String> {
        let mut data = Vec::with_capacity(archive.gets.len());
        for d in archive.gets {
            let page_get_data = PageGetData::try_from(d)?;
            data.push(page_get_data);
        }

        self.clean_all_get_data();

        for d in data {
            self.add_get_data(d);
        }
        Ok(())
    }

    pub fn show(&mut self, ctx: &Context) {
        SidePanel::left("page_get_panel_left")
            .resizable(true)
            .show(ctx, |ui| {
                self.show_gets_name(ui);
            });

        CentralPanel::default().show(ctx, |ui| {
            let data = match self.data_map.get_mut(&self.selected_data_id) {
                None => {
                    return;
                }
                Some(o) => o,
            };

            data.show(
                ui,
                &mut self.events,
                &mut self.show_reply_viewer_window,
                &mut self.reply_viewer_window,
            );
        });

        let window = Window::new("Reply info")
            .id(Id::new("view reply window"))
            .collapsible(false)
            .scroll([true, true])
            .open(&mut self.show_reply_viewer_window)
            .resizable(true)
            .default_width(200.0)
            .min_width(200.0);

        window.show(ctx, |ui| {
            self.reply_viewer_window.show(ui);
        });
    }

    fn show_gets_name(&mut self, ui: &mut Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new(" + ").code())
                .on_hover_text("copy add")
                .clicked()
            {
                if let Some(d) = self.data_map.get(&self.selected_data_id) {
                    self.add_get_data(PageGetData::from(d));
                } else {
                    self.add_get_data(PageGetData::default());
                }
            };

            if ui
                .button(RichText::new(" - ").code())
                .on_hover_text("del")
                .clicked()
            {
                self.del_get_data(self.selected_data_id);
            };
        });

        ui.add_space(10.0);

        ScrollArea::both()
            .max_width(200.0)
            .auto_shrink([true, false])
            .show(ui, |ui| {
                dnd(ui, "page_get_list").show_vec(
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

    fn add_get_data(&mut self, mut data: PageGetData) {
        self.get_id_count += 1;
        data.id = self.get_id_count;
        self.data_map.insert(self.get_id_count, data);
        self.selected_data_id = self.get_id_count;
        self.dnd_items.push(DndItem::new(self.get_id_count))
    }

    fn del_get_data(&mut self, get_id: u64) {
        if self.data_map.len() < 2 {
            return;
        }

        let _ = self.data_map.remove(&get_id);
        let mut del_index = None;
        for (i, di) in self.dnd_items.iter().enumerate() {
            if di.key_id == get_id {
                del_index = Some(i);
                break;
            }
        }
        if let Some(i) = del_index {
            self.dnd_items.remove(i);
        }
    }

    fn clean_all_get_data(&mut self) {
        self.data_map.clear();
        self.selected_data_id = 0;
    }

    pub fn processing_get_res(&mut self, res: Box<(u64, Reply)>) {
        let (id, reply) = *res;
        if let Some(d) = self.data_map.get_mut(&id) {
            d.error_info = None;
            d.replies.push(reply);
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
