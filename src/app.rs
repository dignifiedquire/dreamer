use crate::{
    state::AppState,
    widgets::{add_account::AddAccount, mainpanel::render_main_panel, sidebar::render_sidebar},
};
use egui::{FontData, FontDefinitions, FontFamily, Visuals};
use std::fs;

pub enum WidgetOption {
    AddAccount(AddAccount),
}

pub struct App {
    state: AppState,
    right_side: Option<WidgetOption>,
}

pub const FONT_LIGHT: &str = "OpenSans-Light";
pub const FONT_REGULAR: &str = "OpenSans-Regular";
pub const FONT_SEMI_BOLD: &str = "OpenSans-SemiBold";

impl App {
    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut AppState {
        &mut self.state
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(Visuals::light());

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

        cc.egui_ctx.set_fonts(fonts);

        let state = AppState::new(&cc.egui_ctx);

        App {
            right_side: Some(WidgetOption::AddAccount(AddAccount::new())),
            state,
        }
    }
}

impl epi::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut epi::Frame) {
        ctx.set_debug_on_hover(true);
        let width_total = ctx.available_rect().width();
        let side_panel_size = (width_total * 0.25).round();
        render_sidebar(ctx, self.state(), side_panel_size);
        if let Some(widget) = &mut self.right_side {
            match widget {
                WidgetOption::AddAccount(ac) => {
                    ac.ui(ctx, &self.state, side_panel_size);
                }
            };
        }
        render_main_panel(ctx, self.state_mut());
    }
}
