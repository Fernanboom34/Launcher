use crate::{
    catalog::{self, Kind},
    model::{Event, GameProfile},
};
use anyhow::Result;
use std::{
    path::{Path, PathBuf},
    sync::{mpsc::Sender, Mutex},
};
#[cfg(windows)]
use {crate::model, anyhow::Context};
static OWNER: Mutex<Option<String>> = Mutex::new(None);
pub fn is_open() -> bool {
    OWNER.lock().is_ok_and(|o| o.is_some())
}
pub fn is_open_for(id: &str) -> bool {
    OWNER.lock().is_ok_and(|o| o.as_deref() == Some(id))
}
struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        if let Ok(mut owner) = OWNER.lock() {
            *owner = None;
        }
    }
}
pub fn site_url(kind: Kind, curseforge: bool) -> String {
    if curseforge {
        format!(
            "https://www.curseforge.com/minecraft/{}",
            match kind {
                Kind::Mod => "mc-mods",
                Kind::ResourcePack => "texture-packs",
                Kind::Shader => "shaders",
                Kind::DataPack => "data-packs",
            }
        )
    } else {
        format!(
            "https://modrinth.com/{}",
            match kind {
                Kind::Mod => "mods",
                Kind::ResourcePack => "resourcepacks",
                Kind::Shader => "shaders",
                Kind::DataPack => "datapacks",
            }
        )
    }
}
pub fn open(
    profile: GameProfile,
    kind: Kind,
    world: Option<PathBuf>,
    curseforge: bool,
    tx: Sender<Event>,
) -> Result<()> {
    let initial = site_url(kind, curseforge);
    open_url(profile, kind, world, initial, tx)
}
pub fn open_project(
    profile: GameProfile,
    kind: Kind,
    world: Option<PathBuf>,
    slug: &str,
    tx: Sender<Event>,
) -> Result<()> {
    let category = match kind {
        Kind::Mod => "mod",
        Kind::ResourcePack => "resourcepack",
        Kind::Shader => "shader",
        Kind::DataPack => "datapack",
    };
    let mut url = url::Url::parse(&format!("https://modrinth.com/{category}/"))?;
    url.path_segments_mut()
        .map_err(|_| anyhow::anyhow!("URL inválida"))?
        .pop_if_empty()
        .push(slug);
    open_url(profile, kind, world, url.into(), tx)
}
fn open_url(
    profile: GameProfile,
    kind: Kind,
    world: Option<PathBuf>,
    initial: String,
    tx: Sender<Event>,
) -> Result<()> {
    let mut owner = OWNER
        .lock()
        .map_err(|_| anyhow::anyhow!("Navegador ocupado"))?;
    anyhow::ensure!(
        owner.is_none(),
        "Cierra el navegador de contenido anterior para cambiar de instancia o de destino"
    );
    let dest = catalog::destination(&profile, kind, world.as_deref())?;
    *owner = Some(profile.id.clone());
    drop(owner);
    std::thread::spawn(move || {
        let _guard = Guard;
        let result = window(&profile, kind, &dest, &initial, &tx, false);
        if let Err(e) = result {
            let _ = tx.send(Event::Error(format!(
                "No se pudo abrir el navegador: {e:#}"
            )));
        }
    });
    Ok(())
}
#[cfg(windows)]
fn candidate(dir: &Path, name: &str, kind: Kind) -> Result<PathBuf> {
    catalog::safe_filename(name, kind)?;
    let original = Path::new(name);
    let stem = original.file_stem().unwrap().to_string_lossy();
    for i in 0..1000 {
        let name = if i == 0 {
            name.to_owned()
        } else {
            format!("{stem} ({i}).{}", kind.extension())
        };
        let path = dir.join(name);
        if !path.exists() {
            return Ok(path);
        }
    }
    anyhow::bail!("Hay demasiados archivos con el mismo nombre")
}
#[cfg(windows)]
fn ensure_runtime(tx: &Sender<Event>) -> Result<()> {
    if wry::webview_version().is_ok() {
        return Ok(());
    }
    tx.send(Event::Status(
        "Descargando el navegador de Microsoft para Luma…".into(),
    ))?;
    let file = model::root().join("downloads/MicrosoftEdgeWebview2Setup.exe");
    crate::net::download(
        &crate::net::client()?,
        "https://go.microsoft.com/fwlink/p/?LinkId=2124703",
        &file,
        None,
        false,
    )?;
    let quoted = format!(
        "'{}'",
        crate::cleanup::display_path(&file).replace('\'', "''")
    );
    let script=format!("$file={quoted};$signature=Get-AuthenticodeSignature -LiteralPath $file;if($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'Microsoft Corporation'){{throw 'El instalador de WebView2 no tiene una firma válida de Microsoft'}};$p=Start-Process -FilePath $file -ArgumentList @('/silent','/install') -WindowStyle Hidden -PassThru -Wait;if($p.ExitCode -ne 0){{throw 'No se pudo instalar WebView2'}}");
    crate::distribution::ps_output(&script)?;
    wry::webview_version().context("Instala Microsoft Edge WebView2 para abrir el navegador")?;
    let _ = std::fs::remove_file(file);
    Ok(())
}
#[cfg(windows)]
fn window(
    profile: &GameProfile,
    kind: Kind,
    dest: &Path,
    url: &str,
    tx: &Sender<Event>,
    probe: bool,
) -> Result<()> {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    use tao::{
        event::{Event as WindowEvent, WindowEvent as NativeEvent},
        event_loop::{ControlFlow, EventLoopBuilder},
        platform::{run_return::EventLoopExtRunReturn, windows::EventLoopBuilderExtWindows},
        window::WindowBuilder,
    };
    if probe {
        wry::webview_version().context("La prueba requiere WebView2 ya instalado")?;
    } else {
        ensure_runtime(tx)?;
    }
    let mut events = EventLoopBuilder::new().with_any_thread(true).build();
    let window = WindowBuilder::new()
        .with_visible(!probe)
        .with_title(format!(
            "Luma · {} · {} · Descargas a {}",
            profile.name,
            kind.label(),
            crate::cleanup::display_path(dest)
        ))
        .with_inner_size(tao::dpi::LogicalSize::new(1150., 780.))
        .build(&events)?;
    let stage = profile.game_dir.join(".luma/web-downloads").join(format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir_all(&stage)?;
    let data = if probe {
        profile.storage_dir.clone()
    } else {
        model::root()
    };
    let mut context = wry::WebContext::new(Some(data.join("webview")));
    let view: Rc<RefCell<Option<wry::WebView>>> = Rc::new(RefCell::new(None));
    let active = Rc::new(Cell::new(0usize));
    let title = serde_json::to_string(&format!(
        "{} · Minecraft {} · {} · {}",
        profile.name,
        profile.minecraft,
        kind.label(),
        crate::cleanup::display_path(dest)
    ))?;
    let modrinth = serde_json::to_string(&site_url(kind, false))?;
    let curse = serde_json::to_string(&site_url(kind, true))?;
    let toolbar = format!(
        r#"addEventListener('DOMContentLoaded',()=>{{if(window.top!==window||document.getElementById('luma-toolbar'))return;const bar=document.createElement('div');bar.id='luma-toolbar';bar.style.cssText='position:fixed;top:0;left:0;right:0;z-index:2147483647;background:#202328;color:#f2f4f6;display:flex;align-items:center;gap:8px;padding:8px 12px;font:13px system-ui;height:42px;box-sizing:border-box';for(const [text,action] of [['←',()=>history.back()],['→',()=>history.forward()],['↻',()=>location.reload()],['Modrinth',()=>location.href={modrinth}],['CurseForge',()=>location.href={curse}]]){{const b=document.createElement('button');b.textContent=text;b.onclick=action;b.style.cssText='background:#353940;border:0;border-radius:6px;color:white;padding:4px 8px;cursor:pointer';bar.append(b);}}const label=document.createElement('span');label.textContent={title};label.style.cssText='overflow:hidden;white-space:nowrap;text-overflow:ellipsis';bar.append(label);document.documentElement.append(bar);document.body.style.marginTop='42px';}});"#
    );
    let starter_tx = tx.clone();
    let starter_stage = stage.clone();
    let count = active.clone();
    let finished_tx = tx.clone();
    let finished_dest = dest.to_path_buf();
    let finished_stage = stage.clone();
    let instance = profile.id.clone();
    let count_done = active.clone();
    let completed = Rc::new(Cell::new(false));
    let complete_flag = completed.clone();
    let popup = view.clone();
    let webview = wry::WebViewBuilder::new_with_web_context(&mut context)
        .with_url(url)
        .with_background_color((22, 24, 28, 255))
        .with_initialization_script(&toolbar)
        .with_navigation_handler(|u| u.starts_with("https://") || u == "about:blank")
        .with_new_window_req_handler(move |u, _| {
            if u.starts_with("https://") {
                if let Some(view) = popup.borrow().as_ref() {
                    let _ = view.load_url(&u);
                }
            }
            wry::NewWindowResponse::Deny
        })
        .with_download_started_handler(move |u, path| {
            let result = (|| -> Result<PathBuf> {
                anyhow::ensure!(u.starts_with("https://"), "La descarga debe usar HTTPS");
                let name = path
                    .file_name()
                    .context("La web no proporcionó un nombre de archivo")?
                    .to_string_lossy();
                candidate(&starter_stage, &name, kind)
            })();
            match result {
                Ok(target) => {
                    *path = target;
                    count.set(count.get() + 1);
                    let _ = starter_tx.send(Event::Status(
                        "Descargando contenido desde el navegador…".into(),
                    ));
                    true
                }
                Err(e) => {
                    let _ = starter_tx.send(Event::Status(format!(
                        "Descarga rechazada: {e}. Elige {} en Cargar archivos.",
                        kind.label()
                    )));
                    false
                }
            }
        })
        .with_download_completed_handler(move |_, path, success| {
            count_done.set(count_done.get().saturating_sub(1));
            complete_flag.set(true);
            let result = (|| -> Result<()> {
                anyhow::ensure!(success, "La web canceló la descarga");
                let path = path
                    .context("La descarga no devolvió su archivo")?
                    .canonicalize()?;
                anyhow::ensure!(
                    path.starts_with(finished_stage.canonicalize()?),
                    "Archivo fuera del destino de descarga"
                );
                let name = path
                    .file_name()
                    .context("Nombre inválido")?
                    .to_string_lossy();
                catalog::safe_filename(&name, kind)?;
                anyhow::ensure!(
                    path.metadata()?.len() <= 1024 * 1024 * 1024,
                    "El archivo supera 1 GB"
                );
                zip::ZipArchive::new(std::fs::File::open(&path)?)
                    .context("El archivo descargado no es un JAR/ZIP válido")?;
                let target = candidate(&finished_dest, &name, kind)?;
                std::fs::rename(&path, &target)?;
                finished_tx.send(Event::ContentChanged(instance.clone()))?;
                finished_tx.send(Event::Status(format!(
                    "Guardado en {}",
                    crate::cleanup::display_path(&target)
                )))?;
                Ok(())
            })();
            if let Err(e) = result {
                let _ =
                    finished_tx.send(Event::Status(format!("No se instaló la descarga: {e:#}")));
            }
        })
        .build(&window)?;
    *view.borrow_mut() = Some(webview);
    tx.send(Event::Status(
        "Navegador abierto: las descargas pertenecen a esta instancia".into(),
    ))?;
    let started = std::time::Instant::now();
    events.run_return(|event, _, flow| {
        *flow = if probe {
            ControlFlow::WaitUntil(
                std::time::Instant::now() + std::time::Duration::from_millis(100),
            )
        } else {
            ControlFlow::Wait
        };
        match event {
            WindowEvent::MainEventsCleared
                if probe
                    && (completed.get()
                        || started.elapsed() > std::time::Duration::from_secs(35)) =>
            {
                *flow = ControlFlow::Exit
            }
            WindowEvent::WindowEvent {
                event: NativeEvent::CloseRequested,
                ..
            } => {
                if active.get() == 0 {
                    *flow = ControlFlow::Exit
                } else {
                    if rfd::MessageDialog::new()
                        .set_title("Hay descargas en curso")
                        .set_description(
                            "¿Cerrar el navegador y cancelar las descargas pendientes?",
                        )
                        .set_buttons(rfd::MessageButtons::YesNo)
                        .show()
                        == rfd::MessageDialogResult::Yes
                    {
                        *flow = ControlFlow::Exit;
                    }
                }
            }
            WindowEvent::WindowEvent {
                event: NativeEvent::Resized(size),
                ..
            } => {
                if let Some(view) = view.borrow().as_ref() {
                    let _ = view.set_bounds(wry::Rect {
                        position: tao::dpi::LogicalPosition::new(0., 0.).into(),
                        size: wry::dpi::PhysicalSize::new(size.width, size.height).into(),
                    });
                }
            }
            _ => (),
        }
    });
    *view.borrow_mut() = None;
    let _ = crate::cleanup::delete_verified_directory(&stage, &stage.canonicalize()?);
    Ok(())
}
#[cfg(not(windows))]
fn window(
    _: &GameProfile,
    kind: Kind,
    dest: &Path,
    url: &str,
    tx: &Sender<Event>,
    _: bool,
) -> Result<()> {
    webbrowser::open(url)?;
    tx.send(Event::Status(format!(
        "Web abierta en el navegador del sistema. Descarga el archivo y añádelo desde Cargar archivos → Elegir archivos… a {} ({}).",
        crate::cleanup::display_path(dest), kind.label()
    )))?;
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Abre un WebView2 oculto y descarga un archivo real en una carpeta aislada"]
    fn live_webview_download_uses_the_selected_instance_folder() {
        let profile = crate::catalog::test_profile();
        let version = crate::catalog::versions(&profile, Kind::Mod, "fabric-api")
            .unwrap()
            .remove(0);
        let file = version.files.iter().find(|f| f.primary).unwrap();
        let target = crate::catalog::destination(&profile, Kind::Mod, None).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        window(&profile, Kind::Mod, &target, &file.url, &tx, true).unwrap();
        for event in rx.try_iter() {
            if let Event::Status(s) = event {
                println!("{s}");
            }
        }
        assert_eq!(
            crate::net::sha1_file(&target.join(&file.filename)).unwrap(),
            *file.hashes.get("sha1").unwrap()
        );
        let mut cleaned = false;
        for _ in 0..10 {
            if crate::cleanup::delete_verified_directory(&profile.storage_dir, &profile.storage_dir)
                .is_ok()
            {
                cleaned = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        assert!(cleaned);
    }
}
