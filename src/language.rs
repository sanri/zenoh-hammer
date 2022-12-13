use eframe::egui::{FontData, FontDefinitions, FontFamily};

pub fn load_fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "wqy-microhei".to_owned(),
        FontData::from_static(include_bytes!("../fonts/wqy-microhei.ttc")),
    );

    fonts.font_data.insert(
        "JetBrainsMono-Medium".to_owned(),
        FontData::from_static(include_bytes!("../fonts/JetBrainsMono-Medium.ttf")),
    );

    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .push("wqy-microhei".to_owned());

    fonts
        .families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .push("JetBrainsMono-Medium".to_owned());

    fonts
        .families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .push("wqy-microhei".to_owned());

    fonts
}
