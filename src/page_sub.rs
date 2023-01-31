use crate::app::{
    f64_create_rich_text, i64_create_rich_text, json_create_rich_text, text_plant_create_rich_text,
    VALUE_BUFFER_SIZE_DEFAULT,
};
use egui::{
    vec2, Align, Button, CollapsingHeader, Color32, Context, Direction, DragValue, Id, Layout,
    Resize, RichText, ScrollArea, SelectableLabel, TextEdit, TextStyle, Ui,
};
use egui_extras::{Column, Size, TableBuilder};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    time::{Duration, SystemTime},
};
use zenoh::{
    prelude::{keyexpr, Encoding, KeyExpr, KnownEncoding, Sample, SampleKind, SplitBuffer, Value},
    time::{Timestamp, NTP64},
};

pub enum Event {
    AddSub(u64, KeyExpr<'static>), // id, key expr
    DelSub(u64),                   // id
}

pub struct DataSubValue {
    deque: VecDeque<(Value, SampleKind, Option<Timestamp>)>,
    buffer_size: usize,
}

impl Default for DataSubValue {
    fn default() -> Self {
        DataSubValue {
            deque: VecDeque::with_capacity(VALUE_BUFFER_SIZE_DEFAULT),
            buffer_size: VALUE_BUFFER_SIZE_DEFAULT,
        }
    }
}

impl DataSubValue {
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

pub struct DataSubKeyGroup {
    pub name: String,
    pub key_expr: String,
    pub map: BTreeMap<String, DataSubValue>, // key
}

#[derive(Default)]
pub struct AddSubWindow {
    id_count: u64,
    name: String,
    key_expr: String,
    error_str: Option<String>,
}

impl AddSubWindow {
    fn show(
        &mut self,
        ctx: &Context,
        is_open: &mut bool,
        key_group: &mut BTreeMap<u64, DataSubKeyGroup>,
        events: &mut VecDeque<Event>,
    ) {
        if (*is_open) == false {
            return;
        }

        let window = egui::Window::new("注册新订阅")
            .id(Id::new("add sub window"))
            .collapsible(false)
            .resizable(true)
            .default_width(200.0)
            .min_width(200.0);

        window.show(ctx, |ui| {
            let mut show = |ui: &mut Ui| {
                ui.label("name");
                let te = TextEdit::singleline(&mut self.name).font(TextStyle::Monospace);
                ui.add(te);
                ui.end_row();

                ui.label("key expr");
                let te = TextEdit::multiline(&mut self.key_expr)
                    .font(TextStyle::Monospace)
                    .desired_rows(2);
                ui.add(te);
                ui.end_row();
            };

            egui::Grid::new("config_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    show(ui);
                });

            ui.label("");
            if let Some(s) = &self.error_str {
                let text = RichText::new(s).color(Color32::RED);
                ui.label(text);
                ui.label("");
            }

            ui.horizontal(|ui| {
                ui.label("                 ");

                if ui.button("确定").clicked() {
                    let name: String = self.name.replace(&[' ', '\t', '\n', '\r'], "");

                    if name.is_empty() {
                        self.error_str = Some(format!("字段 name 不能为空!"));
                        return;
                    }

                    let key_expr: String = self.key_expr.replace(&[' ', '\t', '\n', '\r'], "");
                    if key_expr.is_empty() {
                        self.error_str = Some(format!("字段 key_expr 不能为空!"));
                        return;
                    }

                    let z_key_expr = match KeyExpr::new(key_expr.clone()) {
                        Ok(ke) => ke,
                        Err(e) => {
                            self.error_str = Some(format!("{}", e));
                            return;
                        }
                    };

                    let skg = DataSubKeyGroup {
                        name,
                        key_expr,
                        map: BTreeMap::new(),
                    };
                    self.id_count += 1;
                    let _ = key_group.insert(self.id_count, skg);

                    events.push_back(Event::AddSub(self.id_count, z_key_expr));

                    *is_open = false;
                    self.name.clear();
                    self.key_expr.clear();
                    self.error_str = None;
                }

                ui.label("  ");

                if ui.button("取消").clicked() {
                    *is_open = false;
                    self.name.clear();
                    self.key_expr.clear();
                    self.error_str = None;
                }
            });
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
    raw_data_offset: u64,
}

impl Default for ViewValueWindow {
    fn default() -> Self {
        ViewValueWindow {
            selected_page: ViewValueWindowPage::Parse,
            key: String::new(),
            value: Value::empty(),
            kind: SampleKind::Put,
            timestamp: None,
            raw_data_offset: u64::MAX,
        }
    }
}

impl ViewValueWindow {
    fn show(&mut self, ctx: &Context, is_open: &mut bool) {
        let window = egui::Window::new("详细信息")
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
    }

    fn show_base_info(&mut self, ui: &mut Ui) {
        let mut show_ui = |ui: &mut Ui| {
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
                .selectable_label(self.selected_page == ViewValueWindowPage::Parse, "解析数据")
                .clicked()
            {
                self.selected_page = ViewValueWindowPage::Parse;
            }

            if ui
                .selectable_label(self.selected_page == ViewValueWindowPage::Raw, "原始数据")
                .clicked()
            {
                self.selected_page = ViewValueWindowPage::Raw;
            }
        });
    }

    fn show_page_raw(&mut self, ui: &mut Ui) {
        ui.label(format!("offset: {}", self.raw_data_offset));
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

pub struct PageSub {
    pub events: VecDeque<Event>,
    pub new_sub_key_flag: bool,
    filtered: bool,
    filter_str: String,
    window_tree_height: f32,
    buffer_size_tmp: u32,
    buffer_size: u32,
    selected_sub_id: u64,
    selected_key: String,
    selected_sub_id_or_key_changed: bool,
    show_selected_key_expr: String,
    key_list_before_filtration: Vec<String>,
    key_tree: Tree,                                // tree be show
    pub key_group: BTreeMap<u64, DataSubKeyGroup>, // <sub id, key group>
    show_add_sub_window: bool,
    add_sub_window: AddSubWindow,
    show_view_value_window: bool,
    view_value_window: ViewValueWindow,
}

impl Default for PageSub {
    fn default() -> Self {
        PageSub {
            events: VecDeque::new(),
            filtered: false,
            filter_str: String::new(),
            window_tree_height: 400.0,
            buffer_size_tmp: VALUE_BUFFER_SIZE_DEFAULT as u32,
            buffer_size: VALUE_BUFFER_SIZE_DEFAULT as u32,
            selected_sub_id: 0,
            show_selected_key_expr: String::new(),
            key_list_before_filtration: Vec::new(),
            selected_key: String::new(),
            key_group: BTreeMap::new(),
            key_tree: Tree::default(),
            new_sub_key_flag: false,
            selected_sub_id_or_key_changed: false,
            show_add_sub_window: false,
            add_sub_window: AddSubWindow::default(),
            show_view_value_window: false,
            view_value_window: ViewValueWindow::default(),
        }
    }
}

impl PageSub {
    pub fn show(&mut self, ctx: &Context, ui: &mut Ui) {
        if self.new_sub_key_flag {
            self.new_sub_key_flag = false;
            if let Some(skg) = self.key_group.get(&self.selected_sub_id) {
                self.key_list_before_filtration.clear();
                for (key, _) in &skg.map {
                    self.key_list_before_filtration.push(key.clone());
                }
                self.filter_key_tree();
            }
        }

        ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button(RichText::new(" + ").code()).clicked() {
                        self.show_add_sub_window = true;
                    };

                    if ui.button(RichText::new(" - ").code()).clicked() {
                        self.events.push_back(Event::DelSub(self.selected_sub_id));
                        self.selected_sub_id = 0;
                        self.show_selected_key_expr.clear();
                        self.key_list_before_filtration.clear();
                        self.filter_key_tree();
                    };
                });

                ui.label("");

                self.show_subscribers(ui);
            });

            ui.separator();

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("key expr:");
                    ui.label(RichText::new(self.show_selected_key_expr.as_str()).monospace());
                });

                ui.separator();

                self.window_tree_height = ui.available_height();

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ScrollArea::both()
                            .id_source("scroll_area_tree")
                            .max_width(200.0)
                            .min_scrolled_width(200.0)
                            .min_scrolled_height(self.window_tree_height)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if ui.checkbox(&mut self.filtered, "过滤").changed() {
                                        self.filter_key_tree();
                                    }
                                    let te = TextEdit::singleline(&mut self.filter_str)
                                        .code_editor()
                                        .interactive(self.filtered);
                                    if ui.add(te).changed() {
                                        self.filter_key_tree();
                                    };
                                });

                                self.show_key_tree(ui);
                            });
                    });

                    ui.separator();

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("key:");
                            ui.label(RichText::new(&self.selected_key).monospace());
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.button("清理缓存数据").clicked() {
                                    if let Some(skg) = self.key_group.get_mut(&self.selected_sub_id)
                                    {
                                        if let Some(sd) =
                                            skg.map.get_mut(self.selected_key.as_str())
                                        {
                                            sd.clear();
                                        }
                                    }
                                }

                                // if ui.button("value").clicked() {
                                //     use uhlc::ID;
                                //     use uuid::Uuid;
                                //
                                //     self.show_view_value_window = true;
                                //     self.view_value_window.key = "demo/example/test1".to_string();
                                //     self.view_value_window.value = Value::from("hello world");
                                //     let time: NTP64 = Duration::from_secs(100).into();
                                //     let buf = [
                                //         0x1a, 0x2b, 0x3c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                                //         0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                                //     ];
                                //     let id = ID::from(Uuid::new_v4());
                                //     self.view_value_window.timestamp =
                                //         Some(Timestamp::new(time, id));
                                // }
                            });
                        });

                        ui.horizontal(|ui| {
                            if self.selected_sub_id_or_key_changed {
                                self.selected_sub_id_or_key_changed = false;
                                if let Some(skg) = self.key_group.get(&self.selected_sub_id) {
                                    if let Some(sd) = skg.map.get(self.selected_key.as_str()) {
                                        self.buffer_size = sd.buffer_size as u32;
                                    }
                                }
                            }
                            ui.label(format!("buffer size: {}", &self.buffer_size));
                            ui.label("   ");
                            let dv = DragValue::new(&mut self.buffer_size_tmp)
                                .speed(10.0)
                                .clamp_range(VALUE_BUFFER_SIZE_DEFAULT..=10000);
                            ui.add(dv);
                            if ui.button("更新缓存大小").clicked() {
                                self.buffer_size = self.buffer_size_tmp;
                                if let Some(skg) = self.key_group.get_mut(&self.selected_sub_id) {
                                    if let Some(sd) = skg.map.get_mut(self.selected_key.as_str()) {
                                        sd.set_buffer_size(self.buffer_size as usize);
                                    }
                                }
                            }
                        });

                        ScrollArea::horizontal()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                self.show_value_table(ui);
                            });
                    });
                });
            });
        });

        self.add_sub_window.show(
            ctx,
            &mut self.show_add_sub_window,
            &mut self.key_group,
            &mut self.events,
        );

        self.view_value_window
            .show(ctx, &mut self.show_view_value_window);
    }

    fn show_subscribers(&mut self, ui: &mut Ui) {
        ScrollArea::both()
            .max_width(160.0)
            .auto_shrink([true, false])
            .show(ui, |ui| {
                let mut clicked = false;
                for (i, d) in &self.key_group {
                    let text = RichText::new(d.name.clone()).monospace();
                    if ui
                        .selectable_label((*i) == self.selected_sub_id, text)
                        .clicked()
                    {
                        self.selected_sub_id_or_key_changed = true;
                        self.selected_sub_id = *i;
                        self.show_selected_key_expr = d.key_expr.clone();
                        self.key_list_before_filtration.clear();
                        for (key, _) in &d.map {
                            self.key_list_before_filtration.push(key.clone());
                        }
                        clicked = true;
                    }
                }
                if clicked {
                    self.filter_key_tree();
                }
            });
    }

    fn show_key_tree(&mut self, ui: &mut Ui) {
        self.key_tree.show_ui(
            &mut self.selected_key,
            &mut self.selected_sub_id_or_key_changed,
            ui,
        );
    }

    fn filter_key_tree(&mut self) {
        if self.filtered {
            let filtered = filter(&self.key_list_before_filtration, &self.filter_str);
            self.key_tree = Tree::new(&filtered);
        } else {
            self.key_tree = Tree::new(&self.key_list_before_filtration);
        }
    }

    fn show_value_table(&mut self, ui: &mut Ui) {
        let mut table = TableBuilder::new(ui)
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
                    ui.label("值");
                });
                header.col(|ui| {
                    ui.label("类型");
                });
                header.col(|ui| {
                    ui.label("时间戳");
                });
            })
            .body(|mut body| {
                if let Some(skg) = self.key_group.get(&self.selected_sub_id) {
                    if let Some(sd) = skg.map.get(self.selected_key.as_str()) {
                        let key = &self.selected_key;
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
                                        let text =
                                            RichText::new(format!("{}", timestamp.get_time()))
                                                .size(12.0);
                                        ui.label(text);
                                    } else {
                                        ui.label("-");
                                    }
                                });
                            });
                        }
                    }
                }
            });
    }

    pub fn processing_sub_cb(&mut self, d: Box<(u64, Sample)>) {
        let (id, data): (u64, Sample) = *d;
        let key = data.key_expr.as_str();

        if let Some(skg) = self.key_group.get_mut(&id) {
            if let Some(sv) = skg.map.get_mut(key) {
                sv.add_data((data.value, data.kind, data.timestamp));
            } else {
                println!("new key: {}", key);
                let mut sv = DataSubValue::default();
                sv.add_data((data.value, data.kind, data.timestamp));
                let _ = skg.map.insert(key.to_string(), sv);
                self.new_sub_key_flag = true;
            }
        }
    }

    pub fn processing_del_sub_res(&mut self, id: u64) {
        let _ = self.key_group.remove(&id);
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

    fn show_ui<'a>(
        &'a self,
        tree: &'a Tree,
        selected_key: &'a mut String,
        selected_key_changed: &mut bool,
        ui: &'a mut Ui,
    ) {
        let name = self.name.clone();
        if let Some(k) = self.key.clone() {
            if ui
                .selectable_label(*selected_key == k, name.clone())
                .clicked()
            {
                *selected_key = k;
                *selected_key_changed = true;
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
                    child.show_ui(tree, selected_key, selected_key_changed, ui);
                }
            });
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

    pub fn show_ui(&self, selected_key: &mut String, selected_key_changed: &mut bool, ui: &mut Ui) {
        for (_, index_top) in &self.index_top_node {
            let top_node: &TreeNode = self.mem.get((*index_top) as usize).unwrap();
            top_node.show_ui(self, selected_key, selected_key_changed, ui);
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
        let mut index_now = u32::MAX;
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
                let mut index_child: u32 = u32::MAX;
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

fn filter(list: &Vec<String>, filter_str: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in list {
        if s.contains(filter_str) {
            out.push(s.clone());
        }
    }

    out
}
