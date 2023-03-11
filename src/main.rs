mod app;
mod file;
mod hex_viewer;
mod language;
mod page_get;
mod page_put;
mod page_session;
mod page_sub;
mod zenoh;

use crate::{app::HammerApp, language::load_fonts};
use eframe::{AppCreator, HardwareAcceleration, NativeOptions};

fn main() {
    let options = NativeOptions {
        hardware_acceleration: HardwareAcceleration::Required,
        ..NativeOptions::default()
    };
    let fonts = load_fonts();
    let hammer_app = HammerApp::default();
    let create: AppCreator = Box::new(|cc| {
        cc.egui_ctx.set_fonts(fonts);
        Box::new(hammer_app)
    });
    let _ = eframe::run_native("Zenoh 🔨", options, create);
}
