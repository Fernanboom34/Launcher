#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[cfg(windows)]
#[allow(dead_code)]
#[path = "../cleanup.rs"]
mod cleanup;
#[cfg(windows)]
#[allow(dead_code)]
#[path = "../distribution.rs"]
mod distribution;
#[cfg(windows)]
use eframe::egui;
#[cfg(windows)]
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
};
#[cfg(windows)]
struct Installer {
    preview_started: std::time::Instant,
    preview_requested: bool,
    cleanup_queued: bool,
    desktop: bool,
    status: String,
    busy: bool,
    installed: bool,
    uninstalled: bool,
    uninstall_mode: bool,
    plan: Option<cleanup::UninstallPlan>,
    shared: bool,
    ack: bool,
    remove_setup: bool,
    rx: Option<Receiver<Result<String, String>>>,
}
#[cfg(windows)]
impl Default for Installer {
    fn default() -> Self {
        Self {
            preview_started: std::time::Instant::now(),
            preview_requested: false,
            cleanup_queued: false,
            desktop: true,
            status: "Descarga la versión más reciente desde GitHub Releases.".into(),
            busy: false,
            installed: false,
            uninstalled: false,
            uninstall_mode: false,
            plan: None,
            shared: false,
            ack: false,
            remove_setup: true,
            rx: None,
        }
    }
}
#[cfg(windows)]
impl Installer {
    fn refresh_plan(&mut self) {
        self.ack = false;
        self.shared = false;
        match cleanup::uninstall_plan() {
            Ok(plan) => {
                self.plan = Some(plan);
                self.status =
                    "Revisa las rutas. Cierra Luma y Minecraft antes de continuar.".into();
            }
            Err(e) => {
                self.plan = None;
                self.status = format!("{e:#}");
            }
        }
    }
    fn install(&mut self) {
        self.busy = true;
        let desktop = self.desktop;
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<()> {
                let release = distribution::latest()?;
                let folder = distribution::install_dir();
                let stage = std::env::temp_dir().join(format!(
                    "Luma-install-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_nanos()
                ));
                std::fs::create_dir_all(&stage)?;
                let target = stage.join("Luma.exe");
                let (progress, updates) = mpsc::channel();
                let notify = tx.clone();
                let reporter = std::thread::spawn(move || {
                    for s in updates {
                        let _ = notify.send(Ok(s));
                    }
                });
                let result = distribution::download(&release, &target, &progress);
                drop(progress);
                let _ = reporter.join();
                if let Err(error) = result {
                    let _ = cleanup::delete_verified_directory(&stage, &stage.canonicalize()?);
                    return Err(error);
                }
                let setup = folder.join("Luma-Setup.exe");
                let installed =
                    distribution::install_application(&release, &target, &std::env::current_exe()?);
                let _ = cleanup::delete_verified_directory(&stage, &stage.canonicalize()?);
                installed?;
                let target = folder.join("Luma.exe");
                cleanup::register_installation(&target, &setup, &release.version)?;
                if desktop {
                    distribution::shortcut(&target, "Luma", "")?;
                }
                Ok(())
            })();
            let _ = tx.send(
                result
                    .map(|_| "Luma está instalado".into())
                    .map_err(|e| format!("{e:#}")),
            );
        });
    }
    fn uninstall(&mut self) {
        let Some(mut plan) = self.plan.clone() else {
            return;
        };
        if self.shared {
            for path in &plan.excluded {
                match cleanup::validate_private_directory(path, &[]) {
                    Ok(checked) => plan.directories.push(checked),
                    Err(e) => {
                        self.status = e.to_string();
                        return;
                    }
                }
            }
        }
        self.busy = true;
        self.status = "Eliminando Luma y los datos confirmados…".into();
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        std::thread::spawn(move || {
            let result = cleanup::uninstall(&plan);
            let _ = tx.send(
                result
                    .map(|_| "Desinstalación completa".into())
                    .map_err(|e| format!("{e:#}")),
            );
        });
    }
    fn queue_cleanup(&mut self) {
        if self.cleanup_queued {
            return;
        }
        if let Ok(exe) = std::env::current_exe() {
            let temporary = cleanup::temporary_setup_directory();
            let parent = temporary.as_deref();
            if (self.uninstalled && self.remove_setup) || parent.is_some() {
                match cleanup::delete_setup_after_exit(&exe, parent) {
                    Ok(()) => self.cleanup_queued = true,
                    Err(e) => self.status = e.to_string(),
                }
            }
        }
    }
    fn close(&mut self, ctx: &egui::Context) {
        self.queue_cleanup();
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}
#[cfg(windows)]
impl eframe::App for Installer {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if let Some(path) = std::env::var_os("LUMA_SETUP_PREVIEW") {
            if let Some(image) = ctx.input(|i| {
                i.events.iter().find_map(|e| {
                    if let egui::Event::Screenshot { image, .. } = e {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            }) {
                let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                let _ = image::save_buffer(
                    std::path::Path::new(&path),
                    &bytes,
                    image.size[0] as u32,
                    image.size[1] as u32,
                    image::ColorType::Rgba8,
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else if !self.preview_requested
                && self.preview_started.elapsed() > std::time::Duration::from_millis(500)
            {
                self.preview_requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
        }
        if let Some(rx) = &self.rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    Ok(s) => {
                        if s == "Luma está instalado" {
                            self.installed = true;
                            self.busy = false;
                        }
                        if s == "Desinstalación completa" {
                            self.uninstalled = true;
                            self.busy = false;
                        }
                        self.status = s;
                    }
                    Err(e) => {
                        self.status = e;
                        self.busy = false;
                    }
                }
            }
        }
        if self.uninstall_mode && self.busy && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if ctx.input(|i| i.viewport().close_requested()) && self.uninstalled {
            self.close(ctx);
        }
        egui::CentralPanel::default().frame(egui::Frame::new().fill(egui::Color32::from_rgb(22,24,28)).inner_margin(24.)).show(ctx,|ui|{
            ui.add_space(12.);ui.heading(egui::RichText::new("Luma").size(36.).color(egui::Color32::from_rgb(68,221,139)));
            ui.horizontal(|ui|{if ui.add_enabled(!self.busy,egui::Button::new("Instalar / actualizar").selected(!self.uninstall_mode)).clicked(){self.uninstall_mode=false;}
if ui.add_enabled(!self.busy,egui::Button::new("Desinstalar").selected(self.uninstall_mode)).clicked(){self.uninstall_mode=true;self.refresh_plan();}});ui.separator();
            if self.uninstall_mode{
                ui.heading("Eliminar Luma y sus datos");
                ui.label("Se borrarán permanentemente la aplicación, Java y archivos descargados, cuentas guardadas, mundos, mods, capturas y ajustes de Luma.");
                if self.plan.is_none()&&ui.button("Volver a examinar").clicked(){self.refresh_plan();}
                if let Some(plan)=&self.plan{
                    egui::ScrollArea::vertical().max_height(160.).show(ui,|ui|{for path in plan.directories.iter().chain(plan.files.iter()){ui.label(egui::RichText::new(cleanup::display_path(path)).small().monospace());}
                        if !plan.excluded.is_empty(){ui.separator();ui.label("Carpetas externas compartidas con otros launchers:");for path in &plan.excluded{ui.label(egui::RichText::new(cleanup::display_path(path)).small().monospace());}}
                    });
                    if !plan.excluded.is_empty(){ui.checkbox(&mut self.shared,"Borrar también las carpetas compartidas (afecta a otros launchers)");}
                }
                if cleanup::temporary_setup_directory().is_some(){ui.label("El instalador temporal se eliminará al cerrar.");}else{ui.add_enabled(!self.busy,egui::Checkbox::new(&mut self.remove_setup,"Eliminar también este instalador al cerrar"));}
                ui.add_enabled(!self.busy,egui::Checkbox::new(&mut self.ack,"Entiendo que perderé permanentemente los archivos indicados"));
                if ui.add_enabled(!self.busy&&!self.uninstalled&&self.plan.is_some()&&self.ack,egui::Button::new("Desinstalar y borrar los datos confirmados").fill(egui::Color32::from_rgb(155,45,60))).clicked(){self.uninstall();}
            }else{
                ui.label(format!("Instalar en {}",distribution::install_dir().display()));ui.add_enabled(!self.busy,egui::Checkbox::new(&mut self.desktop,"Crear acceso directo en el escritorio"));
                if ui.add_enabled(!self.busy,egui::Button::new("Descargar e instalar Luma")).clicked(){self.install();}
            }
            ui.add_space(12.);ui.label(&self.status);if self.busy{ui.spinner();}
            if self.installed&&!self.uninstall_mode&&ui.button("Abrir Luma").clicked(){let _=std::process::Command::new(distribution::install_dir().join("Luma.exe")).spawn();ctx.send_viewport_cmd(egui::ViewportCommand::Close);}
            if self.uninstalled&&ui.button("Cerrar").clicked(){self.close(ctx);}
        });
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.queue_cleanup();
    }
}
#[cfg(windows)]
fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    if let Some(index) = args.iter().position(|s| s == "--wait-for") {
        if let Some(id) = args.get(index + 1).and_then(|s| s.parse::<u32>().ok()) {
            let _ = cleanup::wait_for_process(id);
        }
    }
    if args.get(1).is_some_and(|s| s == "--install-to") {
        let result = (|| -> anyhow::Result<()> {
            let folder = PathBuf::from(
                args.get(2)
                    .ok_or_else(|| anyhow::anyhow!("Falta la carpeta de destino"))?,
            );
            let release = distribution::latest()?;
            let (tx, rx) = mpsc::channel();
            let reporter = std::thread::spawn(move || {
                for line in rx {
                    println!("{line}");
                }
            });
            let result = distribution::download(&release, &folder.join("Luma.exe"), &tx);
            drop(tx);
            let _ = reporter.join();
            result?;
            cleanup::register_executable(&folder.join("Luma.exe"))?;
            Ok(())
        })();
        if let Err(e) = result {
            eprintln!("{e:#}");
            std::process::exit(1)
        }
        return Ok(());
    }
    match cleanup::relocate_setup() {
        Ok(Some(_)) => return Ok(()),
        Ok(None) => (),
        Err(e) => {
            eprintln!("{e:#}");
            std::process::exit(1)
        }
    }
    let uninstall = args.iter().any(|s| s == "--uninstall");
    // The installed folder must not stay locked as this process's working directory.
    let _ = std::env::set_current_dir(std::env::temp_dir());
    eframe::run_native(
        "Luma · Instalación y desinstalación",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([670., 620.])
                .with_min_inner_size([620., 580.]),
            ..Default::default()
        },
        Box::new(move |cc| {
            let mut style = (*cc.egui_ctx.style()).clone();
            style.visuals = egui::Visuals::dark();
            style.visuals.override_text_color = Some(egui::Color32::from_rgb(242, 244, 246));
            style.spacing.item_spacing = egui::vec2(10., 12.);
            style.spacing.button_padding = egui::vec2(12., 8.);
            style.spacing.interact_size.y = 30.;
            style
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(15.));
            style
                .text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(15.));
            for widget in [
                &mut style.visuals.widgets.inactive,
                &mut style.visuals.widgets.hovered,
                &mut style.visuals.widgets.active,
            ] {
                widget.corner_radius = egui::CornerRadius::same(8);
            }
            cc.egui_ctx.set_style(style);
            let mut app = Installer {
                uninstall_mode: uninstall,
                ..Default::default()
            };
            if uninstall {
                app.refresh_plan();
            }
            Ok(Box::new(app))
        }),
    )
}

#[cfg(not(windows))]
fn main() {
    println!("En Linux instala el paquete .deb o ejecuta ./install.sh desde el paquete Luma-linux-x64.tar.gz. Consulta docs/LINUX.md.");
}
