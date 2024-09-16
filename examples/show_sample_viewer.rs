#[path = "../src/hex_viewer.rs"]
mod hex_viewer;
#[path = "../src/sample_viewer.rs"]
mod sample_viewer;
#[path = "../src/zenoh_data.rs"]
mod zenoh_data;

use eframe::{
    egui::{CentralPanel, Context},
    AppCreator, Frame, HardwareAcceleration, NativeOptions,
};
use std::io::Read;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uhlc::{Timestamp, ID, NTP64};
use zenoh::{
    bytes::{Encoding, ZBytes},
    sample::{SampleKind, SourceInfo},
};

use crate::{
    sample_viewer::{BaseInfo, SampleViewer},
    zenoh_data::{ZCongestionControl, ZPriority, ZReliability},
};

struct AppHexViewer {
    viewer: SampleViewer,
}

impl Default for AppHexViewer {
    fn default() -> Self {
        let base_info = base_info();
        let (data, arc_data) = data();
        let viewer = SampleViewer::new(base_info, data, arc_data);
        AppHexViewer { viewer }
    }
}

impl eframe::App for AppHexViewer {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| {
            self.viewer.show(ui);
        });
    }
}

fn main() {
    let options = NativeOptions {
        hardware_acceleration: HardwareAcceleration::Required,
        ..NativeOptions::default()
    };
    let app = AppHexViewer::default();
    let create: AppCreator = Box::new(|cc| Ok(Box::new(app)));
    let _ = eframe::run_native("SampleViewer", options, create);
}

fn base_info() -> BaseInfo {
    let key = "test/a/b/c/1".to_string();
    let encoding = Encoding::ZENOH_BOOL;
    let kind = SampleKind::Put;
    let time = NTP64::from(SystemTime::now().duration_since(UNIX_EPOCH).unwrap());
    let timestamp = Some(Timestamp::new(time, ID::rand()));
    let congestion_control = ZCongestionControl::Block;
    let priority = ZPriority::RealTime;
    let reliability = ZReliability::Reliable;
    let express = true;
    let source_info = SourceInfo {
        source_id: None,
        source_sn: Some(123),
    };
    let attachment = b"a=1".to_vec();

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

fn data() -> (ZBytes, Arc<Vec<u8>>) {
    let z_bytes = ZBytes::from(true);
    let mut buf = Vec::new();
    let mut reader = z_bytes.reader();
    let _ = reader.read_to_end(&mut buf).unwrap();

    (z_bytes, Arc::new(buf))
}
