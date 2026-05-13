#[path = "../src/hex_viewer.rs"]
mod hex_viewer;
#[path = "../src/payload_editor.rs"]
mod payload_editor;
#[path = "../src/zenoh_data.rs"]
mod zenoh_data;

use eframe::{
    App, AppCreator, Frame, HardwareAcceleration, NativeOptions,
    egui::{CentralPanel, Ui},
    run_native,
};
use env_logger::Env;

use crate::payload_editor::PayloadEdit;

struct AppPayloadEditor {
    editor: PayloadEdit,
}

impl Default for AppPayloadEditor {
    fn default() -> Self {
        AppPayloadEditor {
            editor: Default::default(),
        }
    }
}

impl App for AppPayloadEditor {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        CentralPanel::default().show_inside(ui, |ui| {
            self.editor.show(ui);
        });
    }
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("show_payload_editor=info"))
        .init();

    let options = NativeOptions {
        hardware_acceleration: HardwareAcceleration::Required,
        ..NativeOptions::default()
    };
    let app = AppPayloadEditor::default();
    let create: AppCreator = Box::new(|_cc| Ok(Box::new(app)));
    let _ = run_native("PayloadEditor", options, create);
}
