use dreamer::app::App;

fn main() {
    tracing_subscriber::fmt::init();
    let options = eframe::NativeOptions::default();

    eframe::run_native("Dreamer", options, Box::new(|cc| Box::new(App::new(cc))))
        .expect("failed to run");
}
