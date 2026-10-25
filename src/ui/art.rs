use super::*;

#[derive(Clone, Copy)]
pub(super) enum Glyph {
    Play,
    Skin,
    Image,
    Settings,
    Activity,
    Plus,
    Close,
    Star,
    User,
    Back,
}

pub(super) fn glyph(p: &egui::Painter, rect: Rect, icon: Glyph, color: Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.34;
    let s = Stroke::new(1.7_f32, color);
    let pt = |x: f32, y: f32| c + Vec2::new(x * r, y * r);
    let line = |a: (f32, f32), b: (f32, f32)| p.line_segment([pt(a.0, a.1), pt(b.0, b.1)], s);
    match icon {
        Glyph::Play => {
            p.add(egui::Shape::convex_polygon(
                vec![pt(-0.55, -0.85), pt(0.85, 0.), pt(-0.55, 0.85)],
                color,
                Stroke::NONE,
            ));
        }
        Glyph::Image => {
            p.rect_stroke(
                Rect::from_min_max(pt(-0.95, -0.9), pt(0.95, 0.9)),
                2.,
                s,
                egui::StrokeKind::Inside,
            );
            p.circle_stroke(pt(-0.35, -0.35), r * 0.2, s);
            line((-0.8, 0.7), (-0.05, -0.1));
            line((-0.05, -0.1), (0.85, 0.7));
        }
        Glyph::Skin => {
            p.add(egui::Shape::closed_line(
                vec![
                    pt(-0.35, -0.85),
                    pt(-0.95, -0.45),
                    pt(-0.65, 0.05),
                    pt(-0.45, -0.1),
                    pt(-0.45, 0.9),
                    pt(0.45, 0.9),
                    pt(0.45, -0.1),
                    pt(0.65, 0.05),
                    pt(0.95, -0.45),
                    pt(0.35, -0.85),
                    pt(0., -0.6),
                ],
                s,
            ));
        }
        Glyph::Settings => {
            p.circle_stroke(c, r * 0.65, s);
            p.circle_stroke(c, r * 0.23, s);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.;
                let v = Vec2::angled(a);
                p.line_segment([c + v * r * 0.65, c + v * r], s);
            }
        }
        Glyph::Activity => {
            line((-1., 0.6), (-0.55, 0.6));
            line((-0.55, 0.6), (-0.15, -0.85));
            line((-0.15, -0.85), (0.25, 0.85));
            line((0.25, 0.85), (0.65, -0.15));
            line((0.65, -0.15), (1., -0.15));
        }
        Glyph::Plus => {
            line((-0.8, 0.), (0.8, 0.));
            line((0., -0.8), (0., 0.8));
        }
        Glyph::Close => {
            line((-0.6, -0.6), (0.6, 0.6));
            line((-0.6, 0.6), (0.6, -0.6));
        }
        Glyph::Star => {
            let points = (0..10)
                .map(|i| {
                    let a = i as f32 * std::f32::consts::PI / 5. - std::f32::consts::FRAC_PI_2;
                    c + Vec2::angled(a) * r * if i % 2 == 0 { 1. } else { 0.45 }
                })
                .collect();
            p.add(egui::Shape::closed_line(points, s));
        }
        Glyph::User => {
            p.circle_stroke(pt(0., -0.4), r * 0.38, s);
            p.add(egui::Shape::line(
                vec![pt(-0.75, 0.9), pt(-0.65, 0.3), pt(0.65, 0.3), pt(0.75, 0.9)],
                s,
            ));
        }
        Glyph::Back => {
            line((0.4, -0.8), (-0.4, 0.));
            line((-0.4, 0.), (0.4, 0.8));
        }
    }
}

pub(super) fn icon_button(
    ui: &mut egui::Ui,
    icon: Glyph,
    label: &str,
    selected: bool,
    enabled: bool,
    size: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::splat(size),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if selected || response.hovered() {
        ui.painter().rect_filled(
            rect,
            10.,
            if selected {
                palette().soft
            } else {
                palette().hover
            },
        );
    }
    glyph(
        ui.painter(),
        rect.shrink(size * 0.19),
        icon,
        if !enabled {
            palette().muted.gamma_multiply(0.4)
        } else if selected {
            MINT
        } else {
            palette().muted
        },
    );
    response.on_hover_text(label)
}

pub(super) fn loader_color(loader: Loader, seed: u32) -> Color32 {
    if seed == 0 {
        return match loader {
            Loader::Vanilla => Color32::from_rgb(56, 162, 80),
            Loader::Fabric => Color32::from_rgb(231, 190, 65),
            Loader::NeoForge => Color32::from_rgb(223, 125, 68),
            Loader::Forge => Color32::from_rgb(82, 128, 172),
        };
    }
    [
        Color32::from_rgb(94, 153, 111),
        Color32::from_rgb(104, 136, 192),
        Color32::from_rgb(170, 114, 184),
        Color32::from_rgb(207, 139, 100),
        Color32::from_rgb(185, 159, 80),
        Color32::from_rgb(73, 161, 155),
    ][(seed as usize - 1) % 6]
}

pub(super) fn cover(
    p: &egui::Painter,
    rect: Rect,
    loader: Loader,
    seed: u32,
    image: Option<egui::TextureId>,
) {
    if let Some(texture) = image {
        p.add(
            egui::epaint::RectShape::new(
                rect,
                12.,
                Color32::WHITE,
                Stroke::NONE,
                egui::StrokeKind::Inside,
            )
            .with_texture(texture, Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.))),
        );
        return;
    }
    let color = loader_color(loader, seed);
    p.rect_filled(rect, 12., color.gamma_multiply(0.72));
    let clip = p.with_clip_rect(rect.shrink(3.));
    clip.circle_filled(
        rect.max - Vec2::splat(rect.width() * 0.12),
        rect.width() * 0.68,
        Color32::from_white_alpha(5),
    );
    let c = rect.center() + Vec2::new(0., rect.height() * 0.02);
    let u = rect.width().min(rect.height()) * 0.28;
    let pt = |x: f32, y: f32| c + Vec2::new(x * u, y * u);
    p.add(egui::Shape::convex_polygon(
        vec![pt(-1., 0.8), pt(0., 1.3), pt(1.35, 0.7), pt(0.35, 0.2)],
        Color32::from_black_alpha(60),
        Stroke::NONE,
    ));
    match loader {
        Loader::Vanilla | Loader::Fabric => {
            let (top, left, right) = if loader == Loader::Vanilla {
                (
                    Color32::from_rgb(131, 197, 98),
                    Color32::from_rgb(110, 79, 60),
                    Color32::from_rgb(82, 58, 48),
                )
            } else {
                (
                    Color32::from_rgb(247, 238, 208),
                    Color32::from_rgb(214, 202, 173),
                    Color32::from_rgb(174, 163, 140),
                )
            };
            p.add(egui::Shape::convex_polygon(
                vec![pt(0., -1.05), pt(0.95, -0.52), pt(0., 0.), pt(-0.95, -0.52)],
                top,
                Stroke::NONE,
            ));
            p.add(egui::Shape::convex_polygon(
                vec![pt(-0.95, -0.52), pt(0., 0.), pt(0., 1.05), pt(-0.95, 0.53)],
                left,
                Stroke::NONE,
            ));
            p.add(egui::Shape::convex_polygon(
                vec![pt(0.95, -0.52), pt(0., 0.), pt(0., 1.05), pt(0.95, 0.53)],
                right,
                Stroke::NONE,
            ));
            if loader == Loader::Vanilla {
                for (x, y) in [(-0.7, -0.2), (-0.35, 0.05), (-0.7, 0.22)] {
                    p.rect_filled(
                        Rect::from_min_size(pt(x, y), Vec2::splat(u * 0.16)),
                        0.,
                        Color32::from_rgb(146, 105, 72),
                    );
                }
                p.line_segment(
                    [pt(-0.95, -0.40), pt(0., 0.14)],
                    Stroke::new(u * 0.14, Color32::from_rgb(100, 160, 74)),
                );
            } else {
                for y in [0.12, 0.33, 0.54] {
                    p.line_segment(
                        [pt(0.15, y), pt(0.75, y - 0.32)],
                        Stroke::new(u * 0.07, Color32::from_rgb(119, 109, 92)),
                    );
                }
            }
        }
        Loader::NeoForge => {
            p.add(egui::Shape::convex_polygon(
                vec![
                    pt(-0.9, -0.65),
                    pt(-0.7, -1.25),
                    pt(-0.25, -0.65),
                    pt(0.35, -0.65),
                    pt(0.85, -1.15),
                    pt(0.92, -0.5),
                    pt(0.65, 0.85),
                    pt(-0.55, 0.75),
                ],
                Color32::from_rgb(246, 168, 91),
                Stroke::NONE,
            ));
            p.add(egui::Shape::convex_polygon(
                vec![
                    pt(-0.8, 0.),
                    pt(-0.1, 0.38),
                    pt(0.8, -0.1),
                    pt(0.6, 0.88),
                    pt(-0.5, 0.8),
                ],
                Color32::from_rgb(249, 234, 207),
                Stroke::NONE,
            ));
            for x in [-0.5, 0.45] {
                p.rect_filled(
                    Rect::from_min_size(pt(x, -0.37), Vec2::splat(u * 0.16)),
                    0.,
                    palette().bg,
                );
            }
            p.add(egui::Shape::convex_polygon(
                vec![pt(-0.16, 0.18), pt(0.2, 0.18), pt(0., 0.45)],
                palette().bg,
                Stroke::NONE,
            ));
        }
        Loader::Forge => {
            p.add(egui::Shape::convex_polygon(
                vec![
                    pt(-1.2, -0.35),
                    pt(0.6, -0.75),
                    pt(1.2, -0.35),
                    pt(0., 0.05),
                ],
                Color32::from_rgb(203, 211, 218),
                Stroke::NONE,
            ));
            p.add(egui::Shape::convex_polygon(
                vec![
                    pt(-1.2, -0.35),
                    pt(0., 0.05),
                    pt(0., 0.35),
                    pt(-0.55, 0.1),
                    pt(-0.35, 0.65),
                    pt(-0.85, 0.85),
                    pt(0.15, 1.2),
                    pt(0.85, 0.85),
                    pt(0.35, 0.6),
                    pt(0.6, 0.25),
                    pt(1.2, -0.05),
                    pt(1.2, -0.35),
                    pt(0., 0.05),
                ],
                Color32::from_rgb(102, 119, 135),
                Stroke::NONE,
            ));
        }
    }
}

pub(super) fn text_fit(p: &egui::Painter, rect: Rect, text: &str, size: f32, color: Color32) {
    let font = FontId::proportional(size);
    let mut shown = text.to_owned();
    if p.layout_no_wrap(shown.clone(), font.clone(), color)
        .size()
        .x
        > rect.width()
    {
        while shown.chars().count() > 1
            && p.layout_no_wrap(format!("{shown}…"), font.clone(), color)
                .size()
                .x
                > rect.width()
        {
            shown.pop();
        }
        shown.push('…');
    }
    p.with_clip_rect(rect)
        .text(rect.left_top(), egui::Align2::LEFT_TOP, shown, font, color);
}
