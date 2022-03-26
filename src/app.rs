use std::fs;

use egui::{FontData, FontDefinitions, FontFamily, Visuals};

use crate::{
    state::AppState,
    widgets::{mainpanel::render_main_panel, sidebar::render_sidebar},
};

pub struct App {
    state: Option<AppState>,
}

impl App {
    pub fn new() -> Self {
        App { state: None }
    }
}

pub const FONT_LIGHT: &str = "OpenSans-Light";
pub const FONT_REGULAR: &str = "OpenSans-Regular";
pub const FONT_SEMI_BOLD: &str = "OpenSans-SemiBold";

impl App {
    pub fn state(&self) -> &AppState {
        self.state.as_ref().expect("must be called after setup")
    }

    pub fn state_mut(&mut self) -> &mut AppState {
        self.state.as_mut().expect("must be called after setup")
    }
}

impl epi::App for App {
    fn name(&self) -> &str {
        "Dreamer"
    }

    fn setup(
        &mut self,
        ctx: &egui::Context,
        frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        self.state = Some(AppState::new(frame));
        ctx.set_visuals(Visuals::light());

        let mut fonts = FontDefinitions::default();
        let mut load_font = |name: &str, path| match fs::read(path) {
            Ok(font) => {
                fonts
                    .font_data
                    .insert(name.to_string(), FontData::from_owned(font));
                fonts
                    .families
                    .entry(FontFamily::Name(name.into()))
                    .or_insert_with(Vec::new)
                    .push(name.to_string());
            }
            Err(err) => {
                log::warn!("failed to load {name}: {:?}", err);
            }
        };

        load_font(FONT_LIGHT, "./fonts/OpenSans-Light.ttf");
        load_font(FONT_REGULAR, "./fonts/OpenSans-Regular.ttf");
        load_font(FONT_SEMI_BOLD, "./fonts/OpenSans-SemiBold.ttf");

        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .push(FONT_REGULAR.to_string());

        ctx.set_fonts(fonts);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        self.state_mut().poll(ctx);

        render_sidebar(ctx, self.state());
        render_main_panel(ctx, self.state_mut());
    }
}
