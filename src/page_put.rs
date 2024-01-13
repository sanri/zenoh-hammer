use eframe::{
    egui,
    egui::{
        Color32, ColorImage, Context, Layout, RichText, ScrollArea, TextEdit, TextStyle,
        TextureHandle, TextureOptions, Ui,
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
use zenoh::prelude::{CongestionControl, Encoding, KnownEncoding, OwnedKeyExpr, Priority, Value};

use crate::{app::ZenohValue, zenoh::PutData};

pub enum Event {
    Put(Box<PutData>),
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ZCongestionControl {
    Block,
    Drop,
}

impl From<CongestionControl> for ZCongestionControl {
    fn from(value: CongestionControl) -> Self {
        match value {
            CongestionControl::Block => ZCongestionControl::Block,
            CongestionControl::Drop => ZCongestionControl::Drop,
        }
    }
}

impl Into<CongestionControl> for ZCongestionControl {
    fn into(self) -> CongestionControl {
        match self {
            ZCongestionControl::Block => CongestionControl::Block,
            ZCongestionControl::Drop => CongestionControl::Drop,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ZPriority {
    RealTime,
    InteractiveHigh,
    InteractiveLow,
    DataHigh,
    Data,
    DataLow,
    Background,
}

impl From<Priority> for ZPriority {
    fn from(value: Priority) -> Self {
        match value {
            Priority::RealTime => ZPriority::RealTime,
            Priority::InteractiveHigh => ZPriority::InteractiveHigh,
            Priority::InteractiveLow => ZPriority::InteractiveLow,
            Priority::DataHigh => ZPriority::DataHigh,
            Priority::Data => ZPriority::Data,
            Priority::DataLow => ZPriority::DataLow,
            Priority::Background => ZPriority::Background,
        }
    }
}

impl Into<Priority> for ZPriority {
    fn into(self) -> Priority {
        match self {
            ZPriority::RealTime => Priority::RealTime,
            ZPriority::InteractiveHigh => Priority::InteractiveHigh,
            ZPriority::InteractiveLow => Priority::InteractiveLow,
            ZPriority::DataHigh => Priority::DataHigh,
            ZPriority::Data => Priority::Data,
            ZPriority::DataLow => Priority::DataLow,
            ZPriority::Background => Priority::Background,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DataItem {
    name: String,
    key: String,
    congestion_control: ZCongestionControl,
    priority: ZPriority,
    value: ZenohValue,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Data {
    puts: Vec<DataItem>,
}

pub struct PagePutData {
    id: u64,
    name: String,
    input_key: String,
    selected_congestion_control: CongestionControl,
    selected_priority: Priority,
    selected_encoding: KnownEncoding,
    edit_str: String,
    image_file_dialog: Option<FileDialog>,
    image_file_path: Option<PathBuf>,
    image_texture: Option<TextureHandle>,
    pub info: Option<RichText>,
}

impl Default for PagePutData {
    fn default() -> Self {
        PagePutData {
            id: 1,
            name: "demo".to_string(),
            input_key: "demo/example".to_string(),
            selected_congestion_control: CongestionControl::Block,
            selected_priority: Priority::RealTime,
            selected_encoding: KnownEncoding::TextPlain,
            edit_str: String::new(),
            image_file_path: None,
            image_file_dialog: None,
            image_texture: None,
            info: None,
        }
    }
}

impl PagePutData {
    fn from(data: &DataItem) -> PagePutData {
        let (encoding, s) = data.value.to();
        PagePutData {
            id: 0,
            name: data.name.clone(),
            input_key: data.key.clone(),
            selected_congestion_control: data.congestion_control.into(),
            selected_priority: data.priority.into(),
            selected_encoding: encoding,
            edit_str: s,
            image_file_path: None,
            image_file_dialog: None,
            image_texture: None,
            info: None,
        }
    }

    fn to(&self) -> DataItem {
        let value = ZenohValue::from(self.selected_encoding, self.edit_str.clone());
        DataItem {
            name: self.name.clone(),
            key: self.input_key.clone(),
            congestion_control: self.selected_congestion_control.into(),
            priority: self.selected_priority.into(),
            value,
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
            edit_str: ppd.edit_str.clone(),
            image_file_path: None,
            image_file_dialog: None,
            image_texture: None,
            info: None,
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
            ui.label("name: ");
            let te = TextEdit::singleline(&mut self.name)
                .desired_width(600.0)
                .font(TextStyle::Monospace);
            ui.add(te);
            ui.end_row();

            ui.label("key: ");
            let te = TextEdit::multiline(&mut self.input_key)
                .desired_rows(2)
                .desired_width(600.0)
                .font(TextStyle::Monospace);
            ui.add(te);

            ui.end_row();
        };
        egui::Grid::new("input_grid")
            .num_columns(2)
            .striped(false)
            .show(ui, |ui| {
                input_grid(ui);
            });
    }

    fn show_options(&mut self, ui: &mut Ui) {
        let mut show_grid = |ui: &mut Ui| {
            ui.label("congestion control: ");
            egui::ComboBox::new("congestion control", "")
                .selected_text(format!("{:?}", self.selected_congestion_control))
                .show_ui(ui, |ui| {
                    let options = [CongestionControl::Block, CongestionControl::Drop];
                    for option in options {
                        ui.selectable_value(
                            &mut self.selected_congestion_control,
                            option,
                            format!("{:?}", option),
                        );
                    }
                });
            ui.end_row();

            ui.label("priority: ");
            egui::ComboBox::new("priority", "")
                .selected_text(format!("{:?}", self.selected_priority))
                .show_ui(ui, |ui| {
                    let options = [
                        Priority::RealTime,
                        Priority::InteractiveHigh,
                        Priority::InteractiveLow,
                        Priority::DataHigh,
                        Priority::Data,
                        Priority::DataLow,
                        Priority::Background,
                    ];
                    for option in options {
                        ui.selectable_value(
                            &mut self.selected_priority,
                            option,
                            format!("{:?}", option),
                        );
                    }
                });
            ui.end_row();

            ui.label("encoding: ");
            egui::ComboBox::new("encoding", "")
                .selected_text(format!("{}", Encoding::Exact(self.selected_encoding)))
                .show_ui(ui, |ui| {
                    let options = [
                        KnownEncoding::AppOctetStream,
                        KnownEncoding::TextPlain,
                        KnownEncoding::TextJson,
                        KnownEncoding::AppJson,
                        KnownEncoding::AppInteger,
                        KnownEncoding::AppFloat,
                        KnownEncoding::AppSql,
                        KnownEncoding::AppXml,
                        KnownEncoding::AppXhtmlXml,
                        KnownEncoding::TextHtml,
                        KnownEncoding::TextXml,
                        KnownEncoding::TextCss,
                        KnownEncoding::TextCsv,
                        KnownEncoding::TextJavascript,
                        KnownEncoding::ImageJpeg,
                        KnownEncoding::ImagePng,
                    ];
                    for option in options {
                        ui.selectable_value(
                            &mut self.selected_encoding,
                            option,
                            format!("{}", Encoding::from(option)),
                        );
                    }
                });
            ui.end_row();
        };

        egui::Grid::new("options_grid")
            .num_columns(2)
            .striped(false)
            .show(ui, |ui| {
                show_grid(ui);
            });

        let text_edit_multiline = |edit_str: &mut String, info: &Option<RichText>, ui: &mut Ui| {
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
        };

        let image_view = |ui: &mut Ui,
                          image_file_path: &mut Option<PathBuf>,
                          image_file_dialog: &mut Option<FileDialog>,
                          image_texture: &mut Option<TextureHandle>,
                          info: &mut Option<RichText>,
                          image_format: ImageFormat| {
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
                                    let text =
                                        RichText::new("file format error").color(Color32::RED);
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
        };

        match self.selected_encoding {
            KnownEncoding::TextPlain => {
                text_edit_multiline(&mut self.edit_str, &self.info, ui);
            }
            KnownEncoding::AppJson => {
                text_edit_multiline(&mut self.edit_str, &self.info, ui);
            }
            KnownEncoding::AppInteger => {
                if let Some(rt) = &self.info {
                    ui.label(rt.clone());
                };
                ui.add(TextEdit::singleline(&mut self.edit_str));
            }
            KnownEncoding::AppFloat => {
                if let Some(rt) = &self.info {
                    ui.label(rt.clone());
                };
                ui.add(TextEdit::singleline(&mut self.edit_str));
            }
            KnownEncoding::TextJson => {
                text_edit_multiline(&mut self.edit_str, &self.info, ui);
            }
            KnownEncoding::Empty => {}
            KnownEncoding::AppOctetStream => {
                text_edit_multiline(&mut self.edit_str, &self.info, ui);
            }
            KnownEncoding::AppCustom => {}
            KnownEncoding::AppProperties => {}
            KnownEncoding::AppSql => {
                text_edit_multiline(&mut self.edit_str, &self.info, ui);
            }
            KnownEncoding::AppXml => {
                text_edit_multiline(&mut self.edit_str, &self.info, ui);
            }
            KnownEncoding::AppXhtmlXml => {
                text_edit_multiline(&mut self.edit_str, &self.info, ui);
            }
            KnownEncoding::AppXWwwFormUrlencoded => {}
            KnownEncoding::TextHtml => {
                text_edit_multiline(&mut self.edit_str, &self.info, ui);
            }
            KnownEncoding::TextXml => {
                text_edit_multiline(&mut self.edit_str, &self.info, ui);
            }
            KnownEncoding::TextCss => {
                text_edit_multiline(&mut self.edit_str, &self.info, ui);
            }
            KnownEncoding::TextCsv => {
                text_edit_multiline(&mut self.edit_str, &self.info, ui);
            }
            KnownEncoding::TextJavascript => {
                text_edit_multiline(&mut self.edit_str, &self.info, ui);
            }
            KnownEncoding::ImageJpeg => {
                image_view(
                    ui,
                    &mut self.image_file_path,
                    &mut self.image_file_dialog,
                    &mut self.image_texture,
                    &mut self.info,
                    ImageFormat::Jpeg,
                );
            }
            KnownEncoding::ImagePng => {
                image_view(
                    ui,
                    &mut self.image_file_path,
                    &mut self.image_file_dialog,
                    &mut self.image_texture,
                    &mut self.info,
                    ImageFormat::Png,
                );
            }
            KnownEncoding::ImageGif => {}
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

    fn send(&mut self, events: &mut VecDeque<Event>) {
        let key_str = self.input_key.replace(&[' ', '\t', '\n', '\r'], "");
        let key: OwnedKeyExpr = match OwnedKeyExpr::from_str(key_str.as_str()) {
            Ok(o) => o,
            Err(e) => {
                let rt = RichText::new(format!("{}", e)).color(Color32::RED);
                self.info = Some(rt);
                return;
            }
        };
        let value = match self.selected_encoding {
            KnownEncoding::AppJson => {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(self.edit_str.as_str()) {
                    let rt = RichText::new(format!("{}", e)).color(Color32::RED);
                    self.info = Some(rt);
                    return;
                }
                Value::from(self.edit_str.as_str()).encoding(KnownEncoding::AppJson.into())
            }
            KnownEncoding::AppInteger => {
                let i: i64 = match self.edit_str.parse::<i64>() {
                    Ok(i) => i,
                    Err(e) => {
                        let rt = RichText::new(format!("{}", e)).color(Color32::RED);
                        self.info = Some(rt);
                        return;
                    }
                };
                Value::from(i)
            }
            KnownEncoding::AppFloat => {
                let f: f64 = match self.edit_str.parse::<f64>() {
                    Ok(f) => f,
                    Err(e) => {
                        let rt = RichText::new(format!("{}", e)).color(Color32::RED);
                        self.info = Some(rt);
                        return;
                    }
                };
                Value::from(f)
            }
            KnownEncoding::TextJson => {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(self.edit_str.as_str()) {
                    let rt = RichText::new(format!("{}", e)).color(Color32::RED);
                    self.info = Some(rt);
                    return;
                }
                Value::from(self.edit_str.as_str()).encoding(KnownEncoding::TextJson.into())
            }
            KnownEncoding::Empty => {
                return;
            }
            KnownEncoding::AppOctetStream => {
                Value::from(self.edit_str.as_bytes()).encoding(KnownEncoding::AppOctetStream.into())
            }
            KnownEncoding::AppCustom => {
                return;
            }
            KnownEncoding::AppProperties => {
                return;
            }
            KnownEncoding::AppXWwwFormUrlencoded => {
                return;
            }
            KnownEncoding::ImageJpeg => {
                if let Some(image_file) = &self.image_file_path {
                    match std::fs::read(image_file.as_path()) {
                        Ok(d) => Value::from(d).encoding(KnownEncoding::ImageJpeg.into()),
                        Err(e) => {
                            let text = RichText::new(e.to_string()).color(Color32::RED);
                            self.info = Some(text);
                            return;
                        }
                    }
                } else {
                    return;
                }
            }
            KnownEncoding::ImagePng => {
                if let Some(image_file) = &self.image_file_path {
                    match std::fs::read(image_file.as_path()) {
                        Ok(d) => Value::from(d).encoding(KnownEncoding::ImagePng.into()),
                        Err(e) => {
                            let text = RichText::new(e.to_string()).color(Color32::RED);
                            self.info = Some(text);
                            return;
                        }
                    }
                } else {
                    return;
                }
            }
            KnownEncoding::ImageGif => {
                return;
            }
            KnownEncoding::TextPlain => {
                Value::from(self.edit_str.as_str()).encoding(KnownEncoding::TextPlain.into())
            }
            KnownEncoding::AppSql => {
                Value::from(self.edit_str.as_str()).encoding(KnownEncoding::AppSql.into())
            }
            KnownEncoding::AppXml => {
                Value::from(self.edit_str.as_str()).encoding(KnownEncoding::AppXml.into())
            }
            KnownEncoding::AppXhtmlXml => {
                Value::from(self.edit_str.as_str()).encoding(KnownEncoding::AppXhtmlXml.into())
            }
            KnownEncoding::TextHtml => {
                Value::from(self.edit_str.as_str()).encoding(KnownEncoding::TextHtml.into())
            }
            KnownEncoding::TextXml => {
                Value::from(self.edit_str.as_str()).encoding(KnownEncoding::TextXml.into())
            }
            KnownEncoding::TextCss => {
                Value::from(self.edit_str.as_str()).encoding(KnownEncoding::TextCss.into())
            }
            KnownEncoding::TextCsv => {
                Value::from(self.edit_str.as_str()).encoding(KnownEncoding::TextCsv.into())
            }
            KnownEncoding::TextJavascript => {
                Value::from(self.edit_str.as_str()).encoding(KnownEncoding::TextJavascript.into())
            }
        };
        let put_data = PutData {
            id: self.id,
            key,
            congestion_control: self.selected_congestion_control,
            priority: self.selected_priority,
            value,
        };
        events.push_back(Event::Put(Box::new(put_data)));
        self.info = None;
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
        egui::SidePanel::left("page_put_panel_left")
            .resizable(true)
            .show(ctx, |ui| {
                self.show_puts_name(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
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
            pd.info = if b {
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
