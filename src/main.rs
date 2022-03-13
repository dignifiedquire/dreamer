use dreamer::app::App;

fn main() {
    tracing_subscriber::fmt::init();
    let app = App::new();
    let options = eframe::NativeOptions::default();

    eframe::run_native(Box::new(app), options);
}
