use crate::page_get::PageGet;
use crate::page_pub::PagePub;
use crate::page_put::PagePut;
use crate::page_session::PageSession;
use crate::page_sub::PageSub;
use eframe::emath::Align;
use eframe::Frame;
use egui::{Context, Layout};
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, AsRefStr, EnumIter)]
pub enum Page {
    Session,
    Sub,
    Get,
    Pub,
    Put,
}

pub struct HammerApp {
    language_changed: bool,
    selected_page: Page,
    p_session: PageSession,
    p_sub: PageSub,
    p_get: PageGet,
    p_pub: PagePub,
    p_put: PagePut,
}

impl Default for HammerApp {
    fn default() -> Self {
        HammerApp {
            language_changed: false,
            selected_page: Page::Session,
            p_session: PageSession::default(),
            p_sub: PageSub::default(),
            p_get: PageGet::default(),
            p_pub: PagePub::default(),
            p_put: PagePut::default(),
        }
    }
}

impl HammerApp {
    fn bar_contents(&mut self, ui: &mut egui::Ui, frame: &mut Frame) {
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

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            egui::widgets::global_dark_light_mode_switch(ui);
        });
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
                self.p_sub.show(ui);
            }
            Page::Put => {
                self.p_put.show(ui);
            }
            Page::Get => {
                self.p_get.show(ui);
            }
            Page::Pub => {
                self.p_pub.show(ui);
            }
        });
    }
}
