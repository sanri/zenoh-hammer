use egui::{Align, DragValue, Layout, RichText, ScrollArea, TextEdit, TextStyle, Ui};
use std::collections::BTreeMap;
use zenoh::prelude::{QueryConsolidation, QueryTarget, Value};
use zenoh::query::Mode::Auto;
use zenoh::query::{ConsolidationMode, Mode};

pub struct Data {
    id: u64,
    name: String,
    input_key: String,
    selected_target: QueryTarget,
    selected_consolidation: QueryConsolidation,
    timeout: u64,
    query_value: Option<Value>,
}

impl Default for Data {
    fn default() -> Self {
        Data {
            id: 0,
            name: "demo".to_string(),
            input_key: "demo/test".to_string(),
            selected_target: Default::default(),
            selected_consolidation: Default::default(),
            timeout: 1000,
            query_value: None,
        }
    }
}

impl Data {
    fn show(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            self.show_name_key(ui);
            self.show_options(ui);
        });
    }

    fn show_name_key(&mut self, ui: &mut Ui) {
        let mut input_grid = |ui: &mut Ui| {
            ui.label(" ");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("发送").clicked() {}
            });
            ui.end_row();

            ui.label("name: ");
            let te = TextEdit::singleline(&mut self.name)
                .desired_width(600.0)
                .font(TextStyle::Monospace);
            ui.add(te);
            ui.end_row();

            ui.label("key: ");
            let te = TextEdit::multiline(&mut self.input_key)
                .desired_rows(2)
                .desired_width(600.0)
                .font(TextStyle::Monospace);
            ui.add(te);
        };
        egui::Grid::new("input_grid")
            .num_columns(2)
            .striped(false)
            .show(ui, |ui| {
                input_grid(ui);
            });
    }

    fn show_options(&mut self, ui: &mut Ui) {
        let mut show_grid = |ui: &mut Ui| {
            ui.label("target: ");
            egui::ComboBox::new("query target", "")
                .selected_text(format!("{:?}", self.selected_target))
                .show_ui(ui, |ui| {
                    let options = [
                        QueryTarget::BestMatching,
                        QueryTarget::All,
                        QueryTarget::AllComplete,
                    ];
                    for option in options {
                        ui.selectable_value(
                            &mut self.selected_target,
                            option,
                            format!("{:?}", option),
                        );
                    }
                });
            ui.end_row();

            let dc = |c: QueryConsolidation| match c.mode() {
                Mode::Auto => "Auto",
                Mode::Manual(m) => match m {
                    ConsolidationMode::None => "None",
                    ConsolidationMode::Monotonic => "Monotonic",
                    ConsolidationMode::Latest => "Latest",
                },
            };

            ui.label("consolidation: ");
            egui::ComboBox::new("consolidation", "")
                .selected_text(dc(self.selected_consolidation))
                .show_ui(ui, |ui| {
                    let options = [
                        QueryConsolidation::AUTO,
                        QueryConsolidation::from(ConsolidationMode::None),
                        QueryConsolidation::from(ConsolidationMode::Monotonic),
                        QueryConsolidation::from(ConsolidationMode::Latest),
                    ];
                    for option in options {
                        ui.selectable_value(&mut self.selected_consolidation, option, dc(option));
                    }
                });
            ui.end_row();

            ui.label("timeout: ");
            let dv = DragValue::new(&mut self.timeout)
                .speed(10.0)
                .clamp_range(0..=10000);
            ui.add(dv);
            ui.end_row();
        };

        egui::Grid::new("options_grid")
            .num_columns(2)
            .striped(false)
            .show(ui, |ui| {
                show_grid(ui);
            });
    }
}

pub struct PageGet {
    pub data_map: BTreeMap<u64, Data>,
    selected_data: u64,
    data_id_count: u64,
}

impl Default for PageGet {
    fn default() -> Self {
        let mut btm = BTreeMap::new();
        btm.insert(1, Data::default());
        PageGet {
            data_map: btm,
            selected_data: 1,
            data_id_count: 1,
        }
    }
}

impl PageGet {
    pub fn show(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
            self.show_gets(ui);

            ui.separator();

            let mut data = match self.data_map.get_mut(&self.selected_data) {
                None => {
                    return;
                }
                Some(o) => o,
            };

            data.show(ui);
        });
    }

    fn show_gets(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if ui.button(RichText::new(" + ").code()).clicked() {
                    self.data_id_count += 1;
                    let data = Data {
                        id: self.data_id_count,
                        ..Data::default()
                    };
                    self.data_map.insert(self.data_id_count, data);
                };

                if ui.button(RichText::new(" - ").code()).clicked() {
                    if self.data_map.len() < 2 {
                        return;
                    }

                    let _ = self.data_map.remove(&self.selected_data);
                    for (k, _) in &self.data_map {
                        self.selected_data = *k;
                        break;
                    }
                };
            });

            ui.label("");

            ScrollArea::both()
                .max_width(160.0)
                .auto_shrink([true, false])
                .show(ui, |ui| {
                    for (i, d) in &self.data_map {
                        let text = RichText::new(d.name.clone()).monospace();
                        ui.selectable_value(&mut self.selected_data, *i, text);
                    }
                });
        });
    }
}
