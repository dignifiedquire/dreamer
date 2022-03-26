use egui::{
    style::Margin, Color32, Label, Layout, Rect, Response, RichText, Rounding, Sense, Shape,
    Stroke, Ui, Vec2, Widget,
};

use crate::app::FONT_SEMI_BOLD;

pub struct Avatar {
    name: String,
    size: Vec2,
    margin: Margin,
    fill: Color32,
    rounding: Rounding,
    stroke: Stroke,
}

impl Avatar {
    pub fn new(name: String, size: Vec2, fill: Color32) -> Self {
        Avatar {
            name,
            size,
            margin: Margin::same(0.),
            fill,
            rounding: Rounding::none(),
            stroke: Stroke::none(),
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
}

impl Widget for Avatar {
    fn ui(self, ui: &mut Ui) -> Response {
        let resp = ui.allocate_ui(self.size, |ui| {
            let outer_rect_bounds = ui.available_rect_before_wrap();
            let inner_rect = outer_rect_bounds.shrink2(self.margin.sum());

            let bg = ui.painter().add(Shape::Noop);
            let mut content_ui = ui.child_ui(
                inner_rect,
                Layout::centered_and_justified(egui::Direction::LeftToRight)
                    .with_cross_justify(true),
            );
            content_ui.set_min_width(40.);

            let icon = RichText::new(self.name.get(0..1).unwrap_or("x").to_ascii_uppercase())
                .strong()
                .size(24.)
                .family(egui::FontFamily::Name(FONT_SEMI_BOLD.into()))
                .color(Color32::WHITE);
            let resp1 = content_ui.add(Label::new(icon));

            let outer_rect = Rect::from_min_max(
                outer_rect_bounds.min,
                content_ui.min_rect().max + self.margin.sum(),
            );
            let (rect, resp2) =
                ui.allocate_at_least(outer_rect.size(), Sense::hover().union(Sense::click()));
            ui.painter().set(
                bg,
                epaint::RectShape {
                    fill: self.fill,
                    stroke: self.stroke,
                    rounding: self.rounding,
                    rect,
                },
            );
            resp1 | resp2
        });

        resp.inner | resp.response
    }
}
