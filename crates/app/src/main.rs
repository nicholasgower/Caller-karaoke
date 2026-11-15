mod app;
mod audio;
mod defaults;
mod source;
mod theme;
mod tts;

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 780.0])
            .with_min_inner_size([360.0, 560.0])
            .with_title("Contra Karaoke"),
        ..Default::default()
    };
    eframe::run_native(
        "Contra Karaoke",
        options,
        Box::new(|cc| Ok(Box::new(app::ContraApp::new(cc)))),
    )
}
