use crate::{
    engine,
    model::{self, Event, GameProfile, Loader, Manifest, ProfileRequest, Settings, Version},
    net,
};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::{
    collections::HashSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::Sender,
};

pub fn load(root: &Path) -> Result<Vec<GameProfile>> {
    let path = root.join("profiles.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    model::read_json(&path)
}

fn save(root: &Path, profiles: &[GameProfile]) -> Result<()> {
    model::write_json(&root.join("profiles.json"), &profiles)
}

fn unique_id() -> Result<String> {
    let mut bytes = [0u8; 12];
    getrandom::getrandom(&mut bytes)
        .map_err(|_| anyhow::anyhow!("No se pudo crear el identificador del perfil"))?;
    Ok(format!(
        "luma-{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    ))
}

fn canonical_path(path: &Path) -> Result<PathBuf> {
    let canonical = fs::canonicalize(path)?;
    #[cfg(windows)]
    {
        let text = canonical.to_string_lossy();
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            return Ok(PathBuf::from(format!(r"\\{rest}")));
        }
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            return Ok(PathBuf::from(rest));
        }
    }
    Ok(canonical)
}

pub fn vanilla_version(id: &str) -> Result<Version> {
    let manifest: Manifest = model::read_json(&model::root().join("manifest.json"))?;
    manifest
        .versions
        .into_iter()
        .find(|v| v.id == id)
        .with_context(|| {
            format!("Minecraft {id} no está en el catálogo. Actualiza las versiones con conexión.")
        })
}

fn infer_loader(value: &Value) -> Loader {
    let text = format!(
        "{} {}",
        value["id"].as_str().unwrap_or(""),
        value["libraries"]
    );
    if text.contains("neoforge") || text.contains("net.neoforged") {
        Loader::NeoForge
    } else if text.contains("fabric-loader") || text.contains("net.fabricmc") {
        Loader::Fabric
    } else if text.to_lowercase().contains("forge") {
        Loader::Forge
    } else {
        Loader::Vanilla
    }
}

fn base_version(storage: &Path, id: &str, seen: &mut HashSet<String>) -> Result<String> {
    model::safe_component(id)?;
    anyhow::ensure!(
        seen.len() < 16 && seen.insert(id.into()),
        "Herencia circular en {id}"
    );
    let metadata: Value =
        model::read_json(&storage.join("versions").join(id).join(format!("{id}.json")))?;
    if let Some(parent) = metadata["inheritsFrom"].as_str() {
        let path = storage
            .join("versions")
            .join(model::safe_component(parent)?)
            .join(format!("{parent}.json"));
        if path.is_file() {
            base_version(storage, parent, seen)
        } else {
            Ok(parent.into())
        }
    } else {
        Ok(id.into())
    }
}

fn detected_profile(
    root: &Path,
    storage: &Path,
    id: &str,
    name: Option<&str>,
    game: Option<PathBuf>,
) -> Result<GameProfile> {
    model::safe_component(id)?;
    let metadata: Value =
        model::read_json(&storage.join("versions").join(id).join(format!("{id}.json")))?;
    let loader = infer_loader(&metadata);
    let minecraft = base_version(storage, id, &mut HashSet::new())?;
    let imported = storage != root;
    let mut hash = Sha1::new();
    hash.update(storage.to_string_lossy().to_lowercase().as_bytes());
    hash.update(id.as_bytes());
    if let Some(path) = &game {
        hash.update(path.to_string_lossy().to_lowercase().as_bytes());
    }
    let profile_id = format!("import-{:x}", hash.finalize());
    let game_dir = game.unwrap_or_else(|| root.join("instances").join(&profile_id));
    Ok(GameProfile {
        id: profile_id,
        name: name.unwrap_or(id).into(),
        minecraft,
        version_id: id.into(),
        loader,
        loader_version: String::new(),
        storage_dir: storage.into(),
        game_dir,
        imported,
    })
}

/// Reads installer-created launcher entries and version JSONs, including the
/// official launcher's directory. Existing profiles and folders stay in place.
pub fn discover(root: &Path, extra: Option<&Path>) -> Result<Vec<GameProfile>> {
    let profiles = load(root)?;
    let mut sources = vec![root.to_path_buf()];
    if let Some(roaming) = std::env::var_os("APPDATA") {
        sources.push(PathBuf::from(roaming).join(".minecraft"));
    }
    sources.extend(profiles.iter().map(|p| p.storage_dir.clone()));
    if let Some(path) = extra {
        anyhow::ensure!(
            path.is_dir(),
            "No existe la carpeta de Minecraft: {}",
            path.display()
        );
        anyhow::ensure!(
            path.join("versions").is_dir(),
            "La carpeta debe contener versions/ y los archivos del cliente de Minecraft."
        );
        sources.push(canonical_path(path)?);
    }
    discover_sources(root, profiles, sources)
}

fn discover_sources(
    root: &Path,
    mut profiles: Vec<GameProfile>,
    sources: Vec<PathBuf>,
) -> Result<Vec<GameProfile>> {
    let original = profiles.clone();
    let mut seen_sources = HashSet::new();
    for source in sources {
        let key = source.to_string_lossy().to_lowercase();
        if !seen_sources.insert(key) {
            continue;
        }
        for filename in [
            "launcher_profiles.json",
            "launcher_profiles_microsoft_store.json",
        ] {
            if let Ok(launcher) = model::read_json::<Value>(&source.join(filename)) {
                if let Some(entries) = launcher["profiles"].as_object() {
                    for entry in entries.values() {
                        let Some(id) = entry["lastVersionId"].as_str() else {
                            continue;
                        };
                        let game = entry["gameDir"]
                            .as_str()
                            .map(PathBuf::from)
                            .filter(|p| p.is_absolute())
                            .or_else(|| (source != root).then(|| source.clone()));
                        if let Ok(profile) =
                            detected_profile(root, &source, id, entry["name"].as_str(), game)
                        {
                            if !profiles.iter().any(|p| {
                                p.storage_dir == profile.storage_dir
                                    && p.version_id == profile.version_id
                                    && p.game_dir == profile.game_dir
                            }) {
                                // A profile created by Luma already points to this installer output.
                                if source == root
                                    && profiles
                                        .iter()
                                        .any(|p| p.storage_dir == source && p.version_id == id)
                                {
                                    continue;
                                }
                                profiles.push(profile);
                            }
                        }
                    }
                }
            }
        }
        if let Ok(entries) = fs::read_dir(source.join("versions")) {
            for entry in entries.filter_map(Result::ok) {
                let id = entry.file_name().to_string_lossy().into_owned();
                if profiles
                    .iter()
                    .any(|p| p.storage_dir == source && p.version_id == id)
                {
                    continue;
                }
                if let Ok(profile) = detected_profile(root, &source, &id, None, None) {
                    if profile.loader != Loader::Vanilla {
                        profiles.push(profile);
                    }
                }
            }
        }
    }
    if profiles != original {
        save(root, &profiles)?;
    }
    Ok(profiles)
}

pub fn loader_versions(loader: Loader, minecraft: &str) -> Result<Vec<String>> {
    model::safe_component(minecraft)?;
    let client = net::client()?;
    let mut versions = if loader == Loader::Vanilla {
        vec![]
    } else if loader == Loader::Fabric {
        let mut url = url::Url::parse("https://meta.fabricmc.net/v2/versions/loader/")?;
        url.path_segments_mut()
            .map_err(|_| anyhow::anyhow!("URL de Fabric inválida"))?
            .pop_if_empty()
            .push(minecraft);
        let list: Value = client.get(url).send()?.error_for_status()?.json()?;
        list.as_array()
            .context("Fabric devolvió una lista inválida")?
            .iter()
            .filter_map(|item| item["loader"]["version"].as_str().map(str::to_owned))
            .collect()
    } else {
        let (url, prefix) = if loader == Loader::Forge {
            (
                "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml",
                format!("{minecraft}-"),
            )
        } else if minecraft == "1.20.1" {
            (
                "https://maven.neoforged.net/releases/net/neoforged/forge/maven-metadata.xml",
                "1.20.1-".into(),
            )
        } else {
            let pieces: Vec<_> = minecraft.split('.').collect();
            let prefix = if pieces.first() == Some(&"1") && pieces.len() >= 2 {
                format!("{}.{}.", pieces[1], pieces.get(2).unwrap_or(&"0"))
            } else {
                format!("{minecraft}.")
            };
            (
                "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml",
                prefix,
            )
        };
        let xml = client.get(url).send()?.error_for_status()?.text()?;
        let pattern = regex::Regex::new(r"<version>\s*([^<]+?)\s*</version>")?;
        let mut versions: Vec<String> = pattern
            .captures_iter(&xml)
            .map(|c| c[1].to_string())
            .filter(|v| v.starts_with(&prefix))
            .collect();
        versions.reverse();
        versions
    };
    let mut seen = HashSet::new();
    versions.retain(|v| model::safe_component(v).is_ok() && seen.insert(v.clone()));
    anyhow::ensure!(
        loader == Loader::Vanilla || !versions.is_empty(),
        "{} no publica versiones compatibles con Minecraft {minecraft}.",
        loader.label()
    );
    Ok(versions)
}

fn zip_json(path: &Path, name: &str) -> Result<Value> {
    let mut zip = zip::ZipArchive::new(fs::File::open(path)?)?;
    let mut entry = zip.by_name(name.trim_start_matches('/'))?;
    anyhow::ensure!(
        entry.size() <= 16 * 1024 * 1024,
        "Metadatos del instalador demasiado grandes"
    );
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn installer_info(path: &Path) -> Result<(Value, Value, String, Loader)> {
    let install = zip_json(path, "install_profile.json").context("El archivo debe ser un instalador de cliente Forge o NeoForge. Para Fabric usa Crear perfil o ejecuta su instalador y pulsa Detectar perfiles.")?;
    let version = if install.get("versionInfo").is_some() {
        install["versionInfo"].clone()
    } else {
        zip_json(path, install["json"].as_str().unwrap_or("version.json"))?
    };
    let minecraft = install["minecraft"]
        .as_str()
        .or(install["install"]["minecraft"].as_str())
        .or(version["inheritsFrom"].as_str())
        .context("El instalador no declara su versión de Minecraft")?
        .to_owned();
    let loader = if infer_loader(&version) == Loader::NeoForge {
        Loader::NeoForge
    } else {
        Loader::Forge
    };
    Ok((install, version, minecraft, loader))
}

fn extract_legacy_installer(
    archive: &Path,
    root: &Path,
    install: &Value,
    version: &Value,
) -> Result<()> {
    let mut zip = zip::ZipArchive::new(fs::File::open(archive)?)?;
    // Old installers embed the Forge universal jar and a ready-to-use versionInfo.
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let enclosed = entry
            .enclosed_name()
            .context("Ruta no segura dentro del instalador")?;
        let name = entry.name().to_owned();
        let target = if let Some(relative) = name.strip_prefix("maven/") {
            Some(engine::path_under(&root.join("libraries"), relative)?)
        } else if install["install"]["filePath"].as_str() == Some(name.as_str()) {
            Some(engine::path_under(
                &root.join("libraries"),
                &engine::maven_path(
                    install["install"]["path"]
                        .as_str()
                        .context("Instalador sin coordenada Forge")?,
                )?,
            )?)
        } else {
            None
        };
        let _ = enclosed;
        if let Some(target) = target {
            fs::create_dir_all(target.parent().context("Biblioteca sin directorio")?)?;
            std::io::copy(&mut entry, &mut fs::File::create(target)?)?;
        }
    }
    let id = model::safe_component(
        version["id"]
            .as_str()
            .context("El loader no incluye un ID")?,
    )?;
    model::write_json(
        &root.join("versions").join(id).join(format!("{id}.json")),
        version,
    )
}

fn run_installer(
    root: &Path,
    archive: &Path,
    java: &Path,
    key: &str,
    tx: &Sender<Event>,
) -> Result<String> {
    let (install, version, _, loader) = installer_info(archive)?;
    let id = model::safe_component(
        version["id"]
            .as_str()
            .context("El instalador no declara un ID de cliente")?,
    )?
    .to_owned();
    tx.send(Event::Status(format!(
        "Instalando {} · preparando el cliente y sus parches…",
        loader.label()
    )))?;
    if install.get("versionInfo").is_some() && install.get("processors").is_none() {
        extract_legacy_installer(archive, root, &install, &version)?;
    } else {
        let launcher_file = root.join("launcher_profiles.json");
        if !launcher_file.exists() {
            model::write_json(&launcher_file, &json!({"profiles":{}}))?;
        }
        let logs = root.join("logs");
        fs::create_dir_all(&logs)?;
        let log_path = logs.join(format!("installer-{key}.log"));
        let log = fs::File::create(&log_path)?;
        let mut command = Command::new(java);
        command
            .arg("-Djava.awt.headless=true")
            .arg("-jar")
            .arg(archive)
            .arg("--installClient")
            .arg(root)
            .current_dir(&logs)
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let status = command
            .status()
            .context("No se pudo ejecutar el instalador del loader")?;
        anyhow::ensure!(
            status.success(),
            "El instalador de {} falló. Consulta {}",
            loader.label(),
            log_path.display()
        );
    }
    let output = root.join("versions").join(&id).join(format!("{id}.json"));
    anyhow::ensure!(
        output.is_file(),
        "El instalador no creó el cliente esperado: {}",
        output.display()
    );
    Ok(id)
}

pub fn create(
    root: &Path,
    request: &ProfileRequest,
    settings: &Settings,
    tx: &Sender<Event>,
) -> Result<GameProfile> {
    create_inner(root, request, settings, None, tx)
}

pub fn install_local_jar(
    root: &Path,
    path: &Path,
    name: &str,
    settings: &Settings,
    tx: &Sender<Event>,
) -> Result<GameProfile> {
    let (_, _, minecraft, loader) = installer_info(path)?;
    let request = ProfileRequest {
        name: name.into(),
        minecraft,
        loader,
        loader_version: "Instalador local".into(),
    };
    create_inner(root, &request, settings, Some(path), tx)
}

fn create_inner(
    root: &Path,
    request: &ProfileRequest,
    settings: &Settings,
    local_jar: Option<&Path>,
    tx: &Sender<Event>,
) -> Result<GameProfile> {
    anyhow::ensure!(
        !settings.offline,
        "Desactiva Jugar sin conexión para crear e instalar un perfil nuevo."
    );
    anyhow::ensure!(
        !request.name.trim().is_empty() && request.name.chars().count() <= 80,
        "Escribe un nombre para el perfil de hasta 80 caracteres."
    );
    let vanilla = vanilla_version(&request.minecraft)?;
    let id = unique_id()?;
    let mut profile = GameProfile {
        id: id.clone(),
        name: request.name.trim().into(),
        minecraft: request.minecraft.clone(),
        version_id: vanilla.id.clone(),
        loader: request.loader,
        loader_version: request.loader_version.clone(),
        storage_dir: root.into(),
        game_dir: root.join("instances").join(&id),
        imported: false,
    };
    let mut scoped_settings = settings.clone();
    scoped_settings.game_directory = Some(profile.game_dir.clone());
    let base = engine::prepare(root, &vanilla, &scoped_settings, false, tx)?;
    if request.loader == Loader::Fabric {
        model::safe_component(&request.loader_version)?;
        tx.send(Event::Status("Instalando Fabric Loader…".into()))?;
        let mut url = url::Url::parse("https://meta.fabricmc.net/v2/versions/loader/")?;
        url.path_segments_mut()
            .map_err(|_| anyhow::anyhow!("URL Fabric inválida"))?
            .pop_if_empty()
            .extend([
                request.minecraft.as_str(),
                request.loader_version.as_str(),
                "profile",
                "json",
            ]);
        let metadata: Value = net::client()?.get(url).send()?.error_for_status()?.json()?;
        anyhow::ensure!(
            metadata["inheritsFrom"] == request.minecraft,
            "Fabric devolvió un cliente para otra versión de Minecraft"
        );
        let version_id = model::safe_component(
            metadata["id"]
                .as_str()
                .context("Fabric no devolvió un perfil de cliente")?,
        )?;
        model::write_json(
            &root
                .join("versions")
                .join(version_id)
                .join(format!("{version_id}.json")),
            &metadata,
        )?;
        profile.version_id = version_id.into();
    } else if matches!(request.loader, Loader::Forge | Loader::NeoForge) {
        let archive = if let Some(path) = local_jar {
            canonical_path(path)?
        } else {
            model::safe_component(&request.loader_version)?;
            let version = &request.loader_version;
            let url = match request.loader {
                Loader::Forge => format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{version}/forge-{version}-installer.jar"),
                Loader::NeoForge if request.minecraft == "1.20.1" => format!("https://maven.neoforged.net/releases/net/neoforged/forge/{version}/forge-{version}-installer.jar"),
                _ => format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{version}/neoforge-{version}-installer.jar"),
            };
            let client = net::client()?;
            let hash = client
                .get(format!("{url}.sha1"))
                .send()?
                .error_for_status()?
                .text()?;
            let hash = hash
                .split_whitespace()
                .next()
                .context("El instalador no incluye SHA-1")?;
            anyhow::ensure!(
                hash.len() == 40 && hash.bytes().all(|c| c.is_ascii_hexdigit()),
                "Suma de verificación del instalador inválida"
            );
            let path = root.join("installers").join(format!(
                "{}-{version}.jar",
                request.loader.label().to_lowercase()
            ));
            tx.send(Event::Status(format!(
                "Descargando el instalador oficial de {}…",
                request.loader.label()
            )))?;
            net::download(
                &client,
                &url,
                &path,
                Some(&hash.to_ascii_lowercase()),
                false,
            )?;
            path
        };
        let (_, _, installer_mc, installer_loader) = installer_info(&archive)?;
        anyhow::ensure!(
            installer_mc == request.minecraft && installer_loader == request.loader,
            "El instalador pertenece a otra versión o cargador"
        );
        profile.version_id = run_installer(root, &archive, &base.java, &profile.id, tx)?;
    }
    if request.loader != Loader::Vanilla {
        engine::prepare(
            root,
            &Version::local(&profile.version_id),
            &scoped_settings,
            false,
            tx,
        )?;
        fs::create_dir_all(profile.game_dir.join("mods"))?;
    }
    let mut list = load(root)?;
    list.push(profile.clone());
    save(root, &list)?;
    tx.send(Event::ProfileCreated(profile.clone()))?;
    tx.send(Event::Profiles(list))?;
    tx.send(Event::Status(format!(
        "{} listo · {} {}",
        profile.name,
        profile.loader.label(),
        profile.minecraft
    )))?;
    Ok(profile)
}

pub fn launch_version(profile: &GameProfile) -> Result<Version> {
    if profile.loader == Loader::Vanilla {
        vanilla_version(&profile.version_id).or_else(|error| {
            if profile
                .storage_dir
                .join("versions")
                .join(&profile.version_id)
                .join(format!("{}.json", profile.version_id))
                .is_file()
            {
                Ok(Version::local(&profile.version_id))
            } else {
                Err(error)
            }
        })
    } else {
        Ok(Version::local(&profile.version_id))
    }
}

pub fn rename(root: &Path, id: &str, name: &str) -> Result<Vec<GameProfile>> {
    anyhow::ensure!(
        !name.trim().is_empty() && name.chars().count() <= 80,
        "El nombre debe tener entre 1 y 80 caracteres"
    );
    let mut list = load(root)?;
    let profile = list
        .iter_mut()
        .find(|p| p.id == id)
        .context("No se encontró el perfil")?;
    profile.name = name.trim().into();
    save(root, &list)?;
    Ok(list)
}

pub fn cli(args: &[String]) -> Result<()> {
    let root = model::root();
    let (tx, rx) = std::sync::mpsc::channel();
    let reporter = std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            match event {
                Event::Status(text) => println!("{text}"),
                Event::Progress(n, total) if n.is_multiple_of(500) || n == total => {
                    println!("{n}/{total}")
                }
                Event::GameStarted => println!("Minecraft iniciado"),
                Event::GameExited(code) => println!("Minecraft terminó: {code:?}"),
                _ => (),
            }
        }
    });
    let result = (|| -> Result<()> {
        let mut settings: Settings =
            model::read_json(&root.join("settings.json")).unwrap_or_default();
        match args[1].as_str() {
            "--list-profiles" => {
                for profile in discover(&root, None)? {
                    println!(
                        "{} | {} | {} | {}",
                        profile.id,
                        profile.name,
                        profile.loader.label(),
                        profile.version_id
                    );
                }
            }
            "--import-profiles" => {
                let extra = args.get(2).map(Path::new);
                println!("{} perfiles", discover(&root, extra)?.len());
            }
            "--create-profile" => {
                let name = args.get(2).context("Falta NOMBRE")?;
                let minecraft = args.get(3).context("Falta VERSION de Minecraft")?;
                let loader = Loader::ALL
                    .into_iter()
                    .find(|l| {
                        Some(l.label().to_lowercase()) == args.get(4).map(|s| s.to_lowercase())
                    })
                    .context("Loader: vanilla, fabric, neoforge o forge")?;
                settings.offline = false;
                engine::catalog(&root, &tx)?;
                let loader_version = if loader == Loader::Vanilla {
                    String::new()
                } else if let Some(version) = args.get(5) {
                    version.clone()
                } else {
                    loader_versions(loader, minecraft)?
                        .first()
                        .context("No hay versiones del loader")?
                        .clone()
                };
                let profile = create(
                    &root,
                    &ProfileRequest {
                        name: name.clone(),
                        minecraft: minecraft.clone(),
                        loader,
                        loader_version,
                    },
                    &settings,
                    &tx,
                )?;
                println!("PROFILE_ID={}", profile.id);
            }
            "--install-loader-jar" => {
                let path = Path::new(args.get(2).context("Falta la ruta al instalador .jar")?);
                settings.offline = false;
                engine::catalog(&root, &tx)?;
                let profile = install_local_jar(
                    &root,
                    path,
                    args.get(3)
                        .map(String::as_str)
                        .unwrap_or("Cliente con mods"),
                    &settings,
                    &tx,
                )?;
                println!("PROFILE_ID={}", profile.id);
            }
            command => {
                let id = args.get(2).context("Falta el identificador del perfil")?;
                let profile = load(&root)?
                    .into_iter()
                    .find(|p| &p.id == id)
                    .context("Perfil no encontrado")?;
                settings.game_directory = Some(profile.game_dir.clone());
                let offline = command != "--install-profile";
                let version = launch_version(&profile)?;
                let installation =
                    engine::prepare(&profile.storage_dir, &version, &settings, offline, &tx)?;
                if command == "--play-profile" {
                    settings.local_name = args.get(3).cloned().unwrap_or(settings.local_name);
                    settings.profile = Some(model::Profile::offline(&settings.local_name)?);
                    let (identity, token) = settings.launch_identity(None)?;
                    engine::launch(
                        &profile.storage_dir,
                        &version,
                        &settings,
                        installation,
                        &identity,
                        &token,
                        &tx,
                    )?;
                } else {
                    println!("OK · {}", profile.name);
                }
            }
        }
        Ok(())
    })();
    drop(tx);
    let _ = reporter.join();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_loaders_without_confusing_forge_and_neoforge() {
        assert_eq!(
            infer_loader(&json!({"libraries":[{"name":"net.neoforged:neoforge:21.1.1"}]})),
            Loader::NeoForge
        );
        assert_eq!(
            infer_loader(&json!({"id":"fabric-loader-0.16.0-1.21"})),
            Loader::Fabric
        );
        assert_eq!(
            infer_loader(&json!({"id":"1.20.1-forge-47.0.1"})),
            Loader::Forge
        );
        assert_eq!(infer_loader(&json!({"id":"1.21.1"})), Loader::Vanilla);
    }

    #[test]
    fn profile_identity_and_renaming_preserve_separate_directories() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/test-data")
            .join(unique_id().unwrap());
        let metadata = root.join("versions/fabric-loader-test/fabric-loader-test.json");
        model::write_json(
            &metadata,
            &json!({"id":"fabric-loader-test", "inheritsFrom":"1.21.1"}),
        )
        .unwrap();
        let a = detected_profile(&root, &root, "fabric-loader-test", Some("A"), None).unwrap();
        let b = GameProfile {
            id: unique_id().unwrap(),
            name: "B".into(),
            game_dir: root.join("instances/another"),
            ..a.clone()
        };
        assert_ne!(a.game_dir, b.game_dir);
        save(&root, &[a.clone(), b]).unwrap();
        // Scan only this fixture; registered sources and .minecraft may contain user installations.
        assert_eq!(
            base_version(&root, &a.version_id, &mut HashSet::new()).unwrap(),
            "1.21.1"
        );
        let detected = detected_profile(&root, &root, "fabric-loader-test", None, None).unwrap();
        assert_eq!(a.id, detected.id);
        let renamed = rename(&root, &a.id, "Nuevo nombre").unwrap();
        assert_eq!(renamed[0].game_dir, a.game_dir);
        assert!(metadata.exists());
    }

    #[test]
    fn imports_installer_profiles_preserving_game_directory_and_avoiding_duplicates() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/test-data")
            .join(unique_id().unwrap());
        let root = fixture.join("luma");
        let external = fixture.join("external-minecraft");
        let game = fixture.join("existing-modpack");
        model::write_json(
            &external.join("versions/fabric-loader-test/fabric-loader-test.json"),
            &json!({"id":"fabric-loader-test","inheritsFrom":"1.21.1"}),
        )
        .unwrap();
        let launcher = json!({"profiles":{"loader":{"name":"My existing mods","lastVersionId":"fabric-loader-test","gameDir":game}}});
        for filename in [
            "launcher_profiles.json",
            "launcher_profiles_microsoft_store.json",
        ] {
            model::write_json(&external.join(filename), &launcher).unwrap();
        }
        let original = fs::read(external.join("launcher_profiles.json")).unwrap();
        let first = discover_sources(&root, Vec::new(), vec![external.clone()]).unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].name, "My existing mods");
        assert_eq!(first[0].game_dir, game);
        assert_eq!(first[0].loader, Loader::Fabric);
        assert!(first[0].imported);
        let second = discover_sources(&root, first.clone(), vec![external.clone()]).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            fs::read(external.join("launcher_profiles.json")).unwrap(),
            original
        );
    }
}
