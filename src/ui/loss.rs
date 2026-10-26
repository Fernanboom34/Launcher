use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use std::time::Duration;

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

pub(super) fn show(ctx: &egui::Context, playing: bool) {
    let size = Vec2::new(50., 38.);
    let rect = Rect::from_min_size(ctx.screen_rect().max - size - Vec2::splat(8.), size);
    paint(
        &ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("loss-mark"),
        )),
        rect,
    );
    if playing {
        let monitor = ctx
            .input(|i| i.viewport().monitor_size)
            .unwrap_or(ctx.screen_rect().size());
        ctx.show_viewport_deferred(
            egui::ViewportId::from_hash_of("loss-game-overlay"),
            egui::ViewportBuilder::default()
                .with_title("Luma · Loss")
                .with_inner_size(size)
                .with_position(Pos2::new(
                    (monitor.x - size.x - 12.).max(0.),
                    (monitor.y - size.y - 12.).max(0.),
                ))
                .with_decorations(false)
                .with_resizable(false)
                .with_transparent(true)
                .with_active(false)
                .with_taskbar(false)
                .with_mouse_passthrough(true)
                .with_window_level(egui::WindowLevel::AlwaysOnTop),
            move |overlay, class| {
                if class == egui::ViewportClass::Embedded {
                    return;
                }
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(overlay, |ui| {
                        paint(ui.painter(), Rect::from_min_size(ui.min_rect().min, size));
                    });
                overlay.request_repaint_after(Duration::from_millis(250));
            },
        );
        ctx.request_repaint_after(Duration::from_millis(250));
    }
}
