use eframe::egui::{
    Color32, ColorImage, RichText, TextEdit, TextureHandle, TextureOptions, Ui, Widget,
};
use egui_json_tree::JsonTree;
use egui_plot::{Plot, PlotImage, PlotPoint};
use image::{ImageFormat, ImageReader};
use std::io::Cursor;
use zenoh::bytes::Encoding;

use crate::zenoh_data::KnownEncoding;

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum ViewerJsonPage {
    Source,
    Format,
    Tree,
}

pub enum DataViewer {
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

impl DataViewer {
    pub fn show(&mut self, ui: &mut Ui) {
        match self {
            DataViewer::Bin => {
                let rich_text = RichText::new("This is binary data")
                    .underline()
                    .color(Color32::RED)
                    .strong();
                ui.label(rich_text);
            }
            DataViewer::Text(s) => {
                let text_edit = TextEdit::multiline(s)
                    .desired_width(f32::INFINITY)
                    .code_editor();
                text_edit.ui(ui);
            }
            DataViewer::Json {
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
            DataViewer::Image {
                color_image,
                image_texture_handle,
            } => {
                if image_texture_handle.is_none() {
                    let texture: TextureHandle = ui.ctx().load_texture(
                        "data_viewer_show_image_texture",
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
                    "data_viewer_show_image_plot_image",
                    texture,
                    PlotPoint::new(image_size.x / 2.0, -image_size.y / 2.0),
                    image_size,
                )
                .highlight(false);
                let plot = Plot::new("data_viewer_show_image_plot")
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
            DataViewer::Audio => {
                let rich_text = RichText::new("This is audio data")
                    .underline()
                    .color(Color32::RED)
                    .strong();
                ui.label(rich_text);
            }
            DataViewer::Video => {
                let rich_text = RichText::new("This is video data")
                    .underline()
                    .color(Color32::RED)
                    .strong();
                ui.label(rich_text);
            }
            DataViewer::Error(s) => {
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

    pub fn load(encoding: &Encoding, data: &[u8]) -> DataViewer {
        let known_encoding = KnownEncoding::from_encoding(encoding);
        match known_encoding {
            KnownEncoding::ZBytes => DataViewer::Bin,
            KnownEncoding::ZString => Self::load_text(data),
            KnownEncoding::ZSerialized => DataViewer::Bin,
            KnownEncoding::AppOctetStream => DataViewer::Bin,
            KnownEncoding::TextPlain => Self::load_text(data),
            KnownEncoding::AppJson => Self::load_json(data).unwrap_or_else(|e| e),
            KnownEncoding::TextJson => Self::load_json(data).unwrap_or_else(|e| e),
            KnownEncoding::AppCdr => DataViewer::Bin,
            KnownEncoding::AppCbor => DataViewer::Bin,
            KnownEncoding::AppYaml => Self::load_text(data),
            KnownEncoding::TextYaml => Self::load_text(data),
            KnownEncoding::TextJson5 => Self::load_json5(data).unwrap_or_else(|e| e),
            KnownEncoding::AppPythonSerializedObject => DataViewer::Bin,
            KnownEncoding::AppProtobuf => DataViewer::Bin,
            KnownEncoding::AppJavaSerializedObject => DataViewer::Bin,
            KnownEncoding::AppOpenMetricsText => DataViewer::Bin,
            KnownEncoding::ImagePng => Self::load_image(known_encoding, data),
            KnownEncoding::ImageJpeg => Self::load_image(known_encoding, data),
            KnownEncoding::ImageGif => Self::load_image(known_encoding, data),
            KnownEncoding::ImageBmp => Self::load_image(known_encoding, data),
            KnownEncoding::ImageWebP => Self::load_image(known_encoding, data),
            KnownEncoding::AppXml => Self::load_text(data),
            KnownEncoding::AppXWwwFormUrlencoded => DataViewer::Bin,
            KnownEncoding::TextHtml => Self::load_text(data),
            KnownEncoding::TextXml => Self::load_text(data),
            KnownEncoding::TextCss => Self::load_text(data),
            KnownEncoding::TextJavascript => Self::load_text(data),
            KnownEncoding::TextMarkdown => Self::load_text(data),
            KnownEncoding::TextCsv => Self::load_text(data),
            KnownEncoding::AppSql => Self::load_text(data),
            KnownEncoding::AppCoapPayload => DataViewer::Bin,
            KnownEncoding::AppJsonPathJson => Self::load_text(data),
            KnownEncoding::AppJsonSeq => Self::load_text(data),
            KnownEncoding::AppJsonPath => Self::load_text(data),
            KnownEncoding::AppJwt => DataViewer::Bin,
            KnownEncoding::AppMp4 => DataViewer::Bin,
            KnownEncoding::AppSoapXml => Self::load_text(data),
            KnownEncoding::AppYang => DataViewer::Bin,
            KnownEncoding::AudioAac => DataViewer::Audio,
            KnownEncoding::AudioFlac => DataViewer::Audio,
            KnownEncoding::AudioMp4 => DataViewer::Audio,
            KnownEncoding::AudioOgg => DataViewer::Audio,
            KnownEncoding::AudioVorbis => DataViewer::Audio,
            KnownEncoding::VideoH261 => DataViewer::Video,
            KnownEncoding::VideoH263 => DataViewer::Video,
            KnownEncoding::VideoH264 => DataViewer::Video,
            KnownEncoding::VideoH265 => DataViewer::Video,
            KnownEncoding::VideoH266 => DataViewer::Video,
            KnownEncoding::VideoMp4 => DataViewer::Video,
            KnownEncoding::VideoOgg => DataViewer::Video,
            KnownEncoding::VideoRaw => DataViewer::Video,
            KnownEncoding::VideoVp8 => DataViewer::Video,
            KnownEncoding::VideoVp9 => DataViewer::Video,
            KnownEncoding::Other(_) => DataViewer::Bin,
        }
    }

    fn load_text(data: &[u8]) -> Self {
        match String::from_utf8(data.to_vec()) {
            Ok(d) => DataViewer::Text(d),
            Err(e) => DataViewer::Error(e.to_string()),
        }
    }

    fn load_json(data: &[u8]) -> Result<Self, Self> {
        let source =
            String::from_utf8(data.to_vec()).map_err(|e| DataViewer::Error(e.to_string()))?;

        let v =
            serde_json::from_str(source.as_str()).map_err(|e| DataViewer::Error(e.to_string()))?;

        Ok(DataViewer::Json {
            selected_page: ViewerJsonPage::Source,
            source,
            format: serde_json::to_string_pretty(&v).unwrap(),
            serde_json_value: v,
        })
    }

    fn load_json5(data: &[u8]) -> Result<Self, Self> {
        let source =
            String::from_utf8(data.to_vec()).map_err(|e| DataViewer::Error(e.to_string()))?;

        let v = json5::from_str(source.as_str()).map_err(|e| DataViewer::Error(e.to_string()))?;

        Ok(DataViewer::Json {
            selected_page: ViewerJsonPage::Source,
            source,
            format: serde_json::to_string_pretty(&v).unwrap(),
            serde_json_value: v,
        })
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
                return DataViewer::Error("not image".to_string());
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
                DataViewer::Image {
                    color_image,
                    image_texture_handle: None,
                }
            }
            Err(e) => DataViewer::Error(e.to_string()),
        }
    }
}
