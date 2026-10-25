use super::*;
use crate::{content, distribution};
#[derive(Default)]
pub(super) struct Workspace {
    pub tab: usize,
    pub settings_tab: usize,
    pub global_tab: usize,
    pub pending_world: Option<String>,
    pub import_open: bool,
    pub remove: Option<String>,
    pub minimized: bool,
    pub was_running: bool,
    pub decorations: Option<bool>,
    query: String,
    filter: usize,
    log_filter: usize,
    log_query: String,
    log_text: String,
    log_time: Option<Instant>,
    log_file: PathBuf,
    items: Vec<content::Content>,
    worlds: Vec<content::World>,
    shots: Vec<content::Screenshot>,
    recent_worlds: std::collections::HashMap<String, (Instant, Option<content::World>)>,
    refreshed: Option<Instant>,
    file_path: PathBuf,
    editing: Option<GameProfile>,
    loader_choices: Vec<String>,
    loader_key: Option<(Loader, String)>,
    new_account: String,
    import_kind: usize,
    import_world: usize,
    shot_textures: std::collections::HashMap<PathBuf, egui::TextureHandle>,
    shot_query: String,
}
impl Launcher {
    pub(super) fn open_instance(&mut self, p: &GameProfile) {
        self.select_instance(p);
        self.page = Page::Instance;
        self.workspace.tab = 0;
        self.workspace.log_file = PathBuf::new();
        self.workspace.log_time = None;
        self.workspace.query.clear();
        self.workspace.editing = Some(p.clone());
        self.workspace.file_path = p.game_dir.clone();
        self.refresh_instance(p);
    }
    fn refresh_instance(&mut self, p: &GameProfile) {
        self.workspace.items = content::contents(&p.game_dir);
        self.workspace.worlds = content::worlds(&p.game_dir);
        self.workspace.refreshed = Some(Instant::now());
    }
    pub(super) fn recent_world(&mut self, p: &GameProfile) -> Option<content::World> {
        let stale = self
            .workspace
            .recent_worlds
            .get(&p.id)
            .is_none_or(|(time, _)| time.elapsed() > Duration::from_secs(5));
        if stale {
            let world = content::worlds(&p.game_dir).into_iter().next();
            self.workspace
                .recent_worlds
                .insert(p.id.clone(), (Instant::now(), world));
        }
        self.workspace
            .recent_worlds
            .get(&p.id)
            .and_then(|(_, world)| world.clone())
    }
    pub(super) fn play_recent(&mut self, p: &GameProfile) {
        let version = std::fs::read(
            p.storage_dir
                .join("versions")
                .join(&p.version_id)
                .join(format!("{}.json", p.version_id)),
        )
        .ok()
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
        // Inherited loaders may not repeat the base game's arguments.
        let modern = p
            .minecraft
            .split('.')
            .take(2)
            .filter_map(|s| s.parse::<u32>().ok())
            .collect::<Vec<_>>();
        if version.as_ref().is_some_and(engine::supports_quick_play)
            || modern.first().is_some_and(|n| *n > 1)
            || modern.get(1).is_some_and(|n| *n >= 20)
        {
            self.workspace.pending_world = content::worlds(&p.game_dir)
                .first()
                .map(|w| w.folder.clone());
        }
        self.play_instance(p);
    }
    pub(super) fn instance_page(&mut self, ui: &mut egui::Ui) {
        let Some(p) = self.active_installation().cloned() else {
            ui.label("Selecciona una instancia en la biblioteca.");
            return;
        };
        if self.workspace.editing.as_ref().is_none_or(|e| e.id != p.id) {
            self.workspace.editing = Some(p.clone());
            self.workspace.file_path = p.game_dir.clone();
            self.refresh_instance(&p);
        }
        if self
            .workspace
            .refreshed
            .is_none_or(|t| t.elapsed() > Duration::from_secs(5))
        {
            self.refresh_instance(&p);
        }
        ui.horizontal(|ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(62.), Sense::hover());
            self.paint_cover(ui, rect, &p);
            ui.vertical(|ui| {
                ui.heading(&p.name);
                ui.label(
                    RichText::new(format!(
                        "{} {} · Minecraft {}",
                        p.loader.label(),
                        p.loader_version,
                        p.minecraft
                    ))
                    .color(palette().muted),
                );
                if self.settings.behavior.playtime {
                    let seconds = self
                        .settings
                        .instance_styles
                        .get(&p.id)
                        .map(|s| s.play_seconds)
                        .unwrap_or(0);
                    ui.small(format!("{} min jugados", seconds / 60));
                }
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.menu_button("···", |ui| self.instance_menu(ui, &p));
                if ui
                    .button("⚙")
                    .on_hover_text("Configuración de instancia")
                    .clicked()
                {
                    self.workspace.tab = 4;
                }
                if ui.add_enabled(!self.busy, primary("▶ Jugar")).clicked() {
                    self.play_instance(&p);
                }
            });
        });
        ui.add_space(12.);
        ui.separator();
        ui.horizontal(|ui| {
            for (i, label) in [
                "Contenido",
                "Archivos",
                "Mundos",
                "Registros",
                "Configuración",
            ]
            .iter()
            .enumerate()
            {
                ui.selectable_value(&mut self.workspace.tab, i, *label);
            }
        });
        ui.add_space(12.);
        match self.workspace.tab {
            0 => self.content_page(ui, &p),
            1 => self.files_page(ui, &p),
            2 => self.worlds_page(ui, &p),
            3 => self.logs_page(ui, &p),
            _ => self.instance_settings(ui, &p),
        }
        self.content_import(ui.ctx(), &p);
    }
    fn content_page(&mut self, ui: &mut egui::Ui, p: &GameProfile) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.workspace.query)
                    .hint_text("Buscar contenido…")
                    .desired_width((ui.available_width() - 260.).max(120.)),
            );
            if ui.button("+ Cargar archivos").clicked() {
                self.workspace.import_open = true;
            }
            if ui.button("Refrescar").clicked() {
                self.refresh_instance(p);
            }
        });
        ui.horizontal(|ui| {
            for (i, s) in ["Todo", "Mods", "Resource packs", "Shaders", "Data packs"]
                .iter()
                .enumerate()
            {
                ui.selectable_value(&mut self.workspace.filter, i, *s);
            }
        });
        let query = self.workspace.query.to_lowercase();
        let kind =
            ["Todo", "Mods", "Resource packs", "Shaders", "Data packs"][self.workspace.filter];
        let items: Vec<_> = self
            .workspace
            .items
            .iter()
            .filter(|x| {
                x.name.to_lowercase().contains(&query) && (kind == "Todo" || x.kind == kind)
            })
            .cloned()
            .collect();
        ui.small(format!(
            "{} elementos · Desactivar conserva el archivo. Quitar lo mueve a .luma/removed.",
            items.len()
        ));
        egui::ScrollArea::vertical().id_salt("instance_content").show(ui,|ui|{
            if items.is_empty(){card().show(ui,|ui|{ui.label("Esta vista está vacía. Añade tus archivos para personalizar la instancia.");});}
            for item in items {let response=card().inner_margin(12.).show(ui,|ui|{ui.set_min_width(ui.available_width());ui.horizontal(|ui|{ui.vertical(|ui|{ui.strong(&item.name);ui.label(RichText::new(&item.kind).small().color(palette().muted));});ui.with_layout(Layout::right_to_left(Align::Center),|ui|{let mut enabled=item.enabled;if ui.add_enabled_ui(!self.busy,|ui|switch(ui,&mut enabled)).inner.changed(){self.content_action(content::toggle(&p.game_dir,&item),p);}ui.menu_button("···",|ui|{self.content_menu(ui,p,&item);});});});}).response;
                response.context_menu(|ui|self.content_menu(ui,p,&item));ui.add_space(3.);
            }
        });
    }
    fn content_action(&mut self, result: anyhow::Result<()>, p: &GameProfile) {
        if let Err(e) = result {
            self.error = Some(e.to_string());
        }
        self.refresh_instance(p);
    }
    fn content_menu(&mut self, ui: &mut egui::Ui, p: &GameProfile, item: &content::Content) {
        if ui
            .add_enabled(
                !self.busy,
                egui::Button::new(if item.enabled {
                    "Desactivar"
                } else {
                    "Activar"
                }),
            )
            .clicked()
        {
            self.content_action(content::toggle(&p.game_dir, item), p);
            ui.close_menu();
        }
        if ui.button("Mostrar en carpeta").clicked() {
            self.open_path(item.path.parent().unwrap_or(&p.game_dir).into());
            ui.close_menu();
        }
        if ui.button("Copiar ruta").clicked() {
            ui.ctx().copy_text(item.path.display().to_string());
            ui.close_menu();
        }
        if ui
            .add_enabled(!self.busy, egui::Button::new("Quitar contenido"))
            .clicked()
        {
            self.content_action(content::archive(&p.game_dir, &item.path), p);
            ui.close_menu();
        }
    }
    fn content_import(&mut self, ctx: &egui::Context, p: &GameProfile) {
        if !self.workspace.import_open {
            return;
        }
        let mut open = true;
        egui::Window::new("Añadir contenido")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("Selecciona el tipo de contenido que vas a importar.");
                for (i, label) in [
                    "Mods (.jar)",
                    "Resource packs (.zip)",
                    "Shaders (.zip)",
                    "Data packs (.zip)",
                ]
                .iter()
                .enumerate()
                {
                    ui.radio_value(&mut self.workspace.import_kind, i, *label);
                }
                if self.workspace.import_kind == 3 {
                    egui::ComboBox::from_id_salt("import_world")
                        .selected_text(
                            self.workspace
                                .worlds
                                .get(self.workspace.import_world)
                                .map(|w| w.name.as_str())
                                .unwrap_or("Necesitas un mundo"),
                        )
                        .show_ui(ui, |ui| {
                            for (i, w) in self.workspace.worlds.iter().enumerate() {
                                ui.selectable_value(&mut self.workspace.import_world, i, &w.name);
                            }
                        });
                }
                if ui
                    .add_enabled(
                        !self.busy
                            && (self.workspace.import_kind != 3
                                || !self.workspace.worlds.is_empty()),
                        primary("Elegir archivos…"),
                    )
                    .clicked()
                {
                    let dir = match self.workspace.import_kind {
                        0 => p.game_dir.join("mods"),
                        1 => p.game_dir.join("resourcepacks"),
                        2 => p.game_dir.join("shaderpacks"),
                        _ => self.workspace.worlds[self
                            .workspace
                            .import_world
                            .min(self.workspace.worlds.len() - 1)]
                        .path
                        .join("datapacks"),
                    };
                    let extension = if self.workspace.import_kind == 0 {
                        "jar"
                    } else {
                        "zip"
                    };
                    if let Some(files) = rfd::FileDialog::new()
                        .add_filter("Contenido", &[extension])
                        .pick_files()
                    {
                        let result = (|| -> anyhow::Result<()> {
                            std::fs::create_dir_all(&dir)?;
                            for file in files {
                                let target = dir.join(
                                    file.file_name()
                                        .ok_or_else(|| anyhow::anyhow!("Nombre inválido"))?,
                                );
                                anyhow::ensure!(!target.exists(), "Ya existe {}", target.display());
                                std::fs::copy(file, target)?;
                            }
                            Ok(())
                        })();
                        self.content_action(result, p);
                        self.workspace.import_open = false;
                    }
                }
            });
        if !open {
            self.workspace.import_open = false;
        }
    }
    fn worlds_page(&mut self, ui: &mut egui::Ui, p: &GameProfile) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.workspace.query).hint_text("Buscar mundos…"),
            );
            if ui.button("Refrescar").clicked() {
                self.refresh_instance(p);
            }
        });
        egui::ScrollArea::vertical()
            .id_salt("worlds")
            .show(ui, |ui| {
                if self.workspace.worlds.is_empty() {
                    ui.label("Aún no hay mundos guardados en esta instancia.");
                }
                for world in self.workspace.worlds.clone() {
                    if !world
                        .name
                        .to_lowercase()
                        .contains(&self.workspace.query.to_lowercase())
                    {
                        continue;
                    }
                    let response = card()
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.strong(&world.name);
                                    ui.label(RichText::new(&world.mode).color(palette().muted));
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ui.add_enabled(!self.busy, primary("▶ Jugar")).clicked() {
                                        self.workspace.pending_world = Some(world.folder.clone());
                                        self.play_instance(p);
                                    }
                                    if ui.button("Carpeta").clicked() {
                                        self.open_path(world.path.clone());
                                    }
                                });
                            });
                        })
                        .response;
                    response.context_menu(|ui| {
                        if ui.button("Abrir carpeta").clicked() {
                            self.open_path(world.path.clone());
                            ui.close_menu();
                        }
                        if ui.button("Copiar ruta").clicked() {
                            ui.ctx().copy_text(world.path.display().to_string());
                            ui.close_menu();
                        }
                    });
                }
            });
    }
    fn files_page(&mut self, ui: &mut egui::Ui, p: &GameProfile) {
        let path = self.workspace.file_path.clone();
        ui.horizontal(|ui| {
            if ui
                .add_enabled(path != p.game_dir, egui::Button::new("↑ Subir"))
                .clicked()
            {
                if let Some(parent) = path.parent() {
                    self.workspace.file_path = parent.into();
                }
            }
            if ui.button("Abrir en Explorador").clicked() {
                self.open_path(path.clone());
            }
            ui.label(
                RichText::new(
                    path.strip_prefix(&p.game_dir)
                        .unwrap_or(&path)
                        .display()
                        .to_string(),
                )
                .small(),
            );
        });
        egui::ScrollArea::vertical()
            .id_salt("instance_files")
            .show(ui, |ui| {
                let mut entries: Vec<_> = std::fs::read_dir(&path)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .collect();
                entries.sort_by_key(|e| (!e.path().is_dir(), e.file_name()));
                for e in entries {
                    let is_dir = e.file_type().is_ok_and(|t| t.is_dir() && !t.is_symlink());
                    let response = ui.selectable_label(
                        false,
                        format!(
                            "{}  {}",
                            if is_dir { "▣" } else { "▤" },
                            e.file_name().to_string_lossy()
                        ),
                    );
                    if response.double_clicked() {
                        if is_dir {
                            self.workspace.file_path = e.path();
                        } else {
                            self.open_path(e.path());
                        }
                    }
                    response.context_menu(|ui| {
                        if ui.button("Abrir").clicked() {
                            self.open_path(e.path());
                            ui.close_menu();
                        }
                        if ui.button("Copiar ruta").clicked() {
                            ui.ctx().copy_text(e.path().display().to_string());
                            ui.close_menu();
                        }
                    });
                }
            });
    }
    fn logs_page(&mut self, ui: &mut egui::Ui, p: &GameProfile) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.workspace.log_query)
                    .hint_text("Buscar en registros…"),
            );
            if ui.button("Abrir archivo…").clicked() {
                if let Some(file) = rfd::FileDialog::new()
                    .set_directory(p.game_dir.join("logs"))
                    .add_filter("Registros", &["log", "txt"])
                    .pick_file()
                {
                    self.workspace.log_file = file;
                    self.workspace.log_time = None;
                }
            }
            if ui.button("En vivo").clicked() {
                self.workspace.log_file = p.game_dir.join(".luma/logs/game-latest.log");
                self.workspace.log_time = None;
            }
        });
        ui.horizontal(|ui| {
            for (i, label) in ["Todo", "Error", "Warn", "Info"].iter().enumerate() {
                ui.selectable_value(&mut self.workspace.log_filter, i, *label);
            }
            if ui.button("Copiar").clicked() {
                ui.ctx().copy_text(self.workspace.log_text.clone());
            }
        });
        if self.workspace.log_file.as_os_str().is_empty() {
            self.workspace.log_file = p.game_dir.join(".luma/logs/game-latest.log");
        }
        if self
            .workspace
            .log_time
            .is_none_or(|t| t.elapsed() > Duration::from_millis(500))
        {
            self.workspace.log_text = content::tail(&self.workspace.log_file)
                .or_else(|_| content::tail(&p.game_dir.join("logs/latest.log")))
                .unwrap_or_else(|_| "Inicia la instancia para recibir registros en vivo.".into());
            self.workspace.log_time = Some(Instant::now());
        }
        let query = self.workspace.log_query.to_lowercase();
        let severity = ["", "error", "warn", "info"][self.workspace.log_filter];
        egui::Frame::new()
            .fill(palette().panel)
            .corner_radius(12.)
            .inner_margin(16.)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                egui::ScrollArea::both()
                    .id_salt("live_logs")
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in self.workspace.log_text.lines().filter(|l| {
                            let l = l.to_lowercase();
                            l.contains(&query) && l.contains(severity)
                        }) {
                            let color = if line.to_lowercase().contains("error") {
                                Color32::from_rgb(245, 110, 120)
                            } else {
                                palette().muted
                            };
                            ui.label(RichText::new(line).monospace().size(12.).color(color));
                        }
                    });
            });
        ui.ctx().request_repaint_after(Duration::from_millis(500));
    }
    fn instance_settings(&mut self, ui: &mut egui::Ui, p: &GameProfile) {
        let mut edit = self.workspace.editing.clone().unwrap_or_else(|| p.clone());
        let mut runtime = edit
            .runtime
            .clone()
            .unwrap_or_else(|| profiles::default_runtime(&self.root));
        ui.horizontal_top(|ui|{ui.vertical(|ui|{ui.set_width(145.);for(i,label)in ["General","Instalación","Java y memoria","Ventana","Comandos de inicio"].iter().enumerate(){ui.selectable_value(&mut self.workspace.settings_tab,i,*label);}});ui.separator();
            egui::ScrollArea::vertical().id_salt("instance_settings").show(ui,|ui|{ui.with_layout(Layout::top_down(Align::Min),|ui|{ui.set_width(ui.available_width());match self.workspace.settings_tab{
                0=>{ui.heading("General");ui.label("Nombre de la instancia");ui.text_edit_singleline(&mut edit.name);ui.label(RichText::new(p.game_dir.display().to_string()).small().color(palette().muted));if ui.button("Abrir carpeta").clicked(){self.instance_folder(p,false);}}
                1=>{ui.heading("Editar instalación");ui.horizontal_wrapped(|ui|{for loader in Loader::ALL{ui.selectable_value(&mut edit.loader,loader,loader.label());}});ui.label("Versión de Minecraft");
                    egui::ComboBox::from_id_salt("edit_mc").selected_text(&edit.minecraft).show_ui(ui,|ui|{for v in &self.manifest.versions{ui.selectable_value(&mut edit.minecraft,v.id.clone(),&v.id);}});
                    ui.label("Versión del modloader");ui.add_enabled(edit.loader!=Loader::Vanilla,egui::TextEdit::singleline(&mut edit.loader_version).hint_text("Elige una versión compatible"));
                    if ui.add_enabled(!self.busy&&edit.loader!=Loader::Vanilla,egui::Button::new("Consultar versiones compatibles")).clicked(){let loader=edit.loader;let mc=edit.minecraft.clone();self.workspace.loader_key=Some((loader,mc.clone()));let tx=self.tx.clone();std::thread::spawn(move||{match profiles::loader_versions(loader,&mc){Ok(v)=>{let _=tx.send(Event::EditorLoaderVersions(v));}Err(e)=>{let _=tx.send(Event::Error(e.to_string()));}}});}
                    if self.workspace.loader_key.as_ref()==Some(&(edit.loader,edit.minecraft.clone())){egui::ComboBox::from_id_salt("edit_loader").selected_text(&edit.loader_version).show_ui(ui,|ui|{for v in &self.workspace.loader_choices{ui.selectable_value(&mut edit.loader_version,v.clone(),v);}});}
                    ui.add_space(12.);if ui.add_enabled(!self.busy&&!self.settings.offline,primary("Guardar e instalar")).clicked(){let request=ProfileRequest{name:edit.name.clone(),minecraft:edit.minecraft.clone(),loader:edit.loader,loader_version:edit.loader_version.clone()};let root=self.root.clone();let settings=self.settings.clone();let profile=edit.clone();self.work(move|tx|profiles::update_installation(&root,profile,&request,&settings,&tx));}
                    ui.separator();ui.heading("Reparar instancia");ui.label("Verifica los archivos descargados y reinstala las dependencias del juego y del loader. Conserva los mundos y el contenido.");if ui.add_enabled(!self.busy&&!self.settings.offline,egui::Button::new("Reparar")).clicked(){let request=ProfileRequest{name:p.name.clone(),minecraft:p.minecraft.clone(),loader:p.loader,loader_version:p.loader_version.clone()};let root=self.root.clone();let settings=self.settings.clone();let profile=p.clone();self.work(move|tx|profiles::update_installation(&root,profile,&request,&settings,&tx));}
                }
                2=>{ui.heading("Java y memoria");runtime_editor(ui,&mut runtime);}
                3=>{ui.heading("Ventana del juego");window_editor(ui,&mut runtime);}
                _=>{ui.heading("Comandos de inicio");ui.label("Se ejecutan en la carpeta de la instancia mediante cmd.exe.");ui.label("Antes de iniciar");ui.text_edit_multiline(&mut runtime.before);ui.label("Después de cerrar");ui.text_edit_multiline(&mut runtime.after);}
            }
            if self.workspace.settings_tab!=1{ui.add_space(16.);if ui.add_enabled(!self.busy,primary("Guardar cambios")).clicked(){let mut list=self.installations.clone();if let Some(item)=list.iter_mut().find(|x|x.id==edit.id){if self.workspace.settings_tab==0{item.name=edit.name.clone();}else{item.runtime=Some(runtime.clone());}}match profiles::save(&self.root,&list){Ok(())=>{self.installations=list;self.status="Configuración de instancia guardada".into();},Err(e)=>self.error=Some(e.to_string())}}}
        });});});
        edit.runtime = Some(runtime);
        self.workspace.editing = Some(edit);
    }
    pub(super) fn launcher_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Configuración");
        ui.add_space(12.);
        ui.horizontal_top(|ui|{ui.vertical(|ui|{ui.set_width(150.);for(i,label)in ["Apariencia","Comportamiento","Cuentas","Java y memoria","Ventana","Actualizaciones"].iter().enumerate(){ui.selectable_value(&mut self.workspace.global_tab,i,*label);}});ui.separator();
        egui::ScrollArea::vertical().id_salt("launcher_settings").show(ui,|ui|{ui.with_layout(Layout::top_down(Align::Min),|ui|{ui.set_width(ui.available_width());match self.workspace.global_tab{
            0=>{ui.heading("Apariencia");ui.add_space(16.);ui.horizontal_wrapped(|ui|{for(i,name)in ["Oscuro","Claro","OLED","Sistema"].iter().enumerate(){let fill=match i{1=>Color32::from_gray(235),2=>Color32::BLACK,_=>Color32::from_gray(35)};egui::Frame::new().fill(fill).stroke(Stroke::new(1.0_f32,if self.settings.appearance.theme==i{MINT}else{palette().border})).corner_radius(12.).inner_margin(18.).show(ui,|ui|{ui.set_min_width(90.);ui.add_space(28.);if ui.selectable_label(self.settings.appearance.theme==i,RichText::new(*name).color(if i==1{Color32::BLACK}else{Color32::WHITE})).clicked(){self.settings.appearance.theme=i;}});}});ui.add_space(18.);switch_row(ui,&mut self.settings.appearance.advanced,"Renderizado avanzado");ui.small("Activa sombras y animaciones de la interfaz.");ui.add_space(12.);switch_row(ui,&mut self.settings.appearance.decorations,"Marco de ventana del sistema");}
            1=>{ui.heading("Comportamiento");ui.add_space(14.);switch_row(ui,&mut self.settings.behavior.minimize,"Minimizar Luma al iniciar Minecraft");switch_row(ui,&mut self.settings.behavior.refocus,"Reenfocar Luma al cerrar Minecraft");switch_row(ui,&mut self.settings.behavior.sidebar_button,"Mostrar botón para ocultar la barra lateral");switch_row(ui,&mut self.settings.behavior.compact,"Biblioteca en modo compacto");switch_row(ui,&mut self.settings.behavior.playtime,"Mostrar tiempo de juego");if ui.button("Restaurar tarjetas ocultas de Inicio").clicked(){for s in self.settings.instance_styles.values_mut(){s.hidden_home=false;}}}
            2=>{self.account_manager(ui);ui.separator();self.legacy_settings_page(ui);}
            3=>{ui.heading("Java y memoria predeterminados");ui.label("Solo las nuevas instancias adoptan estos valores. Cada instancia conserva su configuración.");ui.add(egui::Slider::new(&mut self.settings.memory_mb,512..=32768).step_by(256.).suffix(" MB"));ui.label("Java personalizado (vacío = descarga automática)");ui.text_edit_singleline(&mut self.settings.java);ui.separator();ui.label("Argumentos Java predeterminados");ui.text_edit_multiline(&mut self.settings.runtime.jvm_args);ui.label("Variables de entorno (NOMBRE=valor por línea)");ui.text_edit_multiline(&mut self.settings.runtime.environment);}
            4=>{ui.heading("Ventana predeterminada para nuevas instancias");window_editor(ui,&mut self.settings.runtime);}
            _=>{ui.heading("Actualizaciones");ui.label(format!("Luma {} · GitHub Releases",env!("CARGO_PKG_VERSION")));if ui.add_enabled(!self.busy,egui::Button::new("Buscar actualizaciones")).clicked(){self.work(|tx|{let release=distribution::latest()?;if distribution::newer(&release.version,env!("CARGO_PKG_VERSION")){tx.send(Event::UpdateAvailable(release))?;}else{tx.send(Event::Status("Ya tienes la última versión".into()))?;}Ok(())});}
if let Some(release)=self.update_release.clone(){ui.label(format!("Nueva versión disponible: {}",release.version));if ui.add_enabled(!self.busy,primary("Descargar y actualizar")).clicked(){self.download_update(release);}}}
        }ui.add_space(20.);if ui.button("Guardar configuración").clicked(){self.save_settings();self.status="Configuración guardada".into();}
        });});});
    }
    pub(super) fn account_switcher(&mut self, ui: &mut egui::Ui) {
        let name = self
            .settings
            .profile
            .as_ref()
            .map(|p| p.name.as_str())
            .unwrap_or("Jugador")
            .to_owned();
        egui::ComboBox::from_id_salt("account_switcher")
            .width(190.)
            .selected_text(name)
            .show_ui(ui, |ui| {
                for account in self.settings.accounts.clone() {
                    if ui
                        .selectable_label(
                            self.settings
                                .profile
                                .as_ref()
                                .is_some_and(|p| p.id == account.id),
                            format!(
                                "{} · {}",
                                account.name,
                                if account.local { "Local" } else { "Microsoft" }
                            ),
                        )
                        .clicked()
                    {
                        if account.local {
                            self.settings.local_name = account.name.clone();
                        }
                        self.session = self.sessions.get(&account.id).cloned();
                        self.settings.profile = Some(account);
                        self.save_settings();
                    }
                }
                if ui.button("+ Añadir cuenta").clicked() {
                    self.page = Page::Settings;
                    self.workspace.global_tab = 2;
                }
            });
    }
    fn account_manager(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tus cuentas");
        self.account_switcher(ui);
        for account in self.settings.accounts.clone() {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{} · {}",
                    account.name,
                    if account.local {
                        "Local (offline)"
                    } else {
                        "Microsoft"
                    }
                ));
                if ui.small_button("Quitar").clicked() {
                    self.settings.accounts.retain(|p| p.id != account.id);
                    self.sessions.remove(&account.id);
                    if self
                        .settings
                        .profile
                        .as_ref()
                        .is_some_and(|p| p.id == account.id)
                    {
                        self.settings.profile =
                            model::Profile::offline(&self.settings.local_name).ok();
                        self.session = None;
                    }
                    self.save_settings();
                }
            });
        }
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.workspace.new_account)
                    .hint_text("Nombre de jugador local"),
            );
            if ui.button("Añadir cuenta local").clicked() {
                match model::Profile::offline(&self.workspace.new_account) {
                    Ok(p) => {
                        if !self.settings.accounts.iter().any(|a| a.id == p.id) {
                            self.settings.accounts.push(p.clone());
                        }
                        self.settings.local_name = p.name.clone();
                        self.settings.profile = Some(p);
                        self.session = None;
                        self.save_settings();
                    }
                    Err(e) => self.error = Some(e.to_string()),
                }
            }
        });
    }
    pub(super) fn download_update(&mut self, release: distribution::Release) {
        let target = self
            .root
            .join("updates")
            .join(format!("Luma-{}.exe", release.version));
        self.work(move |tx| {
            let (progress, rx) = mpsc::channel();
            let events = tx.clone();
            std::thread::spawn(move || {
                for message in rx {
                    let _ = events.send(Event::Status(message));
                }
            });
            distribution::download(&release, &target, &progress)?;
            tx.send(Event::UpdateReady(target))?;
            Ok(())
        });
    }
    pub(super) fn screenshots_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Capturas de pantalla");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.workspace.shot_query)
                    .hint_text("Buscar por archivo o instancia…")
                    .desired_width(400.),
            );
            if ui.button("Refrescar").clicked() {
                self.workspace.shots = content::screenshots(&self.installations);
            }
        });
        if self.workspace.shots.is_empty() {
            self.workspace.shots = content::screenshots(&self.installations);
        }
        let query = self.workspace.shot_query.to_lowercase();
        let shots: Vec<_> = self
            .workspace
            .shots
            .iter()
            .filter(|s| {
                format!("{} {}", s.instance, s.path.display())
                    .to_lowercase()
                    .contains(&query)
            })
            .cloned()
            .collect();
        ui.small(format!("{} capturas · Más recientes primero", shots.len()));
        egui::ScrollArea::vertical()
            .id_salt("screenshots")
            .show(ui, |ui| {
                let cols = ((ui.available_width() / 270.).floor() as usize).max(1);
                for row in shots.chunks(cols) {
                    ui.horizontal(|ui| {
                        for shot in row {
                            ui.vertical(|ui| {
                                ui.set_width(250.);
                                if !self.workspace.shot_textures.contains_key(&shot.path) {
                                    if let Ok(img) = image::open(&shot.path) {
                                        let img = img.thumbnail(500, 320).to_rgba8();
                                        let color = egui::ColorImage::from_rgba_unmultiplied(
                                            [img.width() as usize, img.height() as usize],
                                            img.as_raw(),
                                        );
                                        let texture = ui.ctx().load_texture(
                                            shot.path.display().to_string(),
                                            color,
                                            egui::TextureOptions::LINEAR,
                                        );
                                        self.workspace
                                            .shot_textures
                                            .insert(shot.path.clone(), texture);
                                    }
                                }
                                if let Some(texture) = self.workspace.shot_textures.get(&shot.path)
                                {
                                    let response = ui.add(
                                        egui::Image::new(texture)
                                            .fit_to_exact_size(Vec2::new(250., 145.))
                                            .corner_radius(10.)
                                            .sense(Sense::click()),
                                    );
                                    if response.clicked() {
                                        self.open_path(shot.path.clone());
                                    }
                                    response.context_menu(|ui| {
                                        if ui.button("Abrir captura").clicked() {
                                            self.open_path(shot.path.clone());
                                            ui.close_menu();
                                        }
                                        if ui.button("Abrir carpeta").clicked() {
                                            self.open_path(shot.path.parent().unwrap().into());
                                            ui.close_menu();
                                        }
                                        if ui.button("Copiar ruta").clicked() {
                                            ui.ctx().copy_text(shot.path.display().to_string());
                                            ui.close_menu();
                                        }
                                    });
                                }
                                ui.strong(&shot.instance);
                                ui.label(
                                    RichText::new(
                                        shot.path.file_name().unwrap_or_default().to_string_lossy(),
                                    )
                                    .small()
                                    .color(palette().muted),
                                );
                            });
                        }
                    });
                }
            });
    }
    pub(super) fn workspace_windows(&mut self, ctx: &egui::Context) {
        if let Some(id) = self.workspace.remove.clone() {
            egui::Window::new("Eliminar instancia de la biblioteca")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(
                        "Sus mundos y archivos permanecerán en disco para que puedas recuperarlos.",
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Eliminar de la biblioteca").clicked() {
                            match profiles::remove(&self.root, &id) {
                                Ok(list) => {
                                    self.installations = list;
                                    self.page = Page::Profiles;
                                }
                                Err(e) => self.error = Some(e.to_string()),
                            }
                            self.workspace.remove = None;
                        }
                        if ui.button("Cancelar").clicked() {
                            self.workspace.remove = None;
                        }
                    });
                });
        }
    }
    pub(super) fn editor_loader_versions(&mut self, choices: Vec<String>) {
        self.workspace.loader_choices = choices;
    }
}
fn runtime_editor(ui: &mut egui::Ui, r: &mut model::RuntimeOptions) {
    ui.label("Memoria máxima");
    ui.horizontal(|ui| {
        ui.add(
            egui::Slider::new(&mut r.memory_mb, 512..=32768)
                .step_by(256.)
                .suffix(" MB"),
        );
        ui.add(egui::DragValue::new(&mut r.memory_mb).range(512..=32768));
    });
    ui.add_space(12.);
    ui.label("Instalación de Java personalizada");
    ui.add(
        egui::TextEdit::singleline(&mut r.java)
            .hint_text("Automático: descargar Java adecuado")
            .desired_width(440.),
    );
    ui.horizontal(|ui| {
        if ui.button("Buscar java.exe…").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Java", &["exe"])
                .pick_file()
            {
                r.java = path.display().to_string();
            }
        }
        if ui.button("Detectar").clicked() {
            if let Ok(output) = std::process::Command::new("where.exe")
                .arg("java.exe")
                .output()
            {
                if let Some(path) = String::from_utf8_lossy(&output.stdout).lines().next() {
                    r.java = path.into();
                }
            }
        }
        if ui.button("Usar Java automático").clicked() {
            r.java.clear();
        }
    });
    ui.add_space(12.);
    ui.label("Argumentos Java personalizados");
    ui.text_edit_multiline(&mut r.jvm_args);
    ui.label("Variables de entorno · NOMBRE=valor por línea");
    ui.text_edit_multiline(&mut r.environment);
}
fn window_editor(ui: &mut egui::Ui, r: &mut model::RuntimeOptions) {
    ui.horizontal(|ui| {
        ui.label("Ancho");
        ui.add(egui::DragValue::new(&mut r.width).range(320..=16384));
        ui.label("Alto");
        ui.add(egui::DragValue::new(&mut r.height).range(240..=16384));
    });
    switch_row(ui, &mut r.fullscreen, "Pantalla completa");
}

fn switch(ui: &mut egui::Ui, value: &mut bool) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(Vec2::new(44., 24.), Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), *value, "")
    });
    let t = ui.ctx().animate_bool(response.id, *value);
    let fill = if *value { MINT } else { palette().hover };
    ui.painter().rect_filled(
        rect,
        12.,
        if ui.is_enabled() {
            fill
        } else {
            fill.gamma_multiply(0.4)
        },
    );
    ui.painter().circle_filled(
        Pos2::new(
            egui::lerp(rect.left() + 12. ..=rect.right() - 12., t),
            rect.center().y,
        ),
        8.,
        if *value {
            Color32::from_rgb(20, 55, 35)
        } else {
            palette().muted
        },
    );
    response
}
fn switch_row(ui: &mut egui::Ui, value: &mut bool, label: &str) -> egui::Response {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| switch(ui, value))
            .inner
    })
    .inner
}
