mod app;
mod language;
mod page_get;
mod page_pub;
mod page_put;
mod page_session;
mod page_sub;
mod zenoh;
mod msg;

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
        cc.egui_ctx.set_pixels_per_point(3.0);
        Box::new(hammer_app)
    });
    eframe::run_native("Zenoh 🔨", options, create);
}
