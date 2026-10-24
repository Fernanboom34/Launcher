#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod engine;
mod model;
mod net;
mod oauth;
mod profiles;
mod ui;

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|arg| {
        [
            "--create-profile",
            "--list-profiles",
            "--import-profiles",
            "--install-loader-jar",
            "--install-profile",
            "--verify-profile",
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
                .with_icon(icon()),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(ui::Launcher::new(cc)))),
    )
}

fn icon() -> eframe::egui::IconData {
    let mut rgba = vec![0; 64 * 64 * 4];
    for y in 0..64usize {
        for x in 0..64usize {
            let corner_x = if x < 12 { 12 - x } else { x.saturating_sub(51) };
            let corner_y = if y < 12 { 12 - y } else { y.saturating_sub(51) };
            if corner_x * corner_x + corner_y * corner_y <= 144 {
                let color = if (18..27).contains(&x) && (14..49).contains(&y)
                    || (18..47).contains(&x) && (40..49).contains(&y)
                {
                    [13, 17, 24, 255]
                } else {
                    [129, 232, 181, 255]
                };
                rgba[(y * 64 + x) * 4..(y * 64 + x + 1) * 4].copy_from_slice(&color);
            }
        }
    }
    eframe::egui::IconData {
        rgba,
        width: 64,
        height: 64,
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
                model::Event::GameStarted => println!("Minecraft iniciado con perfil local"),
                model::Event::GameExited(code) => println!("Minecraft terminó: {code:?}"),
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
            settings.offline = true;
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
