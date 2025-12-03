use eframe::egui::{CollapsingHeader, Grid, RichText, Ui};
use std::{io::Read, sync::Arc};
use uhlc::Timestamp;
use zenoh::{
    bytes::Encoding,
    sample::{Sample, SampleKind, SourceInfo},
};

use crate::{
    data_viewer::DataViewer,
    hex_viewer::HexViewer,
    zenoh_data::{bytes_type, BytesType, ZCongestionControl, ZPriority, ZReliability},
};

#[derive(Eq, PartialEq, Copy, Clone)]
enum SampleViewerPage {
    Raw,
    Parse,
}

pub struct SampleViewer {
    selected_page: SampleViewerPage,
    sample_info: SampleInfo,
    hex_view: HexViewer,
    data_viewer: DataViewer,
}

impl Default for SampleViewer {
    fn default() -> Self {
        SampleViewer {
            selected_page: SampleViewerPage::Raw,
            sample_info: SampleInfo::default(),
            hex_view: HexViewer::new(Arc::new(Vec::new())),
            data_viewer: DataViewer::Bin,
        }
    }
}

impl SampleViewer {
    pub fn new_from_sample(sample: &Sample) -> Self {
        let sample_info = SampleInfo::new_from(sample);
        let arc_data = Arc::new(sample.payload().to_bytes().to_vec());
        let data_viewer = DataViewer::load(sample.encoding(), arc_data.as_slice());
        let hex_view = HexViewer::new(arc_data);

        SampleViewer {
            selected_page: SampleViewerPage::Parse,
            sample_info,
            hex_view,
            data_viewer,
        }
    }

    #[allow(dead_code)]
    pub fn new(base_info: SampleInfo, data: Vec<u8>) -> Self {
        let viewer_data = DataViewer::load(&base_info.encoding, data.as_slice());
        let hex_view = HexViewer::new(Arc::new(data));

        SampleViewer {
            selected_page: SampleViewerPage::Parse,
            sample_info: base_info,
            hex_view,
            data_viewer: viewer_data,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        self.sample_info.show(ui);

        ui.separator();

        self.show_tab_label(ui);

        ui.add_space(10.0);

        match self.selected_page {
            SampleViewerPage::Raw => {
                self.hex_view.show(ui);
            }
            SampleViewerPage::Parse => {
                self.data_viewer.show(ui);
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

pub struct SampleInfo {
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
    pub bytes_type: BytesType,
}

impl Default for SampleInfo {
    fn default() -> Self {
        SampleInfo {
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
            bytes_type: BytesType::Raw,
        }
    }
}

impl SampleInfo {
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
        let bytes_type = bytes_type(sample.payload());

        SampleInfo {
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
            bytes_type,
        }
    }

    fn show(&mut self, ui: &mut Ui) {
        let show_ui = |ui: &mut Ui| {
            ui.label("key:");
            let text = RichText::new(self.key.as_str()).monospace();
            ui.label(text);
            ui.end_row();

            ui.label("kind:");
            let text = RichText::new(self.kind.to_string().to_lowercase()).monospace();
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

            ui.label("bytes_type:");
            let text = RichText::new(self.bytes_type.as_ref()).monospace();
            ui.label(text);
            ui.end_row();
        };

        CollapsingHeader::new("Sample info")
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
