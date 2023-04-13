use arboard::Clipboard;
use egui::{
    Align, CollapsingHeader, Color32, Context, DragValue, Id, Layout, RichText, ScrollArea,
    TextEdit, TextStyle, Ui,
};
use egui_extras::{Column, TableBody, TableBuilder};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    str::FromStr,
};
use zenoh::{
    buffers::reader::{HasReader, Reader},
    prelude::{Encoding, KnownEncoding, OwnedKeyExpr, SampleKind, SplitBuffer, Value},
    sample::Sample,
    time::Timestamp,
};

use crate::{
    app::{
        f64_create_rich_text, i64_create_rich_text, json_create_rich_text,
        text_plant_create_rich_text,
    },
    hex_viewer::HexViewer,
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

// impl DataItem {
//     fn new(name: String, key_expr: String) -> DataItem {
//         DataItem { name, key_expr }
//     }
// }

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
        let mut data = Vec::with_capacity(self.sub_data_group.len());
        for (_, d) in &self.sub_data_group {
            let data_item = d.to();
            data.push(data_item);
        }
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
    }

    fn clean_all_sub_data(&mut self) {
        self.sub_data_group.clear();
        self.selected_sub_id = 0;
    }

    fn show_subscribers_name(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button(RichText::new(" + ").code()).clicked() {
                self.add_sub_data(PageSubData::new(format!("demo"), format!("demo/**")));
            }

            if ui.button(RichText::new(" - ").code()).clicked() {
                if self.sub_data_group.len() < 2 {
                    return;
                }

                let mut flag = false;
                if let Some(data_group) = self.sub_data_group.get(&self.selected_sub_id) {
                    flag = !data_group.subscribed;
                }
                if flag {
                    self.sub_data_group.remove(&self.selected_sub_id);
                    self.selected_sub_id = 0;
                }
            }
        });

        ui.label(" ");

        for (i, d) in &self.sub_data_group {
            let s = if d.subscribed {
                format!("* {}", d.name)
            } else {
                d.name.clone()
            };
            let text = RichText::new(s).monospace();
            if ui
                .selectable_label((*i) == self.selected_sub_id, text)
                .clicked()
            {
                self.selected_sub_id = *i;
            }
        }
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

        if let Some(dv) = data_group.map.get(&selected_key) {
            data_group.buffer_size = dv.buffer_size as u32;
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
            });

            let show_body = |mut body: TableBody| {
                let key = &selected_key;
                if let Some(sd) = data_group.map.get(selected_key.as_str()) {
                    for (d, k, t) in &sd.deque {
                        body.row(20.0, |mut row| {
                            row.col(|ui| match d.encoding {
                                Encoding::Exact(ke) => match ke {
                                    KnownEncoding::TextPlain => {
                                        let text: RichText = text_plant_create_rich_text(d);
                                        if ui.button(text).clicked() {
                                            self.show_view_value_window = true;
                                            self.view_value_window.clone_from(d, key, k, t);
                                        }
                                    }
                                    KnownEncoding::AppJson => {
                                        let text: RichText = json_create_rich_text(d);
                                        if ui.button(text).clicked() {
                                            self.show_view_value_window = true;
                                            self.view_value_window.clone_from(d, key, k, t);
                                        }
                                    }
                                    KnownEncoding::AppInteger => {
                                        let text: RichText = i64_create_rich_text(d);
                                        if ui.button(text).clicked() {
                                            self.show_view_value_window = true;
                                            self.view_value_window.clone_from(d, key, k, t);
                                        }
                                    }
                                    KnownEncoding::AppFloat => {
                                        let text: RichText = f64_create_rich_text(d);
                                        if ui.button(text).clicked() {
                                            self.show_view_value_window = true;
                                            self.view_value_window.clone_from(d, key, k, t);
                                        }
                                    }
                                    KnownEncoding::TextJson => {
                                        let text: RichText = json_create_rich_text(d);
                                        if ui.button(text).clicked() {
                                            self.show_view_value_window = true;
                                            self.view_value_window.clone_from(d, key, k, t);
                                        }
                                    }
                                    _ => {
                                        ui.label("...");
                                    }
                                },
                                Encoding::WithSuffix(_, _) => {
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

pub struct DataValues {
    deque: VecDeque<(Value, SampleKind, Option<Timestamp>)>,
    buffer_size: usize,
}

impl Default for DataValues {
    fn default() -> Self {
        DataValues {
            deque: VecDeque::with_capacity(VALUE_BUFFER_SIZE_DEFAULT),
            buffer_size: VALUE_BUFFER_SIZE_DEFAULT,
        }
    }
}

impl DataValues {
    pub fn add_data(&mut self, d: (Value, SampleKind, Option<Timestamp>)) {
        if self.deque.len() == self.buffer_size {
            let _ = self.deque.pop_front();
        }
        let _ = self.deque.push_back(d);
    }

    pub fn clear(&mut self) {
        self.deque.clear();
    }

    pub fn set_buffer_size(&mut self, size: usize) {
        self.buffer_size = if size < VALUE_BUFFER_SIZE_DEFAULT {
            VALUE_BUFFER_SIZE_DEFAULT
        } else {
            size
        };
        while self.deque.len() >= self.buffer_size {
            let _ = self.deque.pop_front();
        }
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
    value: Value,
    kind: SampleKind,
    timestamp: Option<Timestamp>,
    hex_viewer: HexViewer,
}

impl Default for ViewValueWindow {
    fn default() -> Self {
        ViewValueWindow {
            selected_page: ViewValueWindowPage::Parse,
            key: String::new(),
            value: Value::empty(),
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
        self.value = value.clone();

        let data_len = value.payload.len();
        let mut data: Vec<u8> = Vec::with_capacity(data_len);
        data.resize(data_len, 0);
        let _ = value.payload.reader().read_exact(data.as_mut_slice());
        self.hex_viewer = HexViewer::new(data);
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
            let text = RichText::new(format!("{}", self.value.encoding)).monospace();
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
        match self.value.encoding.prefix() {
            KnownEncoding::TextPlain => {
                let mut s = match String::try_from(&self.value) {
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
                let mut s: String = match serde_json::Value::try_from(&self.value) {
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
                let text: RichText = i64_create_rich_text(&self.value);
                ui.label(text);
            }
            KnownEncoding::AppFloat => {
                let text: RichText = f64_create_rich_text(&self.value);
                ui.label(text);
            }
            KnownEncoding::TextJson => {
                let mut s: String = match serde_json::Value::try_from(&self.value) {
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
