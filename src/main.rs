mod app;
mod archive_file;
mod data_viewer;
mod hex_viewer;
mod language;
mod page_get;
mod page_put;
mod page_session;
mod page_sub;
mod payload_editor;
mod reply_viewer;
mod sample_viewer;
mod task_zenoh;
mod zenoh_data;

use eframe::{
    egui::ViewportBuilder, icon_data::from_png_bytes, AppCreator, HardwareAcceleration,
    NativeOptions,
};
use env_logger::Env;
use std::sync::Arc;

use crate::{app::HammerApp, language::load_fonts};

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("zenoh_hammer=info")).init();

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
        Ok(Box::new(hammer_app))
    });
    let _ = eframe::run_native("Zenoh Hammer", options, create);
}
