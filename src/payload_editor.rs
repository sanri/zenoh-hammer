use eframe::egui::{
    Button, Color32, ColorImage, ComboBox, DragValue, Grid, RichText, TextEdit, TextureHandle,
    TextureOptions, Ui, Widget,
};
use egui_file::{DialogType, FileDialog};
use egui_plot::{Corner, Legend, Plot, PlotImage, PlotPoint};
use hex::decode;
use image::{ImageFormat, ImageReader};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use size_fmt::Buffer;
use std::{
    fs::read,
    io::Cursor,
    path::{Path, PathBuf},
    str::{from_utf8, FromStr},
    sync::Arc,
};
use strum::EnumCount;
use zenoh::{
    bytes::{Encoding, ZBytes},
    internal::{buffers::ZSlice, Value},
};

use crate::{hex_viewer::HexViewer, zenoh_data::KnownEncoding};

pub struct PayloadEdit {
    encoding_id: u16,
    encoding_schema: String,
    page: Page,
    data_load_mode: DataLoadMode,
    payload_str: String,
    file_path_str: String,
    file_path: Option<PathBuf>,
    file_dialog: Option<FileDialog>,
    image_data: Option<ColorImage>,
    image_texture: Option<TextureHandle>,
    file_load_result: Option<Result<String, String>>,
    parse_str_result: Option<Result<String, String>>,
    hex_view: HexViewer,
}

impl Default for PayloadEdit {
    fn default() -> Self {
        PayloadEdit {
            encoding_id: KnownEncoding::TextPlain.to_u16(),
            encoding_schema: "".to_string(),
            page: Page::Source,
            data_load_mode: DataLoadMode::String,
            payload_str: "".to_string(),
            file_path_str: "".to_string(),
            file_path: None,
            file_dialog: None,
            image_data: None,
            image_texture: None,
            file_load_result: None,
            parse_str_result: None,
            hex_view: HexViewer::default(),
        }
    }
}

impl PayloadEdit {
    pub fn show(&mut self, ui: &mut Ui) {
        self.show_encoding_editor(ui);

        ui.add_space(4.0);

        match KnownEncoding::from(self.encoding_id) {
            KnownEncoding::ZBytes => self.show_binary_editor(ui),
            KnownEncoding::ZString => self.show_string_editor(ui),
            KnownEncoding::ZSerialized => self.show_binary_editor(ui),
            KnownEncoding::AppOctetStream => self.show_binary_editor(ui),
            KnownEncoding::TextPlain => self.show_string_editor(ui),
            KnownEncoding::AppJson => self.show_string_editor(ui),
            KnownEncoding::TextJson => self.show_string_editor(ui),
            KnownEncoding::AppCdr => self.show_binary_editor(ui),
            KnownEncoding::AppCbor => self.show_binary_editor(ui),
            KnownEncoding::AppYaml => self.show_string_editor(ui),
            KnownEncoding::TextYaml => self.show_string_editor(ui),
            KnownEncoding::TextJson5 => self.show_string_editor(ui),
            KnownEncoding::AppPythonSerializedObject => self.show_binary_editor(ui),
            KnownEncoding::AppProtobuf => self.show_binary_editor(ui),
            KnownEncoding::AppJavaSerializedObject => self.show_binary_editor(ui),
            KnownEncoding::AppOpenMetricsText => self.show_string_editor(ui),
            KnownEncoding::ImagePng => self.show_image_editor(ui),
            KnownEncoding::ImageJpeg => self.show_image_editor(ui),
            KnownEncoding::ImageGif => self.show_image_editor(ui),
            KnownEncoding::ImageBmp => self.show_image_editor(ui),
            KnownEncoding::ImageWebP => self.show_image_editor(ui),
            KnownEncoding::AppXml => self.show_string_editor(ui),
            KnownEncoding::AppXWwwFormUrlencoded => self.show_string_editor(ui),
            KnownEncoding::TextHtml => self.show_string_editor(ui),
            KnownEncoding::TextXml => self.show_string_editor(ui),
            KnownEncoding::TextCss => self.show_string_editor(ui),
            KnownEncoding::TextJavascript => self.show_string_editor(ui),
            KnownEncoding::TextMarkdown => self.show_string_editor(ui),
            KnownEncoding::TextCsv => self.show_string_editor(ui),
            KnownEncoding::AppSql => self.show_string_editor(ui),
            KnownEncoding::AppCoapPayload => self.show_binary_editor(ui),
            KnownEncoding::AppJsonPathJson => self.show_string_editor(ui),
            KnownEncoding::AppJsonSeq => self.show_string_editor(ui),
            KnownEncoding::AppJsonPath => self.show_string_editor(ui),
            KnownEncoding::AppJwt => self.show_binary_editor(ui),
            KnownEncoding::AppMp4 => self.show_binary_editor(ui),
            KnownEncoding::AppSoapXml => self.show_string_editor(ui),
            KnownEncoding::AppYang => self.show_binary_editor(ui),
            KnownEncoding::AudioAac => self.show_binary_editor(ui),
            KnownEncoding::AudioFlac => self.show_binary_editor(ui),
            KnownEncoding::AudioMp4 => self.show_binary_editor(ui),
            KnownEncoding::AudioOgg => self.show_binary_editor(ui),
            KnownEncoding::AudioVorbis => self.show_binary_editor(ui),
            KnownEncoding::VideoH261 => self.show_binary_editor(ui),
            KnownEncoding::VideoH263 => self.show_binary_editor(ui),
            KnownEncoding::VideoH264 => self.show_binary_editor(ui),
            KnownEncoding::VideoH265 => self.show_binary_editor(ui),
            KnownEncoding::VideoH266 => self.show_binary_editor(ui),
            KnownEncoding::VideoMp4 => self.show_binary_editor(ui),
            KnownEncoding::VideoOgg => self.show_binary_editor(ui),
            KnownEncoding::VideoRaw => self.show_binary_editor(ui),
            KnownEncoding::VideoVp8 => self.show_binary_editor(ui),
            KnownEncoding::VideoVp9 => self.show_binary_editor(ui),
            KnownEncoding::Other(_) => self.show_binary_editor(ui),
        }

        if let Some(dialog) = &mut self.file_dialog {
            if dialog.show(ui.ctx()).selected() {
                if let DialogType::OpenFile = dialog.dialog_type() {
                    self.file_path = dialog.path().map(|p| p.to_path_buf());
                }
            }
        }
    }

    pub fn get_zenoh_value(&mut self) -> Option<Value> {
        let payload: Option<ZBytes> = match KnownEncoding::from(self.encoding_id) {
            KnownEncoding::ZBytes => self.get_binary_zbytes(),
            KnownEncoding::ZString => self.get_string_zbytes(),
            KnownEncoding::ZSerialized => self.get_binary_zbytes(),
            KnownEncoding::AppOctetStream => self.get_binary_zbytes(),
            KnownEncoding::TextPlain => self.get_string_zbytes(),
            KnownEncoding::AppJson => self.get_string_zbytes(),
            KnownEncoding::TextJson => self.get_string_zbytes(),
            KnownEncoding::AppCdr => self.get_binary_zbytes(),
            KnownEncoding::AppCbor => self.get_binary_zbytes(),
            KnownEncoding::AppYaml => self.get_string_zbytes(),
            KnownEncoding::TextYaml => self.get_string_zbytes(),
            KnownEncoding::TextJson5 => self.get_string_zbytes(),
            KnownEncoding::AppPythonSerializedObject => self.get_binary_zbytes(),
            KnownEncoding::AppProtobuf => self.get_binary_zbytes(),
            KnownEncoding::AppJavaSerializedObject => self.get_binary_zbytes(),
            KnownEncoding::AppOpenMetricsText => self.get_string_zbytes(),
            KnownEncoding::ImagePng => self.get_image_zbytes(),
            KnownEncoding::ImageJpeg => self.get_image_zbytes(),
            KnownEncoding::ImageGif => self.get_image_zbytes(),
            KnownEncoding::ImageBmp => self.get_image_zbytes(),
            KnownEncoding::ImageWebP => self.get_image_zbytes(),
            KnownEncoding::AppXml => self.get_string_zbytes(),
            KnownEncoding::AppXWwwFormUrlencoded => self.get_string_zbytes(),
            KnownEncoding::TextHtml => self.get_string_zbytes(),
            KnownEncoding::TextXml => self.get_string_zbytes(),
            KnownEncoding::TextCss => self.get_string_zbytes(),
            KnownEncoding::TextJavascript => self.get_string_zbytes(),
            KnownEncoding::TextMarkdown => self.get_string_zbytes(),
            KnownEncoding::TextCsv => self.get_string_zbytes(),
            KnownEncoding::AppSql => self.get_string_zbytes(),
            KnownEncoding::AppCoapPayload => self.get_binary_zbytes(),
            KnownEncoding::AppJsonPathJson => self.get_string_zbytes(),
            KnownEncoding::AppJsonSeq => self.get_string_zbytes(),
            KnownEncoding::AppJsonPath => self.get_string_zbytes(),
            KnownEncoding::AppJwt => self.get_binary_zbytes(),
            KnownEncoding::AppMp4 => self.get_binary_zbytes(),
            KnownEncoding::AppSoapXml => self.get_string_zbytes(),
            KnownEncoding::AppYang => self.get_binary_zbytes(),
            KnownEncoding::AudioAac => self.get_binary_zbytes(),
            KnownEncoding::AudioFlac => self.get_binary_zbytes(),
            KnownEncoding::AudioMp4 => self.get_binary_zbytes(),
            KnownEncoding::AudioOgg => self.get_binary_zbytes(),
            KnownEncoding::AudioVorbis => self.get_binary_zbytes(),
            KnownEncoding::VideoH261 => self.get_binary_zbytes(),
            KnownEncoding::VideoH263 => self.get_binary_zbytes(),
            KnownEncoding::VideoH264 => self.get_binary_zbytes(),
            KnownEncoding::VideoH265 => self.get_binary_zbytes(),
            KnownEncoding::VideoH266 => self.get_binary_zbytes(),
            KnownEncoding::VideoMp4 => self.get_binary_zbytes(),
            KnownEncoding::VideoOgg => self.get_binary_zbytes(),
            KnownEncoding::VideoRaw => self.get_binary_zbytes(),
            KnownEncoding::VideoVp8 => self.get_binary_zbytes(),
            KnownEncoding::VideoVp9 => self.get_binary_zbytes(),
            KnownEncoding::Other(_) => self.get_binary_zbytes(),
        };

        if let Some(zbytes) = payload {
            let encoding_schema: Option<ZSlice> = if self.encoding_schema.is_empty() {
                None
            } else {
                Some(self.encoding_schema.as_bytes().to_vec().into())
            };
            let encoding = Encoding::new(self.encoding_id, encoding_schema);
            Some(Value::new(zbytes, encoding))
        } else {
            None
        }
    }

    fn show_encoding_editor(&mut self, ui: &mut Ui) {
        let show_grid = |ui: &mut Ui| {
            ui.label("encoding id:");
            ui.horizontal(|ui| {
                DragValue::new(&mut self.encoding_id)
                    .speed(1)
                    .range(0..=1023)
                    .ui(ui);

                ui.add_space(4.0);

                let label_text = Encoding::new(self.encoding_id, None).to_string();
                let list_ui = |ui: &mut Ui| {
                    for i in 0..(KnownEncoding::COUNT - 1) {
                        let id = i as u16;
                        let text = format!("{:>2}: {}", id, Encoding::new(id, None).to_string());
                        let rich_text = RichText::new(text).monospace();
                        ui.selectable_value(&mut self.encoding_id, id, rich_text);
                    }
                };
                ComboBox::new("payload_encoding", "")
                    .selected_text(label_text)
                    .show_ui(ui, list_ui)
            });
            ui.end_row();

            ui.label("encoding schema:");
            TextEdit::singleline(&mut self.encoding_schema)
                .desired_width(300.0)
                .ui(ui);
            ui.end_row();
        };

        Grid::new("payload_editor_encoding_editor")
            .num_columns(2)
            .striped(false)
            .show(ui, show_grid);
    }

    fn show_string_editor(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.page == Page::Source, "source")
                .clicked()
            {
                self.page = Page::Source;
            }

            if ui.selectable_label(self.page == Page::Hex, "hex").clicked() {
                self.page = Page::Hex;
                if self.data_load_mode == DataLoadMode::String {
                    self.hex_view = HexViewer::new(Arc::new(self.payload_str.as_bytes().to_vec()));
                }
            }
        });

        ui.add_space(4.0);

        match self.page {
            Page::Source => {
                ui.horizontal(|ui| {
                    ComboBox::new("string load mode", "")
                        .selected_text(self.data_load_mode.to_str())
                        .show_ui(ui, |ui| {
                            let list = [DataLoadMode::String, DataLoadMode::File];
                            for i in list {
                                ui.selectable_value(&mut self.data_load_mode, i, i.to_str());
                            }
                        });

                    match self.data_load_mode {
                        DataLoadMode::String => {}
                        DataLoadMode::File => {
                            if ui.button("select text file").clicked() {
                                let mut dialog = FileDialog::open_file(self.file_path.clone())
                                    .show_new_folder(false)
                                    .show_rename(false);
                                dialog.open();
                                self.file_dialog = Some(dialog);
                            }
                            if ui.button("load").clicked() {
                                let _ = self.string_load_file();
                            }
                        }
                    }
                });

                ui.add_space(4.0);

                match self.data_load_mode {
                    DataLoadMode::String => {
                        TextEdit::multiline(&mut self.payload_str)
                            .desired_width(f32::INFINITY)
                            .desired_rows(3)
                            .code_editor()
                            .ui(ui);
                    }
                    DataLoadMode::File => {
                        if let Some(p) = &self.file_path {
                            if let Some(s) = p.to_str() {
                                self.file_path_str = s.to_string();
                            }
                        } else {
                            self.file_path_str.clear();
                        }

                        ui.horizontal(|ui| {
                            ui.label("file path:");
                            TextEdit::multiline(&mut self.file_path_str)
                                .desired_width(f32::INFINITY)
                                .desired_rows(1)
                                .code_editor()
                                .ui(ui);
                        });

                        if let Some(load_result) = &self.file_load_result {
                            let text = match load_result {
                                Ok(o) => RichText::new(o),
                                Err(e) => RichText::new(e).color(Color32::RED),
                            };
                            ui.label(text);
                        }
                    }
                }
            }
            Page::Hex => {
                self.hex_view.show(ui);
            }
        }
    }

    fn show_binary_editor(&mut self, ui: &mut Ui) {
        self.show_page_selectable(ui);

        match self.page {
            Page::Source => {
                ui.horizontal(|ui| {
                    ComboBox::new("binary load mode", "")
                        .selected_text(self.data_load_mode.to_str())
                        .show_ui(ui, |ui| {
                            let list = [DataLoadMode::String, DataLoadMode::File];
                            for i in list {
                                ui.selectable_value(&mut self.data_load_mode, i, i.to_str());
                            }
                        });

                    match self.data_load_mode {
                        DataLoadMode::String => {
                            let b = Button::new("parse");
                            if b.ui(ui)
                                .on_hover_text("parse hex string into binary data")
                                .clicked()
                            {
                                let _ = self.binary_parse_str();
                            }
                        }
                        DataLoadMode::File => {
                            if ui.button("select binary file").clicked() {
                                let mut dialog = FileDialog::open_file(self.file_path.clone())
                                    .show_new_folder(false)
                                    .show_rename(false);
                                dialog.open();
                                self.file_dialog = Some(dialog);
                            }
                            if ui.button("load").clicked() {
                                let _ = self.binary_load_file();
                            }
                        }
                    }
                });

                ui.add_space(4.0);

                if self.data_load_mode == DataLoadMode::String {
                    let text = "hex string, ignore [' ', '\\t', '\\n', '\\r']";
                    ui.label(RichText::new(text).monospace());
                }

                match self.data_load_mode {
                    DataLoadMode::String => {
                        TextEdit::multiline(&mut self.payload_str)
                            .desired_width(f32::INFINITY)
                            .desired_rows(3)
                            .code_editor()
                            .ui(ui);

                        if let Some(parse_result) = &self.parse_str_result {
                            let text = match parse_result {
                                Ok(o) => RichText::new(o),
                                Err(e) => RichText::new(e).color(Color32::RED),
                            };
                            ui.label(text);
                        }
                    }
                    DataLoadMode::File => {
                        if let Some(p) = &self.file_path {
                            if let Some(s) = p.to_str() {
                                self.file_path_str = s.to_string();
                            }
                        } else {
                            self.file_path_str.clear();
                        }

                        ui.horizontal(|ui| {
                            ui.label("file path:");
                            TextEdit::multiline(&mut self.file_path_str)
                                .desired_width(f32::INFINITY)
                                .desired_rows(1)
                                .code_editor()
                                .ui(ui);
                        });

                        if let Some(load_result) = &self.file_load_result {
                            let text = match load_result {
                                Ok(o) => RichText::new(o),
                                Err(e) => RichText::new(e).color(Color32::RED),
                            };
                            ui.label(text);
                        }
                    }
                }
            }
            Page::Hex => {
                self.hex_view.show(ui);
            }
        }
    }

    fn show_image_editor(&mut self, ui: &mut Ui) {
        self.show_page_selectable(ui);

        match self.page {
            Page::Source => {
                ui.horizontal(|ui| {
                    if ui.button("select image file").clicked() {
                        let mut dialog = FileDialog::open_file(self.file_path.clone())
                            .show_new_folder(false)
                            .show_rename(false);
                        dialog.open();
                        self.file_dialog = Some(dialog);
                    }

                    if ui.button("load").clicked() {
                        let _ = self.image_load_file();
                    }
                });

                ui.add_space(4.0);

                if let Some(p) = &self.file_path {
                    if let Some(s) = p.to_str() {
                        self.file_path_str = s.to_string();
                    }
                } else {
                    self.file_path_str.clear();
                }

                ui.horizontal(|ui| {
                    ui.label("file path");
                    TextEdit::multiline(&mut self.file_path_str)
                        .desired_width(f32::INFINITY)
                        .desired_rows(1)
                        .code_editor()
                        .ui(ui);
                });

                if let Some(load_result) = &self.file_load_result {
                    let text = match load_result {
                        Ok(o) => RichText::new(o),
                        Err(e) => RichText::new(e).color(Color32::RED),
                    };
                    ui.label(text);
                }

                if let Some(color_image) = self.image_data.take() {
                    let texture_handle = ui.ctx().load_texture(
                        "payload_image_texture",
                        color_image,
                        TextureOptions::NEAREST,
                    );

                    self.image_texture = Some(texture_handle);
                }

                if let Some(image_texture) = &self.image_texture {
                    show_image(ui, image_texture);
                }
            }
            Page::Hex => {
                self.hex_view.show(ui);
            }
        }
    }

    fn show_page_selectable(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.page == Page::Source, "source")
                .clicked()
            {
                self.page = Page::Source;
            }

            if ui.selectable_label(self.page == Page::Hex, "hex").clicked() {
                self.page = Page::Hex;
            }
        });

        ui.add_space(4.0);
    }

    fn string_load_file(&mut self) -> Option<Arc<Vec<u8>>> {
        self.parse_str_result = None;
        self.file_load_result = None;
        self.hex_view = HexViewer::default();
        self.image_texture = None;

        if let Some(file_path) = &self.file_path {
            match read(file_path.as_path()) {
                Ok(o) => {
                    if let Err(e) = from_utf8(o.as_slice()) {
                        let s = format!("load file error, {}", e);
                        warn!("{}", s);
                        self.file_load_result = Some(Err(s));
                        None
                    } else {
                        let mut buffer = Buffer::new();
                        let s = format!(
                            "load file ok, data size {}",
                            buffer.human_fmt(o.len() as u64)
                        );
                        info!("{}", s);
                        self.file_load_result = Some(Ok(s));
                        let out = Arc::new(o);
                        self.hex_view = HexViewer::new(out.clone());
                        Some(out)
                    }
                }
                Err(e) => {
                    let s = format!("load file error, {}", e);
                    warn!("{}", s);
                    self.file_load_result = Some(Err(s));
                    None
                }
            }
        } else {
            let s = "no file selected".to_string();
            warn!("{}", s);
            self.file_load_result = Some(Err(s));
            None
        }
    }

    fn binary_parse_str(&mut self) -> Option<Arc<Vec<u8>>> {
        self.parse_str_result = None;
        self.file_load_result = None;
        self.hex_view = HexViewer::default();
        self.image_texture = None;

        if self.payload_str.is_empty() {
            self.parse_str_result = Some(Ok("no data".to_string()));
            return None;
        }

        match parse_str_to_vec(self.payload_str.as_str()) {
            Ok(o) => {
                let mut buffer = Buffer::new();
                let s = format!("parse ok, data size {}", buffer.human_fmt(o.len() as u64));
                info!("{}", s);
                self.parse_str_result = Some(Ok(s));
                let out = Arc::new(o);
                self.hex_view = HexViewer::new(out.clone());
                Some(out)
            }
            Err(e) => {
                let s = format!("parse error, {}", e);
                warn!("{}", s);
                self.parse_str_result = Some(Err(s));
                None
            }
        }
    }

    fn binary_load_file(&mut self) -> Option<Arc<Vec<u8>>> {
        self.parse_str_result = None;
        self.file_load_result = None;
        self.hex_view = HexViewer::default();
        self.image_texture = None;

        if let Some(file_path) = &self.file_path {
            match read(file_path.as_path()) {
                Ok(o) => {
                    let mut buffer = Buffer::new();
                    let s = format!(
                        "load file ok, data size {}",
                        buffer.human_fmt(o.len() as u64)
                    );
                    info!("{}", s);
                    self.file_load_result = Some(Ok(s));
                    let out = Arc::new(o);
                    self.hex_view = HexViewer::new(out.clone());
                    Some(out)
                }
                Err(e) => {
                    let s = format!("load file error, {}", e);
                    warn!("{}", s);
                    self.file_load_result = Some(Err(s));
                    None
                }
            }
        } else {
            let s = "no file selected".to_string();
            warn!("{}", s);
            self.file_load_result = Some(Err(s));
            None
        }
    }

    fn image_load_file(&mut self) -> Option<Arc<Vec<u8>>> {
        self.parse_str_result = None;
        self.file_load_result = None;
        self.hex_view = HexViewer::default();
        self.image_texture = None;

        let image_format = match KnownEncoding::from(self.encoding_id) {
            KnownEncoding::ImagePng => ImageFormat::Png,
            KnownEncoding::ImageJpeg => ImageFormat::Jpeg,
            KnownEncoding::ImageGif => ImageFormat::Gif,
            KnownEncoding::ImageBmp => ImageFormat::Bmp,
            KnownEncoding::ImageWebP => ImageFormat::WebP,
            _ => {
                return None;
            }
        };

        if let Some(file_path) = &self.file_path {
            match load_image(image_format, file_path.as_path()) {
                Ok((color_image, data)) => {
                    self.image_data = Some(color_image);
                    let mut buffer = Buffer::new();
                    let s = format!(
                        "load image ok, data size {}",
                        buffer.human_fmt(data.len() as u64)
                    );
                    info!("{}", s);
                    self.file_load_result = Some(Ok(s));
                    self.hex_view = HexViewer::new(data.clone());
                    Some(data)
                }
                Err(e) => {
                    let s = format!("load image error, {}", e);
                    warn!("{}", s);
                    self.file_load_result = Some(Err(s));
                    None
                }
            }
        } else {
            let s = "no file selected".to_string();
            warn!("{}", s);
            self.file_load_result = Some(Err(s));
            None
        }
    }

    fn get_string_zbytes(&mut self) -> Option<ZBytes> {
        let data = match self.data_load_mode {
            DataLoadMode::String => {
                if self.payload_str.is_empty() {
                    None
                } else {
                    let arc_data = Arc::new(self.payload_str.as_bytes().to_vec());
                    self.hex_view = HexViewer::new(arc_data.clone());
                    Some(arc_data)
                }
            }
            DataLoadMode::File => self.string_load_file(),
        };

        data.map(|data| ZBytes::from(data.as_slice()))
    }

    fn get_binary_zbytes(&mut self) -> Option<ZBytes> {
        let data = match self.data_load_mode {
            DataLoadMode::String => self.binary_parse_str(),
            DataLoadMode::File => self.binary_load_file(),
        };
        data.map(|data| ZBytes::from(data.as_slice()))
    }

    fn get_image_zbytes(&mut self) -> Option<ZBytes> {
        let data = self.image_load_file();
        data.map(|data| ZBytes::from(data.as_slice()))
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
enum DataLoadMode {
    String,
    File,
}

impl DataLoadMode {
    fn to_str(&self) -> &'static str {
        match self {
            DataLoadMode::String => "parsing from string",
            DataLoadMode::File => "load from file",
        }
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
enum Page {
    Source,
    Hex,
}

fn parse_str_to_vec(s: &str) -> Result<Vec<u8>, String> {
    let buf = s.replace(&[' ', '\t', '\n', '\r'], "");
    decode(buf.as_bytes()).map_err(|e| e.to_string())
}

fn load_image(
    image_format: ImageFormat,
    path: &Path,
) -> Result<(ColorImage, Arc<Vec<u8>>), String> {
    let data = read(path).map_err(|e| e.to_string())?;
    let image_reader = ImageReader::with_format(Cursor::new(data.as_slice()), image_format);
    let dynamic_image = image_reader.decode().map_err(|e| e.to_string())?;
    let rgba_image = dynamic_image.to_rgba8();
    let image_size = [rgba_image.width() as usize, rgba_image.height() as usize];
    let pixels = rgba_image.as_flat_samples();
    let color_image = ColorImage::from_rgba_unmultiplied(image_size, pixels.as_slice());

    // let texture_handle = ctx.load_texture(
    //     "payload_image_texture",
    //     color_image,
    //     TextureOptions::NEAREST,
    // );

    Ok((color_image, Arc::new(data)))
}

fn show_image(ui: &mut Ui, texture: &TextureHandle) {
    let image_size = texture.size_vec2();

    let image = PlotImage::new(
        texture,
        PlotPoint::new(image_size.x / 2.0, -image_size.y / 2.0),
        image_size,
    )
    .highlight(false);

    let plot = Plot::new("payload_plot_image")
        .legend(Legend::default().position(Corner::RightTop))
        .show_x(true)
        .show_y(true)
        .show_axes([false, false])
        .show_grid(false)
        .allow_boxed_zoom(false)
        .allow_scroll(false)
        .show_background(false)
        .data_aspect(1.0);
    plot.show(ui, |plot_ui| {
        plot_ui.image(image);
    });
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ArchivePayloadEdit {
    encoding_id: u16,
    encoding_schema: String,
    page: Page,
    data_load_mode: DataLoadMode,
    payload_str: String,
    file_path_str: String,
}

impl TryFrom<&ArchivePayloadEdit> for PayloadEdit {
    type Error = String;

    fn try_from(value: &ArchivePayloadEdit) -> Result<Self, Self::Error> {
        let (file_path, file_path_str) = if value.file_path_str.is_empty() {
            (None, String::new())
        } else {
            let p = PathBuf::from_str(value.file_path_str.as_str()).map_err(|e| e.to_string())?;
            (Some(p), value.file_path_str.clone())
        };

        let payload_edit = PayloadEdit {
            encoding_id: value.encoding_id,
            encoding_schema: value.encoding_schema.clone(),
            page: value.page,
            data_load_mode: value.data_load_mode,
            payload_str: value.payload_str.clone(),
            file_path_str,
            file_path,
            file_dialog: None,
            image_data: None,
            image_texture: None,
            file_load_result: None,
            parse_str_result: None,
            hex_view: Default::default(),
        };

        Ok(payload_edit)
    }
}

impl TryFrom<ArchivePayloadEdit> for PayloadEdit {
    type Error = String;

    fn try_from(value: ArchivePayloadEdit) -> Result<Self, Self::Error> {
        let (file_path, file_path_str) = if value.file_path_str.is_empty() {
            (None, String::new())
        } else {
            let p = PathBuf::from_str(value.file_path_str.as_str()).map_err(|e| e.to_string())?;
            (Some(p), value.file_path_str)
        };

        let payload_edit = PayloadEdit {
            encoding_id: value.encoding_id,
            encoding_schema: value.encoding_schema,
            page: value.page,
            data_load_mode: value.data_load_mode,
            payload_str: value.payload_str,
            file_path_str,
            file_path,
            file_dialog: None,
            image_data: None,
            image_texture: None,
            file_load_result: None,
            parse_str_result: None,
            hex_view: Default::default(),
        };

        Ok(payload_edit)
    }
}

impl From<&PayloadEdit> for ArchivePayloadEdit {
    fn from(value: &PayloadEdit) -> Self {
        let file_path_str = match &value.file_path {
            None => String::new(),
            Some(o) => match o.to_str() {
                None => String::new(),
                Some(s) => s.to_string(),
            },
        };

        ArchivePayloadEdit {
            encoding_id: value.encoding_id,
            encoding_schema: value.encoding_schema.clone(),
            page: value.page,
            data_load_mode: value.data_load_mode,
            payload_str: value.payload_str.clone(),
            file_path_str,
        }
    }
}

impl From<&PayloadEdit> for PayloadEdit {
    fn from(value: &PayloadEdit) -> Self {
        PayloadEdit {
            encoding_id: value.encoding_id,
            encoding_schema: value.encoding_schema.clone(),
            page: value.page,
            data_load_mode: value.data_load_mode,
            payload_str: value.payload_str.clone(),
            file_path_str: value.file_path_str.clone(),
            file_path: value.file_path.clone(),
            file_dialog: None,
            image_data: None,
            image_texture: None,
            file_load_result: None,
            parse_str_result: None,
            hex_view: Default::default(),
        }
    }
}
