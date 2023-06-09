#[path = "../src/hex_viewer.rs"]
mod hex_viewer;

use eframe::{
    egui::{CentralPanel, Color32, Context, Layout, RichText},
    emath::Align,
    AppCreator, Frame, HardwareAcceleration, NativeOptions,
};

use crate::hex_viewer::HexViewer;

struct AppHexViewer {
    viewer: HexViewer,
}

impl Default for AppHexViewer {
    fn default() -> Self {
        AppHexViewer {
            viewer: HexViewer::new({
                let mut vec = Vec::with_capacity(2000);
                for i in 0..2000 {
                    vec.push(i as u8);
                }
                vec
            }),
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
    let create: AppCreator = Box::new(|cc| Box::new(app));
    let _ = eframe::run_native("HexViewer", options, create);
}
