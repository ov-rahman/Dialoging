// Без консольного окна на Windows в релизной сборке.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod icons;
mod doc;
mod editor;
mod theme;
mod tokens;
mod ui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Dialoging")
            .with_inner_size([1180.0, 820.0])
            .with_min_inner_size([720.0, 560.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Dialoging",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
