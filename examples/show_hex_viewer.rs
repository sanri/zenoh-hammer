#[path = "../src/hex_viewer.rs"]
mod hex_viewer;

use eframe::{
    AppCreator, Frame, HardwareAcceleration, NativeOptions,
    egui::{CentralPanel, Ui},
};
use env_logger::Env;
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
            viewer: HexViewer::new(Arc::new(vec)),
        }
    }
}

impl eframe::App for AppHexViewer {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        CentralPanel::default().show_inside(ui, |ui| {
            self.viewer.show(ui);
        });
    }
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("show_hex_viewer=info")).init();

    let options = NativeOptions {
        hardware_acceleration: HardwareAcceleration::Required,
        ..NativeOptions::default()
    };
    let app = AppHexViewer::default();
    let create: AppCreator = Box::new(|_cc| Ok(Box::new(app)));
    let _ = eframe::run_native("HexViewer", options, create);
}
