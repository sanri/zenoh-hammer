use eframe::egui::{
    Align, CentralPanel, Color32, Context, Grid, Layout, RichText, ScrollArea, SidePanel, TextEdit,
    TextStyle, Ui, Widget,
};
use egui_dnd::dnd;
use egui_file::{DialogType, FileDialog};
use egui_json_tree::JsonTree;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use serde_json;
use std::{
    collections::{BTreeMap, VecDeque},
    fs,
    path::PathBuf,
    str::FromStr,
};

pub enum Event {
    Connect(Box<(u64, PathBuf)>),
    Disconnect,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ArchiveConfigFileData {
    name: String,
    path: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ArchivePageSession {
    config_files: Vec<ArchiveConfigFileData>,
}

struct ConfigFileData {
    id: u64,
    name: String,
    path: Option<PathBuf>,
    path_str: String,
    err_str: Option<String>,
    selected_page: FilePage,
    source: String,
    format: String,
    serde_json_value: serde_json::Value,
}

impl From<&ConfigFileData> for ArchiveConfigFileData {
    fn from(value: &ConfigFileData) -> Self {
        let path = match &value.path {
            None => String::new(),
            Some(o) => o.to_string_lossy().to_string(),
        };

        ArchiveConfigFileData {
            name: value.name.clone(),
            path,
        }
    }
}

impl TryFrom<&ArchiveConfigFileData> for ConfigFileData {
    type Error = String;

    fn try_from(value: &ArchiveConfigFileData) -> Result<Self, Self::Error> {
        Ok(ConfigFileData {
            id: 0,
            name: value.name.clone(),
            path: PathBuf::from_str(value.path.as_str()).ok(),
            path_str: String::new(),
            err_str: None,
            selected_page: FilePage::Source,
            source: String::new(),
            format: String::new(),
            serde_json_value: serde_json::Value::Null,
        })
    }
}

impl TryFrom<ArchiveConfigFileData> for ConfigFileData {
    type Error = String;

    fn try_from(value: ArchiveConfigFileData) -> Result<Self, Self::Error> {
        Ok(ConfigFileData {
            id: 0,
            name: value.name,
            path: PathBuf::from_str(value.path.as_str()).ok(),
            path_str: String::new(),
            err_str: None,
            selected_page: FilePage::Source,
            source: String::new(),
            format: String::new(),
            serde_json_value: serde_json::Value::Null,
        })
    }
}

impl ConfigFileData {
    fn new(name: String, path: PathBuf) -> Self {
        ConfigFileData {
            id: 0,
            name,
            path: Some(path),
            path_str: String::new(),
            err_str: None,
            selected_page: FilePage::Source,
            source: String::new(),
            format: String::new(),
            serde_json_value: serde_json::Value::Null,
        }
    }

    fn show(
        &mut self,
        ui: &mut Ui,
        connected_config_file_id: Option<u64>,
        events: &mut VecDeque<Event>,
    ) {
        self.show_name_path(ui, connected_config_file_id, events);

        if let Some(s) = &self.err_str {
            ui.label(RichText::new(s).color(Color32::RED));
        }

        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.selected_page == FilePage::Source, "source")
                .clicked()
            {
                self.selected_page = FilePage::Source;
            }

            if ui
                .selectable_label(self.selected_page == FilePage::Format, "format")
                .clicked()
            {
                self.selected_page = FilePage::Format;
            }

            if ui
                .selectable_label(self.selected_page == FilePage::Tree, "tree")
                .clicked()
            {
                self.selected_page = FilePage::Tree;
            }
        });

        ui.add_space(4.0);

        ScrollArea::both()
            .auto_shrink([false, true])
            .show(ui, |ui| match self.selected_page {
                FilePage::Source => {
                    TextEdit::multiline(&mut self.source)
                        .desired_width(f32::INFINITY)
                        .code_editor()
                        .interactive(false)
                        .ui(ui);
                }
                FilePage::Format => {
                    TextEdit::multiline(&mut self.format)
                        .desired_width(f32::INFINITY)
                        .code_editor()
                        .interactive(false)
                        .ui(ui);
                }
                FilePage::Tree => {
                    JsonTree::new("page_session_json_tree", &self.serde_json_value).show(ui);
                }
            });
    }

    fn show_name_path(
        &mut self,
        ui: &mut Ui,
        connected_config_file_id: Option<u64>,
        events: &mut VecDeque<Event>,
    ) {
        Grid::new("page_session_config_file")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("name");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let mut flag_self_connected = false;

                    if let Some(id) = connected_config_file_id {
                        flag_self_connected = id == self.id;
                        ui.add_enabled_ui(flag_self_connected, |ui| {
                            if ui
                                .selectable_label(flag_self_connected, "close session")
                                .clicked()
                            {
                                events.push_back(Event::Disconnect);
                            }
                        });
                    } else {
                        if ui.selectable_label(false, "open session").clicked() {
                            self.err_str = None;
                            if let Some(path_buf) = &self.path {
                                let event = Event::Connect(Box::new((self.id, path_buf.clone())));
                                events.push_back(event);
                            } else {
                                self.err_str = Some("no file path".to_string());
                            }
                        }
                    }

                    ui.add_enabled_ui(!flag_self_connected, |ui| {
                        if ui.button("load").clicked() {
                            self.err_str = None;
                            if let Some(path_buf) = &self.path {
                                self.load_from_file(path_buf.clone());
                            } else {
                                self.err_str = Some("no file path".to_string());
                            }
                        }
                    });

                    TextEdit::singleline(&mut self.name)
                        .desired_width(3000.0)
                        .font(TextStyle::Monospace)
                        .interactive(!flag_self_connected)
                        .ui(ui);
                });
                ui.end_row();

                ui.label("path");
                if let Some(s) = &self.path {
                    self.path_str = s.to_string_lossy().to_string();
                }
                TextEdit::multiline(&mut self.path_str)
                    .desired_rows(1)
                    .desired_width(3000.0)
                    .font(TextStyle::Monospace)
                    .ui(ui);
                ui.end_row();
            });
    }

    fn load_from_file(&mut self, p: PathBuf) {
        info!("loading config file \"{}\"", p.display());
        self.source.clear();
        self.format.clear();
        self.serde_json_value = serde_json::Value::Null;
        self.err_str = None;

        let source = match fs::read_to_string(p.as_path()) {
            Ok(o) => o,
            Err(e) => {
                warn!("failed to load config file, {}", e);
                self.err_str = Some("failed to load config file".to_string());
                return;
            }
        };

        let serde_json_value = match json5::from_str::<serde_json::Value>(source.as_str()) {
            Ok(o) => o,
            Err(e) => {
                warn!("failed to load config file, {}", e);
                self.err_str = Some("failed to load config file".to_string());
                return;
            }
        };

        self.source = source;
        self.format = serde_json::to_string_pretty(&serde_json_value).unwrap_or_default();
        self.serde_json_value = serde_json_value;

        info!("load config file ok \"{}\"", p.display());
    }
}

pub struct PageSession {
    pub events: VecDeque<Event>,
    connected_config_file_id: Option<u64>,
    selected_config_file_id: u64,
    config_file_id_count: u64,
    config_files: BTreeMap<u64, ConfigFileData>,
    dnd_items: Vec<DndItem>,
    file_dialog: Option<FileDialog>,
}

impl Default for PageSession {
    fn default() -> Self {
        PageSession {
            events: VecDeque::new(),
            connected_config_file_id: None,
            selected_config_file_id: 0,
            config_file_id_count: 0,
            config_files: BTreeMap::new(),
            dnd_items: Vec::new(),
            file_dialog: None,
        }
    }
}

impl From<&PageSession> for ArchivePageSession {
    fn from(value: &PageSession) -> Self {
        ArchivePageSession {
            config_files: value
                .dnd_items
                .iter()
                .filter_map(|k| value.config_files.get(&k.id))
                .map(|d| d.into())
                .collect(),
        }
    }
}

impl PageSession {
    pub fn load(&mut self, archive: ArchivePageSession) -> Result<(), String> {
        let mut data = Vec::with_capacity(archive.config_files.len());
        for d in archive.config_files {
            let config_file_data = ConfigFileData::try_from(d)?;
            data.push(config_file_data);
        }

        self.clean_all_config_file_data();

        for d in data {
            self.add_config_file(d);
        }
        Ok(())
    }

    pub fn show(&mut self, ctx: &Context) {
        SidePanel::left("page_session_panel_left")
            .resizable(true)
            .show(ctx, |ui| {
                self.show_config_file_list(ui);
            });

        CentralPanel::default().show(ctx, |ui| {
            if let Some(config_file_data) = self.config_files.get_mut(&self.selected_config_file_id)
            {
                config_file_data.show(ui, self.connected_config_file_id, &mut self.events);
            }
        });

        let mut open_file_path: Option<PathBuf> = None;
        if let Some(dialog) = &mut self.file_dialog {
            if dialog.show(ctx).selected() {
                match dialog.dialog_type() {
                    DialogType::SelectFolder | DialogType::SaveFile => {
                        return;
                    }
                    DialogType::OpenFile => {
                        if let Some(p) = dialog.path() {
                            if let Ok(o) = p.canonicalize() {
                                open_file_path = Some(o);
                            }
                        }
                    }
                }
            }
        }
        if let Some(p) = open_file_path {
            let name = match p.file_stem() {
                None => "new file".to_string(),
                Some(o) => o.to_string_lossy().to_string(),
            };
            self.add_config_file(ConfigFileData::new(name, p));
        }
    }

    fn add_config_file(&mut self, mut config_file_data: ConfigFileData) {
        self.config_file_id_count += 1;
        let id = self.config_file_id_count;
        self.selected_config_file_id = id;

        config_file_data.id = id;

        self.config_files.insert(id, config_file_data);
        self.dnd_items.push(DndItem { id });
    }

    fn del_config_file(&mut self) {
        if self.config_files.len() < 2 {
            return;
        }

        let remove_id = self.selected_config_file_id;

        if let Some(id) = self.connected_config_file_id {
            if id == remove_id {
                return;
            }
        }

        let _ = self.config_files.remove(&remove_id);

        let mut del_index = None;
        for (i, di) in self.dnd_items.iter().enumerate() {
            if di.id == remove_id {
                del_index = Some(i);
                break;
            }
        }
        if let Some(i) = del_index {
            self.dnd_items.remove(i);
        }
    }

    pub fn connected(&self) -> bool {
        self.connected_config_file_id.is_some()
    }

    pub fn set_connected(&mut self, connected_id: Option<u64>) {
        self.connected_config_file_id = connected_id;
    }

    pub fn set_connect_result(&mut self, r: Result<u64, (u64, String)>) {
        match r {
            Ok(id) => {
                self.connected_config_file_id = Some(id);
                if let Some(cf) = self.config_files.get_mut(&id) {
                    cf.err_str = None;
                }
            }
            Err((id, s)) => {
                self.connected_config_file_id = None;
                if let Some(cf) = self.config_files.get_mut(&id) {
                    cf.err_str = Some(s);
                }
            }
        }
    }

    fn show_config_file_list(&mut self, ui: &mut Ui) {
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            if ui.button(RichText::new(" + ").code()).clicked() {
                let mut dialog = FileDialog::open_file(None)
                    .show_new_folder(false)
                    .show_rename(false);
                dialog.open();
                self.file_dialog = Some(dialog);
            }

            if ui.button(RichText::new(" - ").code()).clicked() {
                self.del_config_file();
            }
        });

        ui.add_space(10.0);

        ScrollArea::both()
            .max_width(200.0)
            .auto_shrink([true, false])
            .show(ui, |ui| {
                dnd(ui, "page_session_config_list").show_vec(
                    self.dnd_items.as_mut_slice(),
                    |ui, item, handle, _state| {
                        if let Some(d) = self.config_files.get(&item.id) {
                            let text = if let Some(id) = self.connected_config_file_id {
                                if id == item.id {
                                    RichText::new(d.name.as_str()).underline().strong()
                                } else {
                                    RichText::new(d.name.as_str())
                                }
                            } else {
                                RichText::new(d.name.as_str())
                            };

                            handle.ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.selected_config_file_id,
                                    item.id,
                                    text,
                                );
                            });
                        }
                    },
                );
            });
    }

    fn clean_all_config_file_data(&mut self) {
        self.selected_config_file_id = 0;
        self.config_files.clear();
    }
}

#[derive(Hash)]
struct DndItem {
    id: u64,
}

#[derive(Eq, PartialEq, Copy, Clone)]
enum FilePage {
    Source,
    Format,
    Tree,
}
