mod app;
mod helpers;
mod models;

use app::GalaxyApp;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();

    println!("About to call run_native");
    eframe::run_native(
        "Immutable Cosmic Borrow Galaxy",
        options,
        Box::new(|cc| Ok(Box::new(GalaxyApp::new(cc)))),
    )
}
