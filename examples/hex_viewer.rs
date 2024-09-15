#[path = "../src/hex_viewer.rs"]
mod hex_viewer;

use eframe::{
    egui::{CentralPanel, Context},
    AppCreator, Frame, HardwareAcceleration, NativeOptions,
};
use std::sync::Arc;

use crate::hex_viewer::HexViewer;

struct AppHexViewer {
    viewer: HexViewer,
}

impl Default for AppHexViewer {
    fn default() -> Self {
        let len = 4 * 1024 + 512;
        let mut vec = Vec::with_capacity(len);
        for i in 0..len {
            vec.push(i as u8);
        }

        AppHexViewer {
            viewer: HexViewer::new({ Arc::new(vec) }),
        }
    }
}

impl eframe::App for AppHexViewer {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
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
    let _ = eframe::run_native("HexViewer", options, create);
}
