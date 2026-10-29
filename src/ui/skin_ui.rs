use super::*;
use crate::skins::{self, Library, Skin};
use std::collections::HashMap;

pub(super) struct State {
    pub library: Option<Library>,
    pub textures: HashMap<String, egui::TextureHandle>,
    pub(super) preview: Option<String>,
    yaw: f32,
    edit: bool,
    name: String,
    slim: bool,
    replacement: Option<Vec<u8>>,
    remove: bool,
    instance: String,
    pub capes: Vec<skins::Cape>,
    pub cape: String,
    pub account_loaded: Option<String>,
    defaults_attempted: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            library: None,
            textures: HashMap::new(),
            preview: None,
            yaw: 0.35,
            edit: false,
            name: String::new(),
            slim: false,
            replacement: None,
            remove: false,
            instance: String::new(),
            capes: Vec::new(),
            cape: String::new(),
            account_loaded: None,
            defaults_attempted: false,
        }
    }
}
impl Launcher {
    fn download_default_skins(&mut self) {
        self.skins.defaults_attempted = true;
        let root = self.root.clone();
        self.work(move |tx| {
            let mut l = Library::load(&root)?;
            for (name, key, slim) in skins::DEFAULTS {
                if l.skins.iter().any(|s| {
                    s.builtin
                        && s.name == *name
                        && skins::texture_path(&root, &s.id).is_ok_and(|p| p.is_file())
                }) {
                    continue;
                }
                tx.send(Event::Status(format!("Descargando skin {name}…")))?;
                let bytes = skins::download_texture(&format!(
                    "https://textures.minecraft.net/texture/{key}"
                ))?;
                l.import(&root, &bytes, name, *slim, true)?;
            }
            tx.send(Event::SkinsChanged)?;
            Ok(())
        });
    }
    fn import_skin(&mut self, path: PathBuf) {
        let result = (|| -> anyhow::Result<String> {
            anyhow::ensure!(
                std::fs::metadata(&path)?.len() <= 1024 * 1024,
                "La skin supera 1 MB"
            );
            let bytes = std::fs::read(&path)?;
            let name = path.file_stem().unwrap_or_default().to_string_lossy();
            self.skins
                .library
                .as_mut()
                .context("Biblioteca no disponible")?
                .import(
                    &self.root,
                    &bytes,
                    &name,
                    skins::detect_slim(&skins::decode(&bytes)?),
                    false,
                )
        })();
        match result {
            Ok(id) => {
                self.skins.preview = Some(id);
                self.status =
                    "Skin guardada. Puedes elegir brazos clásicos o delgados en Editar skin."
                        .into();
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }
    fn skin_texture(&mut self, ctx: &egui::Context, skin: &Skin) -> Option<egui::TextureId> {
        if !self.skins.textures.contains_key(&skin.id) {
            let image = skins::texture_path(&self.root, &skin.id)
                .and_then(|p| skins::decode(&std::fs::read(p)?));
            match image {
                Ok(image) => {
                    let color = egui::ColorImage::from_rgba_unmultiplied([64, 64], image.as_raw());
                    self.skins.textures.insert(
                        skin.id.clone(),
                        ctx.load_texture(&skin.id, color, egui::TextureOptions::NEAREST),
                    );
                }
                Err(e) => {
                    self.error = Some(format!("No se pudo cargar {}: {e}", skin.name));
                    return None;
                }
            }
        }
        self.skins.textures.get(&skin.id).map(|t| t.id())
    }
    pub(super) fn skins_page(&mut self, ui: &mut egui::Ui) {
        if self.skins.library.is_none() {
            match Library::load(&self.root) {
                Ok(l) => self.skins.library = Some(l),
                Err(e) => {
                    self.error = Some(e.to_string());
                    return;
                }
            }
        }
        if !self.busy {
            for file in ui.ctx().input(|i| i.raw.dropped_files.clone()) {
                if let Some(path) = file.path {
                    self.import_skin(path);
                }
            }
        }
        let account = self
            .settings
            .profile
            .clone()
            .or_else(|| model::Profile::offline(&self.settings.local_name).ok());
        let Some(account) = account else {
            ui.label("Añade una cuenta para elegir su skin.");
            return;
        };
        let session = self
            .session
            .as_ref()
            .filter(|s| s.profile.id == account.id && Instant::now() < s.expires)
            .cloned();
        let entries = self
            .skins
            .library
            .as_ref()
            .map(|l| l.skins.clone())
            .unwrap_or_default();
        if !self.skins.defaults_attempted
            && !self.busy
            && entries.iter().filter(|s| s.builtin).count() < skins::DEFAULTS.len()
            && std::env::var_os("LUMA_PREVIEW").is_none()
        {
            self.download_default_skins();
        }
        let equipped = self
            .skins
            .library
            .as_ref()
            .and_then(|l| l.selected.get(&account.id))
            .cloned();
        let selected = self.skins.preview.clone().or(equipped.clone());
        let current = entries
            .iter()
            .find(|s| Some(&s.id) == selected.as_ref())
            .or_else(|| entries.first())
            .cloned();
        ui.heading("Selector de skins");
        ui.add_space(12.);
        ui.horizontal_top(|ui|{
            ui.vertical(|ui|{
                ui.set_width(270.);
                egui::ScrollArea::vertical().id_salt("skin_details").show(ui,|ui|{
                ui.label(RichText::new(&account.name).size(23.).strong());
                ui.label(if account.local{"Cuenta local"}else{"Cuenta de Minecraft"});
                let (rect,response)=ui.allocate_exact_size(Vec2::new(260.,330.),Sense::drag());
                if response.dragged(){self.skins.yaw-=ui.input(|i|i.pointer.delta().x)*0.012;}
                if let Some(skin)=&current {
                    if let Some(texture)=self.skin_texture(ui.ctx(),skin){super::skin_preview::draw(ui,rect,texture,skin.slim,self.skins.yaw);}
                    ui.label(RichText::new(&skin.name).strong());
                    ui.label(RichText::new("Arrastra el modelo para girarlo").small().color(palette().muted));
                    if ui.add_enabled(!self.busy,egui::Button::new("Editar skin")).clicked(){self.skins.edit=true;self.skins.name=skin.name.clone();self.skins.slim=skin.slim;self.skins.replacement=None;}
                    if account.local {
                        ui.add_space(12.);ui.label("Aplicar en una instancia");
                        if self.skins.instance.is_empty(){self.skins.instance=self.settings.active_installation.clone();}
                        let instance=self.installations.iter().find(|p|p.id==self.skins.instance).cloned();
                        egui::ComboBox::from_id_salt("skin_instance").width(250.).selected_text(instance.as_ref().map(|p|p.name.as_str()).unwrap_or("Elige una instancia")).show_ui(ui,|ui|{for p in &self.installations{ui.selectable_value(&mut self.skins.instance,p.id.clone(),format!("{} · {}",p.name,p.loader.label()));}});
                        ui.label(RichText::new("Descarga CustomSkinLoader y sus dependencias compatibles desde Modrinth.").small().color(palette().muted));
                        if ui.add_enabled(!self.busy && instance.as_ref().is_some_and(|p|!self.is_running(p)),primary("Instalar mod y aplicar skin")).clicked(){
                            let root=self.root.clone();let skin=skin.clone();let name=account.name.clone();let id=account.id.clone();let profile=instance.clone().unwrap();
                            self.work(move|tx|{skins::apply_local(&root,&skin,&name,&profile,|s|{let _=tx.send(Event::Status(s));})?;let mut library=Library::load(&root)?;library.selected.insert(id,skin.id);library.save(&root)?;tx.send(Event::SkinsChanged)?;tx.send(Event::ContentChanged(profile.id))?;Ok(())});
                        }
                        ui.label(RichText::new("Visible en esta instancia. Otros jugadores necesitan su propia configuración de skins.").small().color(palette().muted));
                        if ui.add_enabled(!self.busy && instance.as_ref().is_some_and(|p|!self.is_running(p)),egui::Button::new("Desactivar skin local")).clicked(){let root=self.root.clone();let name=account.name.clone();let id=account.id.clone();let profile=instance.unwrap();self.work(move|tx|{skins::disable_local(&name,&profile.game_dir)?;let mut l=Library::load(&root)?;l.selected.remove(&id);l.save(&root)?;tx.send(Event::SkinsChanged)?;tx.send(Event::Status("Skin local desactivada. El mod se conserva para otras skins.".into()))?;Ok(())});}
                    }else{
                        if ui.add_enabled(!self.busy && session.is_some(),primary("Aplicar a mi cuenta")).clicked(){let root=self.root.clone();let skin=skin.clone();let id=account.id.clone();let session=session.clone().unwrap();self.work(move|tx|{skins::apply_account(&root,&skin,&session)?;let mut l=Library::load(&root)?;l.selected.insert(id,skin.id);l.save(&root)?;tx.send(Event::SkinsChanged)?;tx.send(Event::Status("Skin aplicada a tu cuenta Minecraft".into()))?;Ok(())});}
                        if session.is_none(){ui.label("Inicia sesión con Microsoft para aplicar skins y capas.");}
                        if ui.add_enabled(!self.busy && session.is_some(),egui::Button::new("Restablecer skin de la cuenta")).clicked(){let session=session.clone().unwrap();let root=self.root.clone();let id=account.id.clone();self.work(move|tx|{skins::reset_account(&session)?;let mut l=Library::load(&root)?;l.selected.remove(&id);l.save(&root)?;tx.send(Event::SkinsChanged)?;Ok(())});}
                    }
                    if ui.button("Exportar PNG").clicked(){if let Some(path)=rfd::FileDialog::new().add_filter("Skin PNG",&["png"]).set_file_name(format!("{}.png",skin.name)).save_file(){if let Err(e)=skins::texture_path(&self.root,&skin.id).and_then(|src|{std::fs::copy(src,path)?;Ok(())}){self.error=Some(e.to_string());}}}
                    if !skin.builtin && ui.add_enabled(!self.busy,egui::Button::new("Eliminar de skins guardadas")).clicked(){self.skins.remove=true;}
                }else{ui.painter().text(rect.center(),egui::Align2::CENTER_CENTER,"Añade una skin PNG",FontId::proportional(17.),palette().muted);}
                });
            });ui.separator();
            egui::ScrollArea::vertical().id_salt("skin_gallery").show(ui,|ui|{
                ui.with_layout(Layout::top_down(Align::Min),|ui|{
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui|{
                    if ui.add_enabled(!self.busy,primary("+ Añadir skin PNG")).clicked(){if let Some(path)=rfd::FileDialog::new().add_filter("Skin PNG",&["png"]).pick_file(){self.import_skin(path);}}
                    if ui.add_enabled(!self.busy,egui::Button::new("Actualizar predeterminadas")).clicked(){self.download_default_skins();}
                    if !account.local && ui.add_enabled(!self.busy && session.is_some(),egui::Button::new("Cargar skin y capas de mi cuenta")).clicked(){let root=self.root.clone();let session=session.clone().unwrap();self.work(move|tx|{let appearance=skins::account_appearance(&session)?;if let Some(s)=appearance.skins.iter().find(|s|s.state=="ACTIVE"){let bytes=skins::download_texture(&s.url)?;let mut l=Library::load(&root)?;let id=l.import(&root,&bytes,&session.profile.name,s.variant.eq_ignore_ascii_case("SLIM"),false)?;l.selected.insert(session.profile.id.clone(),id);l.save(&root)?;}tx.send(Event::SkinAccount(session.profile.id,appearance))?;Ok(())});}
                });
                ui.label(RichText::new("Arrastra aquí tu PNG · 64×64 o 64×32 · máximo 1 MB").small().color(palette().muted));ui.add_space(16.);
                for builtin in [false,true]{
                    ui.label(RichText::new(if builtin{"Skins predeterminadas"}else{"Skins guardadas"}).size(20.).strong());
                    let columns=(((ui.clip_rect().right()-ui.cursor().left())/174.).floor() as usize).max(1);
                    egui::Grid::new(("skin_grid",builtin)).num_columns(columns).spacing(Vec2::new(12.,12.)).show(ui,|ui|{for (index,skin) in entries.iter().filter(|s|s.builtin==builtin).enumerate(){
                        let chosen=current.as_ref().is_some_and(|s|s.id==skin.id);
                        egui::Frame::new().fill(if chosen{Color32::from_rgb(35, 65, 51)}else{palette().panel}).stroke(Stroke::new(1.0_f32,if chosen{MINT}else{palette().border})).corner_radius(16).inner_margin(12.).show(ui,|ui|{
                            ui.vertical(|ui|{
                            ui.set_width(138.);let (rect,response)=ui.allocate_exact_size(Vec2::new(138.,190.),Sense::click());
                            if let Some(texture)=self.skin_texture(ui.ctx(),skin){super::skin_preview::draw(ui,rect,texture,skin.slim,0.25);}
                            if response.clicked(){self.skins.preview=Some(skin.id.clone());}
                            if ui.selectable_label(chosen,&skin.name).clicked(){self.skins.preview=Some(skin.id.clone());}
                            ui.label(RichText::new(if equipped.as_ref()==Some(&skin.id){"✓ Seleccionada"}else{" "}).small().color(MINT));
                            });
                        });
                        if (index+1)%columns==0 {ui.end_row();}
                    }});ui.add_space(18.);
                }
                if !account.local && self.skins.account_loaded.as_ref()==Some(&account.id){
                    ui.heading("Capas de tu cuenta");
                    egui::ComboBox::from_id_salt("account_cape").selected_text(self.skins.capes.iter().find(|c|c.id==self.skins.cape).map(|c|c.alias.as_str()).unwrap_or("Sin capa")).show_ui(ui,|ui|{ui.selectable_value(&mut self.skins.cape,String::new(),"Sin capa");for cape in &self.skins.capes{ui.selectable_value(&mut self.skins.cape,cape.id.clone(),&cape.alias).on_hover_text(&cape.url);}});
                    if ui.add_enabled(!self.busy && session.is_some(),egui::Button::new("Aplicar capa")).clicked(){let session=session.unwrap();let cape=self.skins.cape.clone();self.work(move|tx|{skins::set_cape(&session,(!cape.is_empty()).then_some(cape.as_str()))?;tx.send(Event::Status("Capa actualizada".into()))?;Ok(())});}
                }
                });
            });
        });
        if let Some(skin) = current {
            if self.skins.edit {
                let mut close = false;
                egui::Modal::new(egui::Id::new("edit_skin")).show(ui.ctx(), |ui| {
                    ui.set_width(370.);
                    ui.heading("Editar skin");
                    ui.label("Nombre");
                    ui.text_edit_singleline(&mut self.skins.name);
                    let locked_slim = self.skins.replacement.as_deref()
                        .map(skins::decode)
                        .unwrap_or_else(|| skins::texture_path(&self.root, &skin.id)
                            .and_then(|path| skins::read_png(&path))
                            .and_then(|bytes| skins::decode(&bytes)))
                        .map(|image| skins::detect_slim(&image)).unwrap_or(false);
                    if locked_slim { self.skins.slim = true; }
                    ui.add_enabled_ui(!locked_slim, |ui| {
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.skins.slim, false, "Clásico · brazos de 4 px");
                            ui.selectable_value(&mut self.skins.slim, true, "Delgado · brazos de 3 px");
                        });
                    });
                    if locked_slim { ui.label("Este PNG usa brazos delgados. Modelo bloqueado para evitar deformaciones."); }
                    if ui.button("Cambiar archivo PNG").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Skin PNG", &["png"])
                            .pick_file()
                        {
                            match skins::read_png(&path) {
                                Ok(bytes) => self.skins.replacement = Some(bytes),
                                Err(e) => self.error = Some(e.to_string()),
                            }
                        }
                    }
                    if self.skins.replacement.is_some() {
                        ui.label("Se guardará el nuevo PNG como una skin guardada.");
                    }
                    if ui
                        .button(if skin.builtin {
                            "Guardar copia personalizada"
                        } else {
                            "Guardar cambios"
                        })
                        .clicked()
                    {
                        if let Some(l) = self.skins.library.as_mut() {
                            match l.edit(
                                &self.root,
                                &skin.id,
                                &self.skins.name,
                                self.skins.slim,
                                self.skins.replacement.as_deref(),
                            ) {
                                Ok(id) => {
                                    self.skins.preview = Some(id);
                                    self.skins.textures.clear();
                                    close = true;
                                }
                                Err(e) => self.error = Some(e.to_string()),
                            }
                        }
                    }
                    if ui.button("Cancelar").clicked() {
                        close = true;
                    }
                });
                if close {
                    self.skins.edit = false;
                }
            }
            if self.skins.remove {
                let mut close = false;
                egui::Modal::new(egui::Id::new("remove_skin")).show(ui.ctx(), |ui| {
                    ui.heading("Eliminar skin guardada");
                    ui.label(
                        "Se elimina de esta biblioteca. La skin aplicada en Minecraft se conserva.",
                    );
                    if ui.button("Eliminar").clicked() {
                        if let Some(l) = self.skins.library.as_mut() {
                            match l.remove(&self.root, &skin.id) {
                                Ok(()) => {
                                    self.skins.preview = None;
                                    self.skins.textures.remove(&skin.id);
                                }
                                Err(e) => self.error = Some(e.to_string()),
                            }
                        }
                        close = true;
                    }
                    if ui.button("Cancelar").clicked() {
                        close = true;
                    }
                });
                if close {
                    self.skins.remove = false;
                }
            }
        }
    }
}

use anyhow::Context;
