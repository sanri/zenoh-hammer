use arboard::Clipboard;
use eframe::egui::plot::{Corner, Legend, Plot, PlotImage, PlotPoint};
use eframe::egui::{ColorImage, TextureOptions};
use eframe::{
    egui,
    egui::{
        Align, CollapsingHeader, Color32, Context, DragValue, Id, Layout, RichText, ScrollArea,
        TextEdit, TextStyle, TextureHandle, Ui,
    },
};
use egui_dnd::{utils::shift_vec, DragDropItem, DragDropUi};
use egui_extras::{Column, TableBody, TableBuilder};
use image;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    str::FromStr,
};
use zenoh::{
    prelude::{Encoding, KnownEncoding, OwnedKeyExpr, SampleKind, SplitBuffer, Value},
    sample::Sample,
    time::{new_reception_timestamp, Timestamp, TimestampId, NTP64},
};

use crate::{
    app::value_create_rich_text,
    hex_viewer::{HexViewer, HEX_VIEWER_SIZE},
};

pub const VALUE_BUFFER_SIZE_DEFAULT: usize = 10;

pub enum Event {
    AddSub(Box<(u64, OwnedKeyExpr)>), // id, key expr
    DelSub(u64),                      // id
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct DataItem {
    name: String,
    key_expr: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Data {
    subscribers: Vec<DataItem>,
}

pub struct PageSub {
    pub events: VecDeque<Event>,
    sub_id_count: u64,
    selected_sub_id: u64,
    show_view_value_window: bool,
    view_value_window: ViewValueWindow,
    sub_data_group: BTreeMap<u64, PageSubData>, // <sub id, group>
    dnd: DragDropUi,
    dnd_items: Vec<DndItem>,
}

impl Default for PageSub {
    fn default() -> Self {
        let mut p = PageSub {
            events: VecDeque::new(),
            sub_id_count: 0,
            selected_sub_id: 1,
            show_view_value_window: false,
            view_value_window: ViewValueWindow::default(),
            sub_data_group: BTreeMap::new(),
            dnd: DragDropUi::default(),
            dnd_items: Vec::new(),
        };
        p.add_sub_data(PageSubData::new(format!("demo"), format!("demo/**")));
        p
    }
}

impl PageSub {
    pub fn load(&mut self, data: Data) {
        self.clean_all_sub_data();

        for d in data.subscribers {
            let page_data = PageSubData::from(&d);
            self.add_sub_data(page_data);
        }
    }

    pub fn create_store_data(&self) -> Data {
        let data = self
            .dnd_items
            .iter()
            .filter_map(|k| self.sub_data_group.get(&k.key_id))
            .map(|d| d.to())
            .collect();
        Data { subscribers: data }
    }

    pub fn show(&mut self, ctx: &Context) {
        egui::SidePanel::left("page_sub_panel_left")
            .resizable(true)
            .show(ctx, |ui| {
                self.show_subscribers_name(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_name_key(ui);

            ui.separator();

            ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
                self.show_key_tree(ui);

                ui.separator();

                self.show_values(ui);
            });
        });

        self.view_value_window
            .show(ctx, &mut self.show_view_value_window);
    }

    fn add_sub_data(&mut self, data: PageSubData) {
        self.sub_id_count += 1;
        self.sub_data_group.insert(self.sub_id_count, data);
        self.selected_sub_id = self.sub_id_count;
        self.dnd_items.push(DndItem::new(self.sub_id_count))
    }

    fn del_sub_data(&mut self, sub_id: u64) {
        if self.sub_data_group.len() < 2 {
            return;
        }

        let mut flag = false;
        if let Some(data_group) = self.sub_data_group.get(&sub_id) {
            flag = !data_group.subscribed;
        }
        if flag {
            let _ = self.sub_data_group.remove(&sub_id);
            let mut del_index = None;
            for (i, di) in self.dnd_items.iter().enumerate() {
                if di.key_id == sub_id {
                    del_index = Some(i);
                    break;
                }
            }
            if let Some(i) = del_index {
                self.dnd_items.remove(i);
            }
        }
    }

    fn clean_all_sub_data(&mut self) {
        self.sub_data_group.clear();
        self.dnd_items.clear();
        self.selected_sub_id = 0;
    }

    fn show_subscribers_name(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new(" + ").code())
                .on_hover_text("copy add")
                .clicked()
            {
                if let Some(d) = self.sub_data_group.get(&self.selected_sub_id) {
                    self.add_sub_data(PageSubData::new_from(d));
                } else {
                    self.add_sub_data(PageSubData::new(format!("demo"), format!("demo/**")));
                }
            }

            if ui
                .button(RichText::new(" - ").code())
                .on_hover_text("del")
                .clicked()
            {
                self.del_sub_data(self.selected_sub_id);
            }
        });

        ui.label(" ");

        ScrollArea::both()
            .max_width(200.0)
            .auto_shrink([true, false])
            .show(ui, |ui| {
                let response =
                    self.dnd
                        .ui::<DndItem>(ui, self.dnd_items.iter_mut(), |item, ui, handle| {
                            ui.horizontal(|ui| {
                                if let Some(d) = self.sub_data_group.get(&item.key_id) {
                                    handle.ui(ui, item, |ui| {
                                        ui.label("·");
                                    });

                                    let text = if d.subscribed {
                                        RichText::new(d.name.as_str()).underline().strong()
                                    } else {
                                        RichText::new(d.name.as_str())
                                    };
                                    ui.selectable_value(
                                        &mut self.selected_sub_id,
                                        item.key_id,
                                        text,
                                    );
                                }
                            });
                        });

                if let Some(response) = response.completed {
                    shift_vec(response.from, response.to, &mut self.dnd_items);
                }
            });
    }

    fn show_name_key(&mut self, ui: &mut Ui) {
        let data_group = match self.sub_data_group.get_mut(&self.selected_sub_id) {
            None => {
                return;
            }
            Some(o) => o,
        };

        ui.horizontal(|ui| {
            ui.label(RichText::new("name:     ").monospace());
            let te = TextEdit::singleline(&mut data_group.name)
                .desired_width(600.0)
                .font(TextStyle::Monospace)
                .interactive(!data_group.subscribed);
            ui.add(te);

            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                if ui
                    .selectable_label(data_group.subscribed, "declare")
                    .clicked()
                {
                    if !data_group.subscribed {
                        let key_expr_str =
                            data_group.key_expr.replace(&[' ', '\t', '\n', '\r'], "");

                        if key_expr_str.is_empty() {
                            let rt = format!("key expr is empty");
                            data_group.err_str = Some(rt);
                            return;
                        }

                        let key: OwnedKeyExpr = match OwnedKeyExpr::from_str(key_expr_str.as_str())
                        {
                            Ok(o) => o,
                            Err(e) => {
                                data_group.err_str = Some(e.to_string());
                                return;
                            }
                        };

                        self.events
                            .push_back(Event::AddSub(Box::new((self.selected_sub_id, key))));
                    } else {
                        self.events.push_back(Event::DelSub(self.selected_sub_id));
                    }
                    data_group.err_str = None;
                    data_group.subscribed = !data_group.subscribed;
                }
            });
        });

        ui.horizontal(|ui| {
            ui.label(RichText::new("key expr: ").monospace());
            let te = TextEdit::multiline(&mut data_group.key_expr)
                .desired_width(600.0)
                .desired_rows(1)
                .font(TextStyle::Monospace)
                .interactive(!data_group.subscribed);
            ui.add(te);
        });

        if let Some(e) = &data_group.err_str {
            ui.label(RichText::new(e).color(Color32::RED));
        }
    }

    fn show_key_tree(&mut self, ui: &mut Ui) {
        let data_group = match self.sub_data_group.get_mut(&self.selected_sub_id) {
            None => {
                return;
            }
            Some(o) => o,
        };

        data_group.update_tree();

        ScrollArea::both()
            .id_source("scroll_area_tree")
            .max_width(200.0)
            .min_scrolled_width(200.0)
            // .min_scrolled_height(1000.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut data_group.filtered, "filter");
                        ui.add(
                            TextEdit::singleline(&mut data_group.filter_str)
                                .code_editor()
                                .interactive(data_group.filtered),
                        );
                    });

                    data_group
                        .key_tree
                        .show_ui(&mut data_group.selected_key, ui);
                });
            });
    }

    fn show_values(&mut self, ui: &mut Ui) {
        let data_group = match self.sub_data_group.get_mut(&self.selected_sub_id) {
            None => {
                return;
            }
            Some(o) => o,
        };

        let selected_key = data_group.selected_key.clone();

        let mut frequency: Option<Frequency> = None;
        if let Some(dv) = data_group.map.get(&selected_key) {
            data_group.buffer_size = dv.buffer_size as u32;
            frequency = dv.compute_frequency();
        }

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if ui.button("key:").on_hover_text("copy key").clicked() {
                    let mut clipboard = Clipboard::new().unwrap();
                    clipboard.set_text(selected_key.clone()).unwrap();
                }
                ui.label(RichText::new(&selected_key).monospace());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("clean").on_hover_text("clean buffer").clicked() {
                        if let Some(dv) = data_group.map.get_mut(&selected_key) {
                            dv.clear();
                        }
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.label(format!("buffer size: {}", data_group.buffer_size));
                ui.label("   ");
                let dv = DragValue::new(&mut data_group.buffer_size_tmp)
                    .speed(10.0)
                    .clamp_range(VALUE_BUFFER_SIZE_DEFAULT..=10000);
                ui.add(dv);
                if ui.button("update buffer").clicked() {
                    if let Some(data) = data_group.map.get_mut(&selected_key) {
                        data.set_buffer_size(data_group.buffer_size_tmp as usize);
                    }
                }

                if let Some(fr) = frequency {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(fr.to_string());
                    });
                }
            });

            let show_body = |mut body: TableBody| {
                let key = &selected_key;
                if let Some(sd) = data_group.map.get(selected_key.as_str()) {
                    for (d, k, t) in &sd.deque {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                let text: Option<RichText> = value_create_rich_text(d);
                                if let Some(text) = text {
                                    if ui.button(text).clicked() {
                                        self.show_view_value_window = true;
                                        self.view_value_window.clone_from(d, key, k, t);
                                    }
                                } else {
                                    ui.label("...");
                                }
                            });
                            row.col(|ui| {
                                let text = format!("{}", d.encoding);
                                ui.label(text);
                            });
                            row.col(|ui| {
                                if let Some(timestamp) = t {
                                    let text = RichText::new(format!("{}", timestamp.get_time()))
                                        .size(12.0);
                                    ui.label(text);
                                } else {
                                    ui.label("-");
                                }
                            });
                        });
                    }
                }
            };

            ScrollArea::horizontal()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let table = TableBuilder::new(ui)
                        .striped(true)
                        .cell_layout(Layout::left_to_right(Align::Center))
                        .column(
                            Column::initial(40.0)
                                .range(40.0..=160.0)
                                .resizable(true)
                                .clip(true),
                        )
                        .column(Column::auto())
                        .column(Column::remainder())
                        .resizable(true);

                    table
                        .header(20.0, |mut header| {
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
                        .body(show_body);
                });
        });
    }

    pub fn processing_sub_cb(&mut self, id: u64, sample: Sample) {
        let key = sample.key_expr.as_str();
        if let Some(data_group) = self.sub_data_group.get_mut(&id) {
            if let Some(sv) = data_group.map.get_mut(key) {
                sv.add_data((sample.value, sample.kind, sample.timestamp));
            } else {
                println!("new key: {}", key);
                let mut sv = DataValues::default();
                sv.add_data((sample.value, sample.kind, sample.timestamp));
                let _ = data_group.map.insert(key.to_string(), sv);
            }
        }
    }

    pub fn processing_del_sub_res(&mut self, id: u64) {
        if let Some(data_group) = self.sub_data_group.get_mut(&id) {
            data_group.subscribed = false;
        }
    }

    pub fn processing_add_sub_res(&mut self, id: u64, r: Result<(), String>) {
        if let Some(data_group) = self.sub_data_group.get_mut(&id) {
            match r {
                Ok(_) => {
                    data_group.err_str = None;
                    data_group.subscribed = true;
                }
                Err(e) => {
                    data_group.err_str = Some(e);
                    data_group.subscribed = false;
                }
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
        Id::new(format!("page_sub dnd_item {}", self.key_id))
    }
}

enum Frequency {
    Hz(f32), // 单位时间内收到的消息数量. > 1.0
    S(u32),  // 距离上次收到消息过去多少秒. > 1
}

impl Frequency {
    fn to_string(&self) -> String {
        match self {
            Frequency::Hz(v) => {
                if *v < 10.0 {
                    format!("{:.1} Hz", *v)
                } else {
                    format!("{:.0} Hz", *v)
                }
            }
            Frequency::S(v) => {
                format!("{} s", *v)
            }
        }
    }
}

struct DataValues {
    deque: VecDeque<(Value, SampleKind, Option<Timestamp>)>,
    buffer_size: usize,
    lately_local_timestamp: Timestamp,
}

impl Default for DataValues {
    fn default() -> Self {
        DataValues {
            deque: VecDeque::with_capacity(VALUE_BUFFER_SIZE_DEFAULT),
            buffer_size: VALUE_BUFFER_SIZE_DEFAULT,
            lately_local_timestamp: new_reception_timestamp(),
        }
    }
}

impl DataValues {
    fn add_data(&mut self, d: (Value, SampleKind, Option<Timestamp>)) {
        let local_timestamp = if let Some(t) = d.2 {
            if *t.get_id() == TimestampId::try_from([1]).unwrap() {
                t
            } else {
                new_reception_timestamp()
            }
        } else {
            new_reception_timestamp()
        };
        self.lately_local_timestamp = local_timestamp;
        if self.deque.len() == self.buffer_size {
            let _ = self.deque.pop_front();
        }
        let _ = self.deque.push_back(d);
    }

    fn clear(&mut self) {
        self.deque.clear();
    }

    fn set_buffer_size(&mut self, size: usize) {
        self.buffer_size = if size < VALUE_BUFFER_SIZE_DEFAULT {
            VALUE_BUFFER_SIZE_DEFAULT
        } else {
            size
        };
        while self.deque.len() >= self.buffer_size {
            let _ = self.deque.pop_front();
        }
    }

    fn compute_frequency(&self) -> Option<Frequency> {
        if self.deque.is_empty() {
            return None;
        }

        let timestamp_now = new_reception_timestamp();
        let dt = timestamp_now
            .get_diff_duration(&self.lately_local_timestamp)
            .as_secs() as u32;
        if dt >= 1 {
            return Some(Frequency::S(dt));
        } else {
            if self.deque.len() == 1 {
                return Some(Frequency::Hz(1.0));
            }
        }

        // 检索最多100条或最近10s内的记录
        let mut ntp_max = NTP64(0u64);
        let mut ntp_min = NTP64(u64::MAX);
        let mut count = 0f32;
        let mut time = 1f32;
        for (i, (_, _, t)) in self.deque.iter().rev().enumerate() {
            let ntp = *t.unwrap().get_time();
            ntp_max = ntp_max.max(ntp);
            ntp_min = ntp_min.min(ntp);
            count = i as f32;
            time = (ntp_max - ntp_min).to_duration().as_secs_f32();
            if i >= 100 {
                break;
            }
            if time >= 10.0 {
                break;
            }
        }
        let fr = count / time;

        Some(Frequency::Hz(fr))
    }
}

pub struct PageSubData {
    subscribed: bool,
    name: String,
    key_expr: String,
    err_str: Option<String>,
    selected_key: String,
    filtered: bool,
    filter_str: String,
    buffer_size_tmp: u32,
    buffer_size: u32,
    key_tree: Tree,                    // tree be show
    map: BTreeMap<String, DataValues>, // key
}

impl PageSubData {
    fn from(data: &DataItem) -> PageSubData {
        PageSubData::new(data.name.clone(), data.key_expr.clone())
    }

    fn to(&self) -> DataItem {
        DataItem {
            name: self.name.clone(),
            key_expr: self.key_expr.clone(),
        }
    }

    fn new(name: String, key_expr: String) -> PageSubData {
        PageSubData {
            subscribed: false,
            name,
            key_expr,
            err_str: None,
            selected_key: "".to_string(),
            filtered: false,
            filter_str: "".to_string(),
            buffer_size_tmp: VALUE_BUFFER_SIZE_DEFAULT as u32,
            buffer_size: VALUE_BUFFER_SIZE_DEFAULT as u32,
            key_tree: Tree::default(),
            map: BTreeMap::new(),
        }
    }

    fn new_from(psd: &Self) -> Self {
        Self::new(psd.name.clone(), psd.key_expr.clone())
    }

    fn update_tree(&mut self) {
        let keys = self.map.keys().cloned().collect();
        let keys = if self.filtered {
            filter(&keys, self.filter_str.as_str())
        } else {
            keys
        };

        self.key_tree = Tree::new(&keys);
    }
}

#[derive(Default)]
struct Tree {
    index_top_node: BTreeMap<String, u32>, // <top node name, node index>,
    mem: Vec<TreeNode>,
}

impl Tree {
    pub fn new(key_list: &Vec<String>) -> Tree {
        let mut tree = Tree::default();
        for key in key_list {
            tree.add_node(key);
        }
        tree
    }

    pub fn show_ui(&self, selected_key: &mut String, ui: &mut Ui) {
        for (_, index_top) in &self.index_top_node {
            let top_node: &TreeNode = self.mem.get((*index_top) as usize).unwrap();
            top_node.show_ui(self, selected_key, ui);
        }
    }

    fn new_node(&mut self) -> &mut TreeNode {
        let index = self.mem.len();
        self.mem.push(TreeNode {
            name: String::new(),
            index_own: index as u32,
            index_parent: u32::MAX,
            index_children: BTreeMap::new(),
            key: None,
        });
        self.mem.last_mut().unwrap()
    }

    fn add_node(&mut self, key: &str) {
        // let mut index_now = u32::MAX;
        let mut index_now;
        let mut split = key.split('/');
        if let Some(name) = split.next() {
            if let Some(index) = self.index_top_node.get(name) {
                index_now = *index;
            } else {
                let node = self.new_node();
                node.set_name(name);
                index_now = node.index_own;
                self.index_top_node.insert(name.to_string(), index_now);
            }
        } else {
            return;
        }
        'a: loop {
            if let Some(name) = split.next() {
                {
                    let parent: &mut TreeNode = self.mem.get_mut(index_now as usize).unwrap();
                    if let Some(index) = parent.index_children.get(name) {
                        index_now = *index;
                        continue 'a;
                    }
                }
                // let mut index_child: u32 = u32::MAX;
                let index_child: u32;
                {
                    let node = self.new_node();
                    node.set_name(name);
                    node.set_parent(index_now);
                    index_child = node.index_own;
                }
                {
                    let parent: &mut TreeNode = self.mem.get_mut(index_now as usize).unwrap();
                    parent.index_children.insert(name.to_string(), index_child);
                    index_now = index_child;
                }
            } else {
                let nd: &mut TreeNode = self.mem.get_mut(index_now as usize).unwrap();
                nd.set_key(key);
                break 'a;
            }
        }
    }
}

struct TreeNode {
    name: String,
    index_own: u32,
    index_parent: u32,                     // 当为顶级节点时，此值为 u32::MAX
    index_children: BTreeMap<String, u32>, // <child name, child index>, 若没有子节点，此数组为空
    key: Option<String>,                   // 当为叶节点时，存储完整key值
}

impl TreeNode {
    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn set_parent(&mut self, index: u32) {
        self.index_parent = index;
    }

    fn set_key(&mut self, key: &str) {
        self.key = Some(key.to_string());
    }

    fn show_ui<'a>(&'a self, tree: &'a Tree, selected_key: &'a mut String, ui: &'a mut Ui) {
        let name = self.name.clone();
        if let Some(k) = self.key.clone() {
            if ui
                .selectable_label(*selected_key == k, name.clone())
                .clicked()
            {
                *selected_key = k;
            }
        }
        if self.index_children.is_empty() {
            return;
        }
        CollapsingHeader::new(name)
            .default_open(false)
            .show(ui, |ui| {
                for (_, index_child) in &self.index_children {
                    let child: &TreeNode = tree.mem.get((*index_child) as usize).unwrap();
                    child.show_ui(tree, selected_key, ui);
                }
            });
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ViewValueWindowPage {
    Raw,
    Parse,
}

pub struct ViewValueWindow {
    selected_page: ViewValueWindowPage,
    key: String,
    encoding: Encoding,
    value_str: Option<String>,
    value_image: Option<ColorImage>,
    image_texture_handle: Option<TextureHandle>,
    error_str: Option<String>,
    kind: SampleKind,
    timestamp: Option<Timestamp>,
    hex_viewer: HexViewer,
}

impl Default for ViewValueWindow {
    fn default() -> Self {
        ViewValueWindow {
            selected_page: ViewValueWindowPage::Parse,
            key: String::new(),
            encoding: Encoding::default(),
            value_str: None,
            value_image: None,
            image_texture_handle: None,
            error_str: None,
            kind: SampleKind::Put,
            timestamp: None,
            hex_viewer: HexViewer::new(vec![]),
        }
    }
}

impl ViewValueWindow {
    fn show(&mut self, ctx: &Context, is_open: &mut bool) {
        let window = egui::Window::new("Info")
            .id(Id::new("view value window"))
            .collapsible(false)
            .scroll2([true, true])
            .open(is_open)
            .resizable(true)
            .default_width(200.0)
            .min_width(200.0);

        window.show(ctx, |ui| {
            self.show_base_info(ui);

            ui.separator();

            self.show_tab_label(ui);

            match self.selected_page {
                ViewValueWindowPage::Raw => {
                    self.show_page_raw(ui);
                }
                ViewValueWindowPage::Parse => {
                    self.show_page_parse(ui);
                }
            };
        });
    }

    fn clone_from_bin(&mut self) {
        self.value_str = None;
        self.value_image = None;
        self.error_str = None;
        self.image_texture_handle = None;
    }

    fn clone_from_str(&mut self, data: Vec<u8>) {
        match String::from_utf8(data) {
            Ok(s) => {
                self.value_str = Some(s);
                self.value_image = None;
                self.error_str = None;
            }
            Err(e) => {
                self.value_str = None;
                self.value_image = None;
                self.error_str = Some(e.to_string());
            }
        }
        self.image_texture_handle = None;
    }

    fn clone_from_str_to_json(&mut self, data: Vec<u8>) {
        match String::from_utf8(data) {
            Ok(s) => {
                match serde_json::value::Value::from_str(s.as_str()) {
                    Ok(v) => {
                        self.value_str = Some(serde_json::to_string_pretty(&v).unwrap());
                        self.error_str = None;
                    }
                    Err(e) => {
                        self.value_str = Some(s);
                        self.error_str = Some(e.to_string());
                    }
                }
                self.value_image = None;
            }
            Err(e) => {
                self.value_str = None;
                self.value_image = None;
                self.error_str = Some(e.to_string());
            }
        }
        self.image_texture_handle = None;
    }

    fn clone_from_str_to<T>(&mut self, data: Vec<u8>)
    where
        <T as FromStr>::Err: std::fmt::Display,
        T: FromStr,
    {
        match String::from_utf8(data) {
            Ok(s) => {
                if let Err(e) = s.parse::<T>() {
                    self.error_str = Some(format!("{}", e));
                } else {
                    self.error_str = None;
                }
                self.value_str = Some(s);
                self.value_image = None;
            }
            Err(e) => {
                self.value_str = None;
                self.value_image = None;
                self.error_str = Some(e.to_string());
            }
        }
    }

    fn clone_from_image(&mut self, data: &[u8]) {
        match image::load_from_memory(data) {
            Ok(m) => {
                let image_buffer = m.into_rgb8();
                let image_size = [
                    image_buffer.width() as usize,
                    image_buffer.height() as usize,
                ];
                let pixels = image_buffer.as_flat_samples();
                let color_image = ColorImage::from_rgba_unmultiplied(image_size, pixels.as_slice());
                self.value_str = None;
                self.value_image = Some(color_image);
                self.error_str = None;
            }
            Err(e) => {
                self.value_str = None;
                self.value_image = None;
                self.error_str = Some(e.to_string());
            }
        }
        self.image_texture_handle = None;
    }

    fn clone_from(
        &mut self,
        value: &Value,
        key: &String,
        kind: &SampleKind,
        timestamp: &Option<Timestamp>,
    ) {
        self.key = key.clone();
        self.kind = kind.clone();
        self.timestamp = timestamp.clone();
        let mut data: Vec<u8> = Vec::from(value.payload.contiguous());
        self.encoding = value.encoding.clone();
        match value.encoding.prefix() {
            KnownEncoding::Empty => {
                self.clone_from_bin();
            }
            KnownEncoding::AppOctetStream => {
                self.clone_from_bin();
            }
            KnownEncoding::AppCustom => {
                self.clone_from_bin();
            }
            KnownEncoding::TextPlain => {
                self.clone_from_str(data.clone());
            }
            KnownEncoding::AppProperties => {
                self.clone_from_bin();
            }
            KnownEncoding::AppJson => {
                self.clone_from_str_to_json(data.clone());
            }
            KnownEncoding::AppSql => {
                self.clone_from_str(data.clone());
            }
            KnownEncoding::AppInteger => {
                self.clone_from_str_to::<i64>(data.clone());
            }
            KnownEncoding::AppFloat => {
                self.clone_from_str_to::<f64>(data.clone());
            }
            KnownEncoding::AppXml => {
                self.clone_from_str(data.clone());
            }
            KnownEncoding::AppXhtmlXml => {
                self.clone_from_str(data.clone());
            }
            KnownEncoding::AppXWwwFormUrlencoded => {
                self.clone_from_str(data.clone());
            }
            KnownEncoding::TextJson => {
                self.clone_from_str_to_json(data.clone());
            }
            KnownEncoding::TextHtml => {
                self.clone_from_str(data.clone());
            }
            KnownEncoding::TextXml => {
                self.clone_from_str(data.clone());
            }
            KnownEncoding::TextCss => {
                self.clone_from_str(data.clone());
            }
            KnownEncoding::TextCsv => {
                self.clone_from_str(data.clone());
            }
            KnownEncoding::TextJavascript => {
                self.clone_from_str(data.clone());
            }
            KnownEncoding::ImageJpeg => {
                self.clone_from_image(data.as_slice());
            }
            KnownEncoding::ImagePng => {
                self.clone_from_image(data.as_slice());
            }
            KnownEncoding::ImageGif => {
                self.value_str = None;
                self.value_image = None;
                self.error_str = Some(format!("Display not supported"));
            }
        };

        if data.len() < HEX_VIEWER_SIZE {
            self.hex_viewer = HexViewer::new(data);
        } else {
            data.resize(HEX_VIEWER_SIZE, 0);
            self.hex_viewer = HexViewer::new(data);
        }
    }

    fn show_base_info(&mut self, ui: &mut Ui) {
        let show_ui = |ui: &mut Ui| {
            ui.label("key:  ");
            let text = RichText::new(self.key.as_str()).monospace();
            ui.label(text);
            ui.end_row();

            ui.label("kind:  ");
            let text = RichText::new(self.kind.to_string()).monospace();
            ui.label(text);
            ui.end_row();

            ui.label("encoding:  ");
            let text = RichText::new(format!("{}", self.encoding)).monospace();
            ui.label(text);
            ui.end_row();

            ui.label("timestamp:  ");
            let text = if let Some(t) = self.timestamp {
                RichText::new(t.to_string().replace('/', "\n")).monospace()
            } else {
                RichText::new("none").monospace()
            };
            ui.label(text);
            ui.end_row();
        };

        egui::Grid::new("config_grid")
            .num_columns(2)
            .show(ui, |ui| {
                show_ui(ui);
            });
    }

    fn show_tab_label(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.selected_page == ViewValueWindowPage::Parse, "parse")
                .clicked()
            {
                self.selected_page = ViewValueWindowPage::Parse;
            }

            if ui
                .selectable_label(self.selected_page == ViewValueWindowPage::Raw, "raw")
                .clicked()
            {
                self.selected_page = ViewValueWindowPage::Raw;
            }
        });
    }

    fn show_page_raw(&mut self, ui: &mut Ui) {
        self.hex_viewer.show(ui);
    }

    fn show_page_parse(&mut self, ui: &mut Ui) {
        match self.encoding.prefix() {
            KnownEncoding::TextPlain => {
                self.show_text_multiline(ui);
            }
            KnownEncoding::AppJson => {
                self.show_text_multiline(ui);
            }
            KnownEncoding::AppInteger => {
                if let Some(s) = &self.value_str {
                    let text: RichText = RichText::new(s).monospace();
                    ui.label(text);
                }
            }
            KnownEncoding::AppFloat => {
                if let Some(s) = &self.value_str {
                    let text: RichText = RichText::new(s).monospace();
                    ui.label(text);
                }
            }
            KnownEncoding::TextJson => {
                self.show_text_multiline(ui);
            }
            KnownEncoding::Empty => {}
            KnownEncoding::AppOctetStream => {}
            KnownEncoding::AppCustom => {}
            KnownEncoding::AppProperties => {}
            KnownEncoding::AppSql => {
                self.show_text_multiline(ui);
            }
            KnownEncoding::AppXml => {
                self.show_text_multiline(ui);
            }
            KnownEncoding::AppXhtmlXml => {
                self.show_text_multiline(ui);
            }
            KnownEncoding::AppXWwwFormUrlencoded => {
                self.show_text_multiline(ui);
            }
            KnownEncoding::TextHtml => {
                self.show_text_multiline(ui);
            }
            KnownEncoding::TextXml => {
                self.show_text_multiline(ui);
            }
            KnownEncoding::TextCss => {
                self.show_text_multiline(ui);
            }
            KnownEncoding::TextCsv => {
                self.show_text_multiline(ui);
            }
            KnownEncoding::TextJavascript => {
                self.show_text_multiline(ui);
            }
            KnownEncoding::ImageJpeg => {
                self.show_image(ui);
            }
            KnownEncoding::ImagePng => {}
            KnownEncoding::ImageGif => {}
        }

        if let Some(e) = &self.error_str {
            let text = RichText::new(e).color(Color32::RED);
            ui.label(text);
        }
    }

    fn show_text_multiline(&mut self, ui: &mut Ui) {
        if let Some(s) = &mut self.value_str {
            ui.add(
                TextEdit::multiline(s)
                    .desired_width(f32::INFINITY)
                    .code_editor(),
            );
        }
    }

    fn show_image(&mut self, ui: &mut Ui) {
        if let Some(color_image) = self.value_image.take() {
            let texture: &TextureHandle = self.image_texture_handle.get_or_insert_with(|| {
                ui.ctx()
                    .load_texture("page_sub_show_image", color_image, TextureOptions::NEAREST)
            });
            let image_size = texture.size_vec2();
            let plot_image = PlotImage::new(
                texture,
                PlotPoint::new(image_size.x / 2.0, -image_size.y / 2.0),
                image_size,
            )
            .highlight(true);
            let plot = Plot::new("page_sub_show_image_plot")
                .legend(Legend::default().position(Corner::RightTop))
                .show_x(true)
                .show_y(true)
                .show_axes([false, false])
                .allow_boxed_zoom(false)
                .allow_scroll(false)
                .show_background(true)
                .data_aspect(1.0);
            plot.show(ui, |plot_ui| {
                plot_ui.image(plot_image);
            });
        }
    }
}

fn filter(list: &Vec<String>, filter_str: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in list {
        if s.contains(filter_str) {
            out.push(s.clone());
        }
    }

    out
}

#[test]
fn tree_add_node() {
    let mut tree = Tree::default();
    tree.add_node("demo1");
    assert_eq!(tree.index_top_node.len(), 1);
    assert_eq!(*(tree.index_top_node.get("demo1").unwrap()), 0);
    assert_eq!(tree.mem.len(), 1);
    assert_eq!(tree.mem.first().unwrap().name, "demo1".to_string());
    assert_eq!(tree.mem.first().unwrap().key, Some("demo1".to_string()));

    tree.add_node("demo1/example1");
    assert_eq!(tree.index_top_node.len(), 1);
    assert_eq!(tree.mem.len(), 2);
    assert_eq!(tree.mem.get(1).unwrap().name, "example1".to_string());
    assert_eq!(
        tree.mem.get(1).unwrap().key,
        Some("demo1/example1".to_string())
    );
    assert_eq!(tree.mem.get(0).unwrap().index_children.len(), 1);
    assert_eq!(
        *(tree
            .mem
            .get(0)
            .unwrap()
            .index_children
            .get("example1")
            .unwrap()),
        1
    );

    tree.add_node("demo1/example1");
    assert_eq!(tree.index_top_node.len(), 1);
    assert_eq!(tree.mem.len(), 2);
    assert_eq!(tree.mem.get(1).unwrap().name, "example1".to_string());
    assert_eq!(
        tree.mem.get(1).unwrap().key,
        Some("demo1/example1".to_string())
    );

    tree.add_node("demo1/example2");
    assert_eq!(tree.index_top_node.len(), 1);
    assert_eq!(tree.mem.len(), 3);
    assert_eq!(tree.mem.get(2).unwrap().name, "example2".to_string());
    assert_eq!(
        tree.mem.get(2).unwrap().key,
        Some("demo1/example2".to_string())
    );
    assert_eq!(tree.mem.get(0).unwrap().index_children.len(), 2);
    assert_eq!(
        *(tree
            .mem
            .get(0)
            .unwrap()
            .index_children
            .get("example1")
            .unwrap()),
        1
    );
    assert_eq!(
        *(tree
            .mem
            .get(0)
            .unwrap()
            .index_children
            .get("example2")
            .unwrap()),
        2
    );

    tree.add_node("demo2/example1");
    assert_eq!(tree.index_top_node.len(), 2);
    assert_eq!(tree.mem.len(), 5);
    assert_eq!(tree.mem.get(3).unwrap().name, "demo2".to_string());
    assert_eq!(tree.mem.get(3).unwrap().key, None);
    assert_eq!(tree.mem.get(3).unwrap().index_children.len(), 1);
    assert_eq!(
        *(tree
            .mem
            .get(3)
            .unwrap()
            .index_children
            .get("example1")
            .unwrap()),
        4
    );
    assert_eq!(tree.mem.get(4).unwrap().name, "example1".to_string());
    assert_eq!(
        tree.mem.get(4).unwrap().key,
        Some("demo2/example1".to_string())
    );
}
