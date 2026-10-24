use crate::{
    auth, engine,
    model::{self, Event, Manifest, Session, Settings, Version},
};
use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Layout, Pos2, Rect, RichText, Sense, Stroke, Vec2,
};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    time::Duration,
};

const BG: Color32 = Color32::from_rgb(13, 17, 24);
const PANEL: Color32 = Color32::from_rgb(21, 27, 37);
const CARD: Color32 = Color32::from_rgb(27, 35, 47);
const TEXT: Color32 = Color32::from_rgb(233, 239, 245);
const MUTED: Color32 = Color32::from_rgb(139, 154, 174);
const MINT: Color32 = Color32::from_rgb(129, 232, 181);

#[derive(PartialEq)]
enum Page {
    Home,
    Versions,
    Settings,
    Activity,
}

pub struct Launcher {
    root: PathBuf,
    settings: Settings,
    manifest: Manifest,
    session: Option<Session>,
    installed: HashSet<String>,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    page: Page,
    busy: bool,
    status: String,
    progress: Option<(usize, usize)>,
    error: Option<String>,
    auth_url: Option<String>,
    auth_cancel: Option<Arc<AtomicBool>>,
    activity: Vec<String>,
    search: String,
    category: usize,
    game_running: bool,
    cache: bool,
    official: Vec<model::Profile>,
}

impl Launcher {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = BG;
        style.visuals.window_fill = PANEL;
        style.visuals.extreme_bg_color = BG;
        style.visuals.override_text_color = Some(TEXT);
        style.visuals.selection.bg_fill = Color32::from_rgb(45, 91, 77);
        style.visuals.selection.stroke = Stroke::new(1.0_f32, MINT);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(57, 73, 92);
        style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(36, 47, 61);
        style.visuals.widgets.inactive.bg_stroke =
            Stroke::new(1.0_f32, Color32::from_rgb(50, 65, 82));
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(43, 61, 71);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(52, 82, 77);
        style.spacing.item_spacing = Vec2::new(12.0, 12.0);
        style.spacing.button_padding = Vec2::new(16.0, 10.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(15.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(15.0));
        cc.egui_ctx.set_style(style);
        let root = model::root();
        let config_path = root.join("settings.json");
        let config_result = model::read_json::<Settings>(&config_path);
        let error = if config_path.exists() {
            config_result
                .as_ref()
                .err()
                .map(|e| format!("No se pudo leer la configuración: {e:#}"))
        } else {
            None
        };
        let mut settings = config_result.unwrap_or_default();
        if settings.profile.is_none() {
            settings.profile = model::Profile::offline(&settings.local_name).ok();
        }
        let manifest =
            model::read_json::<Manifest>(&root.join("manifest.json")).unwrap_or_default();
        let installed = std::fs::read_dir(root.join("versions"))
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().join("installed.json").is_file())
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();
        let (tx, rx) = mpsc::channel();
        let mut app = Self {
            root,
            settings,
            manifest,
            session: None,
            installed,
            tx,
            rx,
            page: match std::env::var("LUMA_PREVIEW_PAGE").as_deref() {
                Ok("versions") => Page::Versions,
                Ok("settings") => Page::Settings,
                Ok("activity") => Page::Activity,
                _ => Page::Home,
            },
            busy: false,
            status: "Listo para tu próxima aventura".into(),
            progress: None,
            error,
            auth_url: None,
            auth_cancel: None,
            activity: Vec::new(),
            search: String::new(),
            category: 0,
            game_running: false,
            cache: false,
            official: auth::official_profiles(),
        };
        if app.settings.offline || std::env::var_os("LUMA_PREVIEW").is_some() {
            app.cache = true;
            app.status = if app.settings.offline {
                "Modo offline · usando el catálogo guardado"
            } else {
                "Catálogo guardado · listo para jugar"
            }
            .into();
        } else {
            app.refresh();
        }
        app
    }

    fn save_settings(&mut self) {
        if let Err(e) = model::write_json(&self.root.join("settings.json"), &self.settings) {
            self.error = Some(format!("No se pudieron guardar los ajustes: {e:#}"));
        }
    }

    fn work(
        &mut self,
        operation: impl FnOnce(Sender<Event>) -> anyhow::Result<()> + Send + 'static,
    ) {
        self.busy = true;
        self.error = None;
        self.progress = None;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            if let Err(e) = operation(tx.clone()) {
                let _ = tx.send(Event::Error(format!("{e:#}")));
            }
            let _ = tx.send(Event::Done);
        });
    }

    fn refresh(&mut self) {
        let root = self.root.clone();
        self.status = "Actualizando el catálogo de Mojang…".into();
        self.work(move |tx| engine::catalog(&root, &tx));
    }

    fn selected(&self) -> Option<&Version> {
        self.manifest
            .versions
            .iter()
            .find(|v| v.id == self.settings.selected)
            .or_else(|| self.manifest.versions.iter().find(|v| v.kind == "release"))
    }

    fn run(&mut self, play: bool) {
        let Some(version) = self.selected().cloned() else {
            return;
        };
        self.settings.selected = version.id.clone();
        self.save_settings();
        let offline = self.settings.offline;
        let identity = if play {
            match self.settings.launch_identity(self.session.as_ref()) {
                Ok(identity) => {
                    self.settings.profile = Some(identity.0.clone());
                    self.save_settings();
                    Some(identity)
                }
                Err(error) => {
                    self.error = Some(error.to_string());
                    return;
                }
            }
        } else {
            None
        };
        let root = self.root.clone();
        let settings = self.settings.clone();
        self.work(move |tx| {
            let installation = engine::prepare(&root, &version, &settings, offline, &tx)?;
            if let Some((profile, token)) = identity {
                engine::launch(
                    &root,
                    &version,
                    &settings,
                    installation,
                    &profile,
                    &token,
                    &tx,
                )?;
            } else {
                tx.send(Event::Status(
                    "Instalación completa · ya puedes jugar offline".into(),
                ))?;
            }
            Ok(())
        });
    }

    fn events(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Status(status) => {
                    self.activity.push(status.clone());
                    self.status = status;
                }
                Event::Progress(done, total) => self.progress = Some((done, total)),
                Event::Catalog(manifest, cached) => {
                    self.manifest = manifest;
                    self.cache = cached;
                    if self.settings.selected.is_empty() {
                        if let Some(version) = self.selected() {
                            self.settings.selected = version.id.clone();
                        }
                    }
                    self.status = if cached {
                        "Sin conexión · catálogo guardado"
                    } else {
                        "Catálogo actualizado · elige tu versión"
                    }
                    .into();
                }
                Event::AuthLink(url) => self.auth_url = Some(url),
                Event::SignedIn(session) => {
                    self.settings.profile = Some(session.profile.clone());
                    self.session = Some(session);
                    self.auth_url = None;
                    self.save_settings();
                    self.status = "Cuenta Microsoft conectada".into();
                }
                Event::Installed(id) => {
                    self.installed.insert(id);
                }
                Event::GameStarted => {
                    self.game_running = true;
                    self.status = "Minecraft está en ejecución. ¡Disfruta tu mundo!".into();
                    self.progress = None;
                }
                Event::GameExited(code) => {
                    self.game_running = false;
                    self.status = format!(
                        "Minecraft se cerró · código {}",
                        code.map(|c| c.to_string())
                            .unwrap_or_else(|| "desconocido".into())
                    );
                    self.activity.push(self.status.clone());
                    if code != Some(0) {
                        self.error = Some("Minecraft terminó con un error. Abre el registro del juego desde Actividad para revisar la causa.".into());
                    }
                }
                Event::Error(error) => {
                    self.activity.push(error.clone());
                    self.error = Some(error);
                    self.auth_url = None;
                    self.status = "La operación no se completó".into();
                }
                Event::Done => {
                    self.auth_cancel = None;
                    self.auth_url = None;
                    self.busy = false;
                    self.progress = None;
                }
            }
        }
        if self.activity.len() > 300 {
            self.activity.drain(..self.activity.len() - 300);
        }
    }

    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("navigation")
            .exact_width(190.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(PANEL).inner_margin(22.0))
            .show(ctx, |ui| {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(30.0), Sense::hover());
                    ui.painter().rect_filled(rect, 8.0, MINT);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "L",
                        FontId::proportional(24.0),
                        BG,
                    );
                    ui.label(RichText::new("luma").size(29.0).strong());
                });
                ui.label(RichText::new("UN MUNDO POR CREAR").size(9.0).color(MUTED));
                ui.add_space(40.0);
                for (page, label) in [
                    (Page::Home, "Inicio"),
                    (Page::Versions, "Versiones"),
                    (Page::Settings, "Ajustes"),
                    (Page::Activity, "Actividad"),
                ] {
                    let active = self.page == page;
                    let button = egui::Button::new(RichText::new(label).color(if active {
                        MINT
                    } else {
                        MUTED
                    }))
                    .fill(if active {
                        Color32::from_rgb(34, 59, 53)
                    } else {
                        Color32::TRANSPARENT
                    })
                    .corner_radius(8.0);
                    if ui.add_sized([146.0, 43.0], button).clicked() {
                        self.page = page;
                    }
                }
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    ui.label(
                        RichText::new(format!("LUMA  {}", env!("CARGO_PKG_VERSION")))
                            .size(10.0)
                            .color(MUTED),
                    );
                    ui.add_space(12.0);
                    card().show(ui, |ui| {
                        ui.label(
                            RichText::new(
                                self.settings
                                    .profile
                                    .as_ref()
                                    .map(|p| p.name.as_str())
                                    .unwrap_or("Sin cuenta"),
                            )
                            .strong(),
                        );
                        ui.label(
                            RichText::new(
                                if self.settings.profile.as_ref().is_some_and(|p| p.local) {
                                    "Perfil local · sin cuenta"
                                } else if self.settings.profile.is_some() {
                                    "Perfil Microsoft"
                                } else {
                                    "Conecta tu cuenta"
                                },
                            )
                            .size(11.0)
                            .color(MUTED),
                        );
                    });
                });
            });
    }

    fn home(&mut self, ui: &mut egui::Ui) {
        let user = self
            .settings
            .profile
            .as_ref()
            .map(|p| format!(", {}", p.name))
            .unwrap_or_default();
        ui.label(
            RichText::new(format!("Bienvenido{user}."))
                .size(27.0)
                .strong(),
        );
        ui.label(RichText::new("El próximo gran mundo empieza con un bloque.").color(MUTED));
        ui.add_space(10.0);
        landscape(ui);
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("TU AVENTURA")
                    .size(11.0)
                    .strong()
                    .color(MUTED),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(if self.cache {
                        "CATÁLOGO LOCAL"
                    } else {
                        "MINECRAFT JAVA EDITION"
                    })
                    .size(10.0)
                    .color(MINT),
                );
            });
        });
        card().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Nombre para jugar sin cuenta")
                        .size(12.0)
                        .color(MUTED),
                );
                ui.add_enabled(
                    !self.busy,
                    egui::TextEdit::singleline(&mut self.settings.local_name)
                        .desired_width(150.0)
                        .char_limit(16),
                );
                if ui
                    .add_enabled(!self.busy, egui::Button::new("Usar perfil local"))
                    .clicked()
                {
                    match model::Profile::offline(&self.settings.local_name) {
                        Ok(profile) => {
                            self.settings.profile = Some(profile);
                            self.session = None;
                            self.save_settings();
                            self.error = None;
                            self.status = "Perfil local listo · no necesitas una cuenta".into();
                        }
                        Err(error) => self.error = Some(error.to_string()),
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("Versión del juego").size(12.0).color(MUTED));
                    self.version_combo(ui);
                });
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Modo de juego").size(12.0).color(MUTED));
                    if ui
                        .add_enabled(
                            !self.busy,
                            egui::Checkbox::new(&mut self.settings.offline, "Jugar sin conexión"),
                        )
                        .changed()
                    {
                        self.save_settings();
                    }
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let ready = self.selected().is_some() && !self.busy;
                    if ui
                        .add_enabled(
                            ready,
                            primary(if self.game_running {
                                "En el juego"
                            } else {
                                "Jugar"
                            })
                            .min_size(Vec2::new(152.0, 50.0)),
                        )
                        .clicked()
                    {
                        self.run(true);
                    }
                    if ui
                        .add_enabled(
                            ready && !self.settings.offline,
                            egui::Button::new("Instalar").min_size(Vec2::new(90.0, 50.0)),
                        )
                        .clicked()
                    {
                        self.run(false);
                    }
                });
            });
        });
        ui.add_space(8.0);
        ui.columns(3, |cols| {
            stat(
                &mut cols[0],
                "VERSIONES INSTALADAS",
                &self.installed.len().to_string(),
                "Listas en este equipo",
            );
            stat(
                &mut cols[1],
                "MEMORIA ASIGNADA",
                &format!("{:.1} GB", self.settings.memory_mb as f32 / 1024.0),
                "Ajustable a tu manera",
            );
            stat(
                &mut cols[2],
                "TU ESPACIO",
                "Sin límites",
                "Cada versión tiene sus mundos",
            );
        });
    }

    fn version_combo(&mut self, ui: &mut egui::Ui) {
        let selected = self
            .selected()
            .map(|v| v.id.clone())
            .unwrap_or_else(|| "Cargando catálogo…".into());
        ui.add_enabled_ui(!self.busy, |ui| {
            egui::ComboBox::from_id_salt("game_version")
                .width(150.0)
                .selected_text(selected)
                .height(300.0)
                .show_ui(ui, |ui| {
                    for v in &self.manifest.versions {
                        ui.selectable_value(&mut self.settings.selected, v.id.clone(), &v.id);
                    }
                });
        });
    }

    fn versions(&mut self, ui: &mut egui::Ui) {
        title(
            ui,
            "Todos tus mundos.",
            "Releases, snapshots y las primeras versiones de Minecraft.",
        );
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Buscar versión…")
                    .desired_width(190.0),
            );
            egui::ComboBox::from_id_salt("filter")
                .selected_text(
                    ["Todas", "Releases", "Snapshots", "Históricas", "Instaladas"][self.category],
                )
                .show_ui(ui, |ui| {
                    for (index, label) in
                        ["Todas", "Releases", "Snapshots", "Históricas", "Instaladas"]
                            .iter()
                            .enumerate()
                    {
                        ui.selectable_value(&mut self.category, index, *label);
                    }
                });
            if ui
                .add_enabled(
                    !self.busy && !self.settings.offline,
                    egui::Button::new("Actualizar"),
                )
                .clicked()
            {
                self.refresh();
            }
        });
        ui.add_space(8.0);
        let matches: Vec<Version> = self
            .manifest
            .versions
            .iter()
            .filter(|v| {
                v.id.to_lowercase().contains(&self.search.to_lowercase())
                    && match self.category {
                        1 => v.kind == "release",
                        2 => v.kind == "snapshot",
                        3 => v.kind.starts_with("old_"),
                        4 => self.installed.contains(&v.id),
                        _ => true,
                    }
            })
            .cloned()
            .collect();
        ui.label(
            RichText::new(format!("{} versiones", matches.len()))
                .size(12.0)
                .color(MUTED),
        );
        egui::ScrollArea::vertical()
            .id_salt("versions_list")
            .max_height(ui.available_height() - 75.0)
            .show_rows(ui, 62.0, matches.len(), |ui, rows| {
                for index in rows {
                    let version = &matches[index];
                    let installed = self.installed.contains(&version.id);
                    card().inner_margin(12.0).show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&version.id).size(17.0).strong());
                            ui.label(RichText::new(kind(&version.kind)).size(11.0).color(MUTED));
                            ui.label(
                                RichText::new(version.release_time.get(..10).unwrap_or(""))
                                    .size(11.0)
                                    .color(MUTED),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .add_enabled(
                                        !self.busy,
                                        egui::Button::new(
                                            if self.settings.selected == version.id {
                                                "Seleccionada"
                                            } else {
                                                "Elegir"
                                            },
                                        ),
                                    )
                                    .clicked()
                                {
                                    self.settings.selected = version.id.clone();
                                    self.save_settings();
                                    self.page = Page::Home;
                                }
                                if installed {
                                    ui.label(RichText::new("Instalada").size(11.0).color(MINT));
                                }
                            });
                        });
                    });
                }
            });
        if matches.is_empty() {
            ui.label("No hay versiones que coincidan. Conéctate para descargar el catálogo.");
        }
    }

    fn settings_page(&mut self, ui: &mut egui::Ui) {
        title(
            ui,
            "A tu manera.",
            "Tu cuenta, tus recursos y tu forma de jugar.",
        );
        egui::ScrollArea::vertical().show(ui, |ui| {
            card().show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(RichText::new("Cuenta de Microsoft").size(18.0).strong());
                if let Some(profile) = &self.settings.profile {
                    ui.label(RichText::new(format!("{} · {}", profile.name, if profile.local { "perfil local, sin cuenta" } else { "perfil Microsoft" })).color(MINT));
                } else { ui.label(RichText::new("Microsoft es opcional. Puedes usar un perfil local desde Inicio.").color(MUTED)); }
                ui.label(RichText::new("Continúa en tu navegador y elige tu cuenta Microsoft. No necesitas copiar códigos.").size(12.0).color(MUTED));
                if auth::configured_client_id(&self.settings.client_id).is_empty() {
                    ui.label(RichText::new("El acceso Microsoft aún necesita que el desarrollador configure el registro de Luma.").size(12.0).color(Color32::from_rgb(240, 195, 125)));
                }
                ui.collapsing("Configuración del desarrollador", |ui| {
                    ui.label(RichText::new("Client ID de Luma (se configura una sola vez; no es tu correo ni tu contraseña)").size(12.0).color(MUTED));
                    ui.add_enabled(!self.busy, egui::TextEdit::singleline(&mut self.settings.client_id).desired_width(420.0).hint_text("Usar el Client ID incluido en la compilación"));
                });
                ui.horizontal(|ui| {
                    if ui.add_enabled(!self.busy && !auth::configured_client_id(&self.settings.client_id).is_empty(), primary("Continuar con Microsoft")).clicked() {
                        self.save_settings();
                        let id = self.settings.client_id.clone();
                        self.status = "Continúa en el navegador con tu cuenta Microsoft…".into();
                        let cancel = Arc::new(AtomicBool::new(false));
                        self.auth_cancel = Some(cancel.clone());
                        self.work(move |tx| { let session = auth::login(&id, &tx, &cancel)?; tx.send(Event::SignedIn(session))?; Ok(()) });
                    }
                    if ui.add_enabled(!self.busy && self.settings.profile.as_ref().is_some_and(|p| !p.local), egui::Button::new("Cerrar sesión")).clicked() {
                        self.session = None; self.settings.profile = model::Profile::offline(&self.settings.local_name).ok(); self.save_settings();
                    }
                });
                ui.label(RichText::new("Tus contraseñas se introducen únicamente en Microsoft. Los tokens no se guardan en disco.").size(11.0).color(MUTED));
                ui.add_space(8.0);
                ui.separator();
                ui.label(RichText::new("Perfil del launcher oficial · solo para jugar offline").size(12.0).color(MUTED));
                if self.official.is_empty() {
                    ui.label(RichText::new("No se encontraron perfiles guardados en .minecraft/launcher_accounts.json.").size(11.0).color(MUTED));
                }
                for profile in self.official.clone() {
                    if ui.add_enabled(!self.busy, egui::Button::new(format!("Importar a {}", profile.name))).clicked() {
                        self.session = None;
                        self.settings.profile = Some(profile);
                        self.settings.offline = true;
                        self.save_settings();
                        self.status = "Perfil importado · instala una versión con conexión antes de jugar offline".into();
                    }
                }
            });
            card().show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(RichText::new("Rendimiento").size(18.0).strong());
                ui.label(RichText::new("Memoria máxima de Minecraft").color(MUTED));
                ui.add_enabled(!self.busy, egui::Slider::new(&mut self.settings.memory_mb, 1024..=16384).step_by(512.0).suffix(" MB"));
                ui.label(RichText::new("4 GB es un buen punto de partida. Deja memoria disponible para Windows.").size(12.0).color(MUTED));
                ui.add_space(8.0);
                ui.label("Java personalizado (opcional)");
                ui.add_enabled(!self.busy, egui::TextEdit::singleline(&mut self.settings.java).desired_width(550.0).hint_text("Automático · Luma descarga el Java adecuado para cada versión"));
                ui.label(RichText::new("Para elegirlo manualmente, pega la ruta completa a java.exe.").size(12.0).color(MUTED));
            });
            card().show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.label(RichText::new("Archivos y mundos").size(18.0).strong());
                ui.label(RichText::new(self.root.display().to_string()).size(12.0).color(MUTED));
                ui.horizontal(|ui| {
                    if ui.button("Abrir carpeta").clicked() { self.open_path(self.root.clone()); }
                    if ui.add_enabled(!self.busy, primary("Guardar ajustes")).clicked() { self.save_settings(); self.status = "Ajustes guardados".into(); }
                });
            });
        });
    }

    fn open_path(&mut self, path: PathBuf) {
        if let Err(error) = std::fs::create_dir_all(&self.root).and_then(|_| {
            std::process::Command::new("explorer.exe")
                .arg(&path)
                .spawn()
                .map(|_| ())
        }) {
            self.error = Some(format!("No se pudo abrir {}: {error}", path.display()));
        }
    }

    fn activity_page(&mut self, ui: &mut egui::Ui) {
        title(
            ui,
            "Detrás de la aventura.",
            "Descargas, instalaciones y estado del juego.",
        );
        ui.horizontal(|ui| {
            if ui.button("Abrir registros").clicked() {
                let path = self.root.join("logs");
                if let Err(e) = std::fs::create_dir_all(&path) {
                    self.error = Some(e.to_string());
                } else {
                    self.open_path(path);
                }
            }
            if ui.button("Limpiar actividad").clicked() {
                self.activity.clear();
            }
        });
        card().show(ui, |ui| {
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if self.activity.is_empty() {
                        ui.label(
                            RichText::new("Todo en calma. Aquí aparecerá la actividad.")
                                .color(MUTED),
                        );
                    }
                    for line in &self.activity {
                        ui.label(RichText::new(line).monospace().size(12.0).color(MUTED));
                    }
                });
        });
    }
}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Captura únicamente el framebuffer de Luma para verificar su diseño.
        if let Ok(path) = std::env::var("LUMA_PREVIEW") {
            if ctx.cumulative_pass_nr() == 3 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            let screenshot = ctx.input(|input| {
                input.events.iter().find_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(image) = screenshot {
                let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                if let Err(error) = image::save_buffer(
                    &path,
                    &bytes,
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                ) {
                    eprintln!("No se pudo guardar la vista previa: {error}");
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint();
        }
        self.events();
        self.sidebar(ctx);
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(24, 14)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if self.busy && !self.game_running {
                        ui.spinner();
                    } else {
                        let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                        ui.painter().circle_filled(rect.center(), 3.0, MINT);
                    }
                    ui.label(RichText::new(&self.status).size(12.0).color(MUTED));
                });
                if let Some((done, total)) = self.progress {
                    ui.add(
                        egui::ProgressBar::new(done as f32 / total.max(1) as f32)
                            .fill(MINT)
                            .text(format!("{done} / {total} archivos")),
                    );
                }
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(28.0))
            .show(ctx, |ui| {
                if let Some(error) = self.error.clone() {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(65, 36, 42))
                        .corner_radius(8.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(error)
                                        .size(12.0)
                                        .color(Color32::from_rgb(255, 184, 184)),
                                );
                                if ui.small_button("Cerrar").clicked() {
                                    self.error = None;
                                }
                            });
                        });
                    ui.add_space(8.0);
                }
                match self.page {
                    Page::Home => {
                        egui::ScrollArea::vertical().show(ui, |ui| self.home(ui));
                    }
                    Page::Versions => self.versions(ui),
                    Page::Settings => self.settings_page(ui),
                    Page::Activity => self.activity_page(ui),
                }
            });
        if let Some(url) = self.auth_url.clone() {
            egui::Window::new("Conecta tu cuenta")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(
                        "Elige tu cuenta en la ventana de Microsoft que se abrió en tu navegador.",
                    );
                    ui.label(
                        RichText::new("Si ya tienes sesión allí, Microsoft puede usar esa cuenta.")
                            .size(12.0)
                            .color(MUTED),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Volver a abrir Microsoft").clicked() {
                            if let Err(e) = webbrowser::open(&url) {
                                self.error = Some(e.to_string());
                            }
                        }
                        if ui.button("Cancelar").clicked() {
                            if let Some(cancel) = &self.auth_cancel {
                                cancel.store(true, Ordering::Relaxed);
                            }
                            self.auth_url = None;
                        }
                    });
                    ui.label(
                        RichText::new("Esta ventana se cerrará al terminar la verificación.")
                            .size(12.0)
                            .color(MUTED),
                    );
                });
        }
        if self.busy {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(cancel) = &self.auth_cancel {
            cancel.store(true, Ordering::Relaxed);
        }
        self.save_settings();
    }
}

fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(CARD)
        .corner_radius(12.0)
        .inner_margin(18.0)
}

fn primary(label: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(label).strong().color(BG))
        .fill(MINT)
        .corner_radius(9.0)
}

fn title(ui: &mut egui::Ui, heading: &str, description: &str) {
    ui.label(RichText::new(heading).size(29.0).strong());
    ui.label(RichText::new(description).color(MUTED));
    ui.add_space(18.0);
}

fn stat(ui: &mut egui::Ui, heading: &str, value: &str, description: &str) {
    card().show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(RichText::new(heading).size(9.0).color(MUTED));
        ui.label(RichText::new(value).size(23.0).strong());
        ui.label(RichText::new(description).size(11.0).color(MUTED));
    });
}

fn kind(value: &str) -> &str {
    match value {
        "release" => "RELEASE",
        "snapshot" => "SNAPSHOT",
        "old_beta" => "BETA",
        "old_alpha" => "ALPHA",
        _ => value,
    }
}

fn landscape(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 240.0), Sense::hover());
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, CornerRadius::same(14), Color32::from_rgb(20, 43, 54));
    let point = |x: f32, y: f32| {
        Pos2::new(
            rect.left() + x * rect.width(),
            rect.top() + y * rect.height(),
        )
    };
    for i in 0..48 {
        let x = ((i * 137 + 41) % 997) as f32 / 997.0;
        let y = ((i * 79 + 17) % 251) as f32 / 600.0;
        painter.circle_filled(
            point(x, y),
            if i % 4 == 0 { 1.4 } else { 0.8 },
            Color32::from_rgba_unmultiplied(181, 224, 221, 95),
        );
    }
    painter.circle_filled(point(0.78, 0.23), 28.0, Color32::from_rgb(191, 220, 201));
    painter.circle_filled(point(0.795, 0.20), 25.0, Color32::from_rgb(20, 43, 54));
    for (points, color) in [
        (
            vec![
                (0., 0.70),
                (0.13, 0.37),
                (0.22, 0.55),
                (0.40, 0.26),
                (0.58, 0.65),
                (0.73, 0.42),
                (0.90, 0.59),
                (1., 0.35),
                (1., 1.),
                (0., 1.),
            ],
            Color32::from_rgb(33, 67, 76),
        ),
        (
            vec![
                (0., 0.85),
                (0.18, 0.58),
                (0.35, 0.80),
                (0.54, 0.48),
                (0.73, 0.82),
                (0.90, 0.55),
                (1., 0.76),
                (1., 1.),
                (0., 1.),
            ],
            Color32::from_rgb(31, 83, 79),
        ),
    ] {
        for pair in points[..points.len() - 2].windows(2) {
            let (x1, y1) = pair[0];
            let (x2, y2) = pair[1];
            painter.add(egui::Shape::convex_polygon(
                vec![point(x1, y1), point(x2, y2), point(x2, 1.0), point(x1, 1.0)],
                color,
                Stroke::NONE,
            ));
        }
    }
    // Siluetas por escalones: paisaje original dibujado en tiempo real.
    for i in 0..32 {
        let x = i as f32 / 31.0;
        let h = 0.10 + ((i * 17 + 5) % 11) as f32 * 0.009;
        let base = 0.86 + (x * 11.0).sin() * 0.045;
        let trunk = Rect::from_min_max(point(x, base - h * 0.5), point(x + 0.005, base + 0.08));
        painter.rect_filled(trunk, 0.0, Color32::from_rgb(17, 42, 41));
        for level in 0..3 {
            let y = base - h + level as f32 * h * 0.25;
            let half = 0.010 + level as f32 * 0.004;
            painter.rect_filled(
                Rect::from_min_max(point(x - half, y), point(x + half, y + h * 0.40)),
                0.0,
                Color32::from_rgb(22, 56, 50),
            );
        }
    }
    painter.rect_filled(
        Rect::from_min_max(point(0., 0.94), point(1., 1.)),
        0.0,
        Color32::from_rgb(18, 37, 37),
    );
    // Cabaña iluminada en la orilla del bosque.
    painter.rect_filled(
        Rect::from_min_max(point(0.74, 0.80), point(0.84, 0.95)),
        0.0,
        Color32::from_rgb(77, 68, 51),
    );
    painter.rect_filled(
        Rect::from_min_max(point(0.73, 0.77), point(0.85, 0.81)),
        0.0,
        Color32::from_rgb(37, 45, 42),
    );
    painter.rect_filled(
        Rect::from_min_max(point(0.75, 0.84), point(0.77, 0.88)),
        1.0,
        Color32::from_rgb(249, 201, 116),
    );
    painter.rect_filled(
        Rect::from_min_max(point(0.80, 0.84), point(0.82, 0.88)),
        1.0,
        Color32::from_rgb(249, 201, 116),
    );
    // Placa oscura mantiene legibles los textos sin tapar el paisaje.
    let plaque = Rect::from_min_max(point(0.025, 0.24), point(0.55, 0.86));
    painter.rect_filled(
        plaque,
        12.0,
        Color32::from_rgba_unmultiplied(13, 25, 32, 215),
    );
    painter.text(
        point(0.05, 0.32),
        egui::Align2::LEFT_TOP,
        "EXPLORA. CONSTRUYE. VUELVE.",
        FontId::proportional(10.0),
        MINT,
    );
    painter.text(
        point(0.05, 0.43),
        egui::Align2::LEFT_TOP,
        "Tu mundo,\na tu ritmo.",
        FontId::proportional(38.0),
        TEXT,
    );
    painter.text(
        point(0.05, 0.78),
        egui::Align2::LEFT_TOP,
        "Una nueva aventura te espera.",
        FontId::proportional(12.0),
        Color32::from_rgb(174, 199, 199),
    );
}
