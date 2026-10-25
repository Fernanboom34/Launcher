#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[allow(dead_code)]
#[path = "../distribution.rs"]
mod distribution;
use eframe::egui;
use std::sync::mpsc::{self, Receiver};
struct Installer {
    desktop: bool,
    status: String,
    busy: bool,
    done: bool,
    rx: Option<Receiver<Result<String, String>>>,
}
impl Default for Installer {
    fn default() -> Self {
        Self {
            desktop: true,
            status: "Descarga la versión más reciente desde GitHub Releases.".into(),
            busy: false,
            done: false,
            rx: None,
        }
    }
}
impl eframe::App for Installer {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        if let Some(rx) = &self.rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    Ok(s) => {
                        self.done = s == "Luma está instalado";
                        self.status = s;
                        if self.done {
                            self.busy = false
                        }
                    }
                    Err(e) => {
                        self.status = e;
                        self.busy = false
                    }
                }
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(22.);
            ui.heading(
                egui::RichText::new("Luma")
                    .size(38.)
                    .color(egui::Color32::from_rgb(68, 221, 139)),
            );
            ui.label("Tu próxima aventura empieza aquí.");
            ui.add_space(24.);
            ui.label(format!(
                "Instalar en {}",
                distribution::install_dir().display()
            ));
            ui.add_enabled(
                !self.busy,
                egui::Checkbox::new(&mut self.desktop, "Crear acceso directo en el escritorio"),
            );
            ui.add_space(18.);
            ui.label(&self.status);
            if self.busy {
                ui.spinner();
            }
            if ui
                .add_enabled(
                    !self.busy && !self.done,
                    egui::Button::new("Descargar e instalar Luma"),
                )
                .clicked()
            {
                self.busy = true;
                let desktop = self.desktop;
                let (tx, rx) = mpsc::channel();
                self.rx = Some(rx);
                std::thread::spawn(move || {
                    let result = (|| -> anyhow::Result<()> {
                        let r = distribution::latest()?;
                        let target = distribution::install_dir().join("Luma.exe");
                        let (progress, updates) = mpsc::channel();
                        let notify = tx.clone();
                        let reporter = std::thread::spawn(move || {
                            for s in updates {
                                let _ = notify.send(Ok(s));
                            }
                        });
                        let downloaded = distribution::download(&r, &target, &progress);
                        drop(progress);
                        let _ = reporter.join();
                        downloaded?;
                        if desktop {
                            distribution::shortcut(&target, "Luma", "")?
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
            if self.done && ui.button("Abrir Luma").clicked() {
                let _ = std::process::Command::new(distribution::install_dir().join("Luma.exe"))
                    .spawn();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}
fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "--install-to") {
        let result = (|| -> anyhow::Result<()> {
            let folder = args
                .get(2)
                .ok_or_else(|| anyhow::anyhow!("Falta la carpeta de destino"))?;
            let release = distribution::latest()?;
            let (tx, rx) = mpsc::channel();
            let reporter = std::thread::spawn(move || {
                for line in rx {
                    println!("{line}");
                }
            });
            let result = distribution::download(
                &release,
                &std::path::Path::new(folder).join("Luma.exe"),
                &tx,
            );
            drop(tx);
            let _ = reporter.join();
            result?;
            Ok(())
        })();
        if let Err(e) = result {
            eprintln!("{e:#}");
            std::process::exit(1)
        }
        return Ok(());
    }
    eframe::run_native(
        "Instalar Luma",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([560., 360.])
                .with_resizable(false),
            ..Default::default()
        },
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::<Installer>::default())
        }),
    )
}
