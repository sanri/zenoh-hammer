pub struct PagePut {}

impl Default for PagePut {
    fn default() -> Self {
        PagePut {}
    }
}

impl PagePut {
    pub fn show(&mut self, ui: &mut egui::Ui) {}
}
