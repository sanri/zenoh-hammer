use crate::page_session::PageSession;
use eframe::Frame;
use egui::Context;
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, AsRefStr, EnumIter)]
pub enum Page {
    Session,
    Sub,
    Put,
    Get,
    Pub,
}

pub struct HammerApp {
    language_changed: bool,
    selected_page: Page,
    p_session: PageSession,
}

impl Default for HammerApp {
    fn default() -> Self {
        HammerApp {
            language_changed: false,
            selected_page: Page::Session,
            p_session: PageSession::default(),
        }
    }
}

impl HammerApp {
    fn bar_contents(&mut self, ui: &mut egui::Ui, frame: &mut Frame) {
        egui::widgets::global_dark_light_mode_switch(ui);

        ui.separator();

        ui.menu_button("文件", |ui| {
            ui.set_min_width(80.0);

            if ui.add(egui::Button::new("打开")).clicked() {
                ui.close_menu();
            }

            if ui.add(egui::Button::new("保存")).clicked() {
                ui.close_menu();
            }
        });

        ui.menu_button("帮助", |ui| {
            ui.set_min_width(80.0);
            ui.style_mut().wrap = Some(false);

            if ui.add(egui::Button::new("关于")).clicked() {
                ui.close_menu();
            }

            if ui.add(egui::Button::new("使用说明")).clicked() {
                ui.close_menu();
            }

            ui.separator();

            ui.menu_button("语言", |ui| {
                if ui.add(egui::Button::new("中文")).clicked() {
                    ui.close_menu();
                }

                if ui.add(egui::Button::new("English")).clicked() {
                    ui.close_menu();
                }
            });
        });

        ui.separator();

        for page in Page::iter() {
            if ui
                .selectable_label(page == self.selected_page, page.as_ref())
                .clicked()
            {
                self.selected_page = page;
            }
        }

        ui.separator();
    }
}

impl eframe::App for HammerApp {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        ctx.set_pixels_per_point(3.0);

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                self.bar_contents(ui, frame);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.selected_page {
            Page::Session => {
                self.p_session.show(ui);
            }
            Page::Sub => {
                ui.label("page sub");
            }
            Page::Put => {
                ui.label("page put");
            }
            Page::Get => {
                ui.label("page get");
            }
            Page::Pub => {
                ui.label("page pub");
            }
        });
    }
}
