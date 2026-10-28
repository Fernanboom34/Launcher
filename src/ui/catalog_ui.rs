use super::*;
use crate::catalog::{self, Kind, Project, Version as ContentVersion};
enum Message {
    Search(u64, anyhow::Result<catalog::Search>),
    Versions(u64, anyhow::Result<Vec<ContentVersion>>),
    Icon(String, Option<PathBuf>),
}
#[derive(Default)]
pub(super) struct State {
    pub(super) open: bool,
    quick_install: bool,
    pending_icons: HashSet<String>,
    textures: std::collections::HashMap<String, egui::TextureHandle>,
    icons: std::collections::HashMap<String, Option<PathBuf>>,
    profile: Option<GameProfile>,
    kind: Option<Kind>,
    world: Option<PathBuf>,
    query: String,
    offset: usize,
    total: usize,
    projects: Vec<Project>,
    selected: Option<Project>,
    versions: Vec<ContentVersion>,
    version: usize,
    loading: bool,
    error: Option<String>,
    generation: u64,
    search_due: Option<Instant>,
    tx: Option<Sender<Message>>,
    rx: Option<Receiver<Message>>,
}
impl State {
    fn search(&mut self) {
        let (Some(profile), Some(kind), Some(tx)) =
            (self.profile.clone(), self.kind, self.tx.clone())
        else {
            return;
        };
        self.generation += 1;
        let generation = self.generation;
        let query = self.query.clone();
        let offset = self.offset;
        self.loading = true;
        self.error = None;
        self.selected = None;
        self.versions.clear();
        std::thread::spawn(move || {
            let _ = tx.send(Message::Search(
                generation,
                catalog::search(&profile, kind, &query, offset),
            ));
        });
    }
    fn versions(&mut self, project: Project) {
        self.search_due = None;
        let (Some(profile), Some(kind), Some(tx)) =
            (self.profile.clone(), self.kind, self.tx.clone())
        else {
            return;
        };
        self.generation += 1;
        let generation = self.generation;
        let id = project.project_id.clone();
        self.selected = Some(project);
        self.loading = true;
        self.versions.clear();
        self.error = None;
        self.version = 0;
        std::thread::spawn(move || {
            let _ = tx.send(Message::Versions(
                generation,
                catalog::versions(&profile, kind, &id),
            ));
        });
    }
    fn poll(&mut self) {
        if let Some(rx) = &self.rx {
            while let Ok(message) = rx.try_recv() {
                match message {
                    Message::Search(generation, result) if generation == self.generation => {
                        self.loading = false;
                        match result {
                            Ok(search) => {
                                self.projects = search.hits;
                                self.total = search.total_hits;
                            }
                            Err(e) => self.error = Some(format!("{e:#}")),
                        }
                    }
                    Message::Versions(generation, result) if generation == self.generation => {
                        self.loading = false;
                        match result {
                            Ok(versions) => self.versions = versions,
                            Err(e) => self.error = Some(format!("{e:#}")),
                        }
                    }
                    Message::Icon(id, path) => {
                        self.pending_icons.remove(&id);
                        self.icons.insert(id, path);
                    }
                    _ => (),
                }
            }
        }
    }
}
impl Launcher {
    pub(super) fn open_catalog(
        &mut self,
        profile: &GameProfile,
        kind: Kind,
        world: Option<PathBuf>,
    ) {
        let (tx, rx) = mpsc::channel();
        self.catalog = State {
            open: true,
            profile: Some(profile.clone()),
            kind: Some(kind),
            world,
            tx: Some(tx),
            rx: Some(rx),
            ..Default::default()
        };
        self.page = Page::Instance;
        self.select_instance(profile);
        self.workspace.tab = 0;
        self.catalog.search();
    }
    pub(super) fn catalog_page(&mut self, ui: &mut egui::Ui) {
        let mut state = std::mem::take(&mut self.catalog);
        state.poll();
        let Some(profile) = state.profile.clone() else {
            self.catalog = state;
            return;
        };
        let kind = state.kind.unwrap_or(Kind::Mod);
        if state.search_due.is_some_and(|due| Instant::now() >= due) {
            state.search_due = None;
            state.offset = 0;
            state.search();
        }
        let mut install = None;
        if state.quick_install && !state.loading {
            state.quick_install = false;
            if let Some(version) = state
                .versions
                .first()
                .filter(|v| matches!(v.version_type.as_str(), "release" | "beta" | "alpha"))
            {
                install = Some(version.clone());
                state.selected = None;
            } else if state.error.is_none() {
                state.error = Some("No hay versiones compatibles disponibles.".into());
            }
        }
        ui.horizontal(|ui| {
            if ui.button("< Mi contenido").clicked() {
                state.open = false;
            }
            ui.add_space(8.);
            ui.heading("Descubrir contenido");
        });
        if let Some(world) = &state.world {
            ui.label(format!(
                "Mundo: {}",
                world.file_name().unwrap_or_default().to_string_lossy()
            ));
        }
        ui.add_space(12.);
        ui.horizontal(|ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut state.query)
                    .hint_text("Buscar en Modrinth…")
                    .margin(Vec2::splat(10.))
                    .desired_width((ui.available_width() - 95.).max(100.)),
            );
            if edit.changed() {
                state.generation += 1;
                state.search_due = Some(Instant::now() + Duration::from_millis(350));
                state.selected = None;
                state.quick_install = false;
            }
            if ui.button("Buscar").clicked()
                || (edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                state.search_due = None;
                state.offset = 0;
                state.search();
            }
        });
        ui.add_space(10.);
        ui.horizontal_wrapped(|ui| {
            for category in [Kind::Mod, Kind::ResourcePack, Kind::Shader] {
                if ui
                    .selectable_label(kind == category, category.label())
                    .clicked()
                {
                    state.kind = Some(category);
                    state.world = None;
                    state.offset = 0;
                    state.search_due = None;
                    state.quick_install = false;
                    state.search();
                }
            }
            ui.separator();
            ui.label(
                RichText::new(format!("{} proyectos compatibles", state.total))
                    .small()
                    .color(palette().muted),
            );
        });
        let kind = state.kind.unwrap_or(kind);
        ui.add_space(12.);
        if self.is_running(&profile) {
            ui.label("Detén la instancia para instalar contenido.");
        }
        if state.loading || state.search_due.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Cargando…");
            });
        }
        if let Some(error) = &state.error {
            ui.colored_label(Color32::from_rgb(245, 110, 120), error);
        }
        let enabled = !self.busy
            && !self.is_running(&profile)
            && !crate::browser::is_open()
            && !state.loading
            && state.search_due.is_none();
        if let Some(project) = state.selected.clone().filter(|_| !state.quick_install) {
            if ui.button("< Resultados").clicked() {
                state.selected = None;
                state.generation += 1;
                state.loading = false;
                state.quick_install = false;
            }
            ui.add_space(8.);
            card().show(ui, |ui| {
                ui.heading(&project.title);
                ui.label(RichText::new(format!("por {}", project.author)).color(MINT));
                ui.label(&project.description);
                ui.add_space(12.);
                ui.label("Versión compatible");
                egui::ComboBox::from_id_salt("catalog-version")
                    .width((ui.available_width() - 20.).min(500.))
                    .selected_text(
                        state
                            .versions
                            .get(state.version)
                            .map(|v| format!("{} · {}", v.version_number, v.version_type))
                            .unwrap_or_else(|| {
                                if state.loading {
                                    "Cargando versiones…".into()
                                } else {
                                    "Sin versiones compatibles".into()
                                }
                            }),
                    )
                    .show_ui(ui, |ui| {
                        for (i, v) in state.versions.iter().enumerate() {
                            ui.selectable_value(
                                &mut state.version,
                                i,
                                format!(
                                    "{} · {} · {}",
                                    v.version_number,
                                    v.version_type,
                                    v.date_published.get(..10).unwrap_or("")
                                ),
                            );
                        }
                    });
                ui.add_space(12.);
                if ui
                    .add_enabled(
                        enabled && state.versions.get(state.version).is_some(),
                        primary("Instalar esta versión"),
                    )
                    .clicked()
                {
                    install = state.versions.get(state.version).cloned();
                }
                ui.small("Se instalarán también las dependencias obligatorias compatibles.");
                if ui.link("Página del proyecto en Modrinth").clicked() {
                    if let Err(error) = crate::browser::open_project(
                        profile.clone(),
                        kind,
                        state.world.clone(),
                        &project.slug,
                        self.tx.clone(),
                    ) {
                        self.error = Some(error.to_string());
                    }
                }
            });
        } else {
            egui::ScrollArea::vertical().id_salt("catalog-results").max_height((ui.available_height()-48.).max(100.)).show(ui,|ui|{
                if state.projects.is_empty() && !state.loading {ui.label("No encontramos proyectos. Prueba otra búsqueda.");}
                for project in state.projects.clone() {
                    let id=project.project_id.clone();
                    if !state.icons.contains_key(&id) {
                        state.icons.insert(id.clone(),None);
                        if let (Some(url),Some(tx))=(project.icon_url.clone(),state.tx.clone()) {
                            state.pending_icons.insert(id.clone());
                            let folder=self.root.join("cache/catalog-icons"); let key=id.clone();
                            std::thread::spawn(move||{let path=catalog_icon(&folder,&key,&url);let _=tx.send(Message::Icon(key,path));});
                        }
                    }
                    if !state.textures.contains_key(&id) {
                        if let Some(Some(path))=state.icons.get(&id) {if let Ok(image)=image::open(path){let image=image.to_rgba8();let size=[image.width() as usize,image.height() as usize];state.textures.insert(id.clone(),ui.ctx().load_texture(&id,egui::ColorImage::from_rgba_unmultiplied(size,image.as_raw()),Default::default()));}}
                    }
                    egui::Frame::new().fill(palette().card).corner_radius(14).inner_margin(12.).show(ui,|ui|{
                        ui.set_width(ui.available_width());
                        ui.horizontal_top(|ui|{
                            if let Some(texture)=state.textures.get(&id){ui.add(egui::Image::new(texture).fit_to_exact_size(Vec2::splat(48.)).corner_radius(10));}
                            else {let(rect,_)=ui.allocate_exact_size(Vec2::splat(48.),Sense::hover());ui.painter().rect_filled(rect,10,palette().soft);ui.painter().text(rect.center(),egui::Align2::CENTER_CENTER,project.title.chars().next().unwrap_or('?'),FontId::proportional(26.),MINT);}
                            ui.add_space(8.);
                            let width=(ui.available_width()-145.).max(100.);
                            ui.vertical(|ui|{ui.set_width(width);ui.label(RichText::new(&project.title).size(19.).strong());ui.label(RichText::new(format!("por {}  ·  {} descargas",project.author,project.downloads)).size(13.).color(palette().muted));ui.add_space(4.);ui.add(egui::Label::new(&project.description).wrap());});
                            ui.vertical(|ui|{
                                if ui.add_enabled(enabled,primary("Instalar")).on_hover_text("Última release compatible; si no existe, beta y luego alpha").clicked(){state.versions(project.clone());state.quick_install=true;}
                                if ui.add_enabled(!state.loading,egui::Button::new("Elegir versión")).clicked(){state.quick_install=false;state.versions(project.clone());}
                            });
                        });
                    });
                    ui.add_space(10.);
                }
            });
            ui.add_space(8.);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !state.loading && state.offset > 0,
                        egui::Button::new("Anterior"),
                    )
                    .clicked()
                {
                    state.offset = state.offset.saturating_sub(20);
                    state.search();
                }
                ui.label(format!(
                    "Página {} · {} resultados",
                    state.offset / 20 + 1,
                    state.total
                ));
                if ui
                    .add_enabled(
                        !state.loading && state.offset + 20 < state.total,
                        egui::Button::new("Siguiente"),
                    )
                    .clicked()
                {
                    state.offset += 20;
                    state.search();
                }
            });
        }
        if let Some(version) = install
            .filter(|_| !self.busy && !self.is_running(&profile) && !crate::browser::is_open())
        {
            match catalog::destination(&profile, kind, state.world.as_deref()) {
                Ok(dest) => {
                    let root = self.root.clone();
                    let instance = profile.id.clone();
                    self.work(move |tx| {
                        let current = profiles::load(&root)?
                            .into_iter()
                            .find(|p| p.id == profile.id)
                            .ok_or_else(|| {
                                anyhow::anyhow!("La instancia ya no está en la biblioteca")
                            })?;
                        anyhow::ensure!(
                            current.game_dir == profile.game_dir
                                && current.minecraft == profile.minecraft
                                && current.loader == profile.loader,
                            "La instancia cambió. Vuelve a abrir el catálogo"
                        );
                        let count = catalog::install(&profile, kind, version, &dest, |status| {
                            let _ = tx.send(Event::Status(status));
                        })?;
                        tx.send(Event::ContentChanged(instance))?;
                        tx.send(Event::Status(format!(
                            "Contenido instalado · {count} archivos verificados"
                        )))?;
                        Ok(())
                    });
                }
                Err(e) => self.error = Some(e.to_string()),
            }
        }
        if state.loading
            || state.search_due.is_some()
            || !state.pending_icons.is_empty()
            || self.busy
        {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
        self.catalog = state;
    }
}
fn catalog_icon(folder: &std::path::Path, id: &str, url: &str) -> Option<PathBuf> {
    use std::io::Read;
    if !id
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return None;
    }
    let parsed = url::Url::parse(url).ok()?;
    if parsed.scheme() != "https" || parsed.host_str() != Some("cdn.modrinth.com") {
        return None;
    }
    std::fs::create_dir_all(folder).ok()?;
    let path = folder.join(format!("{id}.png"));
    if path.is_file() {
        return Some(path);
    }
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(15))
        .build()
        .ok()?;
    let mut bytes = Vec::new();
    client
        .get(url)
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    crate::content_meta::save_icon(folder, id, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn late_search_cannot_replace_current_results() {
        let (tx, rx) = mpsc::channel();
        let mut state = State {
            generation: 2,
            loading: true,
            rx: Some(rx),
            ..Default::default()
        };
        tx.send(Message::Search(
            1,
            Ok(catalog::Search {
                hits: Vec::new(),
                total_hits: 99,
            }),
        ))
        .unwrap();
        state.poll();
        assert!(state.loading);
        assert_eq!(state.total, 0);
        tx.send(Message::Search(
            2,
            Ok(catalog::Search {
                hits: Vec::new(),
                total_hits: 3,
            }),
        ))
        .unwrap();
        state.poll();
        assert!(!state.loading);
        assert_eq!(state.total, 3);
    }
}
