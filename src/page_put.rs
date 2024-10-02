use eframe::{
    egui::{
        CentralPanel, Color32, ColorImage, ComboBox, Context, Grid, Layout, RichText, ScrollArea,
        SidePanel, TextBuffer, TextEdit, TextStyle, TextureHandle, TextureOptions, Ui,
    },
    emath::Align,
};
use egui_dnd::dnd;
use egui_file::{DialogType, FileDialog};
use egui_plot::{Corner, Legend, Plot, PlotImage, PlotPoint};
use image::ImageFormat;
use serde::{Deserialize, Serialize};
use serde_json;
use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
    str::FromStr,
};
use strum::IntoEnumIterator;
use zenoh::{bytes::ZBytes, key_expr::OwnedKeyExpr};

use crate::{
    task_zenoh::PutData,
    zenoh_data::{parse_str_to_vec, KnownEncoding, ZCongestionControl, ZPriority},
};

pub enum Event {
    Put(Box<PutData>),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DataItem {
    name: String,
    key: String,
    congestion_control: ZCongestionControl,
    priority: ZPriority,
    encoding: u16,
    encoding_schema: String,
    payload: String,
    image_file_path: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Data {
    puts: Vec<DataItem>,
}

pub struct PagePutData {
    id: u64,
    name: String,
    input_key: String,
    selected_congestion_control: ZCongestionControl,
    selected_priority: ZPriority,
    selected_encoding: KnownEncoding,
    encoding_schema_edit_str: String,
    payload_edit_str: String,
    image_file_dialog: Option<FileDialog>,
    image_file_path: Option<PathBuf>,
    image_texture: Option<TextureHandle>,
    error_info: Option<RichText>,
}

impl Default for PagePutData {
    fn default() -> Self {
        PagePutData {
            id: 1,
            name: "demo".to_string(),
            input_key: "demo/example".to_string(),
            selected_congestion_control: ZCongestionControl::Block,
            selected_priority: ZPriority::RealTime,
            selected_encoding: KnownEncoding::TextPlain,
            encoding_schema_edit_str: String::new(),
            payload_edit_str: String::new(),
            image_file_path: None,
            image_file_dialog: None,
            image_texture: None,
            error_info: None,
        }
    }
}

impl PagePutData {
    fn from(data: &DataItem) -> PagePutData {
        let image_file_path = match &data.image_file_path {
            None => None,
            Some(o) => PathBuf::from_str(o.as_str()).ok(),
        };

        PagePutData {
            id: 0,
            name: data.name.clone(),
            input_key: data.key.clone(),
            selected_congestion_control: data.congestion_control,
            selected_priority: data.priority,
            selected_encoding: KnownEncoding::from(data.encoding),
            encoding_schema_edit_str: data.encoding_schema.clone(),
            payload_edit_str: data.payload.clone(),
            image_file_path,
            image_file_dialog: None,
            image_texture: None,
            error_info: None,
        }
    }

    fn to(&self) -> DataItem {
        DataItem {
            name: self.name.clone(),
            key: self.input_key.clone(),
            congestion_control: self.selected_congestion_control.into(),
            priority: self.selected_priority.into(),
            encoding: self.selected_encoding.into(),
            encoding_schema: self.encoding_schema_edit_str.clone(),
            payload: self.payload_edit_str.clone(),
            image_file_path: None,
        }
    }

    fn new_from(ppd: &PagePutData) -> Self {
        PagePutData {
            id: 0,
            name: ppd.name.clone(),
            input_key: ppd.input_key.clone(),
            selected_congestion_control: ppd.selected_congestion_control,
            selected_priority: ppd.selected_priority,
            selected_encoding: ppd.selected_encoding,
            encoding_schema_edit_str: ppd.encoding_schema_edit_str.clone(),
            payload_edit_str: ppd.payload_edit_str.clone(),
            image_file_path: ppd.image_file_path.clone(),
            image_file_dialog: None,
            image_texture: None,
            error_info: None,
        }
    }

    fn show(&mut self, ui: &mut Ui, events: &mut VecDeque<Event>) {
        ui.vertical(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                if ui.button("send").clicked() {
                    self.send(events);
                }
            });

            ui.add_space(10.0);

            self.show_name_key(ui);
            self.show_options(ui);
        });
    }

    fn show_name_key(&mut self, ui: &mut Ui) {
        let mut input_grid = |ui: &mut Ui| {
            ui.label("name:");
            let te = TextEdit::singleline(&mut self.name)
                .desired_width(600.0)
                .font(TextStyle::Monospace);
            ui.add(te);
            ui.end_row();

            ui.label("key:");
            let te = TextEdit::multiline(&mut self.input_key)
                .desired_rows(2)
                .desired_width(600.0)
                .font(TextStyle::Monospace);
            ui.add(te);

            ui.end_row();
        };

        Grid::new("input_grid")
            .num_columns(2)
            .striped(false)
            .show(ui, |ui| {
                input_grid(ui);
            });
    }

    fn show_options(&mut self, ui: &mut Ui) {
        let mut show_grid = |ui: &mut Ui| {
            ui.label("congestion control:");
            ComboBox::new("congestion control", "")
                .selected_text(self.selected_congestion_control.as_ref())
                .show_ui(ui, |ui| {
                    for option in ZCongestionControl::iter() {
                        ui.selectable_value(
                            &mut self.selected_congestion_control,
                            option,
                            option.as_ref(),
                        );
                    }
                });
            ui.end_row();

            ui.label("priority:");
            ComboBox::new("priority", "")
                .selected_text(self.selected_priority.as_ref())
                .show_ui(ui, |ui| {
                    for option in ZPriority::iter() {
                        ui.selectable_value(&mut self.selected_priority, option, option.as_ref());
                    }
                });
            ui.end_row();

            ui.label("encoding:");
            ui.horizontal(|ui| {
                ComboBox::new("encoding", "")
                    .selected_text(format!("{}", self.selected_encoding.to_encoding()))
                    .show_ui(ui, |ui| {
                        for option in KnownEncoding::iter() {
                            ui.selectable_value(
                                &mut self.selected_encoding,
                                option,
                                format!("{}", option.to_encoding()),
                            );
                        }
                    });
            });
            ui.end_row();
        };

        Grid::new("options_grid")
            .num_columns(2)
            .striped(false)
            .show(ui, |ui| {
                show_grid(ui);
            });

        match self.selected_encoding {
            KnownEncoding::TextPlain => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::AppJson => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            // KnownEncoding::AppInteger => {
            // }
            KnownEncoding::TextJson => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::AppOctetStream => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::AppSql => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::AppXml => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::AppXWwwFormUrlencoded => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::TextHtml => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::TextXml => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::TextCss => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::TextCsv => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::TextJavascript => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::ImageJpeg => {
                Self::image_view(
                    ui,
                    &mut self.image_file_path,
                    &mut self.image_file_dialog,
                    &mut self.image_texture,
                    &mut self.error_info,
                    ImageFormat::Jpeg,
                );
            }
            KnownEncoding::ImagePng => {
                Self::image_view(
                    ui,
                    &mut self.image_file_path,
                    &mut self.image_file_dialog,
                    &mut self.image_texture,
                    &mut self.error_info,
                    ImageFormat::Png,
                );
            }
            KnownEncoding::ImageGif => {
                Self::image_view(
                    ui,
                    &mut self.image_file_path,
                    &mut self.image_file_dialog,
                    &mut self.image_texture,
                    &mut self.error_info,
                    ImageFormat::Gif,
                );
            }
            KnownEncoding::ZBytes => {}
            KnownEncoding::ZInt8 => {
                ui.add(TextEdit::singleline(&mut self.payload_edit_str));
            }
            KnownEncoding::ZInt16 => {
                ui.add(TextEdit::singleline(&mut self.payload_edit_str));
            }
            KnownEncoding::ZInt32 => {
                ui.add(TextEdit::singleline(&mut self.payload_edit_str));
            }
            KnownEncoding::ZInt64 => {
                ui.add(TextEdit::singleline(&mut self.payload_edit_str));
            }
            KnownEncoding::ZInt128 => {
                ui.add(TextEdit::singleline(&mut self.payload_edit_str));
            }
            KnownEncoding::ZUint8 => {
                ui.add(TextEdit::singleline(&mut self.payload_edit_str));
            }
            KnownEncoding::ZUint16 => {
                ui.add(TextEdit::singleline(&mut self.payload_edit_str));
            }
            KnownEncoding::ZUint32 => {
                ui.add(TextEdit::singleline(&mut self.payload_edit_str));
            }
            KnownEncoding::ZUint64 => {
                ui.add(TextEdit::singleline(&mut self.payload_edit_str));
            }
            KnownEncoding::ZUint128 => {
                ui.add(TextEdit::singleline(&mut self.payload_edit_str));
            }
            KnownEncoding::ZFloat32 => {
                ui.add(TextEdit::singleline(&mut self.payload_edit_str));
            }
            KnownEncoding::ZFloat64 => {
                ui.add(TextEdit::singleline(&mut self.payload_edit_str));
            }
            KnownEncoding::ZBool => {
                ui.add(TextEdit::singleline(&mut self.payload_edit_str));
            }
            KnownEncoding::ZString => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::ZError => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::AppCdr => {}
            KnownEncoding::AppCbor => {}
            KnownEncoding::AppYaml => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::TextYaml => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::TextJson5 => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::AppPythonSerializedObject => {}
            KnownEncoding::AppProtobuf => {}
            KnownEncoding::AppJavaSerializedObject => {}
            KnownEncoding::AppOpenMetricsText => {}
            KnownEncoding::ImageBmp => {
                Self::image_view(
                    ui,
                    &mut self.image_file_path,
                    &mut self.image_file_dialog,
                    &mut self.image_texture,
                    &mut self.error_info,
                    ImageFormat::Bmp,
                );
            }
            KnownEncoding::ImageWebP => {
                Self::image_view(
                    ui,
                    &mut self.image_file_path,
                    &mut self.image_file_dialog,
                    &mut self.image_texture,
                    &mut self.error_info,
                    ImageFormat::WebP,
                );
            }
            KnownEncoding::TextMarkdown => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::AppCoapPayload => {}
            KnownEncoding::AppJsonPathJson => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::AppJsonSeq => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::AppJsonPath => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::AppJwt => {}
            KnownEncoding::AppMp4 => {}
            KnownEncoding::AppSoapXml => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
            KnownEncoding::AppYang => {}
            KnownEncoding::AudioAac => {}
            KnownEncoding::AudioFlac => {}
            KnownEncoding::AudioMp4 => {}
            KnownEncoding::AudioOgg => {}
            KnownEncoding::AudioVorbis => {}
            KnownEncoding::VideoH261 => {}
            KnownEncoding::VideoH263 => {}
            KnownEncoding::VideoH264 => {}
            KnownEncoding::VideoH265 => {}
            KnownEncoding::VideoH266 => {}
            KnownEncoding::VideoMp4 => {}
            KnownEncoding::VideoOgg => {}
            KnownEncoding::VideoRaw => {}
            KnownEncoding::VideoVp8 => {}
            KnownEncoding::VideoVp9 => {}
            KnownEncoding::Other(_) => {
                Self::text_edit_multiline(&mut self.payload_edit_str, &self.error_info, ui);
            }
        }
    }

    fn text_edit_multiline(edit_str: &mut String, info: &Option<RichText>, ui: &mut Ui) {
        ui.label("value: ");

        if let Some(rt) = info {
            ui.label(rt.clone());
        };

        ui.add(
            TextEdit::multiline(edit_str)
                .desired_width(f32::INFINITY)
                .desired_rows(3)
                .code_editor(),
        );
    }

    fn image_view(
        ui: &mut Ui,
        image_file_path: &mut Option<PathBuf>,
        image_file_dialog: &mut Option<FileDialog>,
        image_texture: &mut Option<TextureHandle>,
        info: &mut Option<RichText>,
        image_format: ImageFormat,
    ) {
        ui.horizontal(|ui| {
            ui.label("image path:");
            if let Some(p) = image_file_path {
                let s = p.to_str().unwrap_or("").to_string();
                ui.label(s);
            }
        });
        ui.horizontal(|ui| {
            if ui.button("load").clicked() {
                let mut dialog = FileDialog::open_file(image_file_path.clone())
                    .show_new_folder(false)
                    .show_rename(false);
                dialog.open();
                *image_file_dialog = Some(dialog);
            }
            if let Some(p) = image_file_path {
                if ui.button("reload").clicked() {
                    let file = p.to_path_buf();
                    match Self::load_image(file.as_path(), ui.ctx()) {
                        Ok((texture, format)) => {
                            if Some(image_format) == format {
                                *image_texture = Some(texture);
                                *image_file_path = Some(file);
                                *info = None;
                            } else {
                                let text = RichText::new("file format error").color(Color32::RED);
                                *image_texture = None;
                                *image_file_path = None;
                                *info = Some(text);
                            }
                        }
                        Err(e) => {
                            let text = RichText::new(e).color(Color32::RED);
                            *info = Some(text);
                            *image_texture = None;
                            *image_file_path = None;
                        }
                    }
                }
            }
        });

        if let Some(rt) = info {
            ui.label(rt.clone());
        };

        if let Some(dialog) = image_file_dialog {
            if dialog.show(ui.ctx()).selected() {
                if DialogType::OpenFile == dialog.dialog_type() {
                    if let Some(file) = dialog.path() {
                        let file = file.to_path_buf();
                        match Self::load_image(file.as_path(), ui.ctx()) {
                            Ok((texture, format)) => {
                                if Some(image_format) == format {
                                    *image_texture = Some(texture);
                                    *image_file_path = Some(file);
                                    *info = None;
                                } else {
                                    let text =
                                        RichText::new("file format error").color(Color32::RED);
                                    *info = Some(text);
                                    *image_texture = None;
                                    *image_file_path = None;
                                }
                            }
                            Err(e) => {
                                let text = RichText::new(e).color(Color32::RED);
                                *info = Some(text);
                                *image_texture = None;
                                *image_file_path = None;
                            }
                        }
                    }
                }
            }
        }

        if let Some(image_texture) = image_texture {
            Self::show_image(ui, image_texture);
        }
    }

    fn load_image(
        path: &Path,
        ctx: &Context,
    ) -> Result<(TextureHandle, Option<ImageFormat>), String> {
        let data = match image::io::Reader::open(path) {
            Ok(o) => o,
            Err(e) => {
                return Err(e.to_string());
            }
        };
        let format = data.format().clone();
        let dynamic_image = match data.decode() {
            Ok(o) => o,
            Err(e) => {
                return Err(e.to_string());
            }
        };
        let rgba_image = dynamic_image.to_rgba8();
        let image_size = [rgba_image.width() as usize, rgba_image.height() as usize];
        let pixels = rgba_image.as_flat_samples();
        let color_image = ColorImage::from_rgba_unmultiplied(image_size, pixels.as_slice());

        let texture_handle = ctx.load_texture(
            "page_pub_image_texture",
            color_image,
            TextureOptions::NEAREST,
        );

        Ok((texture_handle, format))
    }

    fn show_image(ui: &mut Ui, texture: &TextureHandle) {
        let image_size = texture.size_vec2();

        let image = PlotImage::new(
            texture,
            PlotPoint::new(image_size.x / 2.0, -image_size.y / 2.0),
            image_size,
        )
        .highlight(false);

        let plot = Plot::new("page_put_plot_image")
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

    fn generate_payload(&self) -> Result<ZBytes, String> {
        let z_bytes: ZBytes = match self.selected_encoding {
            KnownEncoding::ZBytes => {
                let v = parse_str_to_vec(self.payload_edit_str.as_str())?;
                ZBytes::from(v)
            }
            KnownEncoding::ZInt8 => {
                let v_str = self
                    .payload_edit_str
                    .replace(&[' ', ',', '\t', '\n', '\r'], "");
                let v = i8::from_str(v_str.as_str()).map_err(|e| e.to_string())?;
                ZBytes::from(v)
            }
            KnownEncoding::ZInt16 => {
                let v_str = self
                    .payload_edit_str
                    .replace(&[' ', ',', '\t', '\n', '\r'], "");
                let v = i16::from_str(v_str.as_str()).map_err(|e| e.to_string())?;
                ZBytes::from(v)
            }
            KnownEncoding::ZInt32 => {
                let v_str = self
                    .payload_edit_str
                    .replace(&[' ', ',', '\t', '\n', '\r'], "");
                let v = i32::from_str(v_str.as_str()).map_err(|e| e.to_string())?;
                ZBytes::from(v)
            }
            KnownEncoding::ZInt64 => {
                let v_str = self
                    .payload_edit_str
                    .replace(&[' ', ',', '\t', '\n', '\r'], "");
                let v = i64::from_str(v_str.as_str()).map_err(|e| e.to_string())?;
                ZBytes::from(v)
            }
            KnownEncoding::ZInt128 => {
                let v_str = self
                    .payload_edit_str
                    .replace(&[' ', ',', '\t', '\n', '\r'], "");
                let v = i128::from_str(v_str.as_str()).map_err(|e| e.to_string())?;
                ZBytes::from(v)
            }
            KnownEncoding::ZUint8 => {
                let v_str = self
                    .payload_edit_str
                    .replace(&[' ', ',', '\t', '\n', '\r'], "");
                let v = u8::from_str(v_str.as_str()).map_err(|e| e.to_string())?;
                ZBytes::from(v)
            }
            KnownEncoding::ZUint16 => {
                let v_str = self
                    .payload_edit_str
                    .replace(&[' ', ',', '\t', '\n', '\r'], "");
                let v = u16::from_str(v_str.as_str()).map_err(|e| e.to_string())?;
                ZBytes::from(v)
            }
            KnownEncoding::ZUint32 => {
                let v_str = self
                    .payload_edit_str
                    .replace(&[' ', ',', '\t', '\n', '\r'], "");
                let v = u32::from_str(v_str.as_str()).map_err(|e| e.to_string())?;
                ZBytes::from(v)
            }
            KnownEncoding::ZUint64 => {
                let v_str = self
                    .payload_edit_str
                    .replace(&[' ', ',', '\t', '\n', '\r'], "");
                let v = u64::from_str(v_str.as_str()).map_err(|e| e.to_string())?;
                ZBytes::from(v)
            }
            KnownEncoding::ZUint128 => {
                let v_str = self
                    .payload_edit_str
                    .replace(&[' ', ',', '\t', '\n', '\r'], "");
                let v = u128::from_str(v_str.as_str()).map_err(|e| e.to_string())?;
                ZBytes::from(v)
            }
            KnownEncoding::ZFloat32 => {
                let v_str = self
                    .payload_edit_str
                    .replace(&[' ', ',', '\t', '\n', '\r'], "");
                let v = f32::from_str(v_str.as_str()).map_err(|e| e.to_string())?;
                ZBytes::from(v)
            }
            KnownEncoding::ZFloat64 => {
                let v_str = self
                    .payload_edit_str
                    .replace(&[' ', ',', '\t', '\n', '\r'], "");
                let v = f64::from_str(v_str.as_str()).map_err(|e| e.to_string())?;
                ZBytes::from(v)
            }
            KnownEncoding::ZBool => {
                let v_str = self
                    .payload_edit_str
                    .replace(&[' ', ',', '\t', '\n', '\r'], "");
                let v = bool::from_str(v_str.as_str()).map_err(|e| e.to_string())?;
                ZBytes::from(v)
            }
            KnownEncoding::ZString => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::ZError => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::AppOctetStream => {
                let v = parse_str_to_vec(self.payload_edit_str.as_str())?;
                ZBytes::from(v)
            }
            KnownEncoding::TextPlain => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::AppJson => {
                let v = serde_json::from_str::<serde_json::Value>(self.payload_edit_str.as_str())
                    .map_err(|e| e.to_string())?;
                ZBytes::serialize(v)
            }
            KnownEncoding::TextJson => {
                let v = serde_json::from_str::<serde_json::Value>(self.payload_edit_str.as_str())
                    .map_err(|e| e.to_string())?;
                ZBytes::serialize(v)
            }
            KnownEncoding::AppCdr => return Err("Not supported yet".to_string()),
            KnownEncoding::AppCbor => return Err("Not supported yet".to_string()),
            KnownEncoding::AppYaml => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::TextYaml => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::TextJson5 => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::AppPythonSerializedObject => return Err("Not supported yet".to_string()),
            KnownEncoding::AppProtobuf => return Err("Not supported yet".to_string()),
            KnownEncoding::AppJavaSerializedObject => return Err("Not supported yet".to_string()),
            KnownEncoding::AppOpenMetricsText => return Err("Not supported yet".to_string()),
            KnownEncoding::ImagePng
            | KnownEncoding::ImageJpeg
            | KnownEncoding::ImageGif
            | KnownEncoding::ImageBmp
            | KnownEncoding::ImageWebP => {
                if let Some(image_file) = &self.image_file_path {
                    let d = std::fs::read(image_file.as_path()).map_err(|e| e.to_string())?;
                    ZBytes::from(d)
                } else {
                    return Err("No image file is selected".to_string());
                }
            }
            KnownEncoding::AppXml => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::AppXWwwFormUrlencoded => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::TextHtml => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::TextXml => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::TextCss => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::TextJavascript => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::TextMarkdown => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::TextCsv => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::AppSql => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::AppCoapPayload => return Err("Not supported yet".to_string()),
            KnownEncoding::AppJsonPathJson => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::AppJsonSeq => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::AppJsonPath => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::AppJwt => return Err("Not supported yet".to_string()),
            KnownEncoding::AppMp4 => return Err("Not supported yet".to_string()),
            KnownEncoding::AppSoapXml => ZBytes::from(self.payload_edit_str.as_str()),
            KnownEncoding::AppYang => return Err("Not supported yet".to_string()),
            KnownEncoding::AudioAac => return Err("Not supported yet".to_string()),
            KnownEncoding::AudioFlac => return Err("Not supported yet".to_string()),
            KnownEncoding::AudioMp4 => return Err("Not supported yet".to_string()),
            KnownEncoding::AudioOgg => return Err("Not supported yet".to_string()),
            KnownEncoding::AudioVorbis => return Err("Not supported yet".to_string()),
            KnownEncoding::VideoH261 => return Err("Not supported yet".to_string()),
            KnownEncoding::VideoH263 => return Err("Not supported yet".to_string()),
            KnownEncoding::VideoH264 => return Err("Not supported yet".to_string()),
            KnownEncoding::VideoH265 => return Err("Not supported yet".to_string()),
            KnownEncoding::VideoH266 => return Err("Not supported yet".to_string()),
            KnownEncoding::VideoMp4 => return Err("Not supported yet".to_string()),
            KnownEncoding::VideoOgg => return Err("Not supported yet".to_string()),
            KnownEncoding::VideoRaw => return Err("Not supported yet".to_string()),
            KnownEncoding::VideoVp8 => return Err("Not supported yet".to_string()),
            KnownEncoding::VideoVp9 => return Err("Not supported yet".to_string()),
            KnownEncoding::Other(_) => {
                let v = parse_str_to_vec(self.payload_edit_str.as_str())?;
                ZBytes::from(v)
            }
        };

        Ok(z_bytes)
    }

    fn send(&mut self, events: &mut VecDeque<Event>) {
        let key_str = self.input_key.replace(&[' ', '\t', '\n', '\r'], "");
        let key: OwnedKeyExpr = match OwnedKeyExpr::from_str(key_str.as_str()) {
            Ok(o) => o,
            Err(e) => {
                let rt = RichText::new(format!("{}", e)).color(Color32::RED);
                self.error_info = Some(rt);
                return;
            }
        };

        let payload: ZBytes = match self.generate_payload() {
            Ok(o) => {
                self.error_info = None;
                o
            }
            Err(e) => {
                let rt = RichText::new(format!("{}", e)).color(Color32::RED);
                self.error_info = Some(rt);
                return;
            }
        };

        let put_data = PutData {
            id: self.id,
            key,
            congestion_control: self.selected_congestion_control.into(),
            priority: self.selected_priority.into(),
            encoding: self.selected_encoding.to_encoding(),
            payload,
        };
        events.push_back(Event::Put(Box::new(put_data)));
        self.error_info = None;
    }
}

pub struct PagePut {
    pub events: VecDeque<Event>,
    pub data_map: BTreeMap<u64, PagePutData>,
    selected_data_id: u64,
    put_id_count: u64,
    dnd_items: Vec<DndItem>,
}

impl Default for PagePut {
    fn default() -> Self {
        let mut p = PagePut {
            events: VecDeque::new(),
            data_map: BTreeMap::new(),
            selected_data_id: 1,
            put_id_count: 0,
            dnd_items: Vec::new(),
        };
        p.add_put_data(PagePutData::default());
        p
    }
}

impl PagePut {
    pub fn load(&mut self, data: Data) {
        self.clean_all_put_data();

        for d in data.puts {
            let page_data = PagePutData::from(&d);
            self.add_put_data(page_data);
        }
    }

    pub fn create_store_data(&self) -> Data {
        let data = self
            .dnd_items
            .iter()
            .filter_map(|k| self.data_map.get(&k.key_id))
            .map(|d| d.to())
            .collect();
        Data { puts: data }
    }

    pub fn show(&mut self, ctx: &Context) {
        SidePanel::left("page_put_panel_left")
            .resizable(true)
            .show(ctx, |ui| {
                self.show_puts_name(ui);
            });

        CentralPanel::default().show(ctx, |ui| {
            let data = match self.data_map.get_mut(&self.selected_data_id) {
                None => {
                    return;
                }
                Some(o) => o,
            };

            data.show(ui, &mut self.events);
        });
    }

    fn show_puts_name(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(RichText::new(" + ").code())
                    .on_hover_text("copy add")
                    .clicked()
                {
                    if let Some(d) = self.data_map.get(&self.selected_data_id) {
                        self.add_put_data(PagePutData::new_from(d));
                    } else {
                        self.add_put_data(PagePutData::default());
                    }
                };

                if ui
                    .button(RichText::new(" - ").code())
                    .on_hover_text("del")
                    .clicked()
                {
                    self.del_put_data(self.selected_data_id);
                };
            });

            ui.label(" ");

            ScrollArea::both()
                .max_width(200.0)
                .auto_shrink([true, false])
                .show(ui, |ui| {
                    dnd(ui, "page_put_list").show_vec(
                        self.dnd_items.as_mut_slice(),
                        |ui, item, handle, _state| {
                            if let Some(d) = self.data_map.get(&item.key_id) {
                                handle.ui(ui, |ui| {
                                    let text = RichText::new(d.name.as_str());
                                    ui.selectable_value(
                                        &mut self.selected_data_id,
                                        item.key_id,
                                        text,
                                    );
                                });
                            }
                        },
                    )
                });
        });
    }

    fn add_put_data(&mut self, mut data: PagePutData) {
        self.put_id_count += 1;
        data.id = self.put_id_count;
        self.data_map.insert(self.put_id_count, data);
        self.selected_data_id = self.put_id_count;
        self.dnd_items.push(DndItem::new(self.put_id_count));
    }

    fn del_put_data(&mut self, put_id: u64) {
        if self.data_map.len() < 2 {
            return;
        }

        let _ = self.data_map.remove(&put_id);
        let mut del_index = None;
        for (i, di) in self.dnd_items.iter().enumerate() {
            if di.key_id == put_id {
                del_index = Some(i);
                break;
            }
        }
        if let Some(i) = del_index {
            self.dnd_items.remove(i);
        }
    }

    fn clean_all_put_data(&mut self) {
        self.data_map.clear();
        self.dnd_items.clear();
        self.selected_data_id = 0;
    }

    pub fn processing_put_res(&mut self, r: Box<(u64, bool, String)>) {
        let (id, b, s) = *r;
        if let Some(pd) = self.data_map.get_mut(&id) {
            pd.error_info = if b {
                Some(RichText::new(s))
            } else {
                Some(RichText::new(s).color(Color32::RED))
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
