use arboard::Clipboard;
use eframe::egui::{
    Align, CentralPanel, CollapsingHeader, Color32, Context, DragValue, Grid, Id, Layout, RichText,
    ScrollArea, SidePanel, TextEdit, TextStyle, Ui, Window,
};
use egui_dnd::dnd;
use egui_extras::{Column, TableBody, TableBuilder};
use log::info;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    ops::Add,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use zenoh::{key_expr::OwnedKeyExpr, sample::Sample};

use crate::{sample_viewer::SampleViewer, zenoh_data::zenoh_value_abstract};

pub const VALUE_BUFFER_SIZE_DEFAULT: usize = 10;

pub enum Event {
    AddSub(Box<(u64, OwnedKeyExpr)>), // id, key expr
    DelSub(u64),                      // id
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ArchivePageSubData {
    name: String,
    key_expr: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ArchivePageSub {
    subs: Vec<ArchivePageSubData>,
}

pub struct PageSub {
    pub events: VecDeque<Event>,
    sub_id_count: u64,
    selected_sub_id: u64,
    show_sample_viewer_window: bool,
    sample_viewer_window: SampleViewer,
    sub_data_group: BTreeMap<u64, PageSubData>, // <sub id, group>
    dnd_items: Vec<DndItem>,
}

impl Default for PageSub {
    fn default() -> Self {
        let mut p = PageSub {
            events: VecDeque::new(),
            sub_id_count: 0,
            selected_sub_id: 1,
            show_sample_viewer_window: false,
            sample_viewer_window: SampleViewer::default(),
            sub_data_group: BTreeMap::new(),
            dnd_items: Vec::new(),
        };
        p.add_sub_data(PageSubData::new("demo".to_string(), "demo/**".to_string()));
        p
    }
}

impl From<&PageSub> for ArchivePageSub {
    fn from(value: &PageSub) -> Self {
        ArchivePageSub {
            subs: value
                .dnd_items
                .iter()
                .filter_map(|k| value.sub_data_group.get(&k.key_id))
                .map(|d| d.into())
                .collect(),
        }
    }
}

impl PageSub {
    pub fn load(&mut self, archive: ArchivePageSub) -> Result<(), String> {
        let mut data = Vec::with_capacity(archive.subs.len());
        for d in archive.subs {
            let page_sub_data = PageSubData::try_from(d)?;
            data.push(page_sub_data);
        }

        self.clean_all_sub_data();
        for d in data {
            self.add_sub_data(d);
        }
        Ok(())
    }

    pub fn show(&mut self, ctx: &Context) {
        SidePanel::left("page_sub_panel_left")
            .resizable(true)
            .show(ctx, |ui| {
                self.show_subscribers_name(ui);
            });

        CentralPanel::default().show(ctx, |ui| {
            self.show_name_key(ui);

            ui.separator();

            ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
                self.show_key_tree(ui);

                ui.separator();

                self.show_values(ui);
            });
        });

        let window = Window::new("Info")
            .id(Id::new("view sample window"))
            .collapsible(false)
            .scroll([true, true])
            .open(&mut self.show_sample_viewer_window)
            .resizable(true)
            .default_width(200.0)
            .min_width(200.0);

        window.show(ctx, |ui| {
            self.sample_viewer_window.show(ui);
        });
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
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new(" + ").code())
                .on_hover_text("copy add")
                .clicked()
            {
                if let Some(d) = self.sub_data_group.get(&self.selected_sub_id) {
                    self.add_sub_data(PageSubData::from(d));
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

        ui.add_space(10.0);

        ScrollArea::both()
            .max_width(200.0)
            .auto_shrink([true, false])
            .show(ui, |ui| {
                dnd(ui, "page_sub_list").show_vec(
                    self.dnd_items.as_mut_slice(),
                    |ui, item, handle, _state| {
                        if let Some(d) = self.sub_data_group.get(&item.key_id) {
                            let text = if d.subscribed {
                                RichText::new(d.name.as_str()).underline().strong()
                            } else {
                                RichText::new(d.name.as_str())
                            };
                            handle.ui(ui, |ui| {
                                ui.selectable_value(&mut self.selected_sub_id, item.key_id, text);
                            });
                        }
                    },
                )
            });
    }

    fn show_name_key(&mut self, ui: &mut Ui) {
        let data_group = match self.sub_data_group.get_mut(&self.selected_sub_id) {
            None => {
                return;
            }
            Some(o) => o,
        };

        Grid::new("page_sub_name_key")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("name:");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
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

                            let key: OwnedKeyExpr =
                                match OwnedKeyExpr::from_str(key_expr_str.as_str()) {
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

                    let te = TextEdit::singleline(&mut data_group.name)
                        .desired_width(3000.0)
                        .font(TextStyle::Monospace)
                        .interactive(!data_group.subscribed);
                    ui.add(te);
                });

                ui.end_row();

                ui.label("key expr:");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let te = TextEdit::multiline(&mut data_group.key_expr)
                        .desired_rows(1)
                        .desired_width(3000.0)
                        .font(TextStyle::Monospace)
                        .interactive(!data_group.subscribed);
                    ui.add(te);
                });

                ui.end_row();
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
            .id_salt("scroll_area_tree")
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
                    .range(VALUE_BUFFER_SIZE_DEFAULT..=10000);
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
                if let Some(sd) = data_group.map.get(selected_key.as_str()) {
                    for (sample, _) in &sd.deque {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                let text =
                                    zenoh_value_abstract(sample.encoding(), sample.payload());
                                let rich_text = match text {
                                    Ok(o) => RichText::new(o),
                                    Err(e) => RichText::new(e).color(Color32::RED),
                                };
                                if ui.button(rich_text).clicked() {
                                    self.show_sample_viewer_window = true;
                                    self.sample_viewer_window =
                                        SampleViewer::new_from_sample(sample);
                                }
                            });
                            row.col(|ui| {
                                let text = sample.encoding().to_string();
                                ui.label(text);
                            });
                            row.col(|ui| {
                                let text = if let Some(timestamp) = sample.timestamp() {
                                    let s = timestamp.to_string_rfc3339_lossy();
                                    let time_str = s.split_once('/').unwrap_or(("-", "-"));
                                    RichText::new(time_str.0).size(12.0)
                                } else {
                                    RichText::new("-").size(12.0)
                                };
                                ui.label(text);
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

    pub fn processing_sub_cb(&mut self, id: u64, sample: Sample, receipt_time: SystemTime) {
        let key = sample.key_expr().to_string();
        if let Some(data_group) = self.sub_data_group.get_mut(&id) {
            if let Some(sv) = data_group.map.get_mut(&key) {
                sv.add_data(sample, receipt_time);
            } else {
                info!("new key: {}", key);
                let mut sv = DataValues::default();
                sv.add_data(sample, receipt_time);
                let _ = data_group.map.insert(key, sv);
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

#[derive(Hash)]
struct DndItem {
    key_id: u64,
}

impl DndItem {
    fn new(k: u64) -> Self {
        DndItem { key_id: k }
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
    deque: VecDeque<(Sample, SystemTime)>,
    buffer_size: usize,
    lately_local_timestamp: SystemTime,
}

impl Default for DataValues {
    fn default() -> Self {
        DataValues {
            deque: VecDeque::with_capacity(VALUE_BUFFER_SIZE_DEFAULT),
            buffer_size: VALUE_BUFFER_SIZE_DEFAULT,
            lately_local_timestamp: SystemTime::now(),
        }
    }
}

impl DataValues {
    fn add_data(&mut self, sample: Sample, system_time: SystemTime) {
        if self.deque.len() == self.buffer_size {
            let _ = self.deque.pop_front();
        }
        let _ = self.deque.push_back((sample, system_time));
        self.lately_local_timestamp = system_time;
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

        let now_time = SystemTime::now();
        let dt = now_time
            .duration_since(self.lately_local_timestamp)
            .unwrap_or(Duration::default())
            .as_secs() as u32;
        if dt >= 1 {
            return Some(Frequency::S(dt));
        } else {
            if self.deque.len() == 1 {
                return Some(Frequency::Hz(1.0));
            }
        }

        // 检索最多100条或最近10s内的记录
        let mut system_time_max = UNIX_EPOCH;
        let mut system_time_min = system_time_max.add(Duration::from_secs(3600 * 24 * 365 * 100));
        let mut count = 0f32;
        let mut time = 1f32;
        for (i, d) in self.deque.iter().rev().enumerate() {
            let t = d.1;
            system_time_max = system_time_max.max(t);
            system_time_min = system_time_min.min(t);
            count = i as f32;
            time = system_time_max
                .duration_since(system_time_min)
                .unwrap_or(Duration::default())
                .as_secs() as f32;
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

impl From<&PageSubData> for PageSubData {
    fn from(value: &PageSubData) -> Self {
        PageSubData::new(value.name.clone(), value.key_expr.clone())
    }
}

impl From<&PageSubData> for ArchivePageSubData {
    fn from(value: &PageSubData) -> Self {
        ArchivePageSubData {
            name: value.name.clone(),
            key_expr: value.key_expr.clone(),
        }
    }
}

impl TryFrom<&ArchivePageSubData> for PageSubData {
    type Error = String;

    fn try_from(value: &ArchivePageSubData) -> Result<Self, Self::Error> {
        Ok(PageSubData::new(value.name.clone(), value.key_expr.clone()))
    }
}

impl TryFrom<ArchivePageSubData> for PageSubData {
    type Error = String;

    fn try_from(value: ArchivePageSubData) -> Result<Self, Self::Error> {
        Ok(PageSubData::new(value.name, value.key_expr))
    }
}

impl PageSubData {
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
