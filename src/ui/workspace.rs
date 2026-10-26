use super::*;
use crate::{content, distribution};
#[derive(Default)]
pub(super) struct Workspace {
    pub tab: usize,
    pub settings_tab: usize,
    pub(super) settings_open: bool,
    pub global_tab: usize,
    empty_mascot: Option<egui::TextureHandle>,
    pub pending_world: Option<String>,
    pub import_open: bool,
    pub remove: Option<String>,
    pub remove_files: bool,
    pub deleting_instance: bool,
    remove_ack: bool,
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
    selected_content: HashSet<PathBuf>,
    pub(super) content_info: std::collections::HashMap<PathBuf, crate::content_meta::Info>,
    content_textures: std::collections::HashMap<PathBuf, egui::TextureHandle>,
    content_stamp: Vec<(PathBuf, u64, u64)>,
    pub(super) metadata_generation: u64,
    sort_desc: bool,
    worlds: Vec<content::World>,
    shots: Vec<content::Screenshot>,
    recent_worlds: std::collections::HashMap<String, (Instant, Option<content::World>)>,
    refreshed: Option<Instant>,
    file_path: PathBuf,
    editing: Option<GameProfile>,
    loader_choices: Vec<String>,
    stable_loaders: HashSet<String>,
    all_mc: bool,
    fabric_builds: bool,
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
        if std::env::var_os("LUMA_PREVIEW").is_some()
            && std::env::var_os("LUMA_PREVIEW_SELECT_ALL").is_some()
        {
            self.workspace
                .selected_content
                .extend(self.workspace.items.iter().map(|i| i.path.clone()));
        }
    }
    pub(super) fn refresh_instance(&mut self, p: &GameProfile) {
        self.workspace.items = content::contents(&p.game_dir);
        self.workspace
            .selected_content
            .retain(|path| self.workspace.items.iter().any(|item| item.path == *path));
        let stamp = self
            .workspace
            .items
            .iter()
            .map(|item| {
                let m = std::fs::metadata(&item.path).ok();
                (
                    item.path.clone(),
                    m.as_ref().map(|m| m.len()).unwrap_or(0),
                    m.and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_nanos() as u64)
                        .unwrap_or(0),
                )
            })
            .collect::<Vec<_>>();
        if self.workspace.content_stamp != stamp {
            self.workspace.content_stamp = stamp;
            self.load_content_metadata(p, false);
        }
        self.workspace.worlds = content::worlds(&p.game_dir);
        self.workspace.refreshed = Some(Instant::now());
    }
    fn load_content_metadata(&mut self, p: &GameProfile, force: bool) {
        self.workspace.metadata_generation += 1;
        let generation = self.workspace.metadata_generation;
        let profile = p.clone();
        let items = self.workspace.items.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let _ = crate::content_meta::load(&profile.game_dir, &items, force, |info| {
                let _ = tx.send(Event::ContentInfo(profile.id.clone(), generation, info));
            });
        });
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
        if self.is_running(p) {
            self.play_instance(p);
            return;
        }
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
                    self.workspace.settings_open = true;
                }
                if ui
                    .add_enabled(
                        !self.busy || self.is_running(&p),
                        primary(self.play_label(&p)),
                    )
                    .clicked()
                {
                    self.play_instance(&p);
                }
            });
        });
        ui.add_space(12.);
        ui.separator();
        egui::Frame::new()
            .fill(palette().panel)
            .stroke(Stroke::new(1.0_f32, palette().border))
            .corner_radius(24)
            .inner_margin(5.)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (i, label) in ["Contenido", "Archivos", "Mundos", "Registros"]
                        .iter()
                        .enumerate()
                    {
                        if pill_choice(ui, self.workspace.tab == i, label).clicked() {
                            self.workspace.tab = i;
                        }
                    }
                });
            });
        ui.add_space(12.);
        match self.workspace.tab {
            0 => self.content_page(ui, &p),
            1 => self.files_page(ui, &p),
            2 => self.worlds_page(ui, &p),
            3 => self.logs_page(ui, &p),
            _ => self.logs_page(ui, &p),
        }
        self.content_import(ui.ctx(), &p);
    }
    fn content_page(&mut self, ui: &mut egui::Ui, p: &GameProfile) {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.workspace.query)
                    .hint_text("Buscar proyectos…")
                    .desired_width((ui.available_width() - 350.).max(120.))
                    .margin(Vec2::new(10., 10.)),
            );
            if ui.button("Cargar archivos").clicked() {
                self.workspace.import_open = true;
            }
            if ui
                .add_enabled(!self.busy, primary("Explorar contenido"))
                .clicked()
            {
                let kind = match self.workspace.filter {
                    2 => crate::catalog::Kind::ResourcePack,
                    3 => crate::catalog::Kind::Shader,
                    _ => crate::catalog::Kind::Mod,
                };
                self.open_catalog(p, kind, None);
            }
        });
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("content_order")
                .selected_text(if self.workspace.sort_desc {
                    "Nombre (Z-A)"
                } else {
                    "Nombre (A-Z)"
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.workspace.sort_desc, false, "Nombre (A-Z)");
                    ui.selectable_value(&mut self.workspace.sort_desc, true, "Nombre (Z-A)");
                });
            for (i, label) in ["Todo", "Mods", "Resource packs", "Shaders", "Data packs"]
                .iter()
                .enumerate()
            {
                if pill_choice(ui, self.workspace.filter == i, label).clicked() {
                    self.workspace.filter = i;
                }
            }
            if ui.button("Refrescar").clicked() {
                self.refresh_instance(p);
                self.load_content_metadata(p, true);
            }
        });
        ui.add_space(8.);
        let query = self.workspace.query.to_lowercase();
        let kind =
            ["Todo", "Mods", "Resource packs", "Shaders", "Data packs"][self.workspace.filter];
        let mut items = self
            .workspace
            .items
            .iter()
            .filter(|item| {
                let title = self
                    .workspace
                    .content_info
                    .get(&item.path)
                    .map(|m| m.title.as_str())
                    .unwrap_or(&item.name);
                (item.name.to_lowercase().contains(&query) || title.to_lowercase().contains(&query))
                    && (kind == "Todo" || item.kind == kind)
            })
            .cloned()
            .collect::<Vec<_>>();
        items.sort_by_cached_key(|item| {
            self.workspace
                .content_info
                .get(&item.path)
                .map(|m| m.title.to_lowercase())
                .unwrap_or_else(|| item.name.to_lowercase())
        });
        if self.workspace.sort_desc {
            items.reverse();
        }
        self.workspace
            .selected_content
            .retain(|path| items.iter().any(|i| i.path == *path));
        if items.is_empty() {
            self.empty_content(ui, self.workspace.items.is_empty());
            return;
        }
        let selected = self.workspace.selected_content.len();
        let width = ui.available_width();
        let project_width = ((width - 200.) * 0.53).max(140.);
        let version_width = (width - project_width - 200.).max(110.);
        egui::Frame::new().fill(palette().card).stroke(Stroke::new(1.0_f32,palette().border)).corner_radius(16).inner_margin(0.).show(ui,|ui|{ui.spacing_mut().item_spacing.y=0.;
            egui::Frame::new().fill(palette().panel).corner_radius(egui::CornerRadius{nw:15,ne:15,sw:0,se:0}).inner_margin(12.).show(ui,|ui|{ui.set_width(width-24.);ui.horizontal(|ui|{
                let mut all=!items.is_empty() && items.iter().all(|i|self.workspace.selected_content.contains(&i.path));
                if selection_box(ui,&mut all).on_hover_text("Seleccionar todos los proyectos visibles").changed(){if all{self.workspace.selected_content.extend(items.iter().map(|i|i.path.clone()));}else{self.workspace.selected_content.clear();}}
                for (label,width) in [("Proyecto",project_width),("Versión",version_width)]{let(rect,_)=ui.allocate_exact_size(Vec2::new(width,22.),Sense::hover());ui.painter().text(rect.left_center(),egui::Align2::LEFT_CENTER,label,egui::FontId::proportional(14.),palette().text);}
                ui.with_layout(Layout::right_to_left(Align::Center),|ui|{ui.label(RichText::new("Acciones").strong().color(palette().muted));});
            });});
            egui::Frame::new().inner_margin(egui::Margin{left:0,right:0,top:0,bottom:0}).show(ui,|ui|{
            egui::ScrollArea::vertical().id_salt("instance_content").max_height((ui.available_height()-if selected>0{84.}else{18.}).max(120.)).show(ui,|ui|{
                ui.spacing_mut().item_spacing.y=0.;
                if items.is_empty(){ui.add_space(24.);ui.label("No hay proyectos en esta vista. Añade contenido a la instancia.");ui.add_space(24.);}
                for (row,item) in items.iter().enumerate(){
                    let info=self.workspace.content_info.get(&item.path).cloned().unwrap_or_else(||crate::content_meta::Info{title:item.name.clone(),..Default::default()});
                    let chosen=self.workspace.selected_content.contains(&item.path);
                    let response=ui.scope_builder(egui::UiBuilder::new().sense(Sense::click()),|ui|{
                        egui::Frame::new().fill(if row % 2 == 0 {palette().card}else{palette().card.gamma_multiply(1.12)}).corner_radius(egui::CornerRadius{nw:0,ne:0,sw:if row+1==items.len(){15}else{0},se:if row+1==items.len(){15}else{0}}).inner_margin(egui::Margin{left:12,right:12,top:9,bottom:9}).show(ui,|ui|{ui.set_width(width-24.);ui.horizontal(|ui|{ui.set_height(40.);
                            let mut checked=chosen;if selection_box(ui,&mut checked).changed(){if checked{self.workspace.selected_content.insert(item.path.clone());}else{self.workspace.selected_content.remove(&item.path);}}
                            ui.allocate_ui_with_layout(Vec2::new(project_width,40.),Layout::left_to_right(Align::Center),|ui|{
                                self.content_icon(ui,&info,&item.kind);
                                ui.vertical(|ui|{ui.set_width((project_width-46.).max(80.));ui.add(egui::Label::new(RichText::new(&info.title).strong()).truncate());ui.add(egui::Label::new(RichText::new(if info.author.is_empty(){&item.kind}else{&info.author}).small().color(palette().muted)).truncate());});
                            });
                            ui.allocate_ui_with_layout(Vec2::new(version_width,40.),Layout::top_down(Align::Min),|ui|{ui.add(egui::Label::new(RichText::new(if info.version.is_empty(){"—"}else{&info.version}).strong()).truncate());ui.add(egui::Label::new(RichText::new(&item.name).color(palette().muted)).truncate());});
                            ui.with_layout(Layout::right_to_left(Align::Center),|ui|{
                                ui.menu_button("···",|ui|self.content_menu(ui,p,item));
                                if ui.add_enabled(!self.busy,egui::Button::new("🗑")).on_hover_text("Quitar proyecto; se conserva en .luma/removed").clicked(){self.content_action(content::archive(&p.game_dir,&item.path),p);}
                                let mut enabled=item.enabled;if ui.add_enabled_ui(!self.busy,|ui|switch(ui,&mut enabled)).inner.changed(){self.content_action(content::toggle(&p.game_dir,item),p);}
                            });
                        });});
                    }).response;
                    response.context_menu(|ui|self.content_menu(ui,p,item));

                }
            });
            });
        });
        if !self.workspace.selected_content.is_empty() {
            ui.add_space(10.);
            egui::Frame::new()
                .fill(palette().panel)
                .stroke(Stroke::new(1.0_f32, palette().border))
                .corner_radius(16)
                .inner_margin(12.)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(format!(
                            "{} proyectos seleccionados",
                            self.workspace.selected_content.len()
                        ));
                        if ui.button("Limpiar selección").clicked() {
                            self.workspace.selected_content.clear();
                        }
                        ui.separator();
                        if ui
                            .add_enabled(!self.busy, egui::Button::new("Activar"))
                            .clicked()
                        {
                            self.bulk_content(p, Some(true));
                        }
                        if ui
                            .add_enabled(!self.busy, egui::Button::new("Desactivar"))
                            .clicked()
                        {
                            self.bulk_content(p, Some(false));
                        }
                        if ui
                            .add_enabled(
                                !self.busy,
                                egui::Button::new(
                                    RichText::new("Quitar").color(Color32::from_rgb(255, 90, 115)),
                                ),
                            )
                            .on_hover_text("Mueve los archivos seleccionados a .luma/removed")
                            .clicked()
                        {
                            self.bulk_content(p, None);
                        }
                    });
                });
        }
    }
    fn empty_content(&mut self, ui: &mut egui::Ui, empty: bool) {
        let english = self.settings.language == "en-US";
        let message = match (empty, english) {
            (true, true) => "You haven't added any content to this instance yet.",
            (true, false) => "Todavía no has agregado contenido a esta instancia.",
            (false, true) => "No content matches your search or filters.",
            (false, false) => "No hay contenido que coincida con tu búsqueda o filtros.",
        };
        let height = ui.available_height().max(240.);
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), height),
            Layout::top_down(Align::Center),
            |ui| {
                ui.add_space(((height - 250.) / 2.).max(12.));
                if empty {
                    if self.workspace.empty_mascot.is_none() {
                        if let Ok(image) = image::load_from_memory(include_bytes!(
                            "../../assets/empty-content.jpg"
                        )) {
                            let image = image.to_rgba8();
                            self.workspace.empty_mascot = Some(ui.ctx().load_texture(
                                "empty-content-mascot",
                                egui::ColorImage::from_rgba_unmultiplied(
                                    [image.width() as usize, image.height() as usize],
                                    image.as_raw(),
                                ),
                                egui::TextureOptions::LINEAR,
                            ));
                        }
                    }
                    if let Some(texture) = &self.workspace.empty_mascot {
                        ui.add(
                            egui::Image::new(texture)
                                .fit_to_exact_size(Vec2::new(129., 160.))
                                .corner_radius(10),
                        );
                    }
                    ui.add_space(14.);
                }
                ui.add(egui::Label::new(RichText::new(message).strong().size(18.)).wrap());
                ui.add_space(14.);
                if empty {
                    if ui
                        .add_enabled(
                            !self.busy,
                            primary(if english {
                                "+ Install content"
                            } else {
                                "+ Instalar contenido"
                            }),
                        )
                        .clicked()
                    {
                        self.workspace.import_open = true;
                    }
                } else if ui
                    .button(if english {
                        "Clear search and filters"
                    } else {
                        "Limpiar búsqueda y filtros"
                    })
                    .clicked()
                {
                    self.workspace.query.clear();
                    self.workspace.filter = 0;
                }
            },
        );
    }
    fn content_icon(&mut self, ui: &mut egui::Ui, info: &crate::content_meta::Info, kind: &str) {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(34.), Sense::hover());
        if let Some(path) = &info.icon {
            if !self.workspace.content_textures.contains_key(path) {
                if let Ok(image) = image::open(path) {
                    let image = image.to_rgba8();
                    let size = [image.width() as usize, image.height() as usize];
                    self.workspace.content_textures.insert(
                        path.clone(),
                        ui.ctx().load_texture(
                            path.display().to_string(),
                            egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
                            egui::TextureOptions::LINEAR,
                        ),
                    );
                }
            }
            if let Some(texture) = self.workspace.content_textures.get(path) {
                egui::Image::new((texture.id(), rect.size()))
                    .corner_radius(7)
                    .paint_at(ui, rect);
                return;
            }
        }
        ui.painter().rect_filled(rect, 7., palette().panel);
        super::art::glyph(
            ui.painter(),
            rect.shrink(8.),
            if kind == "Mods" {
                super::art::Glyph::Settings
            } else {
                super::art::Glyph::Image
            },
            palette().muted,
        );
    }
    fn bulk_content(&mut self, p: &GameProfile, enabled: Option<bool>) {
        let result =
            content::bulk(&p.game_dir, &self.workspace.selected_content, enabled).map(|_| ());
        self.workspace.selected_content.clear();
        self.content_action(result, p);
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
                ui.separator();
                let kind=crate::catalog::Kind::ALL[self.workspace.import_kind];
                let world=if kind==crate::catalog::Kind::DataPack{self.workspace.worlds.get(self.workspace.import_world).map(|w|w.path.clone())}else{None};
                let allowed=!self.busy&&(kind!=crate::catalog::Kind::DataPack||world.is_some());
                if kind==crate::catalog::Kind::Mod&&p.loader==Loader::Vanilla{ui.label("Para usar mods .jar, configura Fabric, Forge o NeoForge en la instancia.");}
                ui.horizontal_wrapped(|ui|{
                    if ui.add_enabled(allowed&&(kind!=crate::catalog::Kind::Mod||p.loader!=Loader::Vanilla),primary("Explorar Modrinth")).clicked(){self.open_catalog(p,kind,world.clone());self.workspace.import_open=false;}
                    for (label,curse) in [("Modrinth · Web",false),("CurseForge · Web",true)]{if ui.add_enabled(allowed,egui::Button::new(label)).clicked(){match crate::browser::open(p.clone(),kind,world.clone(),curse,self.tx.clone()){Ok(())=>self.workspace.import_open=false,Err(e)=>self.error=Some(e.to_string())}}}
                });
                let download_note = if cfg!(windows) {
                    "Las descargas van a la carpeta de este tipo de contenido en esta instancia."
                } else {
                    "Las webs se abren en tu navegador. Añade sus descargas con Elegir archivos…; el catálogo integrado instala directamente en esta instancia."
                };
                ui.label(RichText::new(download_note).small().color(palette().muted));
                ui.separator();
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
                                crate::catalog::safe_filename(&file.file_name().ok_or_else(||anyhow::anyhow!("Nombre inválido"))?.to_string_lossy(),kind)?;
                                content::validate_archive(&file)?;
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
                    let response = interactive_card(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.strong(&world.name);
                                ui.label(RichText::new(&world.mode).color(palette().muted));
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .add_enabled(
                                        !self.busy || self.is_running(p),
                                        primary(self.play_label(p)),
                                    )
                                    .clicked()
                                {
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
        ui.horizontal_top(|ui|{ui.vertical(|ui|{ui.set_width(180.);ui.label(RichText::new("INSTANCIA").small().color(palette().muted));for(i,label)in ["General","Instalación","Java y memoria","Ventana","Comandos de inicio"].iter().enumerate(){if i==2{ui.add_space(18.);ui.label(RichText::new("JUEGO").small().color(palette().muted));}ui.add_sized([180.,36.],egui::Button::new(*label).selected(self.workspace.settings_tab==i)).clicked().then(||self.workspace.settings_tab=i);}});ui.separator();
            egui::ScrollArea::vertical().id_salt("instance_settings").show(ui,|ui|{ui.with_layout(Layout::top_down(Align::Min),|ui|{ui.set_width(ui.available_width());match self.workspace.settings_tab{
                0=>{
                    ui.horizontal_top(|ui|{ui.vertical(|ui|{ui.label(RichText::new("Nombre").strong());ui.add(egui::TextEdit::singleline(&mut edit.name).desired_width((ui.available_width()-120.).max(180.)));ui.label(RichText::new(crate::cleanup::display_path(&p.game_dir)).small().color(palette().muted));if ui.button("Abrir carpeta").clicked(){self.instance_folder(p,false);}});ui.vertical(|ui|{ui.label(RichText::new("Ícono").strong());let(rect,_)=ui.allocate_exact_size(Vec2::splat(92.),Sense::hover());self.paint_cover(ui,rect,p);if ui.add_sized([104.,32.],egui::Button::new("Personalizar").wrap_mode(egui::TextWrapMode::Extend)).clicked(){self.library.cover_target=Some(p.id.clone());self.workspace.settings_open=false;}});});
                    ui.add_space(24.);ui.heading("Duplicar instancia");ui.label("Crea una copia con sus mundos, configuración y contenido.");
                    if ui.add_enabled(!self.busy&&!self.is_running(p),egui::Button::new("Duplicar")).clicked(){let root=self.root.clone();let profile=p.clone();self.work(move|tx|{tx.send(Event::Profiles(profiles::duplicate(&root,&profile)?))?;Ok(())});self.workspace.settings_open=false;self.page=Page::Home;}
                    ui.add_space(24.);ui.heading("Eliminar instancia");
                    if ui.add_enabled(!self.busy&&!self.is_running(p),egui::Button::new("Quitar de la biblioteca")).clicked(){self.workspace.remove=Some(p.id.clone());self.workspace.remove_files=false;self.workspace.remove_ack=false;self.workspace.settings_open=false;}
                    if ui.add_enabled(!self.busy&&!self.is_running(p),egui::Button::new(RichText::new("Eliminar permanentemente…").color(Color32::from_rgb(255,80,120)))).clicked(){self.workspace.remove=Some(p.id.clone());self.workspace.remove_files=true;self.workspace.remove_ack=false;self.workspace.settings_open=false;}
                }
                1=>{ui.heading("Editar instalación");if self.is_running(p){ui.label("Detén esta instancia para cambiar o reparar su instalación.");}ui.horizontal_wrapped(|ui|{for loader in Loader::ALL{ui.selectable_value(&mut edit.loader,loader,loader.label());}});ui.label("Versión de Minecraft");
                    egui::ComboBox::from_id_salt("edit_mc").close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside).selected_text(&edit.minecraft).show_ui(ui,|ui|{egui::ScrollArea::vertical().max_height(220.).show(ui,|ui|{for v in &self.manifest.versions{if (self.workspace.all_mc || v.kind=="release") && ui.selectable_value(&mut edit.minecraft,v.id.clone(),&v.id).clicked(){ui.close_menu();}}});ui.separator();if ui.button(if self.workspace.all_mc{"Mostrar solo versiones estables"}else{"Mostrar todas las versiones"}).clicked(){self.workspace.all_mc = !self.workspace.all_mc;}});
                    ui.label("Versión del modloader");ui.add_enabled(edit.loader!=Loader::Vanilla,egui::TextEdit::singleline(&mut edit.loader_version).hint_text("Elige una versión compatible"));
                    if ui.add_enabled(!self.busy&&edit.loader!=Loader::Vanilla,egui::Button::new("Consultar versiones compatibles")).clicked(){let loader=edit.loader;let mc=edit.minecraft.clone();self.workspace.loader_key=Some((loader,mc.clone()));let tx=self.tx.clone();std::thread::spawn(move||{match profiles::loader_versions_with_stability(loader,&mc){Ok((v,stable))=>{let _=tx.send(Event::EditorLoaderVersions(v,stable));}Err(e)=>{let _=tx.send(Event::Error(e.to_string()));}}});}
                    if edit.loader==Loader::Fabric{ui.horizontal(|ui|{ui.selectable_value(&mut self.workspace.fabric_builds,false,"Estables");ui.selectable_value(&mut self.workspace.fabric_builds,true,"Todas las builds");});}
                    if self.workspace.loader_key.as_ref()==Some(&(edit.loader,edit.minecraft.clone())){egui::ComboBox::from_id_salt("edit_loader").selected_text(&edit.loader_version).show_ui(ui,|ui|{for v in &self.workspace.loader_choices{if edit.loader==Loader::Fabric && !self.workspace.fabric_builds && !self.workspace.stable_loaders.contains(v){continue;}ui.selectable_value(&mut edit.loader_version,v.clone(),v);}});}
                    ui.add_space(12.);if ui.add_enabled(!self.busy&&!self.is_running(p),primary("Guardar e instalar")).clicked(){let request=ProfileRequest{name:edit.name.clone(),minecraft:edit.minecraft.clone(),loader:edit.loader,loader_version:edit.loader_version.clone()};let root=self.root.clone();let settings=self.settings.clone();let profile=edit.clone();self.work(move|tx|profiles::update_installation(&root,profile,&request,&settings,&tx));}
                    ui.separator();ui.heading("Reparar instancia");ui.label("Verifica los archivos descargados y reinstala las dependencias del juego y del loader. Conserva los mundos y el contenido.");if ui.add_enabled(!self.busy&&!self.is_running(p),egui::Button::new("Reparar")).clicked(){let request=ProfileRequest{name:p.name.clone(),minecraft:p.minecraft.clone(),loader:p.loader,loader_version:p.loader_version.clone()};let root=self.root.clone();let settings=self.settings.clone();let profile=p.clone();self.work(move|tx|profiles::update_installation(&root,profile,&request,&settings,&tx));}
                }
                2=>{ui.heading("Java y memoria");runtime_editor(ui,&mut runtime);}
                3=>{ui.heading("Ventana del juego");window_editor(ui,&mut runtime);}
                _=>{ui.heading("Comandos de inicio");ui.label(crate::platform::HOOK_DESCRIPTION);ui.label("Antes de iniciar");ui.text_edit_multiline(&mut runtime.before);ui.label("Después de cerrar");ui.text_edit_multiline(&mut runtime.after);}
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
            0=>{ui.heading("Apariencia");
                ui.label("Idioma / Language");
                egui::ComboBox::from_id_salt("language").selected_text(if self.settings.language == "en-US" { "English (United States)" } else { "Español (México)" }).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.settings.language, "es-MX".into(), "Español (México)");
                    ui.selectable_value(&mut self.settings.language, "en-US".into(), "English (United States)");
                });ui.add_space(16.);ui.horizontal_wrapped(|ui|{for(i,name)in ["Oscuro","Claro","OLED","Sistema"].iter().enumerate(){let fill=match i{1=>Color32::from_gray(235),2=>Color32::BLACK,_=>Color32::from_gray(35)};egui::Frame::new().fill(fill).stroke(Stroke::new(1.0_f32,if self.settings.appearance.theme==i{MINT}else{palette().border})).corner_radius(12.).inner_margin(18.).show(ui,|ui|{ui.set_min_width(90.);ui.add_space(28.);if ui.selectable_label(self.settings.appearance.theme==i,RichText::new(*name).color(if i==1{Color32::BLACK}else{Color32::WHITE})).clicked(){self.settings.appearance.theme=i;}});}});ui.add_space(18.);switch_row(ui,&mut self.settings.appearance.advanced,"Renderizado avanzado");ui.small("Activa sombras y animaciones de la interfaz.");ui.add_space(12.);switch_row(ui,&mut self.settings.appearance.decorations,"Marco de ventana del sistema");}
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
            .width(ui.available_width())
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
        let target = self.root.join("updates").join(format!(
            "Luma-{}{}",
            release.version,
            crate::platform::EXECUTABLE_SUFFIX
        ));
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
        if self.workspace.settings_open {
            if let Some(profile) = self.active_installation().cloned() {
                let mut close = false;
                let response = egui::Modal::new(egui::Id::new("instance_settings_modal"))
                    .backdrop_color(Color32::from_black_alpha(170))
                    .frame(
                        egui::Frame::new()
                            .fill(palette().card)
                            .stroke(Stroke::new(1.0_f32, palette().border))
                            .corner_radius(18)
                            .inner_margin(0.),
                    )
                    .show(ctx, |ui| {
                        let width = (ctx.screen_rect().width() - 60.).min(900.);
                        ui.set_width(width);
                        egui::Frame::new()
                            .fill(palette().panel)
                            .inner_margin(22.)
                            .show(ui, |ui| {
                                ui.set_width(width - 44.);
                                ui.horizontal(|ui| {
                                    let (rect, _) =
                                        ui.allocate_exact_size(Vec2::splat(32.), Sense::hover());
                                    self.paint_cover(ui, rect, &profile);
                                    ui.label(
                                        RichText::new(&profile.name)
                                            .strong()
                                            .color(palette().muted),
                                    );
                                    ui.label("›");
                                    ui.label(RichText::new("Configuración").size(20.).strong());
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if ui.button("×").clicked() {
                                            close = true;
                                        }
                                    });
                                });
                            });
                        ui.separator();
                        egui::Frame::new().inner_margin(22.).show(ui, |ui| {
                            ui.set_width(width - 44.);
                            ui.set_height((ctx.screen_rect().height() - 200.).clamp(300., 560.));
                            self.instance_settings(ui, &profile);
                        });
                    });
                if close || response.should_close() {
                    self.workspace.settings_open = false;
                }
            } else {
                self.workspace.settings_open = false;
            }
        }
        if let Some(id) = self.workspace.remove.clone() {
            let Some(profile) = self.installations.iter().find(|p| p.id == id).cloned() else {
                self.workspace.remove = None;
                return;
            };
            let deleting = self.workspace.remove_files;
            let target = profiles::deletion_directory(&self.root, &id);
            let mut close = false;
            let danger = Color32::from_rgb(255, 65, 110);
            let response=egui::Modal::new(egui::Id::new("remove_instance_modal")).backdrop_color(if deleting{Color32::from_rgba_unmultiplied(34,0,8,180)}else{Color32::from_black_alpha(170)}).frame(egui::Frame::new().fill(palette().card).stroke(Stroke::new(1.0_f32,palette().border)).corner_radius(18).inner_margin(24.)).show(ctx,|ui|{
                ui.set_width(450.);
                ui.horizontal(|ui|{ui.label(RichText::new(if deleting{"Eliminar instancia"}else{"Quitar de la biblioteca"}).size(23.).strong());ui.with_layout(Layout::right_to_left(Align::Center),|ui|{if ui.button("×").clicked(){close=true;}});});
                ui.add_space(14.);ui.separator();ui.add_space(14.);
                egui::Frame::new().fill(if deleting{Color32::from_rgb(79,43,53)}else{Color32::from_rgb(34,65,54)}).stroke(Stroke::new(1.0_f32,if deleting{danger}else{MINT})).corner_radius(14).inner_margin(16.).show(ui,|ui|{ui.set_width(418.);ui.strong(if deleting{"Esta acción no se puede deshacer"}else{"Tus mundos y archivos se conservarán"});ui.add_space(6.);ui.label(if deleting{"Se eliminarán permanentemente los mundos, configuraciones, mods, capturas y demás archivos de esta instancia."}else{"Solo se quitará la instancia de la biblioteca de Luma. Sus archivos permanecerán en disco y podrás volver a importarla."});});
                ui.add_space(20.);ui.strong("Instancia");
                card().fill(palette().bg).inner_margin(12.).show(ui,|ui|{ui.set_width(426.);ui.horizontal(|ui|{let(rect,_)=ui.allocate_exact_size(Vec2::splat(48.),Sense::hover());self.paint_cover(ui,rect,&profile);ui.vertical(|ui|{ui.strong(&profile.name);ui.label(RichText::new(format!("{} {}",profile.loader.label(),profile.minecraft)).color(palette().muted));});});});
                if deleting {
                    ui.add_space(10.);
                    match &target{Ok(path)=>{ui.add(egui::Label::new(RichText::new(crate::cleanup::display_path(path)).small().monospace().color(palette().muted)).wrap());},Err(e)=>{ui.colored_label(danger,e.to_string());}}
                    ui.checkbox(&mut self.workspace.remove_ack,"Confirmo que quiero borrar sus archivos");
                }
                if self.is_running(&profile){ui.label("Detén esta instancia antes de quitarla o eliminarla.");}
                ui.add_space(22.);
                ui.with_layout(Layout::right_to_left(Align::Center),|ui|{
                    let enabled=!self.busy&&!self.is_running(&profile)&&(!deleting||(target.is_ok()&&self.workspace.remove_ack));
                    let button=egui::Button::new(RichText::new(if deleting{"Eliminar instancia"}else{"Quitar de la biblioteca"}).strong().color(Color32::from_rgb(18,25,22))).fill(if deleting{danger}else{MINT}).min_size(Vec2::new(170.,36.));
                    if ui.add_enabled(enabled,button).clicked(){
                        let root=self.root.clone();let worker_id=id.clone();let confirmed=target.as_ref().ok().cloned();
                        self.work(move|tx|{let list=if deleting{profiles::remove_completely(&root,&worker_id,&confirmed.ok_or_else(||anyhow::anyhow!("Carpeta inválida"))?)?}else{profiles::remove(&root,&worker_id)?};tx.send(Event::Profiles(list))?;tx.send(Event::InstanceRemoved(worker_id))?;tx.send(Event::Status(if deleting{"Instancia y archivos eliminados"}else{"Instancia retirada; archivos conservados"}.into()))?;Ok(())});
                        self.workspace.deleting_instance=true;self.workspace.remove=None;self.workspace.remove_ack=false;self.page=Page::Home;
                    }
                    if ui.add(egui::Button::new("Cancelar").min_size(Vec2::new(100.,36.))).clicked(){close=true;}
                });
            });
            if close || response.should_close() {
                self.workspace.remove = None;
                self.workspace.remove_ack = false;
            }
        }
    }
    pub(super) fn editor_loader_versions(&mut self, choices: Vec<String>, stable: HashSet<String>) {
        self.workspace.loader_choices = choices;
        self.workspace.stable_loaders = stable;
    }
}
// Register the row's background before its children. Adding click sense to a
// Frame response after drawing its buttons puts the background over them.
fn interactive_card<R>(
    ui: &mut egui::Ui,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.scope_builder(egui::UiBuilder::new().sense(Sense::click()), |ui| {
        card().inner_margin(12.).show(ui, contents).inner
    })
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
        if ui.button("Buscar Java…").clicked() {
            let dialog = rfd::FileDialog::new();
            #[cfg(windows)]
            let dialog = dialog.add_filter("Java", &["exe"]);
            if let Some(path) = dialog.pick_file() {
                r.java = path.display().to_string();
            }
        }
        if ui.button("Detectar").clicked() {
            if let Some(path) = crate::platform::system_java() {
                r.java = path.display().to_string();
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

fn pill_choice(ui: &mut egui::Ui, selected: bool, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(if selected {
            MINT
        } else {
            palette().muted
        }))
        .fill(if selected {
            Color32::from_rgb(34, 65, 54)
        } else {
            Color32::TRANSPARENT
        })
        .stroke(if selected {
            Stroke::new(1.0_f32, MINT.gamma_multiply(0.45))
        } else {
            Stroke::NONE
        })
        .corner_radius(20)
        .min_size(Vec2::new(0., 30.)),
    )
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

fn selection_box(ui: &mut egui::Ui, value: &mut bool) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(Vec2::splat(20.), Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    ui.painter().rect(
        rect.shrink(2.),
        3.,
        if *value { MINT } else { palette().bg },
        Stroke::new(1.5_f32, palette().muted),
        egui::StrokeKind::Inside,
    );
    if *value {
        let stroke = Stroke::new(2.0_f32, palette().bg);
        ui.painter().line_segment(
            [rect.min + Vec2::new(5., 10.), rect.min + Vec2::new(9., 14.)],
            stroke,
        );
        ui.painter().line_segment(
            [rect.min + Vec2::new(9., 14.), rect.min + Vec2::new(15., 6.)],
            stroke,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Checkbox,
            ui.is_enabled(),
            *value,
            "Seleccionar proyecto",
        )
    });
    response
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    #[test]
    fn row_context_menu_does_not_cover_switch_play_or_overflow_menu() {
        let ctx = egui::Context::default();
        let mut enabled = false;
        let mut played = false;
        let mut overflow_open = false;
        let mut rects = [Rect::NOTHING; 3];
        let mut frame = |events: Vec<egui::Event>| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800., 600.))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let row = interactive_card(ui, |ui| {
                            ui.horizontal(|ui| {
                                let toggle = switch(ui, &mut enabled);
                                rects[0] = toggle.rect;
                                let play = ui.button("Jugar");
                                rects[1] = play.rect;
                                played |= play.clicked();
                                let menu = ui.menu_button("···", |ui| {
                                    overflow_open = true;
                                    ui.label("Acciones");
                                });
                                rects[2] = menu.response.rect;
                            });
                        });
                        row.response.context_menu(|ui| {
                            ui.label("Menú contextual");
                        });
                    });
                },
            );
            rects
        };
        let rects = frame(vec![]);
        for rect in rects {
            let pos = rect.center();
            frame(vec![egui::Event::PointerMoved(pos)]);
            for pressed in [true, false] {
                frame(vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }]);
            }
            frame(vec![]);
        }
        assert!(enabled, "el switch debe responder al clic izquierdo");
        assert!(played, "Jugar debe responder al clic izquierdo");
        assert!(overflow_open, "los tres puntos deben abrir su menú");
    }
}
