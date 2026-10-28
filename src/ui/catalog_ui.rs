use super::*;
use crate::catalog::{self, Kind, Project, Version as ContentVersion};
enum Message {
    Search(u64, anyhow::Result<catalog::Search>),
    Versions(u64, anyhow::Result<Vec<ContentVersion>>),
}
#[derive(Default)]
pub(super) struct State {
    open: bool,
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
        self.catalog.search();
    }
    pub(super) fn catalog_window(&mut self, ctx: &egui::Context) {
        let mut state = std::mem::take(&mut self.catalog);
        state.poll();
        if state.search_due.is_some_and(|due| Instant::now() >= due) {
            state.search_due = None;
            state.offset = 0;
            state.search();
        }
        if !state.open {
            self.catalog = state;
            return;
        }
        let Some(profile) = state.profile.clone() else {
            self.catalog = state;
            return;
        };
        let kind = state.kind.unwrap_or(Kind::Mod);
        let mut open = true;
        let mut install = None;
        egui::Window::new("Explorar contenido · Modrinth")
            .open(&mut open)
            .default_size((ctx.screen_rect().size() * 0.85).min(Vec2::new(1100., 800.)))
            .min_size([650., 450.])
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.heading(&profile.name);
                ui.label(
                    RichText::new(format!(
                        "{} · Minecraft {} · {} · versiones compatibles",
                        kind.label(),
                        profile.minecraft,
                        profile.loader.label()
                    ))
                    .color(MINT),
                );
                if let Some(world) = &state.world {
                    ui.label(
                        RichText::new(format!(
                            "Mundo: {}",
                            world.file_name().unwrap_or_default().to_string_lossy()
                        ))
                        .small(),
                    );
                }
                if state.selected.is_none() {
                    ui.horizontal(|ui| {
                        let edit = ui.add(
                            egui::TextEdit::singleline(&mut state.query)
                                .hint_text("Buscar mods, packs, shaders…")
                                .desired_width((ui.available_width() - 110.).max(150.)),
                        );
                        if edit.changed() {
                            state.generation += 1;
                            state.search_due = Some(Instant::now() + Duration::from_millis(350));
                        }
                        if ui.add_enabled(!state.loading, primary("Buscar")).clicked()
                            || (edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                        {
                            state.search_due = None;
                            state.offset = 0;
                            state.search();
                        }
                    });
                }
                if state.loading || state.search_due.is_some() {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Buscando contenido compatible…");
                    });
                }
                if let Some(error) = &state.error {
                    ui.colored_label(Color32::from_rgb(245, 110, 120), error);
                }
                if let Some(project) = state.selected.clone() {
                    if ui.button("← Volver a los resultados").clicked() {
                        state.generation += 1;
                        state.selected = None;
                        state.loading = false;
                        state.error = None;
                        state.versions.clear();
                    }
                    card().inner_margin(12.).show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.strong(&project.title);
                        ui.label(&project.description);
                        egui::ComboBox::from_id_salt("content_versions")
                            .selected_text(
                                state
                                    .versions
                                    .get(state.version)
                                    .map(|v| v.version_number.as_str())
                                    .unwrap_or(if state.loading {
                                        "Cargando versiones…"
                                    } else {
                                        "Sin versiones compatibles"
                                    }),
                            )
                            .show_ui(ui, |ui| {
                                for (i, version) in state.versions.iter().enumerate() {
                                    ui.selectable_value(&mut state.version, i, &version.name);
                                }
                            });
                        if self.is_running(&profile) {
                            ui.label("Detén esta instancia antes de instalar contenido.");
                        }
                        ui.horizontal(|ui| {
                            if ui
                                .add_enabled(
                                    !self.busy
                                        && !self.is_running(&profile)
                                        && !crate::browser::is_open()
                                        && !state.loading
                                        && state.versions.get(state.version).is_some(),
                                    primary("Descargar e instalar"),
                                )
                                .clicked()
                            {
                                install = state.versions.get(state.version).cloned();
                            }
                            if ui.button("Ver proyecto en la web de Luma").clicked() {
                                if let Err(e) = crate::browser::open_project(
                                    profile.clone(),
                                    kind,
                                    state.world.clone(),
                                    &project.slug,
                                    self.tx.clone(),
                                ) {
                                    self.error = Some(e.to_string());
                                }
                            }
                        });
                        ui.label(
                            RichText::new(
                                "Se descargarán también las dependencias obligatorias compatibles.",
                            )
                            .small()
                            .color(palette().muted),
                        );
                    });
                }
                if state.selected.is_none() {
                    egui::ScrollArea::vertical()
                        .id_salt("modrinth_results")
                        .max_height((ui.available_height() - 45.).max(100.))
                        .show(ui, |ui| {
                            if state.projects.is_empty() && !state.loading {
                                ui.label(
                                "No se encontró contenido para esta combinación de juego y loader.",
                            );
                            }
                            for project in state.projects.clone() {
                                card().inner_margin(12.).show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.set_max_width(
                                                (ui.available_width() - 150.).max(200.),
                                            );
                                            ui.strong(&project.title);
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} · {} descargas",
                                                    project.author, project.downloads
                                                ))
                                                .small()
                                                .color(palette().muted),
                                            );
                                            ui.label(&project.description);
                                        });
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                if ui
                                                    .add_enabled(
                                                        !state.loading,
                                                        egui::Button::new("Ver versiones"),
                                                    )
                                                    .clicked()
                                                {
                                                    state.versions(project.clone());
                                                }
                                            },
                                        );
                                    });
                                });
                                ui.add_space(3.);
                            }
                        });
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !state.loading && state.offset > 0,
                                egui::Button::new("← Anterior"),
                            )
                            .clicked()
                        {
                            state.offset = state.offset.saturating_sub(20);
                            state.search();
                        }
                        ui.label(format!(
                            "{}–{} de {}",
                            if state.total == 0 {
                                0
                            } else {
                                state.offset + 1
                            },
                            (state.offset + state.projects.len()).min(state.total),
                            state.total
                        ));
                        if ui
                            .add_enabled(
                                !state.loading && state.offset + 20 < state.total,
                                egui::Button::new("Siguiente →"),
                            )
                            .clicked()
                        {
                            state.offset += 20;
                            state.search();
                        }
                    });
                }
            });
        state.open = open;
        if let Some(version) = install {
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
        if state.loading || state.search_due.is_some() || self.busy {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        self.catalog = state;
    }
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
