mod app;

use app::PomodoroApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top()
            .with_inner_size([200.0, 270.0])
            .with_mouse_passthrough(true),
        ..Default::default()
    };

    eframe::run_native(
        "yasume",
        options,
        Box::new(|cc| Ok(Box::new(PomodoroApp::new(cc)))),
    )
}
