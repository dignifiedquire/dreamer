use egui::widgets::Image;
use egui::{
    style::Margin, Color32, Layout, Response, RichText, Rounding, Sense, Stroke, TextureHandle, Ui,
    Vec2, Widget,
};
use egui::{TextStyle, WidgetText};

use crate::app::FONT_SEMI_BOLD;

pub struct Avatar<'a> {
    name: String,
    size: Vec2,
    margin: Margin,
    fill: Color32,
    rounding: Rounding,
    stroke: Stroke,
    image: Option<Image<'a>>,
    sense: Sense,
}

impl<'a> Avatar<'a> {
    pub fn new(name: String, size: Vec2, fill: Color32) -> Self {
        Avatar {
            name,
            size,
            margin: Margin::same(0.),
            fill,
            rounding: Rounding::ZERO,
            stroke: Stroke::NONE,
            image: None,
            sense: Sense::hover().union(Sense::click()),
        }
    }

    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = stroke;
        self
    }

    pub fn rounding(mut self, rounding: Rounding) -> Self {
        self.rounding = rounding;
        self
    }

    pub fn margin(mut self, margin: Margin) -> Self {
        self.margin = margin;
        self
    }

    pub fn image(mut self, texture: Option<TextureHandle>) -> Self {
        self.image = texture.map(|t| Image::new(&t).max_size(t.size_vec2()));
        self
    }
}

impl<'a> Widget for Avatar<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let sense = self.sense;
        let stroke = self.stroke; //Stroke::new(2., Color32::RED); // self.stroke
        let padding = Vec2::splat(stroke.width);
        let padded_size = self.size + 2.0 * padding;

        let (rect, response) = ui.allocate_exact_size(padded_size, sense);

        if ui.is_rect_visible(rect) {
            // Draw frame background

            let fill = if self.image.is_some() {
                Color32::TRANSPARENT
            } else {
                self.fill
            };

            let expansion = Vec2::splat(0.);

            ui.painter()
                .rect_filled(rect.expand2(expansion), self.rounding, fill);

            if let Some(ref image) = self.image {
                let image_rect = ui
                    .layout()
                    .align_size_within_rect(self.size, rect.shrink2(padding));
                image.paint_at(ui, image_rect);
            } else {
                let icon = WidgetText::RichText(
                    RichText::new(self.name.get(0..1).unwrap_or("x").to_ascii_uppercase())
                        .strong()
                        .size(24.)
                        .family(egui::FontFamily::Name(FONT_SEMI_BOLD.into()))
                        .color(Color32::WHITE),
                );

                let wrap_width = rect.width();
                let text = icon.into_galley(ui, None, wrap_width, TextStyle::Button);
                // center layout
                let text_pos = Layout::centered_and_justified(egui::Direction::LeftToRight)
                    .with_cross_justify(true)
                    .align_size_within_rect(text.size(), rect.shrink2(padding))
                    .min;
                let visuals = ui.style().interact(&response);

                text.paint_with_visuals(ui.painter(), text_pos, visuals);
            }

            ui.painter()
                .rect_stroke(rect.expand2(expansion), self.rounding, stroke);
        }
        response
    }
}
