use super::*;
#[derive(Clone, Copy)]
pub(super) struct Palette {
    pub bg: Color32,
    pub panel: Color32,
    pub card: Color32,
    pub hover: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub soft: Color32,
}
fn rgb(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}
fn colors(theme: usize) -> Palette {
    let c = match theme {
        1 => [
            0xf3f4f6, 0xffffff, 0xe6e9ed, 0xd7dce2, 0xc4cbd4, 0x20252c, 0x566170,
        ],
        2 => [
            0x000000, 0x080a0c, 0x111416, 0x1e2428, 0x30383f, 0xf2f4f6, 0x9aa4b2,
        ],
        _ => [
            0x16181c, 0x202328, 0x272a30, 0x31353c, 0x393d45, 0xf2f4f6, 0x9aa4b2,
        ],
    };
    Palette {
        bg: rgb(c[0]),
        panel: rgb(c[1]),
        card: rgb(c[2]),
        hover: rgb(c[3]),
        border: rgb(c[4]),
        text: rgb(c[5]),
        muted: rgb(c[6]),
        soft: if theme == 1 {
            rgb(0xe0f2e7)
        } else {
            rgb(0x20352b)
        },
    }
}
thread_local! {static COLORS: std::cell::Cell<Palette> = std::cell::Cell::new(colors(0));}
pub(super) fn palette() -> Palette {
    COLORS.with(|c| c.get())
}
impl Launcher {
    pub(super) fn apply_appearance(&mut self, ctx: &egui::Context) {
        let theme = if self.settings.appearance.theme == 3 {
            if ctx.input(|i| i.raw.system_theme) == Some(egui::Theme::Light) {
                1
            } else {
                0
            }
        } else {
            self.settings.appearance.theme
        };
        COLORS.with(|c| c.set(colors(theme)));
        let p = palette();
        let mut visuals = if theme == 1 {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };
        visuals.panel_fill = p.bg;
        visuals.window_fill = p.panel;
        visuals.extreme_bg_color = p.bg;
        visuals.override_text_color = Some(p.text);
        visuals.widgets.inactive.bg_fill = p.card;
        visuals.widgets.inactive.weak_bg_fill = p.card;
        visuals.widgets.hovered.bg_fill = p.hover;
        visuals.selection.bg_fill = if theme == 1 {
            rgb(0xc5e8d2)
        } else {
            rgb(0x24533a)
        };
        visuals.selection.stroke = Stroke::new(1.0_f32, MINT);
        if !self.settings.appearance.advanced {
            visuals.window_shadow = egui::epaint::Shadow::NONE;
            visuals.popup_shadow = egui::epaint::Shadow::NONE;
        }
        visuals.extreme_bg_color = p.card;
        for widget in [
            &mut visuals.widgets.inactive,
            &mut visuals.widgets.hovered,
            &mut visuals.widgets.active,
            &mut visuals.widgets.noninteractive,
            &mut visuals.widgets.open,
        ] {
            widget.corner_radius = CornerRadius::same(8);
        }
        ctx.set_visuals(visuals);
        ctx.style_mut(|s| {
            s.animation_time = if self.settings.appearance.advanced {
                0.12
            } else {
                0.0
            }
        });
        if self.workspace.decorations != Some(self.settings.appearance.decorations) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(
                self.settings.appearance.decorations,
            ));
            self.workspace.decorations = Some(self.settings.appearance.decorations);
        }
    }
}
