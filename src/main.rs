mod app;
mod file;
mod hex_viewer;
mod language;
mod page_get;
mod page_put;
mod page_session;
mod page_sub;
mod zenoh;

use eframe::egui::ViewportBuilder;
use eframe::{icon_data::from_png_bytes, AppCreator, HardwareAcceleration, NativeOptions};
use std::sync::Arc;

use crate::{app::HammerApp, language::load_fonts};

fn main() {
    let options = NativeOptions {
        hardware_acceleration: HardwareAcceleration::Required,
        viewport: ViewportBuilder {
            icon: Some(Arc::new(
                from_png_bytes(&include_bytes!("../media/hammer.png")[..]).unwrap(),
            )),
            ..ViewportBuilder::default()
        },
        ..NativeOptions::default()
    };
    let fonts = load_fonts();
    let hammer_app = HammerApp::default();
    let create: AppCreator = Box::new(|cc| {
        cc.egui_ctx.set_fonts(fonts);
        Box::new(hammer_app)
    });
    let _ = eframe::run_native("Zenoh Hammer", options, create);
}
