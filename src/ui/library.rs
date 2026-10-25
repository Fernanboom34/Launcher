use super::{
    art::{self, Glyph},
    *,
};
use model::InstanceStyle;
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct LibraryState {
    query: String,
    loader_filter: Option<Loader>,
    favorites_only: bool,
    sort: usize,
    pub(super) create_open: bool,
    cover_seed: u32,
    artwork: Option<PathBuf>,
    cover_target: Option<String>,
    mc_search: String,
    loader_policy: usize,
    show_palette: bool,
    loader_due: Option<Instant>,
    pub(super) pending_style: Option<InstanceStyle>,
    textures: HashMap<PathBuf, Option<egui::TextureHandle>>,
}

impl LibraryState {
    pub(super) fn reveal_created(&mut self) {
        self.query.clear();
        self.loader_filter = None;
        self.favorites_only = false;
        self.sort = 0;
    }
}

impl Launcher {
    pub(super) fn topbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("topbar")
            .exact_height(54.)
            .frame(
                egui::Frame::new()
                    .fill(palette().panel)
                    .inner_margin(egui::Margin::symmetric(16, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (logo, _) = ui.allocate_exact_size(Vec2::splat(29.), Sense::hover());
                    ui.painter().rect_filled(logo, 8., MINT);
                    ui.painter().text(
                        logo.center(),
                        egui::Align2::CENTER_CENTER,
                        "L",
                        FontId::proportional(24.),
                        Color32::from_rgb(18, 30, 24),
                    );
                    ui.label(RichText::new("luma").size(25.).strong());
                    if (self.settings.behavior.sidebar_button
                        || !self.settings.behavior.sidebar_visible)
                        && ui.button("☰").clicked()
                    {
                        self.settings.behavior.sidebar_visible =
                            !self.settings.behavior.sidebar_visible;
                        self.save_settings();
                    }
                    ui.add_space(14.);
                    if art::icon_button(ui, Glyph::Back, "Ir a Inicio", false, true, 30.).clicked()
                    {
                        self.page = Page::Home;
                    }
                    ui.label(
                        RichText::new(match self.page {
                            Page::Home => "Inicio",
                            Page::Profiles => "Biblioteca",
                            Page::Versions => "Versiones de Minecraft",
                            Page::Settings => "Ajustes",
                            Page::Activity => "Actividad",
                            Page::Instance => "Instancia",
                            Page::Screenshots => "Capturas de pantalla",
                        })
                        .size(14.)
                        .strong(),
                    );
                    if !self.settings.appearance.decorations {
                        let rect = Rect::from_min_max(
                            Pos2::new(ui.cursor().min.x, ui.min_rect().top()),
                            Pos2::new(
                                (ui.max_rect().right() - 400.).max(ui.cursor().min.x),
                                ui.min_rect().top() + 36.,
                            ),
                        );
                        if ui
                            .interact(rect, ui.id().with("title_drag"), Sense::drag())
                            .drag_started()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if !self.settings.appearance.decorations {
                            if ui.button("×").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                            if ui.button("□").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(
                                    !ctx.input(|i| i.viewport().maximized.unwrap_or(false)),
                                ));
                            }
                            if ui.button("−").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                            }
                        }
                        if let Some(release) = self.update_release.clone() {
                            if ui
                                .add_enabled(
                                    !self.busy,
                                    primary("Nueva versión disponible · Actualizar"),
                                )
                                .clicked()
                            {
                                self.download_update(release);
                            }
                        }

                        egui::Frame::new()
                            .stroke(Stroke::new(1.0_f32, palette().border))
                            .corner_radius(10.)
                            .inner_margin(egui::Margin::symmetric(12, 2))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (dot, _) =
                                        ui.allocate_exact_size(Vec2::splat(8.), Sense::hover());
                                    ui.painter().circle_filled(
                                        dot.center(),
                                        3.,
                                        if self.game_running {
                                            MINT
                                        } else {
                                            palette().muted
                                        },
                                    );
                                    ui.label(
                                        RichText::new(if self.game_running {
                                            "Una instancia en ejecución"
                                        } else if self.busy {
                                            "Preparando instancia…"
                                        } else {
                                            "Ninguna instancia en ejecución"
                                        })
                                        .size(12.)
                                        .color(palette().muted),
                                    );
                                });
                            });
                        ui.label(
                            RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                                .size(11.)
                                .color(palette().muted),
                        );
                    });
                });
            });
    }

    pub(super) fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("navigation")
            .exact_width(70.)
            .resizable(false)
            .frame(egui::Frame::new().fill(palette().panel).inner_margin(10.))
            .show(ctx, |ui| {
                for (page, icon, label) in [
                    (Page::Home, Glyph::Play, "Inicio"),
                    (Page::Profiles, Glyph::Grid, "Biblioteca de instancias"),
                    (Page::Versions, Glyph::Download, "Versiones de Minecraft"),
                    (Page::Activity, Glyph::Activity, "Actividad y descargas"),
                    (Page::Screenshots, Glyph::Image, "Capturas de pantalla"),
                ] {
                    if art::icon_button(ui, icon, label, self.page == page, true, 48.).clicked() {
                        self.page = page;
                    }
                }
                ui.add_space(4.);
                ui.separator();
                ui.add_space(4.);
                for profile in self
                    .installations
                    .iter()
                    .take(5)
                    .cloned()
                    .collect::<Vec<_>>()
                {
                    let (rect, response) = ui.allocate_exact_size(Vec2::splat(46.), Sense::click());
                    self.paint_cover(ui, rect.shrink(5.), &profile);
                    if self.settings.active_installation == profile.id {
                        ui.painter().rect_stroke(
                            rect.shrink(2.),
                            10.,
                            Stroke::new(1.5_f32, MINT),
                            egui::StrokeKind::Inside,
                        );
                    }
                    if response.clicked() {
                        self.open_instance(&profile);
                    }
                    response.on_hover_text(format!(
                        "{}\n{} {}",
                        profile.name,
                        profile.loader.label(),
                        profile.minecraft
                    ));
                }
                if art::icon_button(ui, Glyph::Plus, "Nueva instancia", false, !self.busy, 48.)
                    .clicked()
                    && !self.busy
                {
                    self.open_create();
                }
                ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                    if art::icon_button(ui, Glyph::User, "Cuenta y jugador", false, true, 46.)
                        .clicked()
                    {
                        self.page = Page::Settings;
                    }
                    if art::icon_button(
                        ui,
                        Glyph::Settings,
                        "Ajustes",
                        self.page == Page::Settings,
                        true,
                        46.,
                    )
                    .clicked()
                    {
                        self.page = Page::Settings;
                    }
                });
            });
    }

    pub(super) fn context_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("instance_context").exact_width(244.).resizable(false).frame(egui::Frame::new().fill(palette().panel).inner_margin(16.)).show(ctx, |ui| {
                self.account_switcher(ui);

            egui::ScrollArea::vertical().id_salt("context_scroll").show(ui, |ui| {
                section(ui,"JUGANDO COMO");
                card().inner_margin(12.).show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (avatar,_) = ui.allocate_exact_size(Vec2::splat(40.),Sense::hover());
                        art::cover(ui.painter(),avatar,Loader::Vanilla,0,None);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(self.settings.profile.as_ref().map(|p|p.name.as_str()).unwrap_or(&self.settings.local_name)).strong().size(14.));
                            ui.label(RichText::new(if self.settings.profile.as_ref().is_some_and(|p|!p.local) {"Cuenta Microsoft"}else{"Jugador local"}).size(11.).color(palette().muted));
                        });
                    });
                });
                ui.collapsing("Cambiar jugador", |ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.settings.local_name).desired_width(ui.available_width()).char_limit(16));
                    if ui.add_enabled(!self.busy,egui::Button::new("Usar nombre")).clicked() {
                        match model::Profile::offline(&self.settings.local_name) {
                            Ok(profile)=>{if !self.settings.accounts.iter().any(|p|p.id==profile.id){self.settings.accounts.push(profile.clone());}self.settings.profile=Some(profile);self.session=None;self.save_settings();},
                            Err(error)=>self.error=Some(error.to_string()),
                        }
                    }
                    if ui.small_button("Opciones de cuenta").clicked() {self.page=Page::Settings;self.workspace.global_tab=2;}
                });

                ui.add_space(14.);ui.separator();ui.add_space(12.);
                section(ui,"INSTANCIA SELECCIONADA");
                if let Some(profile)=self.active_installation().cloned() {
                    ui.horizontal(|ui| {
                        let (cover,_) = ui.allocate_exact_size(Vec2::splat(54.),Sense::hover());
                        self.paint_cover(ui,cover,&profile);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&profile.name).size(14.).strong());
                            ui.label(RichText::new(format!("{} {}",profile.loader.label(),profile.minecraft)).size(11.).color(palette().muted));
                        });
                    });
                    if ui.add_enabled(!self.busy,primary("Jugar").min_size(Vec2::new(ui.available_width(),36.))).clicked() {self.play_instance(&profile);}
                    ui.horizontal(|ui| {
                        if ui.add_enabled(profile.loader!=Loader::Vanilla,egui::Button::new("Mods").min_size(Vec2::new(92.,32.))).clicked() {self.instance_folder(&profile,true);}
                        if ui.add(egui::Button::new("Carpeta").min_size(Vec2::new(92.,32.))).clicked() {self.instance_folder(&profile,false);}
                    });
                    if ui.button("Personalizar portada").clicked() {self.library.cover_target=Some(profile.id.clone());}
                    ui.label(RichText::new(if profile.imported {"Usa la carpeta de juego importada."}else{"Mods y mundos en su propia carpeta."}).size(11.).color(palette().muted));
                } else {
                    card().inner_margin(14.).show(ui,|ui| {
                        ui.label(RichText::new("Tu próxima aventura").strong());
                        ui.label(RichText::new("Elige una tarjeta de la biblioteca para ver sus opciones.").size(12.).color(palette().muted));
                    });
                }
                ui.add_space(14.);ui.separator();ui.add_space(12.);
                section(ui,"TU ESPACIO");
                ui.horizontal(|ui| {ui.label(RichText::new(format!("{} instancias",self.installations.len())).size(12.).color(palette().muted));ui.label(RichText::new(format!("{:.1} GB de RAM",self.settings.memory_mb as f32/1024.)).size(12.).color(palette().muted));});
                ui.add_space(12.);
                card().fill(palette().soft).inner_margin(14.).show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(RichText::new("Un lugar para cada mundo").strong().size(15.));
                    ui.label(RichText::new("Crea otra instancia para probar un cargador o una combinación distinta de mods.").size(12.).color(palette().muted));
                    if ui.add_enabled(!self.busy,egui::Button::new("+ Nueva instancia")).clicked() {self.open_create();}
                });
            });
        });
    }

    pub(super) fn home(&mut self, ui: &mut egui::Ui) {
        self.library_page(ui, true);
    }
    pub(super) fn profiles_page(&mut self, ui: &mut egui::Ui) {
        self.library_page(ui, false);
    }

    fn library_page(&mut self, ui: &mut egui::Ui, show_quick: bool) {
        egui::ScrollArea::vertical()
            .id_salt("library_scroll")
            .show(ui, |ui| {
                if self.busy && self.library.pending_style.is_some() {
                    card().fill(palette().soft).show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(
                                RichText::new(format!(
                                    "Creando {}…",
                                    if self.new_profile_name.trim().is_empty() {
                                        format!(
                                            "{} {}",
                                            self.new_loader.label(),
                                            self.new_minecraft
                                        )
                                    } else {
                                        self.new_profile_name.clone()
                                    }
                                ))
                                .strong(),
                            );
                        });
                        ui.label(RichText::new(&self.status).size(12.).color(palette().muted));
                    });
                }
                if show_quick && !self.installations.is_empty() {
                    ui.label(RichText::new("Salta al juego").size(24.).strong());
                    ui.add_space(4.);
                    let mut quick = self.installations.clone();
                    quick.retain(|p| {
                        !self
                            .settings
                            .instance_styles
                            .get(&p.id)
                            .is_some_and(|s| s.hidden_home)
                    });
                    quick.sort_by_key(|p| {
                        let style = self
                            .settings
                            .instance_styles
                            .get(&p.id)
                            .cloned()
                            .unwrap_or_default();
                        (
                            std::cmp::Reverse(p.id == self.settings.active_installation),
                            std::cmp::Reverse(style.last_played.max(style.created_at)),
                            std::cmp::Reverse(style.favorite),
                        )
                    });
                    let columns = if ui.available_width() > 600. { 2 } else { 1 };
                    let width =
                        (ui.available_width() - (columns - 1) as f32 * 12.) / columns as f32;
                    for row in quick
                        .iter()
                        .take(4)
                        .cloned()
                        .collect::<Vec<_>>()
                        .chunks(columns)
                    {
                        ui.horizontal(|ui| {
                            for profile in row {
                                self.quick_card(ui, profile, width);
                            }
                        });
                    }
                    ui.add_space(16.);
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Biblioteca").size(25.).strong());
                    ui.label(
                        RichText::new(format!("{} instancias", self.installations.len()))
                            .size(12.)
                            .color(palette().muted),
                    );
                });
                ui.horizontal(|ui| {
                    let search_width = (ui.available_width() - 270.).max(160.);
                    ui.add_sized(
                        [search_width, 35.],
                        egui::TextEdit::singleline(&mut self.library.query)
                            .hint_text("Buscar instancias…"),
                    );
                    ui.add_enabled_ui(!self.busy, |ui| {
                        ui.menu_button("Importar", |ui| {
                            if ui.button("Desde carpeta .minecraft…").clicked() {
                                ui.close_menu();
                                if let Some(path) = rfd::FileDialog::new()
                                    .set_title("Carpeta de Minecraft")
                                    .pick_folder()
                                {
                                    self.scan_profiles(Some(path));
                                }
                            }
                            if ui.button("Instalador Forge / NeoForge .jar…").clicked() {
                                ui.close_menu();
                                self.install_from_jar();
                            }
                            if ui.button("Detectar instalaciones").clicked() {
                                ui.close_menu();
                                self.scan_profiles(None);
                            }
                            if ui.button("Carpeta para instaladores").clicked() {
                                ui.close_menu();
                                self.prepare_installer_folder();
                            }
                        });
                    });
                    if ui
                        .add_enabled(
                            !self.busy,
                            primary("+ Nueva instancia").min_size(Vec2::new(152., 35.)),
                        )
                        .clicked()
                    {
                        self.open_create();
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    egui::ComboBox::from_id_salt("library_sort")
                        .width(165.)
                        .selected_text(
                            ["Actividad reciente", "Nombre", "Minecraft"][self.library.sort],
                        )
                        .show_ui(ui, |ui| {
                            for (i, label) in ["Actividad reciente", "Nombre", "Minecraft"]
                                .iter()
                                .enumerate()
                            {
                                ui.selectable_value(&mut self.library.sort, i, *label);
                            }
                        });
                    egui::ComboBox::from_id_salt("library_loader_filter")
                        .width(130.)
                        .selected_text(
                            self.library
                                .loader_filter
                                .map(Loader::label)
                                .unwrap_or("Todos los cargadores"),
                        )
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.library.loader_filter,
                                None,
                                "Todos los cargadores",
                            );
                            for loader in Loader::ALL {
                                ui.selectable_value(
                                    &mut self.library.loader_filter,
                                    Some(loader),
                                    loader.label(),
                                );
                            }
                        });
                    if ui
                        .add(egui::Button::new("Favoritos").selected(self.library.favorites_only))
                        .clicked()
                    {
                        self.library.favorites_only = !self.library.favorites_only;
                    }
                    if (!self.library.query.is_empty()
                        || self.library.loader_filter.is_some()
                        || self.library.favorites_only)
                        && ui.small_button("Limpiar").clicked()
                    {
                        self.library.query.clear();
                        self.library.loader_filter = None;
                        self.library.favorites_only = false;
                    }
                });
                ui.add_space(14.);
                let query = self.library.query.to_lowercase();
                let mut matches: Vec<_> = self
                    .installations
                    .iter()
                    .filter(|p| {
                        let style = self
                            .settings
                            .instance_styles
                            .get(&p.id)
                            .cloned()
                            .unwrap_or_default();
                        (self.library.loader_filter.is_none()
                            || self.library.loader_filter == Some(p.loader))
                            && (!self.library.favorites_only || style.favorite)
                            && format!("{} {} {}", p.name, p.minecraft, p.loader.label())
                                .to_lowercase()
                                .contains(&query)
                    })
                    .cloned()
                    .collect();
                matches.sort_by(|a, b| {
                    let sa = self
                        .settings
                        .instance_styles
                        .get(&a.id)
                        .cloned()
                        .unwrap_or_default();
                    let sb = self
                        .settings
                        .instance_styles
                        .get(&b.id)
                        .cloned()
                        .unwrap_or_default();
                    match self.library.sort {
                        1 => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                        2 => version_numbers(&b.minecraft)
                            .cmp(&version_numbers(&a.minecraft))
                            .then_with(|| a.name.cmp(&b.name)),
                        _ => sb
                            .last_played
                            .max(sb.created_at)
                            .cmp(&sa.last_played.max(sa.created_at))
                            .then_with(|| sb.favorite.cmp(&sa.favorite))
                            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
                    }
                });
                if matches.is_empty() {
                    card().inner_margin(28.).show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.label(
                            RichText::new(if self.installations.is_empty() {
                                "Tu biblioteca empieza aquí"
                            } else {
                                "No encontramos esa instancia"
                            })
                            .size(20.)
                            .strong(),
                        );
                        ui.label(
                            RichText::new(if self.installations.is_empty() {
                                "Crea una instancia Vanilla o con tu cargador de mods favorito."
                            } else {
                                "Prueba con otro nombre o cambia los filtros."
                            })
                            .color(palette().muted),
                        );
                        if self.installations.is_empty()
                            && ui
                                .add_enabled(!self.busy, primary("+ Crear mi primera instancia"))
                                .clicked()
                        {
                            self.open_create();
                        }
                    });
                }
                let cols = ((ui.available_width() + 14.) / 190.).floor().clamp(2., 7.) as usize;
                let width =
                    ((ui.available_width() - 14. * (cols - 1) as f32) / cols as f32).min(190.);
                if self.settings.behavior.compact {
                    for profile in &matches {
                        self.compact_instance(ui, profile);
                        ui.add_space(4.);
                    }
                }
                for row in matches.chunks(if self.settings.behavior.compact {
                    matches.len().max(1)
                } else {
                    cols
                }) {
                    if self.settings.behavior.compact {
                        break;
                    }
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 14.;
                        for profile in row {
                            self.instance_card(ui, profile, width);
                        }
                    });
                    ui.add_space(4.);
                }
            });
    }

    fn compact_instance(&mut self, ui: &mut egui::Ui, p: &GameProfile) {
        let response = card()
            .inner_margin(10.)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(44.), Sense::hover());
                    self.paint_cover(ui, r, p);
                    if ui.selectable_label(false, &p.name).clicked() {
                        self.open_instance(p);
                    }
                    ui.label(
                        RichText::new(format!("{} {}", p.loader.label(), p.minecraft))
                            .color(palette().muted),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.add_enabled(!self.busy, primary("Jugar")).clicked() {
                            self.play_instance(p);
                        }
                        ui.menu_button("···", |ui| self.instance_menu(ui, p));
                    });
                });
            })
            .response;
        response.context_menu(|ui| self.instance_menu(ui, p));
    }
    fn quick_card(&mut self, ui: &mut egui::Ui, profile: &GameProfile, width: f32) {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 80.), Sense::click());
        ui.painter().rect_filled(
            rect,
            14.,
            if response.hovered() {
                palette().hover
            } else {
                palette().card
            },
        );
        ui.painter().rect_stroke(
            rect,
            14.,
            Stroke::new(1.0_f32, palette().border),
            egui::StrokeKind::Inside,
        );
        self.paint_cover(
            ui,
            Rect::from_min_size(rect.min + Vec2::splat(12.), Vec2::splat(56.)),
            profile,
        );
        let text_left = rect.left() + 80.;
        let recent = self.recent_world(profile);
        art::text_fit(
            ui.painter(),
            Rect::from_min_max(
                Pos2::new(text_left, rect.top() + 17.),
                Pos2::new(rect.right() - 104., rect.top() + 38.),
            ),
            &recent
                .as_ref()
                .map(|w| w.name.clone())
                .unwrap_or_else(|| profile.name.clone()),
            15.,
            palette().text,
        );
        let style = self
            .settings
            .instance_styles
            .get(&profile.id)
            .cloned()
            .unwrap_or_default();
        art::text_fit(
            ui.painter(),
            Rect::from_min_max(
                Pos2::new(text_left, rect.top() + 43.),
                Pos2::new(rect.right() - 102., rect.bottom() - 12.),
            ),
            &format!(
                "{} {} · {}",
                profile.loader.label(),
                profile.minecraft,
                played_ago(style.last_played)
            ),
            12.,
            palette().muted,
        );
        let play = Rect::from_min_size(
            Pos2::new(rect.right() - 94., rect.center().y - 17.),
            Vec2::new(80., 34.),
        );
        let clicked = self.play_control(ui, play, &profile.id, true);
        if response.clicked() && !clicked {
            self.open_instance(profile);
        }
        if clicked {
            self.play_recent(profile);
        }
        response.context_menu(|ui| self.instance_menu(ui, profile));
    }

    fn instance_card(&mut self, ui: &mut egui::Ui, profile: &GameProfile, width: f32) {
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, width + 70.), Sense::click());
        let selected = self.settings.active_installation == profile.id;
        ui.painter().rect_filled(
            rect,
            15.,
            if response.hovered() {
                palette().hover
            } else {
                palette().card
            },
        );
        ui.painter().rect_stroke(
            rect,
            15.,
            Stroke::new(
                if selected { 1.5_f32 } else { 1.0_f32 },
                if selected {
                    MINT.gamma_multiply(0.7)
                } else {
                    palette().border
                },
            ),
            egui::StrokeKind::Inside,
        );
        let image = Rect::from_min_size(rect.min + Vec2::splat(12.), Vec2::splat(width - 24.));
        self.paint_cover(ui, image, profile);
        let style = self
            .settings
            .instance_styles
            .get(&profile.id)
            .cloned()
            .unwrap_or_default();
        if style.favorite {
            let star = Rect::from_min_size(image.min + Vec2::splat(6.), Vec2::splat(25.));
            ui.painter()
                .rect_filled(star, 7., Color32::from_black_alpha(115));
            art::glyph(
                ui.painter(),
                star.shrink(3.),
                Glyph::Star,
                Color32::from_rgb(255, 220, 120),
            );
        }
        let title_y = image.bottom() + 12.;
        art::text_fit(
            ui.painter(),
            Rect::from_min_max(
                Pos2::new(rect.left() + 13., title_y),
                Pos2::new(rect.right() - 36., title_y + 22.),
            ),
            &profile.name,
            15.,
            palette().text,
        );
        art::text_fit(
            ui.painter(),
            Rect::from_min_max(
                Pos2::new(rect.left() + 13., title_y + 28.),
                Pos2::new(rect.right() - 52., title_y + 48.),
            ),
            &format!("{} {}", profile.loader.label(), profile.minecraft),
            12.,
            palette().muted,
        );
        let play = Rect::from_min_size(
            Pos2::new(rect.right() - 44., rect.bottom() - 40.),
            Vec2::new(30., 27.),
        );
        let clicked = self.play_control(ui, play, &profile.id, false);
        let menu = Rect::from_min_size(
            Pos2::new(rect.right() - 31., title_y - 3.),
            Vec2::new(25., 25.),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(menu), |ui| {
            ui.spacing_mut().button_padding = Vec2::new(4., 2.);
            ui.menu_button("…", |ui| self.instance_menu(ui, profile));
        });
        if response.clicked() && !clicked {
            self.open_instance(profile);
        }
        if response.double_clicked() || clicked {
            self.play_instance(profile);
        }
        response
            .clone()
            .context_menu(|ui| self.instance_menu(ui, profile));
        response.on_hover_text(format!(
            "{}\n{} {}\nClic para ver opciones · Doble clic para jugar",
            profile.name,
            profile.loader.label(),
            profile.minecraft
        ));
    }

    fn play_control(&self, ui: &mut egui::Ui, rect: Rect, id: &str, with_label: bool) -> bool {
        let enabled = !self.busy;
        let response = ui.interact(
            rect,
            ui.id().with(("play", id, with_label)),
            if enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        let running = self.running_installation.as_deref() == Some(id);
        ui.painter().rect_filled(
            rect,
            8.,
            if !enabled {
                Color32::from_rgb(49, 70, 57)
            } else if response.hovered() {
                Color32::from_rgb(99, 237, 161)
            } else {
                MINT
            },
        );
        let icon = if with_label {
            Rect::from_min_size(rect.min + Vec2::new(9., 7.), Vec2::splat(19.))
        } else {
            rect.shrink(5.)
        };
        art::glyph(
            ui.painter(),
            icon,
            if running {
                Glyph::Activity
            } else {
                Glyph::Play
            },
            Color32::from_rgb(18, 30, 24),
        );
        if with_label {
            ui.painter().text(
                rect.center() + Vec2::new(11., 0.),
                egui::Align2::CENTER_CENTER,
                if running { "Jugando" } else { "Jugar" },
                FontId::proportional(13.),
                Color32::from_rgb(18, 30, 24),
            );
        }
        response
            .on_hover_text(if running {
                "Esta instancia está en ejecución"
            } else {
                "Jugar"
            })
            .clicked()
            && enabled
    }

    pub(super) fn instance_menu(&mut self, ui: &mut egui::Ui, profile: &GameProfile) {
        if ui.button("Ver instancia").clicked() {
            ui.close_menu();
            self.open_instance(profile);
        }
        if ui.button("Añadir contenido").clicked() {
            ui.close_menu();
            self.open_instance(profile);
            self.workspace.import_open = true;
        }
        if ui.button("Configuración").clicked() {
            ui.close_menu();
            self.open_instance(profile);
            self.workspace.tab = 4;
        }
        if ui.button("Copiar ruta").clicked() {
            ui.ctx().copy_text(profile.game_dir.display().to_string());
            ui.close_menu();
        }
        if ui
            .add_enabled(!self.busy, egui::Button::new("Duplicar instancia"))
            .clicked()
        {
            let root = self.root.clone();
            let p = profile.clone();
            self.work(move |tx| {
                tx.send(Event::Profiles(profiles::duplicate(&root, &p)?))?;
                Ok(())
            });
            ui.close_menu();
        }
        if ui.button("Ocultar de Inicio").clicked() {
            self.settings
                .instance_styles
                .entry(profile.id.clone())
                .or_default()
                .hidden_home = true;
            self.save_settings();
            ui.close_menu();
        }
        if ui.button("Crear acceso directo").clicked() {
            if let Ok(exe) = std::env::current_exe() {
                if let Err(e) = crate::distribution::shortcut(
                    &exe,
                    &profile.name,
                    &format!("--play-profile {}", profile.id),
                ) {
                    self.error = Some(e.to_string());
                }
            }
            ui.close_menu();
        }
        if ui
            .add_enabled(!self.busy, egui::Button::new("Eliminar de la biblioteca"))
            .clicked()
        {
            self.workspace.remove_files = false;
            self.workspace.remove = Some(profile.id.clone());
            ui.close_menu();
        }
        if ui
            .add_enabled(!self.busy, egui::Button::new("Eliminar completamente…"))
            .clicked()
        {
            self.workspace.remove_files = true;
            self.workspace.remove = Some(profile.id.clone());
            ui.close_menu();
        }
        ui.separator();
        if ui
            .add_enabled(!self.busy, egui::Button::new("Jugar"))
            .clicked()
        {
            ui.close_menu();
            self.play_instance(profile);
        }
        if ui.button("Abrir carpeta de la instancia").clicked() {
            ui.close_menu();
            self.instance_folder(profile, false);
        }
        if profile.loader != Loader::Vanilla && ui.button("Abrir carpeta de mods").clicked() {
            ui.close_menu();
            self.instance_folder(profile, true);
        }
        ui.separator();
        let favorite = self
            .settings
            .instance_styles
            .get(&profile.id)
            .is_some_and(|s| s.favorite);
        if ui
            .button(if favorite {
                "Quitar de favoritos"
            } else {
                "Añadir a favoritos"
            })
            .clicked()
        {
            ui.close_menu();
            self.settings
                .instance_styles
                .entry(profile.id.clone())
                .or_default()
                .favorite = !favorite;
            self.save_settings();
        }
        if ui.button("Personalizar portada").clicked() {
            ui.close_menu();
            self.library.cover_target = Some(profile.id.clone());
        }
        if ui
            .add_enabled(!self.busy, egui::Button::new("Renombrar"))
            .clicked()
        {
            ui.close_menu();
            self.rename_profile = Some((profile.id.clone(), profile.name.clone()));
        }
    }

    pub(super) fn select_instance(&mut self, profile: &GameProfile) {
        self.settings.active_installation = profile.id.clone();
        self.settings.selected = profile.version_id.clone();
        self.save_settings();
    }
    pub(super) fn play_instance(&mut self, profile: &GameProfile) {
        if !self.busy {
            self.select_instance(profile);
            self.run(true);
        }
    }
    pub(super) fn instance_folder(&mut self, profile: &GameProfile, mods: bool) {
        let path = if mods {
            profile.game_dir.join("mods")
        } else {
            profile.game_dir.clone()
        };
        if let Err(error) = std::fs::create_dir_all(&path) {
            self.error = Some(error.to_string());
        } else {
            self.open_path(path);
        }
    }

    pub(super) fn paint_cover(&mut self, ui: &egui::Ui, rect: Rect, profile: &GameProfile) {
        let style = self
            .settings
            .instance_styles
            .get(&profile.id)
            .cloned()
            .unwrap_or_default();
        let texture = style
            .artwork
            .as_ref()
            .and_then(|path| self.cover_texture(ui.ctx(), path));
        art::cover(
            ui.painter(),
            rect,
            profile.loader,
            style.cover_seed,
            texture,
        );
    }

    pub(super) fn cover_texture(
        &mut self,
        ctx: &egui::Context,
        path: &std::path::Path,
    ) -> Option<egui::TextureId> {
        if !self.library.textures.contains_key(path) {
            let texture = (|| -> anyhow::Result<egui::TextureHandle> {
                let image = read_artwork(path)?;
                let rgba = image.to_rgba8();
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    rgba.as_raw(),
                );
                Ok(ctx.load_texture(path.to_string_lossy(), image, egui::TextureOptions::LINEAR))
            })()
            .ok();
            self.library.textures.insert(path.into(), texture);
        }
        self.library
            .textures
            .get(path)
            .and_then(|t| t.as_ref())
            .map(egui::TextureHandle::id)
    }
}

fn section(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(10.)
            .strong()
            .color(palette().muted),
    );
}
fn version_numbers(version: &str) -> Vec<u32> {
    version
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|v| v.parse().ok())
        .collect()
}
fn played_ago(timestamp: u64) -> String {
    if timestamp == 0 {
        return "Sin actividad".into();
    }
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_sub(timestamp);
    match elapsed {
        0..=59 => "Ahora".into(),
        60..=3599 => format!("Hace {} min", elapsed / 60),
        3600..=86399 => format!("Hace {} h", elapsed / 3600),
        _ => format!("Hace {} días", elapsed / 86400),
    }
}

fn read_artwork(path: &std::path::Path) -> anyhow::Result<image::DynamicImage> {
    let mut reader = image::ImageReader::open(path)?.with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    reader.limits(limits);
    Ok(reader
        .decode()?
        .resize_to_fill(512, 512, image::imageops::FilterType::Triangle))
}

impl Launcher {
    pub(super) fn open_create(&mut self) {
        self.new_profile_name.clear();
        self.new_minecraft = self
            .active_installation()
            .map(|p| p.minecraft.clone())
            .or_else(|| self.selected().map(|v| v.id.clone()))
            .unwrap_or_default();
        self.new_loader = Loader::Vanilla;
        self.new_loader_version.clear();
        self.loader_versions.clear();
        self.loader_query = None;
        self.library.loader_policy = 0;
        self.library.loader_due = None;
        self.library.create_open = true;
        self.library.cover_seed = 0;
        self.library.artwork = None;
        self.library.mc_search.clear();
        self.library.show_palette = false;
        self.library.cover_target = None;
        self.rename_profile = None;
        self.error = None;
    }

    pub(super) fn drive_loader_query(&mut self, ctx: &egui::Context) {
        if !self.library.create_open
            || self.new_loader == Loader::Vanilla
            || self.busy
            || std::env::var_os("LUMA_PREVIEW").is_some()
        {
            return;
        }
        if let Some(deadline) = self.library.loader_due {
            ctx.request_repaint_after(deadline.saturating_duration_since(Instant::now()));
        }
        if self
            .library
            .loader_due
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.library.loader_due = None;
            let loader = self.new_loader;
            let minecraft = self.new_minecraft.clone();
            if minecraft.is_empty() {
                return;
            }
            self.status = format!("Buscando {} para Minecraft {}…", loader.label(), minecraft);
            self.work(move |tx| {
                let versions = profiles::loader_versions(loader, &minecraft)?;
                tx.send(Event::LoaderVersions(loader, minecraft, versions))?;
                Ok(())
            });
        }
    }

    pub(super) fn instance_windows(&mut self, ctx: &egui::Context) {
        if self.library.create_open {
            self.create_window(ctx);
        }
        if let Some(id) = self.library.cover_target.clone() {
            let Some(profile) = self.installations.iter().find(|p| p.id == id).cloned() else {
                self.library.cover_target = None;
                return;
            };
            let mut close = false;
            let response = egui::Modal::new(egui::Id::new("customize_cover"))
                .frame(modal_frame())
                .show(ctx, |ui| {
                    ui.set_width(410.);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Personalizar portada").size(21.).strong());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if art::icon_button(ui, Glyph::Close, "Cerrar", false, true, 28.)
                                .clicked()
                            {
                                close = true;
                            }
                        });
                    });
                    ui.label(
                        RichText::new(&profile.name)
                            .size(12.)
                            .color(palette().muted),
                    );
                    ui.add_space(10.);
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(132.), Sense::hover());
                        self.paint_cover(ui, rect, &profile);
                        ui.vertical(|ui| {
                            if ui.button("Subir PNG").clicked() {
                                match self.choose_artwork() {
                                    Ok(Some(path)) => {
                                        self.settings
                                            .instance_styles
                                            .entry(id.clone())
                                            .or_default()
                                            .artwork = Some(path);
                                        self.save_settings();
                                    }
                                    Ok(None) => (),
                                    Err(error) => self.error = Some(error.to_string()),
                                }
                            }
                            if ui.button("Generar otra").clicked() {
                                let style =
                                    self.settings.instance_styles.entry(id.clone()).or_default();
                                style.artwork = None;
                                style.cover_seed = style.cover_seed.wrapping_add(1);
                                self.save_settings();
                            }
                            if ui.button("Restablecer").clicked() {
                                let style =
                                    self.settings.instance_styles.entry(id.clone()).or_default();
                                style.artwork = None;
                                style.cover_seed = 0;
                                self.save_settings();
                            }
                        });
                    });
                    ui.add_space(8.);
                    ui.label(RichText::new("Color de fondo").size(13.).strong());
                    let seed = self
                        .settings
                        .instance_styles
                        .get(&id)
                        .map(|s| s.cover_seed)
                        .unwrap_or(0);
                    if let Some(seed) = color_choices(ui, profile.loader, seed) {
                        let style = self.settings.instance_styles.entry(id.clone()).or_default();
                        style.cover_seed = seed;
                        style.artwork = None;
                        self.save_settings();
                    }
                    ui.add_space(10.);
                    if ui
                        .add_sized([ui.available_width(), 34.], primary("Listo"))
                        .clicked()
                    {
                        close = true;
                    }
                });
            if response.should_close() || close {
                self.library.cover_target = None;
            }
        }
    }

    fn create_window(&mut self, ctx: &egui::Context) {
        let mut close = false;
        let mut create = false;
        let response = egui::Modal::new(egui::Id::new("create_instance"))
            .frame(modal_frame())
            .show(ctx, |ui| {
                ui.set_width(500.);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Crear instancia").size(24.).strong());
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if art::icon_button(ui, Glyph::Close, "Cerrar", false, true, 29.).clicked()
                        {
                            close = true;
                        }
                    });
                });
                ui.add_space(7.);
                ui.separator();
                ui.add_space(12.);
                egui::ScrollArea::vertical()
                    .id_salt("create_form")
                    .max_height((ctx.screen_rect().height() - 250.).clamp(280., 610.))
                    .show(ui, |ui| {
                        let compact = ctx.screen_rect().height() < 780.;
                        if compact {
                            ui.spacing_mut().item_spacing.y = 7.;
                        }
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(
                                Vec2::splat(if compact { 88. } else { 112. }),
                                Sense::hover(),
                            );
                            let texture = self
                                .library
                                .artwork
                                .clone()
                                .as_ref()
                                .and_then(|path| self.cover_texture(ctx, path));
                            art::cover(
                                ui.painter(),
                                rect,
                                self.new_loader,
                                self.library.cover_seed,
                                texture,
                            );
                            ui.add_space(3.);
                            ui.vertical(|ui| {
                                if ui
                                    .add_enabled(!self.busy, egui::Button::new("Subir PNG"))
                                    .clicked()
                                {
                                    match self.choose_artwork() {
                                        Ok(path) => {
                                            if path.is_some() {
                                                self.library.artwork = path;
                                            }
                                        }
                                        Err(error) => self.error = Some(error.to_string()),
                                    }
                                }
                                if ui
                                    .add_enabled(!self.busy, egui::Button::new("Generar otra"))
                                    .clicked()
                                {
                                    self.library.cover_seed =
                                        self.library.cover_seed.wrapping_add(1);
                                    self.library.artwork = None;
                                }
                                if ui.button("Personalizar color").clicked() {
                                    self.library.show_palette = !self.library.show_palette;
                                }
                            });
                        });
                        if self.library.show_palette {
                            if let Some(seed) =
                                color_choices(ui, self.new_loader, self.library.cover_seed)
                            {
                                self.library.cover_seed = seed;
                                self.library.artwork = None;
                            }
                        }
                        ui.add_space(14.);
                        field_label(ui, "Nombre");
                        let suggested =
                            format!("{} {}", self.new_loader.label(), self.new_minecraft);
                        ui.add_enabled(
                            !self.busy,
                            egui::TextEdit::singleline(&mut self.new_profile_name)
                                .hint_text(suggested)
                                .desired_width(ui.available_width())
                                .char_limit(80)
                                .margin(Vec2::new(12., 9.)),
                        );
                        ui.add_space(10.);
                        field_label(ui, "Cargador");
                        let before = (self.new_loader, self.new_minecraft.clone());
                        ui.add_enabled_ui(!self.busy, |ui| {
                            ui.horizontal(|ui| {
                                for loader in Loader::ALL {
                                    let selected = self.new_loader == loader;
                                    let button = egui::Button::new(
                                        RichText::new(loader.label()).color(if selected {
                                            MINT
                                        } else {
                                            palette().muted
                                        }),
                                    )
                                    .fill(if selected {
                                        Color32::from_rgb(39, 78, 56)
                                    } else {
                                        palette().hover
                                    })
                                    .stroke(Stroke::new(
                                        1.0_f32,
                                        if selected { MINT } else { palette().border },
                                    ))
                                    .min_size(Vec2::new(112., 36.));
                                    if ui.add(button).clicked() {
                                        self.new_loader = loader;
                                    }
                                }
                            });
                            ui.add_space(10.);
                            field_label(ui, "Versión de Minecraft");
                            egui::ComboBox::from_id_salt("create_minecraft")
                                .width(ui.available_width())
                                .height(270.)
                                .selected_text(&self.new_minecraft)
                                .show_ui(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.library.mc_search)
                                            .hint_text("Buscar versión…")
                                            .desired_width(ui.available_width()),
                                    );
                                    ui.separator();
                                    let query = self.library.mc_search.to_lowercase();
                                    for version in &self.manifest.versions {
                                        if version.id.to_lowercase().contains(&query) {
                                            ui.selectable_value(
                                                &mut self.new_minecraft,
                                                version.id.clone(),
                                                &version.id,
                                            );
                                        }
                                    }
                                });
                        });
                        if before != (self.new_loader, self.new_minecraft.clone()) {
                            self.loader_versions.clear();
                            self.new_loader_version.clear();
                            self.loader_query = None;
                            self.error = None;
                            self.library.loader_due = (self.new_loader != Loader::Vanilla)
                                .then(|| Instant::now() + Duration::from_millis(350));
                        }
                        if self.new_loader != Loader::Vanilla {
                            ui.add_space(10.);
                            field_label(ui, "Versión del cargador");
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_value(
                                        &mut self.library.loader_policy,
                                        0,
                                        "Última compatible",
                                    )
                                    .clicked()
                                {
                                    if let Some(version) = self.loader_versions.first() {
                                        self.new_loader_version = version.clone();
                                    }
                                }
                                ui.selectable_value(
                                    &mut self.library.loader_policy,
                                    1,
                                    "Elegir versión",
                                );
                            });
                            let previous_version = self.new_loader_version.clone();
                            ui.add_enabled_ui(
                                !self.busy && !self.loader_versions.is_empty(),
                                |ui| {
                                    egui::ComboBox::from_id_salt("create_loader_version")
                                        .width(ui.available_width())
                                        .height(240.)
                                        .selected_text(if self.new_loader_version.is_empty() {
                                            "Consultando versiones…"
                                        } else {
                                            &self.new_loader_version
                                        })
                                        .show_ui(ui, |ui| {
                                            for version in &self.loader_versions {
                                                ui.selectable_value(
                                                    &mut self.new_loader_version,
                                                    version.clone(),
                                                    version,
                                                );
                                            }
                                        });
                                },
                            );
                            if previous_version != self.new_loader_version {
                                self.library.loader_policy = 1;
                            }
                            if self.loader_versions.is_empty()
                                && !self.busy
                                && self.library.loader_due.is_none()
                                && ui.small_button("Volver a consultar").clicked()
                            {
                                self.library.loader_due = Some(Instant::now());
                            }
                        }
                        ui.add_space(12.);
                        ui.label(
                            RichText::new("Mods, configuración y mundos en una carpeta propia.")
                                .size(12.)
                                .color(palette().muted),
                        );
                        if let Some(error) = &self.error {
                            ui.label(
                                RichText::new(error)
                                    .size(12.)
                                    .color(Color32::from_rgb(255, 167, 167)),
                            );
                        }
                        ui.add_space(5.);
                    });
                ui.add_space(12.);
                ui.separator();
                ui.add_space(7.);
                let compatible = self.new_loader == Loader::Vanilla
                    || self.loader_query.as_ref().is_some_and(|(loader, mc)| {
                        *loader == self.new_loader && mc == &self.new_minecraft
                    }) && !self.new_loader_version.is_empty();
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            !self.busy && !self.new_minecraft.is_empty() && compatible,
                            primary("+ Crear instancia").min_size(Vec2::new(158., 37.)),
                        )
                        .clicked()
                    {
                        create = true;
                    }
                    if ui.button("Cancelar").clicked() {
                        close = true;
                    }
                    if self.busy {
                        ui.spinner();
                    }
                });
            });
        if let Some(deadline) = self.library.loader_due {
            ctx.request_repaint_after(deadline.saturating_duration_since(Instant::now()));
        }
        if close || response.should_close() {
            self.library.create_open = false;
        }
        if create {
            let name = if self.new_profile_name.trim().is_empty() {
                format!("{} {}", self.new_loader.label(), self.new_minecraft)
            } else {
                self.new_profile_name.clone()
            };
            let request = ProfileRequest {
                name,
                minecraft: self.new_minecraft.clone(),
                loader: self.new_loader,
                loader_version: self.new_loader_version.clone(),
            };
            self.library.pending_style = Some(InstanceStyle {
                cover_seed: self.library.cover_seed,
                artwork: self.library.artwork.clone(),
                ..Default::default()
            });
            self.library.create_open = false;
            self.page = Page::Profiles;
            let root = self.root.clone();
            let settings = self.settings.clone();
            self.work(move |tx| {
                profiles::create(&root, &request, &settings, &tx)?;
                Ok(())
            });
        }
    }

    fn choose_artwork(&self) -> anyhow::Result<Option<PathBuf>> {
        use sha1::Digest;
        let Some(path) = rfd::FileDialog::new()
            .set_title("Portada de la instancia")
            .add_filter("Imagen PNG", &["png"])
            .pick_file()
        else {
            return Ok(None);
        };
        let image = read_artwork(&path)?;
        let hash = format!("{:x}", sha1::Sha1::digest(image.to_rgba8().as_raw()));
        let directory = self.root.join("artwork");
        std::fs::create_dir_all(&directory)?;
        let target = directory.join(format!("{hash}.png"));
        if !target.exists() {
            image.save(&target)?;
        }
        Ok(Some(target))
    }

    fn install_from_jar(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Instalador Forge / NeoForge", &["jar"])
            .pick_file()
        {
            let root = self.root.clone();
            let settings = self.settings.clone();

            let name = path
                .file_stem()
                .map(|v| v.to_string_lossy().to_string())
                .unwrap_or_else(|| "Instancia importada".into());
            self.library.pending_style = None;
            self.work(move |tx| {
                profiles::install_local_jar(&root, &path, &name, &settings, &tx)?;
                Ok(())
            });
        }
    }

    fn prepare_installer_folder(&mut self) {
        let result = (|| -> anyhow::Result<()> {
            std::fs::create_dir_all(self.root.join("versions"))?;
            let path = self.root.join("launcher_profiles.json");
            if !path.exists() {
                model::write_json(&path, &serde_json::json!({"profiles":{}}))?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.error = Some(error.to_string());
        } else {
            self.open_path(self.root.clone());
        }
    }
}

fn modal_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(palette().card)
        .stroke(Stroke::new(1.0_f32, palette().border))
        .corner_radius(18.)
        .inner_margin(24.)
}
fn field_label(ui: &mut egui::Ui, label: &str) {
    ui.label(RichText::new(label).size(14.).strong());
}
fn color_choices(ui: &mut egui::Ui, loader: Loader, selected: u32) -> Option<u32> {
    let mut result = None;
    ui.horizontal(|ui| {
        for seed in 0..=6 {
            let (rect, response) = ui.allocate_exact_size(Vec2::splat(26.), Sense::click());
            ui.painter()
                .circle_filled(rect.center(), 10., art::loader_color(loader, seed));
            if selected == seed {
                ui.painter().circle_stroke(
                    rect.center(),
                    12.,
                    Stroke::new(1.5_f32, palette().text),
                );
            }
            if response.clicked() {
                result = Some(seed);
            }
        }
    });
    result
}
