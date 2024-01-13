use arboard::Clipboard;
use eframe::{
    egui,
    egui::{
        Align, Color32, Context, DragValue, Id, Layout, RichText, ScrollArea, TextEdit, TextStyle,
        Ui,
    },
};
use egui_dnd::dnd;
use egui_extras::{Column, TableBuilder};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    str::FromStr,
    time::Duration,
};
use zenoh::{
    buffers::reader::{HasReader, Reader},
    prelude::{
        Buffer, Encoding, KnownEncoding, Locality, OwnedKeyExpr, QueryConsolidation, QueryTarget,
        Sample, Value, ZenohId,
    },
    query::{ConsolidationMode, Mode, Reply},
};

use crate::{
    app::{f64_create_rich_text, i64_create_rich_text, value_create_rich_text, ZenohValue},
    hex_viewer::HexViewer,
    zenoh::QueryData,
};

// query
pub enum Event {
    Get(Box<QueryData>),
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ZQueryTarget {
    BestMatching,
    All,
    AllComplete,
}

impl From<QueryTarget> for ZQueryTarget {
    fn from(value: QueryTarget) -> Self {
        match value {
            QueryTarget::BestMatching => ZQueryTarget::BestMatching,
            QueryTarget::All => ZQueryTarget::All,
            QueryTarget::AllComplete => ZQueryTarget::AllComplete,
        }
    }
}

impl Into<QueryTarget> for ZQueryTarget {
    fn into(self) -> QueryTarget {
        match self {
            ZQueryTarget::BestMatching => QueryTarget::BestMatching,
            ZQueryTarget::All => QueryTarget::All,
            ZQueryTarget::AllComplete => QueryTarget::AllComplete,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ZConsolidation {
    Auto,
    None,
    Monotonic,
    Latest,
}

impl From<QueryConsolidation> for ZConsolidation {
    fn from(value: QueryConsolidation) -> Self {
        match value.mode() {
            Mode::Auto => ZConsolidation::Auto,
            Mode::Manual(m) => match m {
                ConsolidationMode::None => ZConsolidation::None,
                ConsolidationMode::Monotonic => ZConsolidation::Monotonic,
                ConsolidationMode::Latest => ZConsolidation::Latest,
            },
        }
    }
}

impl Into<QueryConsolidation> for ZConsolidation {
    fn into(self) -> QueryConsolidation {
        match self {
            ZConsolidation::Auto => QueryConsolidation::AUTO,
            ZConsolidation::None => QueryConsolidation::from(ConsolidationMode::None),
            ZConsolidation::Monotonic => QueryConsolidation::from(ConsolidationMode::Monotonic),
            ZConsolidation::Latest => QueryConsolidation::from(ConsolidationMode::Latest),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ZLocality {
    SessionLocal,
    Remote,
    Any,
}

impl From<Locality> for ZLocality {
    fn from(value: Locality) -> Self {
        match value {
            Locality::SessionLocal => ZLocality::SessionLocal,
            Locality::Remote => ZLocality::Remote,
            Locality::Any => ZLocality::Any,
        }
    }
}

impl Into<Locality> for ZLocality {
    fn into(self) -> Locality {
        match self {
            ZLocality::SessionLocal => Locality::SessionLocal,
            ZLocality::Remote => Locality::Remote,
            ZLocality::Any => Locality::Any,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DataItem {
    name: String,
    key: String,
    target: ZQueryTarget,
    consolidation: ZConsolidation,
    locality: ZLocality,
    timeout: u64,
    value: ZenohValue,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Data {
    gets: Vec<DataItem>,
}

pub struct PageGetData {
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
    info: Option<RichText>,
}

impl Default for PageGetData {
    fn default() -> Self {
        PageGetData {
            id: 1,
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

impl PageGetData {
    fn from(data: &DataItem) -> PageGetData {
        let (encoding, s) = data.value.to();
        PageGetData {
            id: 0,
            name: data.name.clone(),
            input_key: data.key.clone(),
            selected_target: data.target.into(),
            selected_consolidation: data.consolidation.into(),
            selected_locality: data.locality.into(),
            timeout: data.timeout,
            edit_str: s,
            selected_encoding: encoding,
            replies: vec![],
            info: None,
        }
    }

    fn to(&self) -> DataItem {
        let value = ZenohValue::from(self.selected_encoding, self.edit_str.clone());
        DataItem {
            name: self.name.clone(),
            key: self.input_key.clone(),
            target: self.selected_target.into(),
            consolidation: self.selected_consolidation.into(),
            locality: self.selected_locality.into(),
            timeout: self.timeout,
            value,
        }
    }

    fn new_from(pgd: &PageGetData) -> Self {
        PageGetData {
            id: 0,
            name: pgd.name.clone(),
            input_key: pgd.input_key.clone(),
            selected_target: pgd.selected_target,
            selected_consolidation: pgd.selected_consolidation,
            selected_locality: pgd.selected_locality,
            timeout: pgd.timeout,
            edit_str: pgd.edit_str.clone(),
            selected_encoding: pgd.selected_encoding,
            replies: Vec::new(),
            info: None,
        }
    }

    fn show(
        &mut self,
        ui: &mut Ui,
        events: &mut VecDeque<Event>,
        show_window: &mut bool,
        reply_window: &mut ViewReplyWindow,
    ) {
        ui.vertical(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                if ui.button("send").clicked() {
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

            ScrollArea::horizontal()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.show_reply(ui, show_window, reply_window);
                });
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
                        KnownEncoding::AppSql,
                        KnownEncoding::AppXml,
                        KnownEncoding::AppXhtmlXml,
                        KnownEncoding::TextHtml,
                        KnownEncoding::TextXml,
                        KnownEncoding::TextCss,
                        KnownEncoding::TextCsv,
                        KnownEncoding::TextJavascript,
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

        let text_edit_multiline = |edit_str: &mut String, ui: &mut Ui| {
            ui.add(
                TextEdit::multiline(edit_str)
                    .desired_width(f32::INFINITY)
                    .desired_rows(3)
                    .code_editor(),
            );
        };
        match self.selected_encoding {
            KnownEncoding::Empty => {}
            KnownEncoding::TextPlain => {
                text_edit_multiline(&mut self.edit_str, ui);
            }
            KnownEncoding::AppJson => {
                text_edit_multiline(&mut self.edit_str, ui);
            }
            KnownEncoding::AppInteger => {
                ui.add(TextEdit::singleline(&mut self.edit_str));
            }
            KnownEncoding::AppFloat => {
                ui.add(TextEdit::singleline(&mut self.edit_str));
            }
            KnownEncoding::TextJson => {
                text_edit_multiline(&mut self.edit_str, ui);
            }
            KnownEncoding::AppOctetStream => {}
            KnownEncoding::AppCustom => {}
            KnownEncoding::AppProperties => {}
            KnownEncoding::AppSql => {
                text_edit_multiline(&mut self.edit_str, ui);
            }
            KnownEncoding::AppXml => {
                text_edit_multiline(&mut self.edit_str, ui);
            }
            KnownEncoding::AppXhtmlXml => {
                text_edit_multiline(&mut self.edit_str, ui);
            }
            KnownEncoding::AppXWwwFormUrlencoded => {}
            KnownEncoding::TextHtml => {
                text_edit_multiline(&mut self.edit_str, ui);
            }
            KnownEncoding::TextXml => {
                text_edit_multiline(&mut self.edit_str, ui);
            }
            KnownEncoding::TextCss => {
                text_edit_multiline(&mut self.edit_str, ui);
            }
            KnownEncoding::TextCsv => {
                text_edit_multiline(&mut self.edit_str, ui);
            }
            KnownEncoding::TextJavascript => {
                text_edit_multiline(&mut self.edit_str, ui);
            }
            KnownEncoding::ImageJpeg => {}
            KnownEncoding::ImagePng => {}
            KnownEncoding::ImageGif => {}
        };
    }

    fn show_reply(
        &mut self,
        ui: &mut Ui,
        show_window: &mut bool,
        reply_window: &mut ViewReplyWindow,
    ) {
        if self.replies.is_empty() {
            return;
        }

        let table = TableBuilder::new(ui)
            .striped(true)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::initial(100.0).resizable(true).clip(true))
            .column(Column::auto())
            .column(Column::auto())
            .column(Column::remainder())
            .resizable(true);

        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label("key");
                });
                header.col(|ui| {
                    ui.label("value");
                });
                header.col(|ui| {
                    ui.label("type");
                });
                header.col(|ui| {
                    ui.label("timestamp");
                });
            })
            .body(|mut body| {
                for reply in &self.replies {
                    body.row(20.0, |mut row| {
                        match &reply.sample {
                            Ok(o) => {
                                let text_key = o.key_expr.to_string();
                                let text_timestamp = match o.timestamp {
                                    None => "-".to_string(),
                                    Some(s) => s.to_string(),
                                };
                                let text_type = format!("{}", o.encoding);
                                let text_button = value_create_rich_text(&o.value);
                                row.col(|ui| {
                                    ui.label(text_key);
                                });
                                row.col(|ui| {
                                    if let Some(text) = text_button {
                                        if ui.button(text).clicked() {
                                            reply_window.reply = Some(reply.clone());
                                            *show_window = true;
                                        }
                                    } else {
                                        ui.label("...");
                                    }
                                });
                                row.col(|ui| {
                                    ui.label(text_type);
                                });
                                row.col(|ui| {
                                    ui.label(text_timestamp);
                                });
                            }
                            Err(e) => {
                                let text = String::try_from(e).unwrap();
                                let text = RichText::new(text).size(12.0).color(Color32::RED);
                                row.col(|ui| {
                                    ui.label(text);
                                });
                                row.col(|ui| {
                                    if ui.button("...").clicked() {
                                        reply_window.reply = Some(reply.clone());
                                        *show_window = true;
                                    }
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
            });
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
            KnownEncoding::AppOctetStream => None,
            KnownEncoding::AppCustom => None,
            KnownEncoding::AppProperties => None,
            KnownEncoding::AppXWwwFormUrlencoded => None,
            KnownEncoding::ImageJpeg => None,
            KnownEncoding::ImagePng => None,
            KnownEncoding::ImageGif => None,
            str_encoding => {
                let v = Value::from(self.edit_str.as_str()).encoding(str_encoding.into());
                Some(v)
            }
        };
        let d = QueryData {
            id: self.id,
            key_expr: key,
            target: self.selected_target,
            consolidation: self.selected_consolidation,
            locality: self.selected_locality,
            timeout: Duration::from_millis(self.timeout),
            value: v,
        };
        events.push_back(Event::Get(Box::new(d)));
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ViewReplyWindowPage {
    Raw,
    Parse,
}

struct ViewReplyWindow {
    selected_page: ViewReplyWindowPage,
    reply: Option<Reply>,
    hex_viewer: HexViewer,
}

impl Default for ViewReplyWindow {
    fn default() -> Self {
        ViewReplyWindow {
            selected_page: ViewReplyWindowPage::Parse,
            reply: None,
            hex_viewer: HexViewer::new(vec![]),
        }
    }
}

impl ViewReplyWindow {
    fn show(&mut self, ctx: &Context, is_open: &mut bool) {
        let window = egui::Window::new("Info")
            .id(Id::new("view reply window"))
            .collapsible(false)
            .scroll2([true, true])
            .open(is_open)
            .resizable(true)
            .default_width(200.0)
            .min_width(200.0);

        window.show(ctx, |ui| {
            let reply = match &self.reply {
                None => {
                    ui.label("none");
                    return;
                }
                Some(s) => s,
            };

            match &reply.sample {
                Ok(sample) => {
                    Self::show_base_info(reply.replier_id, sample, ui);

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.selectable_value(
                            &mut self.selected_page,
                            ViewReplyWindowPage::Parse,
                            "parse",
                        );

                        if ui
                            .selectable_value(
                                &mut self.selected_page,
                                ViewReplyWindowPage::Raw,
                                "raw",
                            )
                            .clicked()
                        {
                            if let Some(reply) = &self.reply {
                                if let Ok(sample) = &reply.sample {
                                    let value = &sample.value;
                                    let data_len = value.payload.len();
                                    let mut data: Vec<u8> = Vec::with_capacity(data_len);
                                    data.resize(data_len, 0);
                                    let _ = value.payload.reader().read_exact(data.as_mut_slice());
                                    self.hex_viewer = HexViewer::new(data);
                                }
                            }
                        }
                    });

                    match self.selected_page {
                        ViewReplyWindowPage::Raw => {
                            self.hex_viewer.show(ui);
                        }
                        ViewReplyWindowPage::Parse => {
                            Self::show_page_parse(&sample.value, ui);
                        }
                    };
                }
                Err(value) => {
                    ui.horizontal(|ui| {
                        if ui.button("replier id:").on_hover_text("copy").clicked() {
                            let mut clipboard = Clipboard::new().unwrap();
                            clipboard.set_text(reply.replier_id.to_string()).unwrap();
                        }
                        let text = RichText::new(reply.replier_id.to_string()).monospace();
                        ui.label(text);
                    });

                    let text: RichText = match String::try_from(value) {
                        Ok(o) => RichText::new(o).monospace().color(Color32::RED),
                        Err(e) => RichText::new(e.to_string()).monospace().color(Color32::RED),
                    };
                    ui.label(text);
                }
            }
        });
    }

    fn show_page_parse(value: &Value, ui: &mut Ui) {
        match value.encoding.prefix() {
            KnownEncoding::TextPlain => {
                let mut s = match String::try_from(value) {
                    Ok(s) => s,
                    Err(e) => format!("{}", e),
                };
                ui.add(
                    TextEdit::multiline(&mut s)
                        .desired_width(f32::INFINITY)
                        .code_editor(),
                );
            }
            KnownEncoding::AppJson => {
                let mut s: String = match serde_json::Value::try_from(value) {
                    Ok(o) => {
                        format!("{:#}", o)
                    }
                    Err(e) => {
                        format!("{}", e)
                    }
                };
                ui.add(
                    TextEdit::multiline(&mut s)
                        .desired_width(f32::INFINITY)
                        .code_editor(),
                );
            }
            KnownEncoding::AppInteger => {
                let text: RichText = i64_create_rich_text(value);
                ui.label(text);
            }
            KnownEncoding::AppFloat => {
                let text: RichText = f64_create_rich_text(value);
                ui.label(text);
            }
            KnownEncoding::TextJson => {
                let mut s: String = match serde_json::Value::try_from(value) {
                    Ok(o) => {
                        format!("{:#}", o)
                    }
                    Err(e) => {
                        format!("{}", e)
                    }
                };
                ui.add(
                    TextEdit::multiline(&mut s)
                        .desired_width(f32::INFINITY)
                        .code_editor(),
                );
            }
            _ => {}
        }
    }

    fn show_base_info(replier_id: ZenohId, sample: &Sample, ui: &mut Ui) {
        let show_ui = |ui: &mut Ui| {
            if ui.button("replier id:").on_hover_text("copy").clicked() {
                let mut clipboard = Clipboard::new().unwrap();
                clipboard.set_text(replier_id.to_string()).unwrap();
            }
            let text = RichText::new(replier_id.to_string()).monospace();
            ui.label(text);
            ui.end_row();

            if ui.button("key:").on_hover_text("copy").clicked() {
                let mut clipboard = Clipboard::new().unwrap();
                clipboard.set_text(sample.key_expr.as_str()).unwrap();
            }
            let text = RichText::new(sample.key_expr.as_str()).monospace();
            ui.label(text);
            ui.end_row();

            ui.label("kind:  ");
            let text = RichText::new(sample.kind.to_string()).monospace();
            ui.label(text);
            ui.end_row();

            ui.label("encoding:  ");
            let text = RichText::new(format!("{}", sample.value.encoding)).monospace();
            ui.label(text);
            ui.end_row();

            ui.label("timestamp:  ");
            let text = if let Some(t) = sample.timestamp {
                RichText::new(t.to_string().replace('/', "\n")).monospace()
            } else {
                RichText::new("none").monospace()
            };
            ui.label(text);
            ui.end_row();
        };

        egui::Grid::new("base_info").num_columns(2).show(ui, |ui| {
            show_ui(ui);
        });
    }
}

pub struct PageGet {
    pub events: VecDeque<Event>,
    pub data_map: BTreeMap<u64, PageGetData>,
    selected_data_id: u64,
    get_id_count: u64,
    show_view_reply_window: bool,
    view_reply_window: ViewReplyWindow,
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
            show_view_reply_window: false,
            view_reply_window: ViewReplyWindow::default(),
            // dnd: DragDropUi::default(),
            dnd_items: Vec::new(),
        };
        p.add_get_data(PageGetData::default());
        p
    }
}

impl PageGet {
    pub fn load(&mut self, data: Data) {
        self.clean_all_get_data();

        for d in data.gets {
            let page_data = PageGetData::from(&d);
            self.add_get_data(page_data);
        }
    }

    pub fn create_store_data(&self) -> Data {
        let data = self
            .dnd_items
            .iter()
            .filter_map(|k| self.data_map.get(&k.key_id))
            .map(|d| d.to())
            .collect();
        Data { gets: data }
    }

    pub fn show(&mut self, ctx: &Context) {
        egui::SidePanel::left("page_get_panel_left")
            .resizable(true)
            .show(ctx, |ui| {
                self.show_gets_name(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let data = match self.data_map.get_mut(&self.selected_data_id) {
                None => {
                    return;
                }
                Some(o) => o,
            };

            data.show(
                ui,
                &mut self.events,
                &mut self.show_view_reply_window,
                &mut self.view_reply_window,
            );

            self.view_reply_window
                .show(ctx, &mut self.show_view_reply_window);
        });
    }

    fn show_gets_name(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(RichText::new(" + ").code())
                    .on_hover_text("copy add")
                    .clicked()
                {
                    if let Some(d) = self.data_map.get(&self.selected_data_id) {
                        self.add_get_data(PageGetData::new_from(d));
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

            ui.label(" ");

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
                                    ui.selectable_value(
                                        &mut self.selected_data_id,
                                        item.key_id,
                                        text,
                                    );
                                });
                            }
                        },
                    )
                });
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
            d.info = None;
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
