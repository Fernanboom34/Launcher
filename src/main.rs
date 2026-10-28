#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod browser;
mod catalog;
#[allow(dead_code)]
mod cleanup;
mod content;
mod content_meta;
mod distribution;
mod engine;
mod game_process;
#[cfg(windows)]
mod maintenance;
mod model;
mod net;
mod oauth;
mod platform;
mod profiles;
mod skins;
mod ui;

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    #[cfg(windows)]
    if args
        .get(1)
        .is_some_and(|arg| arg == "--manage-uninstall-data")
    {
        return maintenance::run();
    }
    if args.get(1).is_some_and(|arg| arg == "--content-browser") {
        if let Err(error) = browser::child_main() {
            eprintln!("{error:#}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if args.get(1).is_some_and(|arg| {
        [
            "--create-profile",
            "--list-profiles",
            "--import-profiles",
            "--install-loader-jar",
            "--install-profile",
            "--verify-profile",
            "--repair-profile",
            "--play-profile",
        ]
        .contains(&arg.as_str())
    }) {
        if let Err(error) = profiles::cli(&args) {
            eprintln!("{error:#}");
            std::process::exit(1);
        }
        return Ok(());
    }
    if args
        .get(1)
        .is_some_and(|a| a == "--install" || a == "--verify" || a == "--play-local")
    {
        if let Err(error) = headless(&args) {
            eprintln!("{error:#}");
            std::process::exit(1);
        }
        return Ok(());
    }
    eframe::run_native(
        "Luma · Minecraft Launcher",
        eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size(if std::env::var_os("LUMA_PREVIEW_COMPACT").is_some() {
                    [1060.0, 720.0]
                } else {
                    [1360.0, 880.0]
                })
                .with_min_inner_size([1060.0, 720.0])
                .with_icon(icon())
                .with_transparent(true)
                .with_decorations(false)
                .with_resizable(true),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(ui::Launcher::new(cc)))),
    )
}

fn icon() -> eframe::egui::IconData {
    let image = image::load_from_memory(include_bytes!("../assets/luma-moon.png"))
        .expect("embedded icon")
        .to_rgba8();
    eframe::egui::IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    }
}

fn headless(args: &[String]) -> anyhow::Result<()> {
    use std::sync::mpsc;
    let id = args.get(2).ok_or_else(|| {
        anyhow::anyhow!(
            "Uso: luma-launcher --install VERSION, --verify VERSION o --play-local VERSION NOMBRE"
        )
    })?;
    let root = model::root();
    let (tx, rx) = mpsc::channel();
    let reporter = std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            match event {
                model::Event::Status(text) => println!("{text}"),
                model::Event::GameStarted(_) => println!("Minecraft iniciado con perfil local"),
                model::Event::GameExited(_, code) => println!("Minecraft terminó: {code:?}"),
                model::Event::Progress(n, total) if n.is_multiple_of(200) || n == total => {
                    println!("{n}/{total}")
                }
                _ => (),
            }
        }
    });
    let local_play = args[1] == "--play-local";
    let offline = args[1] != "--install";
    let result = (|| -> anyhow::Result<()> {
        if !offline {
            engine::catalog(&root, &tx)?;
        }
        let manifest: model::Manifest = model::read_json(&root.join("manifest.json"))?;
        let version = manifest
            .versions
            .iter()
            .find(|v| &v.id == id)
            .ok_or_else(|| anyhow::anyhow!("Versión no encontrada: {id}"))?;
        let mut settings: model::Settings =
            model::read_json(&root.join("settings.json")).unwrap_or_default();
        if local_play {
            settings.local_name = args.get(3).cloned().unwrap_or_else(|| "Jugador".into());
            settings.profile = Some(model::Profile::offline(&settings.local_name)?);
        }
        let installation = engine::prepare(&root, version, &settings, offline, &tx)?;
        if local_play {
            let (profile, token) = settings.launch_identity(None)?;
            return engine::launch(
                &root,
                version,
                &settings,
                installation,
                &profile,
                &token,
                &tx,
            );
        }
        println!(
            "OK · {id} · {}",
            if offline {
                "verificación sin conexión"
            } else {
                "instalación completa"
            }
        );
        Ok(())
    })();
    drop(tx);
    let _ = reporter.join();
    result
}
