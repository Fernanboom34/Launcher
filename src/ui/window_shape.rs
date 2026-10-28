//! Cut the actual framebuffer alpha, rather than painting fake background corners.
use eframe::{egui, egui_glow, glow};
use glow::HasContext;
use std::sync::Arc;

fn inset(radius: i32, row: i32) -> i32 {
    let y = radius as f32 - row as f32 - 0.5;
    (radius as f32 - ((radius * radius) as f32 - y * y).sqrt()).round() as i32
}

pub(super) fn show(ctx: &egui::Context, decorations: bool) {
    let square = ctx.input(|i| {
        i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)
    });
    if decorations || square {
        return;
    }
    ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("window-corners"),
    ))
    .add(egui::PaintCallback {
        rect: ctx.screen_rect(),
        callback: Arc::new(egui_glow::CallbackFn::new(|info, painter| {
            let gl = painter.gl();
            let [width, height] = info.screen_size_px.map(|v| v as i32);
            let radius = (16.0 * info.pixels_per_point).round() as i32;
            let radius = radius.min(width / 2).min(height / 2);
            // The callback runs on the renderer's GL thread with its context current.
            // Restore the scissor state before handing rendering back to egui.
            unsafe {
                let enabled = gl.is_enabled(glow::SCISSOR_TEST);
                let mut scissor = [0; 4];
                gl.get_parameter_i32_slice(glow::SCISSOR_BOX, &mut scissor);
                gl.enable(glow::SCISSOR_TEST);
                gl.clear_color(0.0, 0.0, 0.0, 0.0);
                for row in 0..radius {
                    let cut = inset(radius, row);
                    for x in [0, width - cut] {
                        for y in [row, height - row - 1] {
                            gl.scissor(x, y, cut, 1);
                            gl.clear(glow::COLOR_BUFFER_BIT);
                        }
                    }
                }
                gl.scissor(scissor[0], scissor[1], scissor[2], scissor[3]);
                if !enabled {
                    gl.disable(glow::SCISSOR_TEST);
                }
            }
        })),
    });
}
