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
mod catalog_ui;
mod library;
mod theme;
mod workspace;
use theme::palette;

const MINT: Color32 = Color32::from_rgb(68, 221, 139);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Versions,
    Profiles,
    Settings,
    Activity,
    Instance,
    Screenshots,
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
    workspace: workspace::Workspace,
    catalog: catalog_ui::State,
    sessions: std::collections::HashMap<String, Session>,
    game_started: Option<Instant>,
    update_release: Option<crate::distribution::Release>,
    update_staged: Option<PathBuf>,
}

impl Launcher {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut style = (*cc.egui_ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = palette().bg;
        style.visuals.window_fill = palette().panel;
        style.visuals.extreme_bg_color = palette().bg;
        style.visuals.override_text_color = Some(palette().text);
        style.visuals.selection.bg_fill = Color32::from_rgb(36, 83, 58);
        style.visuals.selection.stroke = Stroke::new(1.0_f32, MINT);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(64, 69, 78);
        style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(49, 53, 59);
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, palette().border);
        style.visuals.widgets.hovered.bg_fill = palette().hover;
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(45, 88, 63);
        style.visuals.widgets.inactive.corner_radius = CornerRadius::same(8);
        style.visuals.widgets.hovered.corner_radius = CornerRadius::same(8);
        style.visuals.widgets.active.corner_radius = CornerRadius::same(8);
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.interact_size.y = 30.;
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
        let mut installations = profiles::load(&root).unwrap_or_default();
        let mut migrated = false;
        for p in &mut installations {
            if p.runtime.is_none() {
                p.runtime = Some(profiles::default_runtime(&root));
                migrated = true;
            }
        }
        if migrated && std::env::var_os("LUMA_PREVIEW").is_none() {
            let _ = profiles::save(&root, &installations);
        }
        if let Some(profile) = &settings.profile {
            if !settings.accounts.iter().any(|p| p.id == profile.id) {
                settings.accounts.push(profile.clone());
            }
        }
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
                Ok("screenshots") => Page::Screenshots,
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
            workspace: Default::default(),
            catalog: Default::default(),
            sessions: Default::default(),
            game_started: None,
            update_release: None,
            update_staged: None,
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
        let args: Vec<String> = std::env::args().collect();
        if let Some(path) = args
            .iter()
            .position(|a| a == "--update-report")
            .and_then(|i| args.get(i + 1))
            .map(PathBuf::from)
        {
            if let (Ok(file), Ok(base)) =
                (path.canonicalize(), app.root.join("updates").canonicalize())
            {
                if file.starts_with(base) {
                    if let Ok(report) = std::fs::read_to_string(&file) {
                        if report.trim() != "OK" {
                            app.error =
                                Some(format!("No se pudo actualizar Luma: {}", report.trim()));
                        }
                        let _ = std::fs::remove_file(file);
                    }
                }
            }
        }

        if std::env::var_os("LUMA_PREVIEW").is_some() {
            app.cache = true;
            app.status = "Catálogo guardado · listo para jugar".into();
        } else {
            app.refresh();
        }
        if std::env::var_os("LUMA_PREVIEW").is_none() {
            let tx = app.tx.clone();
            std::thread::spawn(move || {
                if let Ok(release) = crate::distribution::latest() {
                    if crate::distribution::newer(&release.version, env!("CARGO_PKG_VERSION")) {
                        let _ = tx.send(Event::UpdateAvailable(release));
                    }
                }
            });
        }
        if matches!(
            std::env::var("LUMA_PREVIEW_PAGE").as_deref(),
            Ok("instance" | "worlds" | "logs" | "instance-settings" | "delete" | "catalog")
        ) {
            if let Some(p) = app.installations.first().cloned() {
                app.open_instance(&p);
                if std::env::var("LUMA_PREVIEW_PAGE").as_deref() == Ok("catalog") {
                    app.open_catalog(&p, crate::catalog::Kind::Mod, None);
                }
                if std::env::var("LUMA_PREVIEW_PAGE").as_deref() == Ok("delete") {
                    app.workspace.remove = Some(p.id.clone());
                    app.workspace.remove_files = true;
                }
                app.workspace.tab = match std::env::var("LUMA_PREVIEW_PAGE").as_deref() {
                    Ok("worlds") => 2,
                    Ok("logs") => 3,
                    Ok("instance-settings") => 4,
                    _ => 0,
                };
            }
        }
        if let Ok(section) = std::env::var("LUMA_PREVIEW_SECTION") {
            if let Ok(i) = section.parse::<usize>() {
                if app.page == Page::Instance {
                    app.workspace.settings_tab = i.min(4);
                } else {
                    app.workspace.global_tab = i.min(5);
                }
            }
        }
        if let Ok(theme) = std::env::var("LUMA_PREVIEW_THEME") {
            app.settings.appearance.theme = theme.parse::<usize>().unwrap_or(0).min(3);
        }
        if std::env::var_os("LUMA_PREVIEW").is_none() && std::env::var_os("LUMA_DATA_DIR").is_none()
        {
            if let Ok(exe) = std::env::current_exe() {
                let _ = crate::cleanup::register_executable(&exe);
            }
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
        if play && crate::browser::is_open() {
            self.error =
                Some("Cierra el navegador de contenido antes de iniciar Minecraft.".into());
            return;
        }

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
        let offline = false;
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
        if let Some(profile) = &installation {
            settings.runtime = profile
                .runtime
                .clone()
                .unwrap_or_else(|| profiles::default_runtime(&self.root));
            settings.memory_mb = settings.runtime.memory_mb;
            settings.java = settings.runtime.java.clone();
        }
        settings.game_directory = installation.map(|p| p.game_dir);
        settings.quick_world = self.workspace.pending_world.take();
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
                    self.sessions
                        .insert(session.profile.id.clone(), session.clone());
                    if !self
                        .settings
                        .accounts
                        .iter()
                        .any(|p| p.id == session.profile.id)
                    {
                        self.settings.accounts.push(session.profile.clone());
                    }
                    self.session = Some(session);
                    self.auth_url = None;
                    self.save_settings();
                    self.status = "Cuenta Microsoft conectada".into();
                }
                Event::Installed(id) => {
                    self.installed.insert(id);
                }
                Event::Profiles(list) => self.installations = list,
                Event::ContentChanged(id) => {
                    if let Some(profile) =
                        self.active_installation().filter(|p| p.id == id).cloned()
                    {
                        self.refresh_instance(&profile);
                    }
                }
                Event::InstanceRemoved(id) => {
                    self.settings.instance_styles.remove(&id);
                    if self.settings.active_installation == id {
                        self.settings.active_installation.clear();
                    }
                    self.workspace.deleting_instance = false;
                    self.save_settings();
                }
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
                    self.game_started = Some(Instant::now());
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
                    if let (Some(start), Some(id)) =
                        (self.game_started.take(), self.running_installation.as_ref())
                    {
                        self.settings
                            .instance_styles
                            .entry(id.clone())
                            .or_default()
                            .play_seconds += start.elapsed().as_secs();
                        self.save_settings();
                    }
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
                    self.workspace.deleting_instance = false;
                    self.library.pending_style = None;
                    self.activity.push(error.clone());
                    self.error = Some(error);
                    self.auth_url = None;
                    self.status = "La operación no se completó".into();
                }
                Event::EditorLoaderVersions(v) => self.editor_loader_versions(v),
                Event::UpdateAvailable(release) => self.update_release = Some(release),
                Event::UpdateReady(path) => self.update_staged = Some(path),
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
                .add_enabled(!self.busy, egui::Button::new("Actualizar"))
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
                .color(palette().muted),
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
                            ui.label(
                                RichText::new(kind(&version.kind))
                                    .size(11.0)
                                    .color(palette().muted),
                            );
                            ui.label(
                                RichText::new(version.release_time.get(..10).unwrap_or(""))
                                    .size(11.0)
                                    .color(palette().muted),
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

    fn legacy_settings_page(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.);
        ui.heading("Añadir cuenta Microsoft");
        ui.label("Continúa en tu navegador para elegir tu cuenta Microsoft.");
        if auth::configured_client_id(&self.settings.client_id).is_empty() {
            ui.label(RichText::new("El desarrollador todavía debe configurar el Client ID de Luma para habilitar Microsoft.").color(palette().muted));
        }
        ui.collapsing("Registro de Luma", |ui| {
            ui.label("Client ID de la aplicación");
            ui.text_edit_singleline(&mut self.settings.client_id);
        });
        if ui
            .add_enabled(
                !self.busy && !auth::configured_client_id(&self.settings.client_id).is_empty(),
                primary("Añadir cuenta Microsoft"),
            )
            .clicked()
        {
            self.save_settings();
            let id = self.settings.client_id.clone();
            let cancel = Arc::new(AtomicBool::new(false));
            self.auth_cancel = Some(cancel.clone());
            self.work(move |tx| {
                let session = auth::login(&id, &tx, &cancel)?;
                tx.send(Event::SignedIn(session))?;
                Ok(())
            });
        }
        ui.small("La contraseña se introduce en Microsoft. Los tokens permanecen en memoria durante esta sesión.");
        if !self.official.is_empty() {
            ui.collapsing("Perfiles del launcher oficial (solo offline)", |ui| {
                for profile in self.official.clone() {
                    if ui.button(format!("Importar {}", profile.name)).clicked() {
                        if !self.settings.accounts.iter().any(|p| p.id == profile.id) {
                            self.settings.accounts.push(profile.clone());
                        }
                        self.settings.profile = Some(profile);

                        self.session = None;
                        self.save_settings();
                    }
                }
            });
        }
    }

    fn open_path(&mut self, path: PathBuf) {
        if let Err(error) = std::fs::create_dir_all(&self.root).and_then(|_| {
            std::process::Command::new(crate::platform::FILE_MANAGER)
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
                let path = self
                    .active_installation()
                    .map(|p| p.game_dir.join(".luma/logs"))
                    .unwrap_or_else(|| self.root.join("logs"));
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
                                .color(palette().muted),
                        );
                    }
                    for line in &self.activity {
                        ui.label(
                            RichText::new(line)
                                .monospace()
                                .size(12.0)
                                .color(palette().muted),
                        );
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
                && self.preview_started.elapsed()
                    >= Duration::from_millis(
                        std::env::var("LUMA_PREVIEW_DELAY_MS")
                            .ok()
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(500),
                    )
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
        if self.workspace.deleting_instance && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if !self.busy
            && Instant::now() >= self.next_profile_scan
            && std::env::var_os("LUMA_PREVIEW").is_none()
        {
            let previous_error = self.error.clone();
            self.scan_profiles(None);
            self.error = previous_error;
        }
        self.drive_loader_query(ctx);
        self.apply_appearance(ctx);
        if self.game_running && self.settings.behavior.minimize && !self.workspace.minimized {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            self.workspace.minimized = true;
        }
        if !self.game_running && (self.workspace.minimized || self.workspace.was_running) {
            if self.settings.behavior.refocus {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            self.workspace.minimized = false;
            self.workspace.was_running = false;
        }
        if self.game_running {
            self.workspace.was_running = true;
        }
        if let Some(path) = self.update_staged.take() {
            match crate::distribution::apply_update(&path) {
                Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Err(e) => self.error = Some(e.to_string()),
            }
        }
        self.topbar(ctx);
        if self.settings.behavior.sidebar_visible {
            self.sidebar(ctx);
        }
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(palette().panel)
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
                    ui.label(
                        RichText::new(&self.status)
                            .size(12.0)
                            .color(palette().muted),
                    );
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
            .frame(egui::Frame::new().fill(palette().bg).inner_margin(24.0))
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
                    Page::Settings => self.launcher_settings(ui),
                    Page::Instance => self.instance_page(ui),
                    Page::Screenshots => self.screenshots_page(ui),
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
                            .color(palette().muted),
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
                            .color(palette().muted),
                    );
                });
        }
        self.instance_windows(ctx);
        self.workspace_windows(ctx);
        self.catalog_window(ctx);
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
        .fill(palette().card)
        .corner_radius(12.0)
        .inner_margin(18.0)
}

fn primary(label: &str) -> egui::Button<'_> {
    egui::Button::new(
        RichText::new(label)
            .strong()
            .color(Color32::from_rgb(18, 30, 24)),
    )
    .fill(MINT)
    .corner_radius(9.0)
}

fn title(ui: &mut egui::Ui, heading: &str, description: &str) {
    ui.label(RichText::new(heading).size(29.0).strong());
    ui.label(RichText::new(description).color(palette().muted));
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
