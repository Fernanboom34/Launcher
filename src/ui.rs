use crate::{
    auth, engine,
    model::{
        self, Event, GameProfile, Loader, Manifest, ProfileRequest, Session, Settings, Version,
    },
    profiles,
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
    time::{Duration, Instant},
};

mod art;
mod library;

const BG: Color32 = Color32::from_rgb(22, 24, 28);
const PANEL: Color32 = Color32::from_rgb(32, 35, 40);
const CARD: Color32 = Color32::from_rgb(39, 42, 48);
const HOVER: Color32 = Color32::from_rgb(49, 53, 60);
const BORDER: Color32 = Color32::from_rgb(57, 61, 69);
const TEXT: Color32 = Color32::from_rgb(242, 244, 246);
const MUTED: Color32 = Color32::from_rgb(154, 164, 178);
const MINT: Color32 = Color32::from_rgb(68, 221, 139);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Versions,
    Profiles,
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
    installations: Vec<GameProfile>,
    new_profile_name: String,
    new_minecraft: String,
    new_loader: Loader,
    new_loader_version: String,
    loader_versions: Vec<String>,
    loader_query: Option<(Loader, String)>,
    rename_profile: Option<(String, String)>,
    next_profile_scan: Instant,
    library: library::LibraryState,
    pending_launch: Option<String>,
    running_installation: Option<String>,
    preview_started: Instant,
    preview_requested: bool,
}

impl Launcher {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = BG;
        style.visuals.window_fill = PANEL;
        style.visuals.extreme_bg_color = BG;
        style.visuals.override_text_color = Some(TEXT);
        style.visuals.selection.bg_fill = Color32::from_rgb(36, 83, 58);
        style.visuals.selection.stroke = Stroke::new(1.0_f32, MINT);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(64, 69, 78);
        style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(49, 53, 59);
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
        style.visuals.widgets.hovered.bg_fill = HOVER;
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(45, 88, 63);
        style.visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
        style.visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
        style.visuals.widgets.active.corner_radius = CornerRadius::same(8);
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.button_padding = Vec2::new(12.0, 8.0);
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
        let manifest: Manifest =
            model::read_json::<Manifest>(&root.join("manifest.json")).unwrap_or_default();
        let new_minecraft = manifest
            .versions
            .iter()
            .find(|v| v.id == settings.selected)
            .or_else(|| manifest.versions.iter().find(|v| v.kind == "release"))
            .map(|v| v.id.clone())
            .unwrap_or_default();
        let installations = profiles::load(&root).unwrap_or_default();
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
                Ok("profiles") | Ok("selected") => Page::Profiles,
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
            installations,
            new_profile_name: "Mi perfil".into(),
            new_minecraft,
            new_loader: Loader::Vanilla,
            new_loader_version: String::new(),
            loader_versions: Vec::new(),
            loader_query: None,
            rename_profile: None,
            next_profile_scan: Instant::now() + Duration::from_secs(10),
            library: library::LibraryState::default(),
            pending_launch: None,
            running_installation: None,
            preview_started: Instant::now(),
            preview_requested: false,
        };
        if std::env::var("LUMA_PREVIEW_PAGE").as_deref() == Ok("selected") {
            if let Some(profile) = app.installations.first() {
                app.settings.active_installation = profile.id.clone();
            }
        }
        if std::env::var("LUMA_PREVIEW_PAGE").as_deref() == Ok("create") {
            app.open_create();
            if let Some(profile) = app
                .installations
                .iter()
                .find(|p| p.loader == Loader::NeoForge)
                .cloned()
            {
                app.new_minecraft = profile.minecraft.clone();
                app.new_loader = profile.loader;
                app.new_loader_version = profile.loader_version.clone();
                app.loader_versions = vec![profile.loader_version];
                app.loader_query = Some((profile.loader, profile.minecraft));
            }
        }
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
        if std::env::var_os("LUMA_PREVIEW").is_some() {
            return;
        }
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
        self.work(move |tx| {
            engine::catalog(&root, &tx)?;
            tx.send(Event::Profiles(profiles::discover(&root, None)?))?;
            Ok(())
        });
    }

    fn active_installation(&self) -> Option<&GameProfile> {
        self.installations
            .iter()
            .find(|p| p.id == self.settings.active_installation)
    }

    fn scan_profiles(&mut self, extra: Option<PathBuf>) {
        let root = self.root.clone();
        self.next_profile_scan = Instant::now() + Duration::from_secs(10);
        self.work(move |tx| {
            tx.send(Event::Profiles(profiles::discover(
                &root,
                extra.as_deref(),
            )?))?;
            Ok(())
        });
    }

    fn selected(&self) -> Option<&Version> {
        self.manifest
            .versions
            .iter()
            .find(|v| v.id == self.settings.selected)
            .or_else(|| self.manifest.versions.iter().find(|v| v.kind == "release"))
    }

    fn run(&mut self, play: bool) {
        self.pending_launch = play
            .then(|| self.settings.active_installation.clone())
            .filter(|id| !id.is_empty());
        let installation = self.active_installation().cloned();
        let version = if let Some(profile) = &installation {
            match profiles::launch_version(profile) {
                Ok(v) => v,
                Err(error) => {
                    self.error = Some(error.to_string());
                    return;
                }
            }
        } else if let Some(version) = self.selected().cloned() {
            version
        } else {
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
        let root = installation
            .as_ref()
            .map(|p| p.storage_dir.clone())
            .unwrap_or_else(|| self.root.clone());
        let mut settings = self.settings.clone();
        settings.game_directory = installation.map(|p| p.game_dir);
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
                    if self.new_minecraft.is_empty() {
                        self.new_minecraft = self
                            .manifest
                            .versions
                            .iter()
                            .find(|v| v.kind == "release")
                            .map(|v| v.id.clone())
                            .unwrap_or_default();
                    }
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
                Event::Profiles(list) => self.installations = list,
                Event::ProfileCreated(profile) => {
                    if let Some(style) = self.library.pending_style.take() {
                        self.settings
                            .instance_styles
                            .insert(profile.id.clone(), style);
                    }
                    self.library.create_open = false;
                    self.library.reveal_created();
                    self.settings
                        .instance_styles
                        .entry(profile.id.clone())
                        .or_default()
                        .created_at = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    self.page = Page::Profiles;
                    self.settings.active_installation = profile.id;
                    self.settings.selected = profile.version_id;
                    self.save_settings();
                }
                Event::LoaderVersions(loader, minecraft, versions) => {
                    if self.new_loader == loader && self.new_minecraft == minecraft {
                        self.new_loader_version = versions.first().cloned().unwrap_or_default();
                        self.loader_versions = versions;
                        self.loader_query = Some((loader, minecraft));
                    }
                }
                Event::GameStarted => {
                    self.running_installation = self.pending_launch.clone();
                    if let Some(id) = &self.running_installation {
                        self.settings
                            .instance_styles
                            .entry(id.clone())
                            .or_default()
                            .last_played = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        self.save_settings();
                    }
                    self.game_running = true;
                    self.status = "Minecraft está en ejecución. ¡Disfruta tu mundo!".into();
                    self.progress = None;
                }
                Event::GameExited(code) => {
                    self.running_installation = None;
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
                    self.library.pending_style = None;
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
                                    .add_enabled(!self.busy, egui::Button::new("Crear instancia"))
                                    .clicked()
                                {
                                    self.open_create();
                                    self.new_minecraft = version.id.clone();
                                    self.new_loader = Loader::Vanilla;
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
            if !self.preview_requested
                && self.preview_started.elapsed() >= Duration::from_millis(500)
            {
                self.preview_requested = true;
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
        if !self.busy
            && Instant::now() >= self.next_profile_scan
            && std::env::var_os("LUMA_PREVIEW").is_none()
        {
            let previous_error = self.error.clone();
            self.scan_profiles(None);
            self.error = previous_error;
        }
        self.drive_loader_query(ctx);
        self.topbar(ctx);
        self.sidebar(ctx);
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(PANEL)
                    .inner_margin(egui::Margin::symmetric(20, 9)),
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
        self.context_panel(ctx);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(24.0))
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
                    Page::Profiles => self.profiles_page(ui),
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
        self.instance_windows(ctx);
        if let Some((id, mut name)) = self.rename_profile.clone() {
            egui::Window::new("Renombrar perfil")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.text_edit_singleline(&mut name);
                    self.rename_profile = Some((id.clone(), name.clone()));
                    ui.horizontal(|ui| {
                        if ui.button("Guardar").clicked() {
                            match profiles::rename(&self.root, &id, &name) {
                                Ok(list) => {
                                    self.installations = list;
                                    self.rename_profile = None;
                                }
                                Err(error) => self.error = Some(error.to_string()),
                            }
                        }
                        if ui.button("Cancelar").clicked() {
                            self.rename_profile = None;
                        }
                    });
                });
        }
        ctx.request_repaint_after(Duration::from_secs(10));
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

fn kind(value: &str) -> &str {
    match value {
        "release" => "RELEASE",
        "snapshot" => "SNAPSHOT",
        "old_beta" => "BETA",
        "old_alpha" => "ALPHA",
        _ => value,
    }
}
