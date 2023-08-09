use eframe::egui::{DragValue, RichText, ScrollArea, Ui};

pub const HEX_VIEWER_SIZE: usize = 5 * 1024;

pub struct HexViewer {
    buffer: Vec<u8>,
    select_index: usize,
    number_columns: usize,
}

impl HexViewer {
    pub fn new(data: Vec<u8>) -> HexViewer {
        HexViewer {
            buffer: data,
            select_index: 0,
            number_columns: 8,
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            self.show_info(ui);

            self.show_header(ui);
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        self.show_hex(ui);
                        ui.separator();
                        self.show_ascii(ui);
                    });
                });
        });
    }

    fn show_info(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("columns:");
            let dv = DragValue::new(&mut self.number_columns)
                .speed(1)
                .clamp_range(8..=16);
            ui.add(dv);

            ui.label("  ");

            ui.label("offset:");
            let dv = DragValue::new(&mut self.select_index)
                .speed(1)
                .clamp_range(0..=self.buffer.len());
            ui.add(dv);
        });
    }

    fn show_header(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("    ")).monospace());
            for i in 0..self.number_columns {
                let _ = ui.selectable_label(false, RichText::new(format!("{:02x}", i)).monospace());
            }
        });
    }

    fn show_hex(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            let rows = self.buffer.len() / self.number_columns;
            for row in 0..=rows {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{:04x}", row * self.number_columns)).monospace(),
                    );
                    for i in 0..self.number_columns {
                        let index = row * self.number_columns + i;
                        if let Some(v) = self.buffer.get(index) {
                            ui.selectable_value(
                                &mut self.select_index,
                                index,
                                RichText::new(format!("{:02X}", v)).monospace(),
                            );
                        } else {
                            let _ = ui.selectable_label(false, RichText::new("  ").monospace());
                        }
                    }
                });
            }
        });
    }

    fn show_ascii(&mut self, ui: &mut Ui) {
        let style = ui.style_mut();
        style.spacing.item_spacing.x = 0.0;
        style.spacing.button_padding.x = 1.0;
        ui.vertical(|ui| {
            let rows = self.buffer.len() / self.number_columns;
            for row in 0..=rows {
                ui.horizontal(|ui| {
                    for i in 0..self.number_columns {
                        let index = row * self.number_columns + i;
                        if let Some(v) = self.buffer.get(index) {
                            let c = u8_to_char(*v);
                            ui.selectable_value(
                                &mut self.select_index,
                                index,
                                RichText::new(c).monospace(),
                            );
                        } else {
                            let _ = ui.selectable_label(false, RichText::new(" ").monospace());
                        }
                    }
                    ui.label(" ");
                });
            }
        });
    }
}

fn u8_to_char(v: u8) -> char {
    match v {
        32u8..=126u8 => char::from(v),
        _ => '.',
    }
}
