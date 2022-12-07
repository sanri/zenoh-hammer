pub struct PagePub {}

impl Default for PagePub {
    fn default() -> Self {
        PagePub {}
    }
}

impl PagePub {
    pub fn show(&mut self, ui: &mut egui::Ui) {}
}
