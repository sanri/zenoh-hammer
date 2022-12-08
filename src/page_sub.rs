use eframe::wgpu::Label;
use egui::{
    vec2, Align, Button, CollapsingHeader, Color32, DragValue, Layout, Resize, RichText,
    ScrollArea, SelectableLabel, TextEdit,
};
use egui_extras::{Size, TableBuilder};

pub struct PageSub {
    filtered: bool,
    filter_str: String,
    window_tree_height: f32,
    buffer_size_tmp: u32,
    buffer_size: u32,
}

impl Default for PageSub {
    fn default() -> Self {
        PageSub {
            filtered: false,
            filter_str: String::new(),
            window_tree_height: 400.0,
            buffer_size_tmp: 100,
            buffer_size: 100,
        }
    }
}

impl PageSub {
    pub fn show(&mut self, ui: &mut egui::Ui) {
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
                    ui.label(RichText::new("demo/example/**").monospace());

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("清理").clicked() {}
                    });
                });

                ui.separator();

                self.window_tree_height = ui.available_height();

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        Resize::default()
                            .default_width(200.0)
                            .default_height(self.window_tree_height)
                            .show(ui, |ui| {
                                ScrollArea::both()
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.checkbox(&mut self.filtered, "过滤");
                                            let te = TextEdit::singleline(&mut self.filter_str)
                                                .code_editor()
                                                .interactive(self.filtered);
                                            if ui.add(te).changed() {
                                                println!("text edit changed: {}", self.filter_str);
                                            }
                                        });

                                        self.show_key_tree(ui);
                                    });
                            });
                    });

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("key:");
                            ui.label(RichText::new("demo/example/test1").monospace());
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.button("清理").clicked() {}
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

                        self.show_value_table(ui);
                    });
                });
            });
        });
    }

    fn show_subscribers(&mut self, ui: &mut egui::Ui) {
        if ui
            .button(RichText::new("hello--------").monospace())
            .clicked()
        {}
        if ui
            .button(RichText::new("world--------").monospace())
            .clicked()
        {}
    }

    fn show_key_tree(&mut self, ui: &mut egui::Ui) {
        let _ = CollapsingHeader::new("header1")
            .default_open(false)
            .show(ui, |ui| {
                let b = SelectableLabel::new(false, "heard11");
                ui.add(b);
                CollapsingHeader::new("heard11")
                    .default_open(false)
                    .show(ui, |ui| {
                        let b = SelectableLabel::new(true, "我的");
                        if ui.add(b).clicked() {
                            println!("我的");
                        }
                    });
            });

        let _ = CollapsingHeader::new("header2")
            .default_open(false)
            .show(ui, |ui| {
                if ui.button("你的").clicked() {
                    println!("你的");
                }
            });
    }

    fn show_value_table(&mut self, ui: &mut egui::Ui) {
        let mut table = TableBuilder::new(ui)
            .striped(true)
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Size::initial(60.0).at_least(40.0))
            .column(Size::initial(60.0).at_least(40.0))
            .column(Size::remainder().at_least(60.0))
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