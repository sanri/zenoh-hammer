use eframe::egui::{
    CollapsingHeader, Color32, ColorImage, Grid, RichText, TextEdit, TextureHandle, TextureOptions,
    Ui, Widget,
};
use egui_json_tree::JsonTree;
use egui_plot::{Corner, Legend, Plot, PlotImage, PlotPoint};
use image::{ImageFormat, ImageReader};
use std::{
    borrow::Cow,
    io::{Cursor, Read},
    sync::Arc,
};
use uhlc::Timestamp;
use zenoh::{
    bytes::{Encoding, ZBytes},
    sample::{Sample, SampleKind, SourceInfo},
};

use crate::{
    hex_viewer::HexViewer,
    zenoh_data::{KnownEncoding, ZCongestionControl, ZPriority, ZReliability},
};

pub struct SampleViewer {
    selected_page: SampleViewerPage,
    base_info: BaseInfo,
    hex_view: HexViewer,
    viewer_data: ViewerData,
}

impl Default for SampleViewer {
    fn default() -> Self {
        SampleViewer {
            selected_page: SampleViewerPage::Raw,
            base_info: BaseInfo::default(),
            hex_view: HexViewer::new(Arc::new(Vec::new())),
            viewer_data: ViewerData::Bin,
        }
    }
}

impl SampleViewer {
    pub fn new_from_sample(sample: &Sample) -> Self {
        let base_info = BaseInfo::new_from(sample);

        let mut data = Vec::new();
        let mut reader = sample.payload().reader();
        let _ = reader.read_to_end(&mut data);
        let arc_data = Arc::new(data);
        let hex_view = HexViewer::new(arc_data.clone());

        let viewer_data = ViewerData::load(sample.encoding(), sample.payload(), arc_data);

        SampleViewer {
            selected_page: SampleViewerPage::Parse,
            base_info,
            hex_view,
            viewer_data,
        }
    }

    pub fn new(base_info: BaseInfo, data: ZBytes, arc_data: Arc<Vec<u8>>) -> Self {
        let viewer_data = ViewerData::load(&base_info.encoding, &data, arc_data.clone());
        let hex_view = HexViewer::new(arc_data);

        SampleViewer {
            selected_page: SampleViewerPage::Parse,
            base_info,
            hex_view,
            viewer_data,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        self.base_info.show(ui);

        ui.separator();

        self.show_tab_label(ui);

        ui.add_space(10.0);

        match self.selected_page {
            SampleViewerPage::Raw => {
                self.hex_view.show(ui);
            }
            SampleViewerPage::Parse => {
                self.viewer_data.show(ui);
            }
        }
    }

    fn show_tab_label(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.selected_page == SampleViewerPage::Parse, "parse")
                .clicked()
            {
                self.selected_page = SampleViewerPage::Parse;
            }

            if ui
                .selectable_label(self.selected_page == SampleViewerPage::Raw, "raw")
                .clicked()
            {
                self.selected_page = SampleViewerPage::Raw;
            }
        });
    }
}

pub struct BaseInfo {
    pub key: String,
    pub encoding: Encoding,
    pub kind: SampleKind,
    pub timestamp: Option<Timestamp>,
    pub congestion_control: ZCongestionControl,
    pub priority: ZPriority,
    pub reliability: ZReliability,
    pub express: bool,
    pub source_info: SourceInfo,
    pub attachment: Vec<u8>,
}

impl Default for BaseInfo {
    fn default() -> Self {
        BaseInfo {
            key: "demo".to_string(),
            encoding: Encoding::ZENOH_BYTES,
            kind: SampleKind::Put,
            timestamp: None,
            congestion_control: ZCongestionControl::Block,
            priority: ZPriority::RealTime,
            reliability: ZReliability::Reliable,
            express: false,
            source_info: SourceInfo::new(None, None),
            attachment: Vec::new(),
        }
    }
}

impl BaseInfo {
    fn new_from(sample: &Sample) -> Self {
        let key = sample.key_expr().to_string();
        let encoding = sample.encoding().clone();
        let kind = sample.kind().clone();
        let timestamp = sample.timestamp().cloned();
        let congestion_control = sample.congestion_control().clone().into();
        let priority = sample.priority().clone().into();
        let reliability = sample.reliability().clone().into();
        let express = sample.express();
        let source_info = sample.source_info().clone();

        let mut attachment = Vec::new();
        if let Some(s) = sample.attachment() {
            let mut reader = s.reader();
            let _ = reader.read_to_end(&mut attachment);
        }

        BaseInfo {
            key,
            encoding,
            kind,
            timestamp,
            congestion_control,
            priority,
            reliability,
            express,
            source_info,
            attachment,
        }
    }

    fn show(&mut self, ui: &mut Ui) {
        let show_ui = |ui: &mut Ui| {
            ui.label("key:");
            let text = RichText::new(self.key.as_str()).monospace();
            ui.label(text);
            ui.end_row();

            ui.label("kind:");
            let text = RichText::new(self.kind.to_string()).monospace();
            ui.label(text);
            ui.end_row();

            let (text_time, text_id) = if let Some(t) = self.timestamp {
                let m = t.to_string_rfc3339_lossy();
                let s = m.split_once('/').unwrap();
                (s.0.to_string(), s.1.to_string())
            } else {
                ("-".to_string(), "-".to_string())
            };
            ui.label("timestamp. time:");
            ui.label(RichText::new(text_time).monospace());
            ui.end_row();

            ui.label("timestamp. id:");
            ui.label(RichText::new(text_id).monospace());
            ui.end_row();

            ui.label("encoding:");
            let text = RichText::new(self.encoding.to_string()).monospace();
            ui.label(text);
            ui.end_row();

            ui.label("attachment:");
            let s = String::from_utf8(self.attachment.clone())
                .unwrap_or(format!("{:?}", self.attachment.as_slice()));
            let text = RichText::new(s).monospace();
            ui.label(text);
            ui.end_row();

            ui.label("source_info. id:");
            let s = match self.source_info.source_id() {
                None => "-".to_string(),
                Some(o) => {
                    format!("{:?}", o)
                }
            };
            let text = RichText::new(s).monospace();
            ui.label(text);
            ui.end_row();

            ui.label("source_info. sn:");
            let s = match self.source_info.source_sn() {
                None => "-".to_string(),
                Some(o) => {
                    format!("{}", o)
                }
            };
            let text = RichText::new(s).monospace();
            ui.label(text);
            ui.end_row();

            ui.label("congestion_control:");
            let text = RichText::new(self.congestion_control.as_ref()).monospace();
            ui.label(text);
            ui.end_row();

            ui.label("priority:");
            let text = RichText::new(self.priority.as_ref()).monospace();
            ui.label(text);
            ui.end_row();

            ui.label("reliability:");
            let text = RichText::new(self.reliability.as_ref()).monospace();
            ui.label(text);
            ui.end_row();

            ui.label("express:");
            let text = RichText::new(self.express.to_string()).monospace();
            ui.label(text);
            ui.end_row();
        };

        CollapsingHeader::new("Base info")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("sample_viewer_base_info_grid")
                    .num_columns(2)
                    .show(ui, |ui| {
                        show_ui(ui);
                    });
            });
    }
}

#[derive(Eq, PartialEq, Copy, Clone)]
enum SampleViewerPage {
    Raw,
    Parse,
}

#[derive(Eq, PartialEq, Copy, Clone)]
enum ViewerJsonPage {
    Source,
    Format,
    Tree,
}

enum ViewerData {
    Simple(String),
    Bin,
    Text(String),
    Json {
        selected_page: ViewerJsonPage,
        source: String,
        format: String,
        serde_json_value: serde_json::Value,
    },
    Image {
        color_image: ColorImage,
        image_texture_handle: Option<TextureHandle>,
    },
    Audio,
    Video,
    Error(String),
}

impl ViewerData {
    fn show(&mut self, ui: &mut Ui) {
        match self {
            ViewerData::Simple(s) => {
                let rich_text = RichText::new(s.as_str()).monospace();
                ui.label(rich_text);
            }
            ViewerData::Bin => {
                let rich_text = RichText::new("This is binary data")
                    .underline()
                    .color(Color32::RED)
                    .strong();
                ui.label(rich_text);
            }
            ViewerData::Text(s) => {
                let text_edit = TextEdit::multiline(s)
                    .desired_width(f32::INFINITY)
                    .code_editor();
                text_edit.ui(ui);
            }
            ViewerData::Json {
                selected_page,
                source,
                format,
                serde_json_value,
            } => {
                Self::show_json_tab(ui, selected_page);
                match selected_page {
                    ViewerJsonPage::Source => {
                        let text_edit = TextEdit::multiline(source)
                            .desired_width(f32::INFINITY)
                            .code_editor();
                        text_edit.ui(ui);
                    }
                    ViewerJsonPage::Format => {
                        let text_edit = TextEdit::multiline(format)
                            .desired_width(f32::INFINITY)
                            .code_editor();
                        text_edit.ui(ui);
                    }
                    ViewerJsonPage::Tree => {
                        let json_tree = JsonTree::new("simple_viewer-json_tree", serde_json_value);
                        json_tree.show(ui);
                    }
                }
            }
            ViewerData::Image {
                color_image,
                image_texture_handle,
            } => {
                if image_texture_handle.is_none() {
                    let texture: TextureHandle = ui.ctx().load_texture(
                        "simple_viewer-show_image",
                        color_image.clone(),
                        TextureOptions::NEAREST,
                    );
                    *image_texture_handle = Some(texture);
                }

                let texture: &TextureHandle = match image_texture_handle {
                    None => {
                        return;
                    }
                    Some(t) => t,
                };

                let image_size = texture.size_vec2();
                let plot_image = PlotImage::new(
                    texture,
                    PlotPoint::new(image_size.x / 2.0, -image_size.y / 2.0),
                    image_size,
                )
                .highlight(false);
                let plot = Plot::new("sample_viewer_show_image_plot")
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
                    plot_ui.image(plot_image);
                });
            }
            ViewerData::Audio => {
                let rich_text = RichText::new("This is audio data")
                    .underline()
                    .color(Color32::RED)
                    .strong();
                ui.label(rich_text);
            }
            ViewerData::Video => {
                let rich_text = RichText::new("This is video data")
                    .underline()
                    .color(Color32::RED)
                    .strong();
                ui.label(rich_text);
            }
            ViewerData::Error(s) => {
                let rich_text = RichText::new(s.as_str()).monospace().color(Color32::RED);
                ui.label(rich_text);
            }
        }
    }

    fn show_json_tab(ui: &mut Ui, selected_page: &mut ViewerJsonPage) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(*selected_page == ViewerJsonPage::Source, "source")
                .clicked()
            {
                *selected_page = ViewerJsonPage::Source;
            }

            if ui
                .selectable_label(*selected_page == ViewerJsonPage::Format, "format")
                .clicked()
            {
                *selected_page = ViewerJsonPage::Format;
            }

            if ui
                .selectable_label(*selected_page == ViewerJsonPage::Tree, "tree")
                .clicked()
            {
                *selected_page = ViewerJsonPage::Tree;
            }
        });
    }

    fn load(encoding: &Encoding, data: &ZBytes, arc_data: Arc<Vec<u8>>) -> ViewerData {
        let known_encoding = KnownEncoding::from_encoding(encoding);
        match known_encoding {
            KnownEncoding::ZBytes => ViewerData::Bin,
            KnownEncoding::ZInt8 => match data.deserialize::<i8>() {
                Ok(i) => ViewerData::Simple(i.to_string()),
                Err(e) => ViewerData::Error(e.to_string()),
            },
            KnownEncoding::ZInt16 => match data.deserialize::<i16>() {
                Ok(i) => ViewerData::Simple(i.to_string()),
                Err(e) => ViewerData::Error(e.to_string()),
            },
            KnownEncoding::ZInt32 => match data.deserialize::<i32>() {
                Ok(i) => ViewerData::Simple(i.to_string()),
                Err(e) => ViewerData::Error(e.to_string()),
            },
            KnownEncoding::ZInt64 => match data.deserialize::<i64>() {
                Ok(i) => ViewerData::Simple(i.to_string()),
                Err(e) => ViewerData::Error(e.to_string()),
            },
            KnownEncoding::ZInt128 => match data.deserialize::<i128>() {
                Ok(i) => ViewerData::Simple(i.to_string()),
                Err(e) => ViewerData::Error(e.to_string()),
            },
            KnownEncoding::ZUint8 => match data.deserialize::<u8>() {
                Ok(i) => ViewerData::Simple(i.to_string()),
                Err(e) => ViewerData::Error(e.to_string()),
            },
            KnownEncoding::ZUint16 => match data.deserialize::<u16>() {
                Ok(i) => ViewerData::Simple(i.to_string()),
                Err(e) => ViewerData::Error(e.to_string()),
            },
            KnownEncoding::ZUint32 => match data.deserialize::<u32>() {
                Ok(i) => ViewerData::Simple(i.to_string()),
                Err(e) => ViewerData::Error(e.to_string()),
            },
            KnownEncoding::ZUint64 => match data.deserialize::<u64>() {
                Ok(i) => ViewerData::Simple(i.to_string()),
                Err(e) => ViewerData::Error(e.to_string()),
            },
            KnownEncoding::ZUint128 => match data.deserialize::<u128>() {
                Ok(i) => ViewerData::Simple(i.to_string()),
                Err(e) => ViewerData::Error(e.to_string()),
            },
            KnownEncoding::ZFloat32 => match data.deserialize::<f32>() {
                Ok(i) => ViewerData::Simple(i.to_string()),
                Err(e) => ViewerData::Error(e.to_string()),
            },
            KnownEncoding::ZFloat64 => match data.deserialize::<f64>() {
                Ok(i) => ViewerData::Simple(i.to_string()),
                Err(e) => ViewerData::Error(e.to_string()),
            },
            KnownEncoding::ZBool => match data.deserialize::<bool>() {
                Ok(i) => ViewerData::Simple(i.to_string()),
                Err(e) => ViewerData::Error(e.to_string()),
            },
            KnownEncoding::ZString => Self::load_text(data),
            KnownEncoding::ZError => ViewerData::Bin,
            KnownEncoding::AppOctetStream => ViewerData::Bin,
            KnownEncoding::TextPlain => match Cow::<str>::try_from(data) {
                Ok(o) => ViewerData::Text(o.to_string()),
                Err(e) => ViewerData::Error(e.to_string()),
            },
            KnownEncoding::AppJson => Self::load_json(data),
            KnownEncoding::TextJson => Self::load_json(data),
            KnownEncoding::AppCdr => ViewerData::Bin,
            KnownEncoding::AppCbor => ViewerData::Bin,
            KnownEncoding::AppYaml => Self::load_text(data),
            KnownEncoding::TextYaml => Self::load_text(data),
            KnownEncoding::TextJson5 => Self::load_json5(data),
            KnownEncoding::AppPythonSerializedObject => ViewerData::Bin,
            KnownEncoding::AppProtobuf => ViewerData::Bin,
            KnownEncoding::AppJavaSerializedObject => ViewerData::Bin,
            KnownEncoding::AppOpenMetricsText => ViewerData::Bin,
            KnownEncoding::ImagePng => Self::load_image(known_encoding, arc_data.as_slice()),
            KnownEncoding::ImageJpeg => Self::load_image(known_encoding, arc_data.as_slice()),
            KnownEncoding::ImageGif => Self::load_image(known_encoding, arc_data.as_slice()),
            KnownEncoding::ImageBmp => Self::load_image(known_encoding, arc_data.as_slice()),
            KnownEncoding::ImageWebP => Self::load_image(known_encoding, arc_data.as_slice()),
            KnownEncoding::AppXml => Self::load_text(data),
            KnownEncoding::AppXWwwFormUrlencoded => ViewerData::Bin,
            KnownEncoding::TextHtml => Self::load_text(data),
            KnownEncoding::TextXml => Self::load_text(data),
            KnownEncoding::TextCss => Self::load_text(data),
            KnownEncoding::TextJavascript => Self::load_text(data),
            KnownEncoding::TextMarkdown => Self::load_text(data),
            KnownEncoding::TextCsv => Self::load_text(data),
            KnownEncoding::AppSql => Self::load_text(data),
            KnownEncoding::AppCoapPayload => ViewerData::Bin,
            KnownEncoding::AppJsonPathJson => Self::load_text(data),
            KnownEncoding::AppJsonSeq => Self::load_text(data),
            KnownEncoding::AppJsonPath => Self::load_text(data),
            KnownEncoding::AppJwt => ViewerData::Bin,
            KnownEncoding::AppMp4 => ViewerData::Bin,
            KnownEncoding::AppSoapXml => Self::load_text(data),
            KnownEncoding::AppYang => ViewerData::Bin,
            KnownEncoding::AudioAac => ViewerData::Audio,
            KnownEncoding::AudioFlac => ViewerData::Audio,
            KnownEncoding::AudioMp4 => ViewerData::Audio,
            KnownEncoding::AudioOgg => ViewerData::Audio,
            KnownEncoding::AudioVorbis => ViewerData::Audio,
            KnownEncoding::VideoH261 => ViewerData::Video,
            KnownEncoding::VideoH263 => ViewerData::Video,
            KnownEncoding::VideoH264 => ViewerData::Video,
            KnownEncoding::VideoH265 => ViewerData::Video,
            KnownEncoding::VideoH266 => ViewerData::Video,
            KnownEncoding::VideoMp4 => ViewerData::Video,
            KnownEncoding::VideoOgg => ViewerData::Video,
            KnownEncoding::VideoRaw => ViewerData::Video,
            KnownEncoding::VideoVp8 => ViewerData::Video,
            KnownEncoding::VideoVp9 => ViewerData::Video,
            KnownEncoding::Other(_) => ViewerData::Bin,
        }
    }

    fn load_text(data: &ZBytes) -> Self {
        match Cow::<str>::try_from(data) {
            Ok(d) => ViewerData::Text(d.to_string()),
            Err(e) => ViewerData::Error(e.to_string()),
        }
    }

    fn load_json(data: &ZBytes) -> Self {
        let source = match Cow::<str>::try_from(data) {
            Ok(o) => o.to_string(),
            Err(e) => {
                return ViewerData::Error(e.to_string());
            }
        };
        match serde_json::Value::try_from(data) {
            Ok(o) => ViewerData::Json {
                selected_page: ViewerJsonPage::Source,
                source,
                format: serde_json::to_string_pretty(&o).unwrap(),
                serde_json_value: o,
            },
            Err(e) => ViewerData::Error(e.to_string()),
        }
    }

    fn load_json5(data: &ZBytes) -> Self {
        let source = match Cow::<str>::try_from(data) {
            Ok(o) => o.to_string(),
            Err(e) => {
                return ViewerData::Error(e.to_string());
            }
        };
        match json5::from_str::<serde_json::Value>(source.as_str()) {
            Ok(o) => ViewerData::Json {
                selected_page: ViewerJsonPage::Source,
                source,
                format: serde_json::to_string_pretty(&o).unwrap(),
                serde_json_value: o,
            },
            Err(e) => ViewerData::Error(e.to_string()),
        }
    }

    fn load_image(known_encoding: KnownEncoding, data: &[u8]) -> Self {
        let mut image_reader = ImageReader::new(Cursor::new(data));

        match known_encoding {
            KnownEncoding::ImagePng => {
                image_reader.set_format(ImageFormat::Png);
            }
            KnownEncoding::ImageJpeg => {
                image_reader.set_format(ImageFormat::Jpeg);
            }
            KnownEncoding::ImageGif => {
                image_reader.set_format(ImageFormat::Gif);
            }
            KnownEncoding::ImageBmp => {
                image_reader.set_format(ImageFormat::Bmp);
            }
            KnownEncoding::ImageWebP => {
                image_reader.set_format(ImageFormat::WebP);
            }
            _ => {
                return ViewerData::Error("not image".to_string());
            }
        }

        match image_reader.decode() {
            Ok(m) => {
                let image_buffer = m.into_rgba8();
                let image_size = [
                    image_buffer.width() as usize,
                    image_buffer.height() as usize,
                ];
                let pixels = image_buffer.as_flat_samples();
                let color_image = ColorImage::from_rgba_unmultiplied(image_size, pixels.as_slice());
                ViewerData::Image {
                    color_image,
                    image_texture_handle: None,
                }
            }
            Err(e) => ViewerData::Error(e.to_string()),
        }
    }
}
