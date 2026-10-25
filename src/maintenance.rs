//! Player-data removal runs before the native uninstaller, in the player's
//! original process context. Program Files is exclusively the installer's job.
use crate::{cleanup, distribution};
use eframe::egui;
use std::sync::{
    atomic::{AtomicI32, Ordering},
    mpsc, Arc,
};

pub fn run() -> eframe::Result {
    let outcome = Arc::new(AtomicI32::new(2));
    let result = outcome.clone();
    eframe::run_native(
        "Luma · Datos del jugador",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([720., 590.])
                .with_min_inner_size([620., 500.]),
            ..Default::default()
        },
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(Removal::new(result)))
        }),
    )?;
    std::process::exit(outcome.load(Ordering::SeqCst));
}

struct Removal {
    plan: Option<cleanup::UninstallPlan>,
    shared: bool,
    acknowledged: bool,
    busy: bool,
    complete: bool,
    status: String,
    receiver: Option<mpsc::Receiver<Result<(), String>>>,
    outcome: Arc<AtomicI32>,
}
fn player_plan(plan: &cleanup::UninstallPlan) -> cleanup::UninstallPlan {
    let mut selected = plan.clone();
    selected.directories.retain(|p| {
        !distribution::is_program_installation(p)
            && !same_path(p, &distribution::legacy_install_dir())
    });
    // These files are registered launcher executables and shortcuts, not
    // player data. Remove them even when the player chooses to keep worlds.
    selected.files.clear();
    selected
}
fn same_path(a: &std::path::Path, b: &std::path::Path) -> bool {
    cleanup::display_path(a).eq_ignore_ascii_case(&cleanup::display_path(b))
}
fn application_plan(
    plan: &cleanup::UninstallPlan,
    legacy: &std::path::Path,
) -> anyhow::Result<cleanup::UninstallPlan> {
    let mut apps = cleanup::UninstallPlan {
        files: plan
            .files
            .iter()
            .filter(|p| {
                !p.parent()
                    .is_some_and(distribution::is_program_installation)
            })
            .cloned()
            .collect(),
        ..Default::default()
    };
    for name in [
        "Luma.exe",
        "Luma-Setup.exe",
        "Luma.exe.previous",
        "Luma.download",
    ] {
        let file = legacy.join(name);
        if file.is_file() {
            apps.files.push(file.canonicalize()?);
        }
    }
    apps.files.sort();
    apps.files.dedup();
    Ok(apps)
}
fn perform_removal(
    protected: &cleanup::UninstallPlan,
    selected: &cleanup::UninstallPlan,
    legacy: &std::path::Path,
) -> anyhow::Result<()> {
    cleanup::ensure_not_running(protected)?;
    let apps = application_plan(protected, legacy)?;
    cleanup::uninstall_files(selected)?;
    cleanup::uninstall_files(&apps)?;
    // Never recursively remove unknown files from the legacy app directory.
    let _ = std::fs::remove_dir(legacy);
    Ok(())
}
impl Removal {
    fn new(outcome: Arc<AtomicI32>) -> Self {
        let (plan, status) = match cleanup::uninstall_plan() {
            Ok(plan) => (Some(plan), String::new()),
            Err(e) => (None, e.to_string()),
        };
        Self {
            plan,
            shared: false,
            acknowledged: false,
            busy: false,
            complete: false,
            status,
            receiver: None,
            outcome,
        }
    }
    fn remove(&mut self, delete_data: bool) {
        let Some(protected) = self.plan.clone() else {
            return;
        };
        let mut selected = if delete_data {
            player_plan(&protected)
        } else {
            cleanup::UninstallPlan::default()
        };
        if delete_data && self.shared {
            for path in &protected.excluded {
                match cleanup::validate_private_directory(path, &[]) {
                    Ok(path) => selected.directories.push(path),
                    Err(e) => {
                        self.status = e.to_string();
                        return;
                    }
                }
            }
        }
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        self.busy = true;
        self.status = "Retirando las copias registradas de Luma y las rutas confirmadas…".into();
        std::thread::spawn(move || {
            let result =
                perform_removal(&protected, &selected, &distribution::legacy_install_dir())
                    .map_err(|e| format!("{e:#}"));
            let _ = tx.send(result);
        });
    }
}
impl eframe::App for Removal {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if let Some(message) = self.receiver.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.receiver = None;
            self.busy = false;
            match message {
                Ok(()) => {
                    self.complete = true;
                    self.status =
                        "Operación completada. Continúa para retirar la instalación de Program Files, si existe."
                            .into();
                }
                Err(e) => self.status = e,
            }
        }
        if self.busy && ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Desinstalar Luma");
            ui.label("Elige si conservas tus mundos, mods, cuentas, capturas y ajustes. Las copias registradas del launcher y sus accesos directos se retirarán en ambos casos. Después se retirará la instalación de Program Files, si existe.");
            ui.separator();
            egui::ScrollArea::vertical().max_height(285.).show(ui, |ui| {
                if let Some(plan) = &self.plan {
                    let selected = player_plan(plan);
                    ui.strong("Datos de este usuario:");
                    for path in selected.directories.iter().chain(selected.files.iter()) { ui.label(cleanup::display_path(path)); }
                    if !plan.excluded.is_empty() {
                        ui.separator();
                        ui.strong("Carpetas externas compartidas (se conservan por defecto):");
                        for path in &plan.excluded { ui.label(cleanup::display_path(path)); }
                    }
                }
            });
            ui.add_enabled_ui(!self.busy && !self.complete, |ui| {
                if self.plan.as_ref().is_some_and(|p| !p.excluded.is_empty()) {
                    ui.checkbox(&mut self.shared, "Incluir las carpetas compartidas indicadas; afecta también a otros launchers");
                }
                ui.checkbox(&mut self.acknowledged, "Entiendo que borrar estos datos es permanente");
            });
            if !self.status.is_empty() { ui.label(&self.status); }
            ui.add_space(12.);
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(!self.busy && self.plan.is_some(), egui::Button::new(if self.complete { "Continuar desinstalación" } else { "Conservar datos y desinstalar" })).clicked() {
                    if self.complete { self.outcome.store(0, Ordering::SeqCst); ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                    else { self.remove(false); }
                }
                if ui.add_enabled(!self.busy && !self.complete && self.acknowledged && self.plan.is_some(), egui::Button::new("Eliminar datos y continuar")).clicked() { self.remove(true); }
                if ui.add_enabled(!self.busy, egui::Button::new("Cancelar")).clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
            });
        });
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn player_data_plan_never_deletes_program_files() {
        let app = distribution::install_dir();
        let data = std::env::temp_dir().join("Luma-player-fixture");
        let plan = cleanup::UninstallPlan {
            directories: vec![app.clone(), data.clone()],
            files: vec![app.join("Luma.exe"), data.join("account.json")],
            excluded: vec![],
        };
        let plan = player_plan(&plan);
        assert_eq!(plan.directories, vec![data.clone()]);
        assert!(plan.files.is_empty());
    }
    #[test]
    fn portable_removal_keeps_worlds_unless_deletion_is_selected() {
        for delete_data in [false, true] {
            let root = std::env::temp_dir().join(format!(
                "luma-removal-{}-{}",
                std::process::id(),
                u8::from(delete_data)
            ));
            std::fs::create_dir_all(root.join("data/saves/world")).unwrap();
            let data = root.join("data").canonicalize().unwrap();
            std::fs::write(data.join("saves/world/level.dat"), b"preserve world").unwrap();
            std::fs::write(root.join("Luma-test.exe"), b"registered portable fixture").unwrap();
            let app = root.join("Luma-test.exe").canonicalize().unwrap();
            let plan = cleanup::UninstallPlan {
                directories: vec![data.clone()],
                files: vec![app.clone()],
                excluded: vec![],
            };
            let selected = if delete_data {
                player_plan(&plan)
            } else {
                Default::default()
            };
            perform_removal(&plan, &selected, &root.join("legacy")).unwrap();
            assert!(!app.exists());
            assert_eq!(data.exists(), !delete_data);
            if !delete_data {
                assert_eq!(
                    std::fs::read(data.join("saves/world/level.dat")).unwrap(),
                    b"preserve world"
                );
            }
            cleanup::delete_verified_directory(&root, &root.canonicalize().unwrap()).unwrap();
        }
    }
}
