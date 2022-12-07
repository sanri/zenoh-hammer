pub struct PageGet {}

impl Default for PageGet {
    fn default() -> Self {
        PageGet {}
    }
}

impl PageGet {
    pub fn show(&mut self, ui: &mut egui::Ui) {}
}
