use eframe::{
    egui,
    egui::{Color32, Context, Id, Layout, RichText, Ui},
    emath::Align,
    Frame,
};
use egui_file::{DialogType, FileDialog};
use flume::{unbounded, TryRecvError};
use include_cargo_toml::include_toml;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter};
use zenoh::{
    prelude::{Buffer, KnownEncoding},
    value::Value,
};

use crate::{
    file::AppStoreData,
    page_get::PageGet,
    page_put::PagePut,
    page_session,
    page_session::PageSession,
    page_sub,
    page_sub::PageSub,
    zenoh::{MsgGuiToZenoh, MsgZenohToGui, Receiver, Sender},
};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, AsRefStr, EnumIter)]
pub enum Page {
    Session,
    Sub,
    Get,
    // Pub,
    Put,
}

pub struct HammerApp {
    sender_to_zenoh: Option<Sender<MsgGuiToZenoh>>,
    receiver_from_zenoh: Option<Receiver<MsgZenohToGui>>,
    opened_file: Option<PathBuf>,
    file_dialog: Option<FileDialog>,
    show_about: bool,
    selected_page: Page,
    p_session: PageSession,
    p_sub: PageSub,
    p_get: PageGet,
    p_put: PagePut,
}

impl Default for HammerApp {
    fn default() -> Self {
        HammerApp {
            sender_to_zenoh: None,
            receiver_from_zenoh: None,
            opened_file: None,
            file_dialog: None,
            show_about: false,
            selected_page: Page::Session,
            p_session: PageSession::default(),
            p_sub: PageSub::default(),
            p_get: PageGet::default(),
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
        self.processing_page_get_events();
        self.show_ui(ctx, frame);
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

impl HammerApp {
    fn show_ui(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.show_bar_contents(ui);
            });
        });

        match self.selected_page {
            Page::Session => {
                self.p_session.show(ctx);
            }
            Page::Sub => {
                self.p_sub.show(ctx);
            }
            Page::Get => {
                self.p_get.show(ctx);
            }
            Page::Put => {
                self.p_put.show(ctx);
            }
        }

        if let Some(dialog) = &mut self.file_dialog {
            if dialog.show(ctx).selected() {
                match dialog.dialog_type() {
                    DialogType::SelectFolder => {
                        return;
                    }
                    DialogType::OpenFile => {
                        if let Some(file) = dialog.path() {
                            let file = file.to_path_buf();
                            self.load_from_file(file);
                        }
                    }
                    DialogType::SaveFile => {
                        if let Some(file) = dialog.path() {
                            let file = file.to_path_buf();
                            self.store_to_file(file);
                        }
                    }
                }
            }
        }

        show_about_window(ctx, &mut self.show_about);
    }

    fn show_bar_contents(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("file", |ui| {
            ui.set_min_width(80.0);

            if ui.add(egui::Button::new("load..")).clicked() {
                if self.p_session.connected {
                    return;
                }

                let mut dialog = FileDialog::open_file(self.opened_file.clone())
                    .show_new_folder(false)
                    .show_rename(false);
                dialog.open();
                self.file_dialog = Some(dialog);
                ui.close_menu();
            }

            if ui.add(egui::Button::new("save")).clicked() {
                if let Some(p) = self.opened_file.clone() {
                    self.store_to_file(p);
                } else {
                    let mut dialog = FileDialog::save_file(self.opened_file.clone())
                        .show_new_folder(true)
                        .show_rename(true);
                    dialog.open();
                    self.file_dialog = Some(dialog);
                }
                ui.close_menu();
            }

            if ui.add(egui::Button::new("save as ..")).clicked() {
                let mut dialog = FileDialog::save_file(self.opened_file.clone())
                    .show_new_folder(true)
                    .show_rename(true);
                dialog.open();
                self.file_dialog = Some(dialog);

                ui.close_menu();
            }
        });

        ui.menu_button("help", |ui| {
            ui.set_min_width(80.0);
            ui.style_mut().wrap = Some(false);

            if ui.add(egui::Button::new("about")).clicked() {
                self.show_about = true;
                ui.close_menu();
            }

            // ui.menu_button("zoom", |ui| {
            //     egui::gui_zoom::zoom_menu_buttons(ui, native_pixels_per_point);
            // });

            // ui.separator();

            // ui.menu_button("language", |ui| {
            //     if ui.add(egui::Button::new("中文")).clicked() {
            //         ui.close_menu();
            //     }
            //
            //     if ui.add(egui::Button::new("English")).clicked() {
            //         ui.close_menu();
            //     }
            // });
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

        if let Some(file_path) = &self.opened_file {
            ui.label(format!("file: {}", file_path.to_str().unwrap()));
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            egui::widgets::global_dark_light_mode_switch(ui);
        });
    }

    fn load_from_file(&mut self, path: PathBuf) {
        match AppStoreData::load(path.as_path()) {
            Ok(o) => {
                self.load(o);
                println!("load file ok, path: {}", path.to_str().unwrap());
                self.opened_file = Some(path);
            }
            Err(e) => {
                println!("load file err, path: {} \n{}", path.to_str().unwrap(), e);
            }
        }
    }

    fn store_to_file(&mut self, path: PathBuf) {
        let asd = self.create_store_data();
        match asd.write(path.as_path()) {
            Ok(_) => {
                println!("save file: {}", path.to_str().unwrap());
                self.opened_file = Some(path);
            }
            Err(e) => {
                println!("save file err, path: {} \n{}", path.to_str().unwrap(), e);
            }
        }
    }

    fn load(&mut self, data: AppStoreData) {
        self.p_sub.load(data.page_sub);
        self.p_put.load(data.page_put);
        self.p_get.load(data.page_get);
    }

    fn create_store_data(&self) -> AppStoreData {
        AppStoreData {
            page_sub: self.p_sub.create_store_data(),
            page_put: self.p_put.create_store_data(),
            page_get: self.p_get.create_store_data(),
        }
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
                MsgZenohToGui::AddSubRes(res) => {
                    let (id, r) = *res;
                    self.p_sub.processing_add_sub_res(id, r);
                }
                MsgZenohToGui::DelSubRes(id) => {
                    self.p_sub.processing_del_sub_res(id);
                }
                MsgZenohToGui::SubCB(d) => {
                    let (id, sample) = *d;
                    self.p_sub.processing_sub_cb(id, sample);
                }
                MsgZenohToGui::GetRes(r) => {
                    self.p_get.processing_get_res(r);
                }
                MsgZenohToGui::PutRes(r) => {
                    self.p_put.processing_put_res(r);
                }
            }
        }
    }

    fn processing_page_session_events(&mut self) {
        use page_session::Event;
        while let Some(event) = self.p_session.events.pop_front() {
            match event {
                Event::Connect(c) => {
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
                Event::Disconnect => {
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
                page_sub::Event::AddSub(event) => {
                    if let Some(sender) = &self.sender_to_zenoh {
                        let _ = sender.send(MsgGuiToZenoh::AddSubReq(event));
                    } else {
                        let id = (*event).0;
                        self.p_sub
                            .processing_add_sub_res(id, Err("not connected".to_string()));
                    }
                }
                page_sub::Event::DelSub(id) => {
                    if let Some(sender) = &self.sender_to_zenoh {
                        let _ = sender.send(MsgGuiToZenoh::DelSubReq(id));
                    }
                }
            }
        }
    }

    fn processing_page_put_events(&mut self) {
        while let Some(event) = self.p_put.events.pop_front() {
            match event {
                crate::page_put::Event::Put(p) => {
                    if let Some(sender) = &self.sender_to_zenoh {
                        let _ = sender.send(MsgGuiToZenoh::PutReq(p));
                    }
                }
            }
        }
    }

    fn processing_page_get_events(&mut self) {
        while let Some(event) = self.p_get.events.pop_front() {
            match event {
                crate::page_get::Event::Get(p) => {
                    if let Some(sender) = &self.sender_to_zenoh {
                        let _ = sender.send(MsgGuiToZenoh::GetReq(p));
                    }
                }
            }
        }
    }
}

pub fn value_create_rich_text(d: &Value) -> Option<RichText> {
    match d.encoding.prefix() {
        KnownEncoding::AppOctetStream => Some(RichText::new("...")),
        KnownEncoding::TextPlain => Some(text_plant_create_rich_text(d)),
        KnownEncoding::AppJson => Some(json_create_rich_text(d)),
        KnownEncoding::AppInteger => Some(i64_create_rich_text(d)),
        KnownEncoding::AppFloat => Some(f64_create_rich_text(d)),
        KnownEncoding::TextJson => Some(json_create_rich_text(d)),
        KnownEncoding::Empty => None,
        KnownEncoding::AppCustom => Some(RichText::new("...")),
        KnownEncoding::AppProperties => None,
        KnownEncoding::AppSql => Some(RichText::new("...")),
        KnownEncoding::AppXml => Some(RichText::new("...")),
        KnownEncoding::AppXhtmlXml => Some(RichText::new("...")),
        KnownEncoding::AppXWwwFormUrlencoded => None,
        KnownEncoding::TextHtml => Some(RichText::new("...")),
        KnownEncoding::TextXml => Some(RichText::new("...")),
        KnownEncoding::TextCss => Some(RichText::new("...")),
        KnownEncoding::TextCsv => Some(RichText::new("...")),
        KnownEncoding::TextJavascript => Some(RichText::new("...")),
        KnownEncoding::ImageJpeg => Some(RichText::new("◪")),
        KnownEncoding::ImagePng => Some(RichText::new("◪")),
        KnownEncoding::ImageGif => None,
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

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "data")]
pub enum ZenohValue {
    Empty,
    TextPlain(String),
    TextJson(String),
    AppJson(String),
    AppInteger(i64),
    AppFloat(f64),
}

impl ZenohValue {
    pub fn to(&self) -> (KnownEncoding, String) {
        match self {
            ZenohValue::Empty => (KnownEncoding::Empty, String::new()),
            ZenohValue::TextPlain(v) => (KnownEncoding::TextPlain, v.clone()),
            ZenohValue::TextJson(v) => (KnownEncoding::TextJson, v.clone()),
            ZenohValue::AppJson(v) => (KnownEncoding::AppJson, v.clone()),
            ZenohValue::AppInteger(v) => (KnownEncoding::AppInteger, v.to_string()),
            ZenohValue::AppFloat(v) => (KnownEncoding::AppFloat, v.to_string()),
        }
    }

    pub fn from(encoding: KnownEncoding, s: String) -> Self {
        match encoding {
            KnownEncoding::Empty => ZenohValue::Empty,
            KnownEncoding::TextPlain => ZenohValue::TextPlain(s),
            KnownEncoding::AppJson => ZenohValue::AppJson(s),
            KnownEncoding::AppInteger => {
                if let Ok(i) = s.parse::<i64>() {
                    ZenohValue::AppInteger(i)
                } else {
                    ZenohValue::Empty
                }
            }
            KnownEncoding::AppFloat => {
                if let Ok(i) = s.parse::<f64>() {
                    ZenohValue::AppFloat(i)
                } else {
                    ZenohValue::Empty
                }
            }
            KnownEncoding::TextJson => ZenohValue::TextJson(s),
            _ => ZenohValue::Empty,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct TestData {
    zv: ZenohValue,
}

#[test]
fn test_zenoh_value_serialize() {
    use serde_json;
    let td = TestData {
        zv: ZenohValue::Empty,
    };
    let json_str = serde_json::to_string(&td).unwrap();
    println!("{}", json_str);

    let td = TestData {
        zv: ZenohValue::TextJson(r#"{"a":1}"#.to_string()),
    };
    let json_str = serde_json::to_string(&td).unwrap();
    println!("{}", json_str);

    let td = TestData {
        zv: ZenohValue::AppFloat(21.0),
    };
    let json_str = serde_json::to_string(&td).unwrap();
    println!("{}", json_str);
}

#[test]
fn test_zenoh_value_deserialize() {
    let json_str = r#"{"zv":{"type":"TextJson","data":"{\"a\": 1}"}}"#;
    let td: TestData = serde_json::from_str(json_str).unwrap();
    if let ZenohValue::TextJson(s) = td.zv {
        println!("{}", s);
    }
}

fn show_about_window(ctx: &Context, is_open: &mut bool) {
    let window = egui::Window::new("About")
        .id(Id::new("show about window"))
        .collapsible(false)
        .scroll2([false, false])
        .open(is_open)
        .resizable(false)
        .default_width(240.0);

    window.show(ctx, |ui| {
        use egui::special_emojis::GITHUB;

        let show_grid = |ui: &mut Ui| {
            ui.label(RichText::new(format!("{:>13}", "Hammer:")).monospace());
            let version = format!("v{}", include_toml!("package"."version"));
            ui.label(RichText::new(version).monospace());
            ui.end_row();

            ui.label(RichText::new(format!("{:>13}", "Zenoh:")).monospace());
            let version = format!("v{}", include_toml!("dependencies"."zenoh"."version"));
            ui.label(RichText::new(version).monospace());
            ui.end_row();
        };

        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            ui.label(RichText::new("Zenoh UI tool.").size(16.0));

            ui.add_space(10.0);
            egui::Grid::new("options_grid")
                .num_columns(2)
                .striped(false)
                .show(ui, |ui| {
                    show_grid(ui);
                });

            ui.add_space(10.0);
            ui.hyperlink_to(
                format!("{} Zenoh-hammer on GitHub", GITHUB),
                "https://github.com/sanri/zenoh-hammer",
            );
            ui.hyperlink_to(
                format!("{} Zenoh on GitHub", GITHUB),
                "https://github.com/eclipse-zenoh/zenoh",
            );
        });
    });
}
