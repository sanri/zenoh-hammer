use eframe::egui::{CollapsingHeader, Grid, RichText, Ui};
use std::sync::Arc;
use uhlc::Timestamp;
use zenoh::{
    bytes::Encoding,
    config::ZenohId,
    query::Reply,
    sample::{SampleKind, SourceInfo},
};

use crate::{
    data_viewer::DataViewer,
    hex_viewer::HexViewer,
    zenoh_data::{ZCongestionControl, ZPriority, ZReliability},
};

#[derive(Eq, PartialEq, Copy, Clone)]
enum ReplyViewerPage {
    Raw,
    Parse,
}

pub struct ReplyViewer {
    selected_page: ReplyViewerPage,
    reply_info: ReplyInfo,
    hex_viewer: HexViewer,
    data_viewer: DataViewer,
}

impl Default for ReplyViewer {
    fn default() -> Self {
        ReplyViewer {
            selected_page: ReplyViewerPage::Raw,
            reply_info: ReplyInfo::default(),
            hex_viewer: HexViewer::new(Arc::new(Vec::new())),
            data_viewer: DataViewer::Bin,
        }
    }
}

impl ReplyViewer {
    pub fn new_from_reply(reply: &Reply) -> Self {
        let reply_info = ReplyInfo::new_from(reply);
        let data = match reply.result() {
            Ok(sample) => sample.payload(),
            Err(err) => err.payload(),
        };
        let arc_data = Arc::new(data.to_bytes().to_vec());
        let data_viewer = DataViewer::load(&reply_info.encoding, arc_data.as_slice());
        let hex_viewer = HexViewer::new(arc_data);

        ReplyViewer {
            selected_page: ReplyViewerPage::Raw,
            reply_info,
            hex_viewer,
            data_viewer,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        self.reply_info.show(ui);

        ui.separator();

        self.show_table_label(ui);

        ui.add_space(10.0);

        match self.selected_page {
            ReplyViewerPage::Raw => {
                self.hex_viewer.show(ui);
            }
            ReplyViewerPage::Parse => {
                self.data_viewer.show(ui);
            }
        }
    }

    fn show_table_label(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.selected_page == ReplyViewerPage::Parse, "parse")
                .clicked()
            {
                self.selected_page = ReplyViewerPage::Parse;
            }

            if ui
                .selectable_label(self.selected_page == ReplyViewerPage::Raw, "raw")
                .clicked()
            {
                self.selected_page = ReplyViewerPage::Raw;
            }
        });
    }
}

struct ReplyInfo {
    ok: bool,
    key: Option<String>,
    kind: Option<SampleKind>,
    timestamp: Option<Timestamp>,
    congestion_control: Option<ZCongestionControl>,
    priority: Option<ZPriority>,
    reliability: Option<ZReliability>,
    express: Option<bool>,
    source_info: Option<SourceInfo>,
    attachment: Option<Vec<u8>>,
    replier_id: Option<ZenohId>,
    encoding: Encoding,
}

impl Default for ReplyInfo {
    fn default() -> Self {
        ReplyInfo {
            ok: false,
            key: None,
            kind: None,
            timestamp: None,
            congestion_control: None,
            priority: None,
            reliability: None,
            express: None,
            source_info: None,
            attachment: None,
            replier_id: None,
            encoding: Encoding::default(),
        }
    }
}

impl ReplyInfo {
    fn new_from(reply: &Reply) -> Self {
        let replier_id = reply.replier_id();
        match reply.result() {
            Ok(sample) => {
                let ok = true;
                let encoding = sample.encoding().clone();
                let key = Some(sample.key_expr().to_string());
                let kind = Some(sample.kind().clone());
                let timestamp = sample.timestamp().cloned();
                let congestion_control = Some(sample.congestion_control().clone().into());
                let priority = Some(sample.priority().clone().into());
                let reliability = Some(sample.reliability().clone().into());
                let express = Some(sample.express());
                let source_info = Some(sample.source_info().clone());
                let attachment = sample.attachment().map(|s| s.to_bytes().to_vec());

                ReplyInfo {
                    ok,
                    key,
                    kind,
                    timestamp,
                    congestion_control,
                    priority,
                    reliability,
                    express,
                    source_info,
                    attachment,
                    replier_id,
                    encoding,
                }
            }
            Err(err) => {
                let encoding = err.encoding().clone();
                ReplyInfo {
                    ok: false,
                    key: None,
                    kind: None,
                    timestamp: None,
                    congestion_control: None,
                    priority: None,
                    reliability: None,
                    express: None,
                    source_info: None,
                    attachment: None,
                    replier_id,
                    encoding,
                }
            }
        }
    }

    fn show(&mut self, ui: &mut Ui) {
        let show_ui = |ui: &mut Ui| {
            if let Some(key) = &self.key {
                ui.label("key:");
                let text = RichText::new(key).monospace();
                ui.label(text);
                ui.end_row();
            }

            if let Some(kind) = &self.kind {
                ui.label("kind:");
                let text = RichText::new(kind.to_string()).monospace();
                ui.label(text);
                ui.end_row();
            }

            if self.ok {
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
            }

            ui.label("encoding:");
            let text = RichText::new(self.encoding.to_string()).monospace();
            ui.label(text);
            ui.end_row();

            if let Some(attachment) = &self.attachment {
                ui.label("attachment:");
                let s = String::from_utf8(attachment.clone())
                    .unwrap_or(format!("{:?}", attachment.as_slice()));
                let text = RichText::new(s).monospace();
                ui.label(text);
                ui.end_row();
            }

            if let Some(source_info) = &self.source_info {
                ui.label("source_info. id:");
                let s = match source_info.source_id() {
                    None => "-".to_string(),
                    Some(o) => {
                        format!("{:?}", o)
                    }
                };
                let text = RichText::new(s).monospace();
                ui.label(text);
                ui.end_row();

                ui.label("source_info. sn:");
                let s = match source_info.source_sn() {
                    None => "-".to_string(),
                    Some(o) => {
                        format!("{}", o)
                    }
                };
                let text = RichText::new(s).monospace();
                ui.label(text);
                ui.end_row();

                ui.label("replier id:");
                let s = match &self.replier_id {
                    None => "-".to_string(),
                    Some(id) => {
                        format!("{}", id)
                    }
                };
                let text = RichText::new(s).monospace();
                ui.label(text);
                ui.end_row();
            }

            if let Some(congestion_control) = &self.congestion_control {
                ui.label("congestion_control:");
                let text = RichText::new(congestion_control.as_ref()).monospace();
                ui.label(text);
                ui.end_row();
            }

            if let Some(priority) = &self.priority {
                ui.label("priority:");
                let text = RichText::new(priority.as_ref()).monospace();
                ui.label(text);
                ui.end_row();
            }

            if let Some(reliability) = &self.reliability {
                ui.label("reliability:");
                let text = RichText::new(reliability.as_ref()).monospace();
                ui.label(text);
                ui.end_row();
            }

            if let Some(express) = &self.express {
                ui.label("express:");
                let text = RichText::new(express.to_string()).monospace();
                ui.label(text);
                ui.end_row();
            }
        };

        let header_text = if self.ok {
            "Sample info"
        } else {
            "Reply error"
        };
        CollapsingHeader::new(header_text)
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("reply_viewer_base_info_grid")
                    .num_columns(2)
                    .show(ui, |ui| {
                        show_ui(ui);
                    });
            });
    }
}
