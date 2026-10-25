use crate::{
    catalog::{self, Kind},
    model::{self, Event, GameProfile},
};
use anyhow::{Context, Result};
use std::{
    io::{BufRead, Read, Write},
    path::{Path, PathBuf},
    sync::{mpsc::Sender, Mutex},
};
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
    catalog::destination(&profile, kind, world.as_deref())?;
    *owner = Some(profile.id.clone());
    drop(owner);
    std::thread::spawn(move || {
        let _guard = Guard;
        let result = child_window(profile, kind, world, initial, &tx);
        if let Err(e) = result {
            let _ = tx.send(Event::Error(format!(
                "No se pudo abrir el navegador: {e:#}"
            )));
        }
    });
    Ok(())
}
#[derive(serde::Serialize, serde::Deserialize)]
struct BrowserRequest {
    profile: GameProfile,
    kind: Kind,
    world: Option<PathBuf>,
    url: String,
}
fn child_window(
    profile: GameProfile,
    kind: Kind,
    world: Option<PathBuf>,
    url: String,
    tx: &Sender<Event>,
) -> Result<()> {
    use std::process::{Command, Stdio};
    let log_dir = model::root().join("browser");
    std::fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join("browser-latest.log");
    let log = std::fs::File::create(&log_path)?;
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("--content-browser")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(log);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .context("No se pudo iniciar el proceso del navegador")?;
    let request = BrowserRequest {
        profile,
        kind,
        world,
        url,
    };
    let input = serde_json::to_vec(&request)?;
    child
        .stdin
        .take()
        .context("Sin canal para el navegador")?
        .write_all(&input)?;
    if let Some(output) = child.stdout.take() {
        for line in std::io::BufReader::new(output).lines() {
            let line = line?;
            if let Ok((kind, message)) = serde_json::from_str::<(String, String)>(&line) {
                let event = match kind.as_str() {
                    "content" => Event::ContentChanged(message),
                    "error" => Event::Error(message),
                    _ => Event::Status(message),
                };
                let _ = tx.send(event);
            }
        }
    }
    let status = child.wait()?;
    anyhow::ensure!(
        status.success(),
        "El navegador se cerró inesperadamente ({status}). Luma sigue abierto. Registro: {}",
        log_path.display()
    );
    Ok(())
}
pub fn child_main() -> Result<()> {
    let mut input = Vec::new();
    std::io::stdin().take(1024 * 1024).read_to_end(&mut input)?;
    let request: BrowserRequest = serde_json::from_slice(&input)?;
    let url = url::Url::parse(&request.url)?;
    anyhow::ensure!(
        url.scheme() == "https"
            && matches!(url.host_str(), Some("modrinth.com" | "www.curseforge.com")),
        "Sitio de contenido no válido"
    );
    let dest = catalog::destination(&request.profile, request.kind, request.world.as_deref())?;
    let (tx, rx) = std::sync::mpsc::channel();
    let reporter = std::thread::spawn(move || {
        for event in rx {
            let pair = match event {
                Event::Status(s) => ("status", s),
                Event::Error(s) => ("error", s),
                Event::ContentChanged(s) => ("content", s),
                _ => continue,
            };
            if let Ok(line) = serde_json::to_string(&pair) {
                println!("{line}");
                let _ = std::io::stdout().flush();
            }
        }
    });
    let result = window(
        &request.profile,
        request.kind,
        &dest,
        &request.url,
        &tx,
        std::env::var_os("LUMA_BROWSER_PROBE").is_some(),
    );
    drop(tx);
    let _ = reporter.join();
    result
}
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
fn secure_download_source(uri: &str) -> bool {
    let Ok(url) = url::Url::parse(uri) else {
        return false;
    };
    let source = if url.scheme() == "blob" {
        let Ok(inner) = url::Url::parse(url.path()) else {
            return false;
        };
        inner
    } else {
        url
    };
    source.scheme() == "https"
        && source.host_str().is_some()
        && source.username().is_empty()
        && source.password().is_none()
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
    window_impl(profile, kind, dest, url, tx, probe, None)
}
#[cfg(windows)]
fn window_impl(
    profile: &GameProfile,
    kind: Kind,
    dest: &Path,
    url: &str,
    tx: &Sender<Event>,
    probe: bool,
    probe_script: Option<&str>,
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
        .with_visible(!probe || std::env::var_os("LUMA_BROWSER_VISIBLE_PROBE").is_some())
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
    // Chromium requires a native Windows path. Profiles read from JSON use
    // ordinary DOS paths, so join(".luma/web-downloads") otherwise leaves a
    // forward slash in ResultFilePath. Canonical fixtures hid this difference.
    let stage = stage.canonicalize()?;
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
        r#"addEventListener('DOMContentLoaded',()=>{{if(window.top!==window||document.getElementById('luma-toolbar'))return;const bar=document.createElement('div');bar.id='luma-toolbar';bar.style.cssText='position:fixed;top:0;left:0;right:0;z-index:2147483647;background:#202328;color:#f2f4f6;display:flex;align-items:center;gap:8px;padding:8px 12px;font:13px system-ui;height:42px;box-sizing:border-box';for(const [text,action] of [['←',()=>history.back()],['→',()=>history.forward()],['↻',()=>location.reload()],['Modrinth',()=>location.href={modrinth}],['CurseForge',()=>location.href={curse}],['Descargas',()=>window.ipc.postMessage('luma:downloads')]]){{const b=document.createElement('button');b.textContent=text;b.onclick=action;b.style.cssText='background:#353940;border:0;border-radius:6px;color:white;padding:4px 8px;cursor:pointer';bar.append(b);}}const label=document.createElement('span');label.textContent={title};label.style.cssText='overflow:hidden;white-space:nowrap;text-overflow:ellipsis';bar.append(label);document.documentElement.append(bar);document.body.style.marginTop='42px';}});"#
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
    let navigation = Rc::new(RefCell::new(Vec::<String>::new()));
    let popup = navigation.clone();
    let downloads_ui = Rc::new(Cell::new(false));
    let downloads_ipc = downloads_ui.clone();
    let rejections = Rc::new(RefCell::new(
        std::collections::HashMap::<String, String>::new(),
    ));
    let rejected_start = rejections.clone();
    let rejected_finish = rejections.clone();
    let webview = wry::WebViewBuilder::new_with_web_context(&mut context)
        .with_url(url)
        .with_background_color((22, 24, 28, 255))
        .with_initialization_script(&toolbar)
        .with_initialization_script(probe_script.unwrap_or(""))
        .with_ipc_handler(move |request| {
            if request.body() == "luma:downloads" {
                downloads_ipc.set(true);
            }
            if probe {
                eprintln!("browser-probe: {}", request.body());
            }
        })
        .with_navigation_handler(|u| u.starts_with("https://") || u == "about:blank")
        .with_new_window_req_handler(move |u, _| {
            if u.starts_with("https://") {
                // Defer navigation until outside WebView2's COM callback.
                popup.borrow_mut().push(u);
            }
            wry::NewWindowResponse::Deny
        })
        .with_download_started_handler(move |u, path| {
            eprintln!(
                "download-start name={:?} scheme={}",
                path.file_name(),
                u.split(':').next().unwrap_or("?")
            );
            let result = (|| -> Result<PathBuf> {
                anyhow::ensure!(
                    secure_download_source(&u),
                    "La descarga debe usar HTTPS o un blob de una página HTTPS"
                );
                let name = path
                    .file_name()
                    .context("La web no proporcionó un nombre de archivo")?
                    .to_string_lossy();
                candidate(&starter_stage, &name, kind)
            })();
            match result {
                Ok(target) => {
                    eprintln!("download-target {}", crate::cleanup::display_path(&target));
                    *path = target;
                    count.set(count.get() + 1);
                    let _ = starter_tx.send(Event::Status(
                        "Descargando contenido desde el navegador…".into(),
                    ));
                    true
                }
                Err(e) => {
                    eprintln!("download-rejected: {e:#}");
                    rejected_start.borrow_mut().insert(u, e.to_string());
                    let _ = starter_tx.send(Event::Status(format!(
                        "Descarga rechazada: {e}. Elige {} en Cargar archivos.",
                        kind.label()
                    )));
                    false
                }
            }
        })
        .with_download_completed_handler(move |uri, path, success| {
            eprintln!("download-finish success={success} path={path:?}");
            if rejected_finish.borrow_mut().remove(&uri).is_some() {
                complete_flag.set(true);
                return;
            }
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
    attach_download_diagnostics(&webview, tx.clone(), downloads_ui.clone())?;
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
            WindowEvent::MainEventsCleared => {
                if downloads_ui.replace(false) {
                    use wry::WebViewExtWindows;
                    use windows_core::Interface;
                    if let Some(view) = view.borrow().as_ref() {
                        if let Ok(downloads) = view.webview().cast::<webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_9>() {
                            let _ = unsafe { downloads.OpenDefaultDownloadDialog() };
                        }
                    }
                }
                let next = navigation.borrow_mut().pop();
                if let Some(url) = next {
                    if let Some(view) = view.borrow().as_ref() {
                        let _ = view.load_url(&url);
                    }
                }
                if probe
                    && (completed.get() || started.elapsed() > std::time::Duration::from_secs(if probe_script.is_some() { 90 } else { 35 }))
                {
                    *flow = ControlFlow::Exit;
                }
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
#[cfg(windows)]
fn attach_download_diagnostics(
    view: &wry::WebView,
    tx: Sender<Event>,
    downloads_ui: std::rc::Rc<std::cell::Cell<bool>>,
) -> Result<()> {
    use webview2_com::{
        BytesReceivedChangedEventHandler, DownloadStartingEventHandler,
        Microsoft::Web::WebView2::Win32::*, StateChangedEventHandler,
    };
    use windows_core::Interface;
    use wry::WebViewExtWindows;
    let view: ICoreWebView2_4 = view.webview().cast()?;
    unsafe {
        view.add_DownloadStarting(&DownloadStartingEventHandler::create(Box::new(move |_, args| {
            let Some(args) = args else { return Ok(()); };
            let operation = args.DownloadOperation()?;
            // Wry suppresses the native UI. Keep it available so pending browser
            // decisions (notably generated JAR downloads) remain visible.
            args.SetHandled(false)?;
            downloads_ui.set(true);
            operation.add_BytesReceivedChanged(&BytesReceivedChangedEventHandler::create(Box::new(move |operation, _| {
                if let Some(operation) = operation {
                    let mut received = 0;
                    let mut total = 0;
                    operation.BytesReceived(&mut received)?;
                    operation.TotalBytesToReceive(&mut total)?;
                    eprintln!("download-bytes {received}/{total}");
                }
                Ok(())
            })), &mut 0)?;
            let tx = tx.clone();
            operation.add_StateChanged(&StateChangedEventHandler::create(Box::new(move |operation, _| {
                let Some(operation) = operation else { return Ok(()); };
                let mut state = COREWEBVIEW2_DOWNLOAD_STATE::default();
                operation.State(&mut state)?;
                eprintln!("download-state {}", state.0);
                if state == COREWEBVIEW2_DOWNLOAD_STATE_INTERRUPTED {
                    let mut reason = COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON::default();
                    operation.InterruptReason(&mut reason)?;
                    eprintln!("webview-download-interrupted reason={}", reason.0);
                    if reason != COREWEBVIEW2_DOWNLOAD_INTERRUPT_REASON_USER_CANCELED {
                        let _ = tx.send(Event::Status(format!("Descarga interrumpida por WebView2 (motivo {}). Registro: browser/browser-latest.log", reason.0)));
                    }
                }
                Ok(())
            })), &mut 0)?;
            Ok(())
        })), &mut 0)?;
    }
    Ok(())
}
#[cfg(not(windows))]
fn window(_: &GameProfile, _: Kind, _: &Path, _: &str, _: &Sender<Event>, _: bool) -> Result<()> {
    anyhow::bail!("El navegador integrado requiere Windows")
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn download_sources_accept_https_blobs_but_not_local_or_opaque_origins() {
        for uri in [
            "https://cdn.modrinth.com/a.jar",
            "blob:https://modrinth.com/uuid",
            "blob:https://www.curseforge.com/uuid",
        ] {
            assert!(secure_download_source(uri), "{uri}");
        }
        for uri in [
            "blob:null/uuid",
            "blob:http://modrinth.com/uuid",
            "data:application/zip;base64,AA",
            "file:///C:/test.jar",
            "http://example.com/a.jar",
            "blob:blob:https://modrinth.com/uuid",
            "https://user:pass@example.com/a.jar",
        ] {
            assert!(!secure_download_source(uri), "{uri}");
        }
    }
    #[test]
    #[ignore = "Abre un WebView2 oculto y descarga un archivo real en una carpeta aislada"]
    fn live_webview_download_uses_the_selected_instance_folder() {
        let mut profile = crate::catalog::test_profile();
        profile.game_dir = PathBuf::from(crate::cleanup::display_path(&profile.game_dir));
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

    #[test]
    #[ignore = "Descarga real mediante el enlace blob de una pagina HTTPS en WebView2"]
    fn live_webview_blob_download_from_page() {
        let profile = crate::catalog::test_profile();
        let version = crate::catalog::versions(&profile, Kind::Mod, "fabric-api")
            .unwrap()
            .remove(0);
        let file = version.files.iter().find(|f| f.primary).unwrap();
        let target = crate::catalog::destination(&profile, Kind::Mod, None).unwrap();
        let script = format!(
            r#"addEventListener('DOMContentLoaded',async()=>{{try{{const response=await fetch({});if(!response.ok)throw new Error(response.status);const blob=await response.blob();const a=document.createElement('a');a.href=URL.createObjectURL(blob);a.download={};document.body.append(a);window.ipc.postMessage('blob link ready');a.click();}}catch(e){{window.ipc.postMessage('probe failed: '+String(e));}}}});"#,
            serde_json::to_string(&file.url).unwrap(),
            serde_json::to_string(&file.filename).unwrap()
        );
        let (tx, rx) = std::sync::mpsc::channel();
        window_impl(
            &profile,
            Kind::Mod,
            &target,
            "https://modrinth.com",
            &tx,
            true,
            Some(&script),
        )
        .unwrap();
        for event in rx.try_iter() {
            if let Event::Status(s) = event {
                println!("{s}");
            }
        }
        let result = crate::net::sha1_file(&target.join(&file.filename));
        let _ =
            crate::cleanup::delete_verified_directory(&profile.storage_dir, &profile.storage_dir);
        assert_eq!(result.unwrap(), *file.hashes.get("sha1").unwrap());
    }
    #[test]
    #[ignore = "Descarga un ZIP blob generado por una página HTTPS en WebView2"]
    fn live_webview_blob_zip_download_from_page() {
        use base64::Engine;
        let mut profile = crate::catalog::test_profile();
        // Match persisted profiles, which have non-verbatim paths, unlike
        // test_profile's canonical (verbatim) Windows paths.
        profile.game_dir = PathBuf::from(crate::cleanup::display_path(&profile.game_dir));
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file("pack.mcmeta", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(br#"{"pack":{"pack_format":34,"description":"Luma download regression"}}"#)
            .unwrap();
        let bytes = zip.finish().unwrap().into_inner();
        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let target = crate::catalog::destination(&profile, Kind::ResourcePack, None).unwrap();
        let script = format!(
            r#"addEventListener('DOMContentLoaded',()=>{{const bytes=Uint8Array.from(atob('{}'),c=>c.charCodeAt(0));const blob=new Blob([bytes],{{type:'application/zip'}});const a=document.createElement('a');a.href=URL.createObjectURL(blob);a.download='regression.zip';document.body.append(a);window.ipc.postMessage('zip blob ready');a.click();}});"#,
            encoded
        );
        let (tx, rx) = std::sync::mpsc::channel();
        window_impl(
            &profile,
            Kind::ResourcePack,
            &target,
            "https://modrinth.com",
            &tx,
            true,
            Some(&script),
        )
        .unwrap();
        for event in rx.try_iter() {
            if let Event::Status(s) = event {
                println!("{s}");
            }
        }
        let result = std::fs::read(target.join("regression.zip"));
        let _ =
            crate::cleanup::delete_verified_directory(&profile.storage_dir, &profile.storage_dir);
        assert_eq!(result.unwrap(), bytes);
    }
}
