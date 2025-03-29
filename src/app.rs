use eframe::{
    egui::{
        global_theme_preference_switch, special_emojis::GITHUB, Button, Context, Grid, Id, Layout,
        RichText, TopBottomPanel, Ui, Window,
    },
    emath::Align,
    Frame,
};
use egui_file::{DialogType, FileDialog};
use env_logger::init_from_env;
use flume::{unbounded, TryRecvError};
use log::{error, info, warn};
use static_toml::static_toml;
use std::{fs, path::PathBuf, time::Duration};
use strum::{AsRefStr, EnumIter, IntoEnumIterator};

use crate::{
    archive_file::ArchiveApp,
    page_get::PageGet,
    page_put::PagePut,
    page_session,
    page_session::PageSession,
    page_sub,
    page_sub::PageSub,
    task_zenoh::{start_async, MsgGuiToZenoh, MsgZenohToGui, Receiver, Sender},
};

static_toml! {
    static CARGO_INFO = include_toml!("Cargo.toml");
}

static ZENOH_HAMMER: &'static str = CARGO_INFO.package.version;
static ZENOH_VERSION: &'static str = CARGO_INFO.dependencies.zenoh.version;
static EGUI_VERSION: &'static str = CARGO_INFO.dependencies.eframe.version;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, AsRefStr, EnumIter)]
pub enum Page {
    Session,
    Sub,
    Get,
    Put,
}

pub struct HammerApp {
    app_config_path: Option<PathBuf>,
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
            app_config_path: None,
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
    pub fn set_app_config_path(&mut self, app_config_path: PathBuf) {
        self.app_config_path = Some(app_config_path);
    }

    pub fn set_opened_file(&mut self, opened_file: PathBuf) {
        self.opened_file = Some(opened_file);
    }

    fn show_ui(&mut self, ctx: &Context, _frame: &mut Frame) {
        TopBottomPanel::top("top_bar").show(ctx, |ui| {
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
                            match self.load_from_file(file.clone()) {
                                Ok(o) => {
                                    self.write_last_opened_file_path(file);
                                    info!("{}", o);
                                }
                                Err(e) => {
                                    warn!("{}", e);
                                }
                            }
                        }
                    }
                    DialogType::SaveFile => {
                        if let Some(file) = dialog.path() {
                            let file = file.to_path_buf();
                            match self.store_to_file(file.clone()) {
                                Ok(_) => {
                                    info!("save file: {}", file.to_string_lossy());
                                    self.write_last_opened_file_path(file);
                                }
                                Err(e) => {
                                    warn!(
                                        "save file err, path: {} \n{}",
                                        file.to_string_lossy(),
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        show_about_window(ctx, &mut self.show_about);
    }

    fn show_bar_contents(&mut self, ui: &mut Ui) {
        ui.menu_button("file", |ui| {
            ui.set_min_width(80.0);

            if ui.add(Button::new("load..")).clicked() {
                if self.p_session.connected() {
                    return;
                }

                let mut dialog = FileDialog::open_file(self.opened_file.clone())
                    .show_new_folder(false)
                    .show_rename(false);
                dialog.open();
                self.file_dialog = Some(dialog);
                ui.close_menu();
            }

            if ui.add(Button::new("save")).clicked() {
                if let Some(p) = self.opened_file.clone() {
                    match self.store_to_file(p.clone()) {
                        Ok(_) => {
                            info!("save file: {}", p.to_string_lossy());
                        }
                        Err(e) => {
                            warn!("save file err, path: {} \n{}", p.to_string_lossy(), e);
                        }
                    }
                } else {
                    let mut dialog = FileDialog::save_file(self.opened_file.clone())
                        .show_new_folder(true)
                        .show_rename(true);
                    dialog.open();
                    self.file_dialog = Some(dialog);
                }
                ui.close_menu();
            }

            if ui.add(Button::new("save as ..")).clicked() {
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
            // ui.style_mut().wrap = Some(false);
            // ui.style_mut().wrap_mode = Some();

            if ui.add(Button::new("about")).clicked() {
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
            ui.label(format!("{}", file_path.to_str().unwrap()));
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            global_theme_preference_switch(ui)
        });
    }

    fn write_last_opened_file_path(&mut self, opened_file_path: PathBuf) {
        if let Some(acp) = &self.app_config_path {
            let ofp_str = opened_file_path.to_string_lossy().to_string();
            if let Err(e) = fs::write(acp.as_path(), ofp_str) {
                error!("write default config file error. {}", e);
            } else {
                info!("write default config file successfully.");
            }
        }
    }

    pub fn load_from_file(&mut self, path: PathBuf) -> Result<String, String> {
        match ArchiveApp::load(path.as_path()) {
            Ok(o) => {
                let s = format!("load file ok, path: {}", path.to_str().unwrap_or_default());
                self.load(o)?;
                self.opened_file = Some(path);
                Ok(s)
            }
            Err(e) => {
                let s = format!(
                    "load file err, path: {} \n{}",
                    path.to_str().unwrap_or_default(),
                    e
                );
                Err(s)
            }
        }
    }

    fn store_to_file(&mut self, path: PathBuf) -> Result<(), String> {
        let asd = self.generate_archive();
        match asd.write(path.as_path()) {
            Ok(_) => {
                self.opened_file = Some(path);
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    fn load(&mut self, data: ArchiveApp) -> Result<(), String> {
        self.p_session.load(data.page_session)?;
        self.p_sub.load(data.page_sub)?;
        self.p_put.load(data.page_put)?;
        self.p_get.load(data.page_get)?;
        Ok(())
    }

    fn generate_archive(&self) -> ArchiveApp {
        ArchiveApp {
            page_session: (&self.p_session).into(),
            page_sub: (&self.p_sub).into(),
            page_put: (&self.p_put).into(),
            page_get: (&self.p_get).into(),
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
                        self.p_session.set_connected(None);
                        return;
                    }
                },
            };
            match msg {
                MsgZenohToGui::OpenSession(b) => {
                    if b.is_err() {
                        self.sender_to_zenoh = None;
                    }
                    self.p_session.set_connect_result(b);
                }
                MsgZenohToGui::AddSubRes(res) => {
                    let (id, r) = *res;
                    self.p_sub.processing_add_sub_res(id, r);
                }
                MsgZenohToGui::DelSubRes(id) => {
                    self.p_sub.processing_del_sub_res(id);
                }
                MsgZenohToGui::SubCB(d) => {
                    let (id, sample, receipt_time) = *d;
                    self.p_sub.processing_sub_cb(id, sample, receipt_time);
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
                    let (id, config_file_path) = *c;
                    if self.sender_to_zenoh.is_none() {
                        let (sender_to_gui, receiver_from_zenoh): (
                            Sender<MsgZenohToGui>,
                            Receiver<MsgZenohToGui>,
                        ) = unbounded();
                        let (sender_to_zenoh, receiver_from_gui): (
                            Sender<MsgGuiToZenoh>,
                            Receiver<MsgGuiToZenoh>,
                        ) = unbounded();

                        start_async(sender_to_gui, receiver_from_gui, id, config_file_path);

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
                        let id = (*event).id;
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

fn show_about_window(ctx: &Context, is_open: &mut bool) {
    let window = Window::new("About")
        .id(Id::new("show about window"))
        .collapsible(false)
        .scroll([false, false])
        .open(is_open)
        .resizable(false)
        .default_width(240.0);

    window.show(ctx, |ui| {
        let show_grid = |ui: &mut Ui| {
            ui.hyperlink_to(
                format!("{} Zenoh Hammer", GITHUB),
                "https://github.com/sanri/zenoh-hammer",
            );
            let version = format!("v{}", ZENOH_HAMMER);
            ui.label(RichText::new(version).monospace());
            ui.end_row();

            ui.hyperlink_to(
                format!("{} Zenoh", GITHUB),
                "https://github.com/eclipse-zenoh/zenoh",
            );
            let version = format!("v{}", ZENOH_VERSION);
            ui.label(RichText::new(version).monospace());
            ui.end_row();

            ui.hyperlink_to(format!("{} egui", GITHUB), "https://github.com/emilk/egui");
            let version = format!("v{}", EGUI_VERSION);
            ui.label(RichText::new(version).monospace());
            ui.end_row();
        };

        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            ui.label(RichText::new("Zenoh Hammer").size(16.0));

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add_space(40.0);
                Grid::new("options_grid")
                    .num_columns(2)
                    .striped(false)
                    .show(ui, |ui| {
                        show_grid(ui);
                    });
            });
        });
    });
}
