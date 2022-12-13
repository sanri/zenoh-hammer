use crate::page_get::PageGet;
use crate::page_pub::PagePub;
use crate::page_put::{Event, PagePut};
use crate::page_session::PageSession;
use crate::page_sub::{DataSubKeyGroup, DataSubValue, PageSub};
use crate::zenoh::{MsgGuiToZenoh, MsgZenohToGui, Receiver, Sender};
use crate::{page_session, page_sub};
use eframe::{emath::Align, Frame};
use egui::{Color32, Context, Layout, RichText};
use flume::{unbounded, TryRecvError};
use std::collections::BTreeSet;
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter};
use zenoh::prelude::SplitBuffer;
use zenoh::{prelude::KeyExpr, sample::Sample, value::Value};

pub const VALUE_BUFFER_SIZE_DEFAULT: usize = 10;

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
    sender_to_zenoh: Option<Sender<MsgGuiToZenoh>>,
    receiver_from_zenoh: Option<Receiver<MsgZenohToGui>>,
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
            sender_to_zenoh: None,
            receiver_from_zenoh: None,
            selected_page: Page::Session,
            p_session: PageSession::default(),
            p_sub: PageSub::default(),
            p_get: PageGet::default(),
            p_pub: PagePub::default(),
            p_put: PagePut::default(),
        }
    }
}

impl eframe::App for HammerApp {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        self.processing_zenoh_msg();
        self.processing_page_session_events();
        self.processing_page_sub_events();
        self.processing_page_put_events();
        self.show_ui(ctx, frame);
        ctx.request_repaint();
    }
}

impl HammerApp {
    fn show_ui(&mut self, ctx: &Context, frame: &mut Frame) {
        ctx.set_pixels_per_point(3.0);

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.show_bar_contents(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.selected_page {
            Page::Session => {
                self.p_session.show(ui);
            }
            Page::Sub => {
                self.p_sub.show(ctx, ui);
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

    fn show_bar_contents(&mut self, ui: &mut egui::Ui) {
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

    fn processing_zenoh_msg(&mut self) {
        let receiver: &Receiver<MsgZenohToGui> = match &self.receiver_from_zenoh {
            None => {
                return;
            }
            Some(s) => s,
        };

        loop {
            let r = receiver.try_recv();
            let msg = match r {
                Ok(m) => m,
                Err(e) => match e {
                    TryRecvError::Empty => {
                        return;
                    }
                    TryRecvError::Disconnected => {
                        self.receiver_from_zenoh = None;
                        self.p_session.connected = false;
                        return;
                    }
                },
            };
            match msg {
                MsgZenohToGui::OpenSession(b) => {
                    self.p_session.connected = b;
                    if b == false {
                        self.sender_to_zenoh = None;
                        return;
                    }
                }
                MsgZenohToGui::AddSubRes(_) => {}
                MsgZenohToGui::DelSubRes(id) => {
                    let _ = self.p_sub.key_group.remove(&id);
                }
                MsgZenohToGui::SubCB(d) => {
                    let (id, data): (u64, Sample) = *d;
                    let key = data.key_expr.as_str();

                    if let Some(skg) = self.p_sub.key_group.get_mut(&id) {
                        if let Some(sv) = skg.map.get_mut(key) {
                            sv.add_data((data.value, data.kind, data.timestamp));
                        } else {
                            println!("new key: {}", key);
                            let mut sv = DataSubValue::default();
                            sv.add_data((data.value, data.kind, data.timestamp));
                            let _ = skg.map.insert(key.to_string(), sv);
                            self.p_sub.new_sub_key_flag = true;
                        }
                    }
                }
                MsgZenohToGui::GetRes(_) => {}
                MsgZenohToGui::AddPubRes => {}
                MsgZenohToGui::DelPubRes => {}
                MsgZenohToGui::PubRes => {}
                MsgZenohToGui::PutRes(r) => {
                    let (id, b, s) = *r;
                    if let Some(pd) = self.p_put.data_map.get_mut(&id) {
                        pd.info = if b {
                            Some(RichText::new(s))
                        } else {
                            Some(RichText::new(s).color(Color32::RED))
                        }
                    }
                }
            }
        }
    }

    fn processing_page_session_events(&mut self) {
        while let Some(event) = self.p_session.events.pop_front() {
            match event {
                page_session::Event::Connect(c) => {
                    if self.sender_to_zenoh.is_none() {
                        let (sender_to_gui, receiver_from_zenoh): (
                            Sender<MsgZenohToGui>,
                            Receiver<MsgZenohToGui>,
                        ) = unbounded();
                        let (sender_to_zenoh, receiver_from_gui): (
                            Sender<MsgGuiToZenoh>,
                            Receiver<MsgGuiToZenoh>,
                        ) = unbounded();

                        crate::zenoh::start_async(sender_to_gui, receiver_from_gui, *c);

                        self.sender_to_zenoh = Some(sender_to_zenoh);
                        self.receiver_from_zenoh = Some(receiver_from_zenoh);
                    }
                }
                page_session::Event::Disconnect => {
                    if let Some(sender) = &self.sender_to_zenoh {
                        let _ = sender.send(MsgGuiToZenoh::Close);
                        self.sender_to_zenoh = None;
                    }
                }
            }
        }
    }

    fn processing_page_sub_events(&mut self) {
        while let Some(event) = self.p_sub.events.pop_front() {
            match event {
                page_sub::Event::AddSub(id, key_expr) => {
                    if let Some(sender) = &self.sender_to_zenoh {
                        let _ = sender.send(MsgGuiToZenoh::AddSubReq(Box::new((id, key_expr))));
                    } else {
                        let _ = self.p_sub.key_group.remove(&id);
                    }
                }
                page_sub::Event::DelSub(id) => {
                    if let Some(sender) = &self.sender_to_zenoh {
                        let _ = sender.send(MsgGuiToZenoh::DelSubReq(id));
                    } else {
                        let _ = self.p_sub.key_group.remove(&id);
                    }
                }
            }
        }
    }

    fn processing_page_put_events(&mut self) {
        while let Some(event) = self.p_put.events.pop_front() {
            match event {
                Event::Put(p) => {
                    if let Some(sender) = &self.sender_to_zenoh {
                        let _ = sender.send(MsgGuiToZenoh::PutReq(p));
                    }
                }
            }
        }
    }
}

pub fn i64_create_rich_text(d: &Value) -> RichText {
    let text: RichText = match i64::try_from(d) {
        Ok(o) => RichText::new(format!("{}", o)).monospace(),
        Err(_) => RichText::new("type err!").monospace().color(Color32::RED),
    };
    text
}

pub fn f64_create_rich_text(d: &Value) -> RichText {
    let text: RichText = match f64::try_from(d) {
        Ok(o) => RichText::new(format!("{}", o)).monospace(),
        Err(_) => RichText::new("type err!").monospace().color(Color32::RED),
    };
    text
}

pub fn text_plant_create_rich_text(d: &Value) -> RichText {
    let text: RichText = if d.payload.len() < 30 {
        match String::try_from(d) {
            Ok(o) => RichText::new(o).monospace(),
            Err(_) => RichText::new("type err!").monospace().color(Color32::RED),
        }
    } else {
        RichText::new("...")
    };
    text
}

pub fn json_create_rich_text(d: &Value) -> RichText {
    let text: RichText = if d.payload.len() < 30 {
        match serde_json::Value::try_from(d) {
            Ok(o) => RichText::new(format!("{}", o)).monospace(),
            Err(_) => RichText::new("type err!").monospace().color(Color32::RED),
        }
    } else {
        RichText::new("...")
    };
    text
}
