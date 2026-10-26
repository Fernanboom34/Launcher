use eframe::egui::{self, Color32, Rect, Stroke, Vec2};

fn paint(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, 5., Color32::from_black_alpha(135));
    let origin = rect.min + Vec2::new(8., 6.);
    let stroke = Stroke::new(2.0_f32, Color32::from_rgb(215, 220, 225));
    for (x, y, horizontal) in [
        (0., 0., false),
        (20., 0., false),
        (26., 0., false),
        (0., 16., false),
        (6., 16., false),
        (20., 16., false),
        (20., 24., true),
    ] {
        let start = origin + Vec2::new(x, y);
        let end = start
            + if horizontal {
                Vec2::new(8., 0.)
            } else {
                Vec2::new(0., 8.)
            };
        painter.line_segment([start, end], stroke);
    }
}

pub(super) fn show(ctx: &egui::Context) {
    let size = Vec2::new(50., 38.);
    let rect = Rect::from_min_size(ctx.screen_rect().max - size - Vec2::splat(8.), size);
    paint(
        &ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("loss-mark"),
        )),
        rect,
    );
}
