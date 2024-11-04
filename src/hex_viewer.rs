use eframe::egui::{DragValue, RichText, ScrollArea, Ui, Widget};
use std::sync::Arc;

const PAGE_SIZE: usize = 1024;

pub struct HexViewer {
    data: Arc<Vec<u8>>,
    offset: usize,
    page_index: usize,
    number_columns: usize,
}

impl Default for HexViewer {
    fn default() -> Self {
        HexViewer {
            data: Arc::new(vec![]),
            offset: 0,
            page_index: 0,
            number_columns: 8,
        }
    }
}

impl HexViewer {
    pub fn new(data: Arc<Vec<u8>>) -> HexViewer {
        HexViewer {
            data,
            offset: 0,
            page_index: 0,
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

    fn page_index_max(&self) -> usize {
        let out = self.data.len() / PAGE_SIZE;
        if (self.data.len() % PAGE_SIZE == 0) && (out > 0) {
            out - 1
        } else {
            out
        }
    }

    fn page_rows(&self) -> usize {
        if self.page_index < self.page_index_max() {
            PAGE_SIZE / self.number_columns
        } else {
            (self.data.len() % PAGE_SIZE) / self.number_columns
        }
    }

    fn show_info(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("columns:");
            let dv = DragValue::new(&mut self.number_columns)
                .speed(1)
                .range(8..=16);
            ui.add(dv);

            ui.add_space(10.0);
            ui.label("offset:");
            DragValue::new(&mut self.offset)
                .speed(1)
                .range(0..=self.data.len())
                .ui(ui);

            ui.add_space(10.0);

            ui.label("page:");
            let page_index_max = self.data.len() / PAGE_SIZE;

            if ui.button("<<").clicked() {
                self.offset -= PAGE_SIZE * self.page_index;
                self.page_index = 0;
            }

            if ui.button("<").clicked() {
                if self.page_index > 0 {
                    self.page_index -= 1;
                    self.offset -= PAGE_SIZE;
                }
            }

            DragValue::new(&mut self.page_index)
                .speed(1)
                .range(0..=page_index_max)
                .ui(ui);

            if ui.button(">").clicked() {
                if self.page_index < page_index_max {
                    self.page_index += 1;
                    self.offset += PAGE_SIZE;
                    if self.offset > self.data.len() {
                        self.offset = self.data.len();
                    }
                }
            }

            if ui.button(">>").clicked() {
                self.offset += (page_index_max - self.page_index) * PAGE_SIZE;
                self.page_index = page_index_max;
                if self.offset >= self.data.len() {
                    self.offset = self.data.len() - 1;
                }
            }
        });
    }

    fn show_header(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.add_space(40.0);
            ui.style_mut().spacing.item_spacing.x = 16.0;
            for i in 0..self.number_columns {
                ui.label(RichText::new(format!("{:02x}", i)).monospace());
            }
        });
    }

    fn show_hex(&mut self, ui: &mut Ui) {
        let rows = self.page_rows();
        ui.vertical(|ui| {
            for row in 0..=rows {
                let row_label_number = row * self.number_columns + self.page_index * PAGE_SIZE;
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{:04x}", row_label_number)).monospace());
                    for i in 0..self.number_columns {
                        let index = self.page_index * PAGE_SIZE + row * self.number_columns + i;
                        if let Some(v) = self.data.get(index) {
                            ui.selectable_value(
                                &mut self.offset,
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

        let rows = self.page_rows();
        ui.vertical(|ui| {
            for row in 0..=rows {
                ui.horizontal(|ui| {
                    for i in 0..self.number_columns {
                        let index = self.page_index * PAGE_SIZE + row * self.number_columns + i;
                        if let Some(v) = self.data.get(index) {
                            let c = u8_to_char(*v);
                            ui.selectable_value(
                                &mut self.offset,
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
