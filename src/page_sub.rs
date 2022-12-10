use eframe::wgpu::Label;
use egui::{
    vec2, Align, Button, CollapsingHeader, Color32, DragValue, Layout, Resize, RichText,
    ScrollArea, SelectableLabel, TextEdit, Ui,
};
use egui_extras::{Column, Size, TableBuilder};
use serde::de::Unexpected::Str;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::ops::Index;
use zenoh::{
    prelude::{keyexpr, Value},
    time::Timestamp,
};

pub struct DataSubValue {
    deque: VecDeque<(Value, Option<Timestamp>)>,
}

pub struct DataSubKeyGroup {
    name: String,
    key_expr: String,
    map: BTreeSet<String>, // key
}

pub struct PageSub {
    filtered: bool,
    filter_str: String,
    window_tree_height: f32,
    buffer_size_tmp: u32,
    buffer_size: u32,
    selected_sub_id: u64,
    selected_key: String,
    show_selected_key_expr: String,
    key_list_before_filtration: Vec<String>,
    key_tree: Tree,                            // tree be show
    key_group: BTreeMap<u64, DataSubKeyGroup>, // <sub id, key group>
    key_value: BTreeMap<String, DataSubValue>, // <key, data deque>
}

impl Default for PageSub {
    fn default() -> Self {
        let mut bm = BTreeMap::new();
        for i in 1..=20 {
            let skg_map: BTreeSet<String> = BTreeSet::from([
                "demo/example/test1".to_string(),
                "demo/example/test2".to_string(),
            ]);

            bm.insert(
                i,
                DataSubKeyGroup {
                    name: format!("sub_{}", i),
                    key_expr: format!("demo/example{}/**", i),
                    map: skg_map,
                },
            );
        }
        PageSub {
            filtered: false,
            filter_str: String::new(),
            window_tree_height: 400.0,
            buffer_size_tmp: 100,
            buffer_size: 100,
            selected_sub_id: 0,
            show_selected_key_expr: String::new(),
            key_list_before_filtration: Vec::new(),
            selected_key: String::new(),
            key_group: bm,
            key_value: BTreeMap::new(),
            key_tree: Tree::default(),
        }
    }
}

impl PageSub {
    pub fn show(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    if ui.button(RichText::new(" + ").code()).clicked() {};

                    if ui.button(RichText::new(" - ").code()).clicked() {};
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
                                if ui.button("清理缓存数据").clicked() {}
                            });
                        });

                        ui.horizontal(|ui| {
                            ui.label(format!("buffer size: {}", &self.buffer_size));
                            ui.label("   ");
                            let dv = DragValue::new(&mut self.buffer_size_tmp)
                                .speed(10.0)
                                .clamp_range(100..=10000);
                            ui.add(dv);
                            if ui.button("更新缓存大小").clicked() {
                                self.buffer_size = self.buffer_size_tmp;
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
    }

    fn show_subscribers(&mut self, ui: &mut Ui) {
        ScrollArea::both()
            .max_width(100.0)
            .auto_shrink([true, false])
            .show(ui, |ui| {
                let mut clicked = false;
                for (i, d) in &self.key_group {
                    let text = RichText::new(d.name.clone()).monospace();
                    if ui
                        .selectable_label((*i) == self.selected_sub_id, text)
                        .clicked()
                    {
                        self.selected_sub_id = *i;
                        self.show_selected_key_expr = d.key_expr.clone();
                        self.key_list_before_filtration = d.map.iter().cloned().collect();
                        clicked = true;
                    }
                }
                if clicked {
                    self.filter_key_tree();
                }
            });
    }

    fn show_key_tree(&mut self, ui: &mut Ui) {
        self.key_tree.show_ui(&mut self.selected_key, ui);
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
            .column(Column::initial(40.0).at_least(40.0))
            .column(Column::auto())
            .column(Column::remainder())
            .resizable(true);

        table
            .header(18.0, |mut header| {
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
                for i in 0..100 {
                    body.row(18.0, |mut row| {
                        row.col(|ui| {
                            ui.with_layout(Layout::default().with_cross_justify(true), |ui| {
                                if i < 30 {
                                    ui.label(i.to_string());
                                } else {
                                    if ui.button(i.to_string()).clicked() {
                                        println!("row clicked {}", i);
                                    }
                                }
                            });
                        });
                        row.col(|ui| {
                            ui.label("nihao");
                        });
                        row.col(|ui| {
                            let text = if (i % 2) == 0 {
                                RichText::new("2022-12-07T16:24:32.789Z")
                                    .underline()
                                    .size(12.0)
                            } else {
                                RichText::new("2022-12-07T16:24:32.789Z").size(12.0)
                            };
                            ui.label(text);
                        });
                    });
                }
            });
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
        if let Some(_) = s.find(filter_str) {
            out.push(s.clone());
        }
    }

    out
}
