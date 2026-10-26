use super::*;
use egui::epaint::{Mesh, Vertex};
type Face = (f32, [[f32; 3]; 4], [f32; 4], Color32);

// Static, pixel-exact skin cuboids. No animation loop or extra rendering engine.
pub(super) fn draw(ui: &egui::Ui, rect: Rect, texture: egui::TextureId, slim: bool, yaw: f32) {
    let arm = if slim { 3. } else { 4. };
    let mut faces: Vec<Face> = Vec::new();
    let mut cube = |bounds: [f32; 6], origin: [f32; 2], dims: [f32; 3], overlay: bool| {
        let [mut x0, mut y0, mut z0, mut x1, mut y1, mut z1] = bounds;
        if overlay {
            let e = 0.23;
            x0 -= e;
            y0 -= e;
            z0 -= e;
            x1 += e;
            y1 += e;
            z1 += e;
        }
        let [u, v] = origin;
        let [w, h, d] = dims;
        let quads = [
            (
                [[x0, y1, z1], [x1, y1, z1], [x1, y0, z1], [x0, y0, z1]],
                [u + d, v + d, w, h],
                [0., 0., 1.],
                1.,
            ),
            (
                [[x1, y1, z0], [x0, y1, z0], [x0, y0, z0], [x1, y0, z0]],
                [u + 2. * d + w, v + d, w, h],
                [0., 0., -1.],
                0.8,
            ),
            (
                [[x0, y1, z0], [x0, y1, z1], [x0, y0, z1], [x0, y0, z0]],
                [u, v + d, d, h],
                [-1., 0., 0.],
                0.78,
            ),
            (
                [[x1, y1, z1], [x1, y1, z0], [x1, y0, z0], [x1, y0, z1]],
                [u + d + w, v + d, d, h],
                [1., 0., 0.],
                0.87,
            ),
            (
                [[x0, y1, z0], [x1, y1, z0], [x1, y1, z1], [x0, y1, z1]],
                [u + d, v, w, d],
                [0., 1., 0.],
                1.,
            ),
            (
                [[x0, y0, z1], [x1, y0, z1], [x1, y0, z0], [x0, y0, z0]],
                [u + d + w, v, w, d],
                [0., -1., 0.],
                0.65,
            ),
        ];
        for (points, uv, normal, shade) in quads {
            if normal[0] * yaw.sin() + normal[2] * yaw.cos() + normal[1] * 0.22 <= 0. {
                continue;
            }
            let depth = points
                .iter()
                .map(|p| p[0] * yaw.sin() + p[2] * yaw.cos() + p[1] * 0.22)
                .sum::<f32>()
                / 4.;
            faces.push((depth, points, uv, Color32::from_gray((shade * 255.) as u8)));
        }
    };
    for layer in [false, true] {
        cube(
            [-4., 24., -4., 4., 32., 4.],
            if layer { [32., 0.] } else { [0., 0.] },
            [8., 8., 8.],
            layer,
        );
        cube(
            [-4., 12., -2., 4., 24., 2.],
            if layer { [16., 32.] } else { [16., 16.] },
            [8., 12., 4.],
            layer,
        );
        cube(
            [-4. - arm, 12., -2., -4., 24., 2.],
            if layer { [40., 32.] } else { [40., 16.] },
            [arm, 12., 4.],
            layer,
        );
        cube(
            [4., 12., -2., 4. + arm, 24., 2.],
            if layer { [48., 48.] } else { [32., 48.] },
            [arm, 12., 4.],
            layer,
        );
        cube(
            [-4., 0., -2., 0., 12., 2.],
            if layer { [0., 32.] } else { [0., 16.] },
            [4., 12., 4.],
            layer,
        );
        cube(
            [0., 0., -2., 4., 12., 2.],
            if layer { [0., 48.] } else { [16., 48.] },
            [4., 12., 4.],
            layer,
        );
    }
    faces.sort_by(|a, b| a.0.total_cmp(&b.0));
    let scale = (rect.height() / 36.).min(rect.width() / 24.);
    let center = Pos2::new(rect.center().x, rect.bottom() - scale * 1.2);
    ui.painter().add(egui::Shape::ellipse_filled(
        Pos2::new(center.x, center.y + 3.),
        Vec2::new(scale * 8., scale * 1.2),
        Color32::from_black_alpha(30),
    ));
    for (_, points, [u, v, w, h], color) in faces {
        let mut mesh = Mesh::with_texture(texture);
        for (point, uv) in points
            .into_iter()
            .zip([[u, v], [u + w, v], [u + w, v + h], [u, v + h]])
        {
            let x = point[0] * yaw.cos() - point[2] * yaw.sin();
            let z = point[0] * yaw.sin() + point[2] * yaw.cos();
            mesh.vertices.push(Vertex {
                pos: Pos2::new(
                    center.x + x * scale,
                    center.y - (point[1] + z * 0.22) * scale,
                ),
                uv: Pos2::new(uv[0] / 64., uv[1] / 64.),
                color,
            });
        }
        mesh.indices = vec![0, 1, 2, 0, 2, 3];
        ui.painter().add(egui::Shape::mesh(mesh));
    }
}
