use crate::{
    model::{self, Event, Manifest, Profile, Settings, Version},
    net,
};
use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc::Sender,
    },
};

const MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

pub fn catalog(root: &Path, tx: &Sender<Event>) -> Result<()> {
    let cache = root.join("manifest.json");
    let result = (|| -> Result<Manifest> {
        let manifest = net::client()?
            .get(MANIFEST)
            .send()?
            .error_for_status()?
            .json()?;
        model::write_json(&cache, &manifest)?;
        Ok(manifest)
    })();
    match result {
        Ok(manifest) => tx.send(Event::Catalog(manifest, false))?,
        Err(error) => {
            let cached: Manifest = model::read_json(&cache).with_context(|| {
                format!("No se pudo obtener el catálogo y no hay una copia guardada: {error:#}")
            })?;
            tx.send(Event::Catalog(cached, true))?;
        }
    }
    Ok(())
}

fn path_under(root: &Path, relative: &str) -> Result<PathBuf> {
    let path = Path::new(relative);
    anyhow::ensure!(
        !path.is_absolute() && path.components().all(|c| matches!(c, Component::Normal(_))),
        "Ruta de archivo no segura: {relative}"
    );
    Ok(root.join(path))
}

fn allows(rules: Option<&Value>) -> bool {
    let Some(rules) = rules.and_then(Value::as_array) else {
        return true;
    };
    let mut allow = false;
    for rule in rules {
        if let Some(os) = rule.get("os") {
            if os
                .get("name")
                .and_then(Value::as_str)
                .is_some_and(|name| name != "windows")
            {
                continue;
            }
            if os
                .get("arch")
                .and_then(Value::as_str)
                .is_some_and(|arch| arch != "x86_64" && arch != "amd64")
            {
                continue;
            }
            if let Some(pattern) = os.get("version").and_then(Value::as_str) {
                // Las reglas oficiales distinguen Windows 10+ de las ediciones anteriores.
                if !regex::Regex::new(pattern).is_ok_and(|r| r.is_match(windows_version())) {
                    continue;
                }
            }
            if let Some(range) = os.get("versionRange") {
                let current = version_tuple(windows_version());
                if range["min"]
                    .as_str()
                    .is_some_and(|v| current < version_tuple(v))
                    || range["max"]
                        .as_str()
                        .is_some_and(|v| current >= version_tuple(v))
                {
                    continue;
                }
            }
        }
        if let Some(features) = rule.get("features").and_then(Value::as_object) {
            if features
                .iter()
                .any(|(key, expected)| expected.as_bool() != Some(key == "has_custom_resolution"))
            {
                continue;
            }
        }
        allow = rule["action"] == "allow";
    }
    allow
}

fn version_tuple(value: &str) -> Vec<u32> {
    value
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .chain(std::iter::repeat(0))
        .take(4)
        .collect()
}

fn windows_version() -> &'static str {
    static VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    VERSION.get_or_init(|| {
        let mut command = Command::new("cmd");
        command.args(["/C", "ver"]);
        hide_console(&mut command);
        command
            .output()
            .ok()
            .and_then(|output| {
                regex::Regex::new(r"\d+\.\d+\.\d+")
                    .ok()?
                    .find(&String::from_utf8_lossy(&output.stdout))
                    .map(|v| v.as_str().to_owned())
            })
            .unwrap_or_else(|| "10.0.0".into())
    })
}

#[derive(Clone)]
struct Download {
    url: String,
    path: PathBuf,
    hash: Option<String>,
}

fn artifact(value: &Value, base: &Path) -> Result<Download> {
    Ok(Download {
        url: value["url"].as_str().context("Archivo sin URL")?.into(),
        path: path_under(base, value["path"].as_str().context("Archivo sin ruta")?)?,
        hash: value["sha1"].as_str().map(str::to_owned),
    })
}

fn add_job(jobs: &mut BTreeMap<PathBuf, Download>, job: Download) {
    jobs.insert(job.path.clone(), job);
}

pub struct Installation {
    pub metadata: Value,
    pub java: PathBuf,
    pub classpath: Vec<PathBuf>,
    pub natives: PathBuf,
    pub assets: PathBuf,
    pub game: PathBuf,
}

pub fn prepare(
    root: &Path,
    version: &Version,
    settings: &Settings,
    offline: bool,
    tx: &Sender<Event>,
) -> Result<Installation> {
    model::safe_component(&version.id)?;
    let client = net::client()?;
    let dir = root.join("versions").join(&version.id);
    let metadata_path = dir.join(format!("{}.json", version.id));
    tx.send(Event::Status("Leyendo los archivos de la versión…".into()))?;
    net::download(
        &client,
        &version.url,
        &metadata_path,
        Some(&version.sha1),
        offline,
    )?;
    let metadata: Value = model::read_json(&metadata_path)?;
    let major = metadata["javaVersion"]["majorVersion"]
        .as_u64()
        .unwrap_or(8);
    let java = java_runtime(root, major, &settings.java, offline, tx)?;
    let libraries = root.join("libraries");
    let assets_root = root.join("assets");
    let natives = dir.join("natives");
    fs::create_dir_all(&natives)?;
    let mut jobs = BTreeMap::new();
    let jar = dir.join(format!("{}.jar", version.id));
    let client_download = &metadata["downloads"]["client"];
    add_job(
        &mut jobs,
        Download {
            url: client_download["url"]
                .as_str()
                .context("Esta versión no incluye un cliente descargable")?
                .into(),
            path: jar.clone(),
            hash: client_download["sha1"].as_str().map(str::to_owned),
        },
    );
    let mut classpath = Vec::new();
    let mut native_archives = Vec::new();
    if let Some(libs) = metadata["libraries"].as_array() {
        for lib in libs {
            if !allows(lib.get("rules")) {
                continue;
            }
            if let Some(item) = lib["downloads"].get("artifact") {
                let job = artifact(item, &libraries)?;
                classpath.push(job.path.clone());
                add_job(&mut jobs, job);
            }
            if let Some(classifier) = lib["natives"]["windows"].as_str() {
                let classifier = classifier.replace("${arch}", "64");
                if let Some(item) = lib["downloads"]["classifiers"].get(&classifier) {
                    let job = artifact(item, &libraries)?;
                    let excludes: Vec<String> = lib["extract"]["exclude"]
                        .as_array()
                        .map(|list| {
                            list.iter()
                                .filter_map(Value::as_str)
                                .map(str::to_owned)
                                .collect()
                        })
                        .unwrap_or_default();
                    native_archives.push((job.path.clone(), excludes));
                    add_job(&mut jobs, job);
                }
            }
        }
    }
    classpath.push(jar);
    if let Some(logging) = metadata["logging"]["client"].get("file") {
        let id = model::safe_component(
            logging["id"]
                .as_str()
                .context("Configuración de log sin ID")?,
        )?;
        add_job(
            &mut jobs,
            Download {
                url: logging["url"]
                    .as_str()
                    .context("Configuración de log sin URL")?
                    .into(),
                path: assets_root.join("log_configs").join(id),
                hash: logging["sha1"].as_str().map(str::to_owned),
            },
        );
    }
    let mut asset_index = Value::Null;
    if let Some(index) = metadata.get("assetIndex") {
        let id = model::safe_component(index["id"].as_str().context("Índice de recursos sin ID")?)?;
        let index_path = assets_root.join("indexes").join(format!("{id}.json"));
        net::download(
            &client,
            index["url"].as_str().context("Índice sin URL")?,
            &index_path,
            index["sha1"].as_str(),
            offline,
        )?;
        asset_index = model::read_json(&index_path)?;
        if let Some(objects) = asset_index["objects"].as_object() {
            for object in objects.values() {
                let hash = object["hash"].as_str().context("Recurso sin hash")?;
                anyhow::ensure!(
                    hash.len() == 40 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
                    "Hash de recurso inválido"
                );
                add_job(
                    &mut jobs,
                    Download {
                        url: format!(
                            "https://resources.download.minecraft.net/{}/{hash}",
                            &hash[..2]
                        ),
                        path: assets_root.join("objects").join(&hash[..2]).join(hash),
                        hash: Some(hash.into()),
                    },
                );
            }
        }
    }
    let total = jobs.len();
    tx.send(Event::Status(
        if offline {
            "Verificando instalación local…"
        } else {
            "Descargando y verificando los archivos del juego…"
        }
        .into(),
    ))?;
    tx.send(Event::Progress(0, total))?;
    let completed = AtomicUsize::new(0);
    rayon::ThreadPoolBuilder::new()
        .num_threads(8)
        .build()?
        .install(|| {
            jobs.values()
                .collect::<Vec<_>>()
                .par_iter()
                .try_for_each(|job| -> Result<()> {
                    net::download(&client, &job.url, &job.path, job.hash.as_deref(), offline)?;
                    let n = completed.fetch_add(1, Ordering::Relaxed) + 1;
                    if n.is_multiple_of(20) || n == total {
                        let _ = tx.send(Event::Progress(n, total));
                    }
                    Ok(())
                })
        })?;
    tx.send(Event::Status("Preparando bibliotecas nativas…".into()))?;
    for (archive, excludes) in native_archives {
        extract_zip(&archive, &natives, &excludes)?;
    }
    let game = root.join("instances").join(&version.id);
    fs::create_dir_all(&game)?;
    let mut assets = assets_root.clone();
    // Los clientes anteriores a 1.7 usan nombres de archivo en vez de objetos por hash.
    if asset_index["virtual"].as_bool().unwrap_or(false)
        || asset_index["map_to_resources"].as_bool().unwrap_or(false)
    {
        let virtual_dir = assets_root
            .join("virtual")
            .join(metadata["assets"].as_str().unwrap_or("legacy"));
        let destination = if asset_index["map_to_resources"].as_bool().unwrap_or(false) {
            game.join("resources")
        } else {
            virtual_dir
        };
        if let Some(objects) = asset_index["objects"].as_object() {
            for (name, object) in objects {
                let hash = object["hash"].as_str().context("Recurso sin hash")?;
                let target = path_under(&destination, name)?;
                fs::create_dir_all(target.parent().context("Recurso sin directorio")?)?;
                if !target.exists() || net::sha1_file(&target)? != hash {
                    fs::copy(
                        assets_root.join("objects").join(&hash[..2]).join(hash),
                        target,
                    )?;
                }
            }
        }
        assets = destination;
    }
    model::write_json(
        &dir.join("installed.json"),
        &serde_json::json!({"id": version.id, "java": major}),
    )?;
    tx.send(Event::Installed(version.id.clone()))?;
    Ok(Installation {
        metadata,
        java,
        classpath,
        natives,
        assets,
        game,
    })
}

pub fn extract_zip(archive: &Path, destination: &Path, excludes: &[String]) -> Result<()> {
    let mut zip = zip::ZipArchive::new(fs::File::open(archive)?)?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        if excludes
            .iter()
            .any(|prefix| entry.name().starts_with(prefix))
            || entry.name().starts_with("META-INF/")
        {
            continue;
        }
        let relative = entry
            .enclosed_name()
            .context("El ZIP contiene una ruta no segura")?;
        anyhow::ensure!(
            entry
                .unix_mode()
                .map(|mode| mode & 0o170000 != 0o120000)
                .unwrap_or(true),
            "El ZIP contiene un enlace simbólico"
        );
        let target = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(target)?;
        } else {
            fs::create_dir_all(target.parent().context("Archivo ZIP sin directorio")?)?;
            std::io::copy(&mut entry, &mut fs::File::create(target)?)?;
        }
    }
    Ok(())
}

fn java_runtime(
    root: &Path,
    major: u64,
    custom: &str,
    offline: bool,
    tx: &Sender<Event>,
) -> Result<PathBuf> {
    if !custom.trim().is_empty() {
        let path = PathBuf::from(custom.trim());
        anyhow::ensure!(
            path.is_file(),
            "La ruta de Java no existe: {}",
            path.display()
        );
        return Ok(path);
    }
    let base = root.join("runtimes").join(format!("java-{major}"));
    let marker = base.join("runtime.json");
    if let Ok(relative) = model::read_json::<String>(&marker) {
        let java = path_under(&base, &relative)?;
        if java.is_file() {
            return Ok(java);
        }
    }
    anyhow::ensure!(
        !offline,
        "Java {major} no está instalado. Instala la versión con conexión primero."
    );
    tx.send(Event::Status(format!(
        "Descargando Java {major} · Eclipse Temurin…"
    )))?;
    let client = net::client()?;
    let entries: Value = client.get(format!("https://api.adoptium.net/v3/assets/latest/{major}/hotspot?architecture=x64&image_type=jre&os=windows&vendor=eclipse"))
        .send()?.error_for_status()?.json()?;
    let package = &entries[0]["binary"]["package"];
    let link = package["link"].as_str().context(
        "No hay un runtime Java disponible para esta versión; configura java.exe en Ajustes",
    )?;
    let expected = package["checksum"]
        .as_str()
        .context("Java no incluye una suma de verificación")?;
    let archive = root.join("runtimes").join(format!("java-{major}.zip"));
    net::download(&client, link, &archive, None, false)?;
    let mut digest = Sha256::new();
    let mut file = fs::File::open(&archive)?;
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    if format!("{:x}", digest.finalize()) != expected {
        fs::remove_file(archive)?;
        bail!("Java no pasó la verificación SHA-256. Reintenta la instalación.");
    }
    extract_zip(&archive, &base, &[])?;
    let java = fs::read_dir(&base)?
        .filter_map(Result::ok)
        .map(|e| e.path().join("bin/java.exe"))
        .find(|path| path.is_file())
        .context("No se encontró java.exe en el runtime descargado")?;
    let relative = java.strip_prefix(&base)?.to_string_lossy().to_string();
    model::write_json(&marker, &relative)?;
    fs::remove_file(archive)?;
    Ok(java)
}

fn expanded(value: &str, variables: &HashMap<String, String>) -> Result<String> {
    let mut result = value.to_owned();
    for (key, replacement) in variables {
        result = result.replace(&format!("${{{key}}}"), replacement);
    }
    anyhow::ensure!(
        !result.contains("${"),
        "La versión requiere un argumento no compatible: {result}"
    );
    Ok(result)
}

fn arguments(value: &Value, variables: &HashMap<String, String>) -> Result<Vec<String>> {
    let mut result = Vec::new();
    if let Some(args) = value.as_array() {
        for arg in args {
            if let Some(text) = arg.as_str() {
                result.push(expanded(text, variables)?);
            } else if allows(arg.get("rules")) {
                if let Some(text) = arg["value"].as_str() {
                    result.push(expanded(text, variables)?);
                } else if let Some(values) = arg["value"].as_array() {
                    for text in values {
                        result.push(expanded(
                            text.as_str().context("Argumento inválido")?,
                            variables,
                        )?);
                    }
                }
            }
        }
    }
    Ok(result)
}

pub fn launch(
    root: &Path,
    version: &Version,
    settings: &Settings,
    installation: Installation,
    profile: &Profile,
    token: &str,
    tx: &Sender<Event>,
) -> Result<()> {
    let Installation {
        metadata,
        java,
        classpath,
        natives,
        assets,
        game,
    } = installation;
    let cp = std::env::join_paths(classpath)?
        .to_string_lossy()
        .to_string();
    let mut variables: HashMap<String, String> = [
        ("auth_player_name", profile.name.clone()),
        ("auth_uuid", profile.id.clone()),
        ("auth_access_token", token.to_owned()),
        ("auth_session", format!("token:{token}:{}", profile.id)),
        ("version_name", version.id.clone()),
        ("version_type", version.kind.clone()),
        ("game_directory", game.to_string_lossy().into_owned()),
        (
            "assets_root",
            root.join("assets").to_string_lossy().into_owned(),
        ),
        ("game_assets", assets.to_string_lossy().into_owned()),
        (
            "assets_index_name",
            metadata["assetIndex"]["id"]
                .as_str()
                .unwrap_or("legacy")
                .into(),
        ),
        ("natives_directory", natives.to_string_lossy().into_owned()),
        ("launcher_name", "Luma".into()),
        ("launcher_version", env!("CARGO_PKG_VERSION").into()),
        ("classpath", cp.clone()),
        ("classpath_separator", ";".into()),
        (
            "library_directory",
            root.join("libraries").to_string_lossy().into_owned(),
        ),
        (
            "user_type",
            if profile.local { "legacy" } else { "msa" }.into(),
        ),
        ("user_properties", "{}".into()),
        (
            "clientid",
            crate::auth::configured_client_id(&settings.client_id).into(),
        ),
        ("auth_xuid", String::new()),
        ("resolution_width", "1280".into()),
        ("resolution_height", "720".into()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect();
    let mut command = Command::new(&java);
    command.current_dir(&game);
    hide_console(&mut command);
    // Conserva el acceso token únicamente en memoria; nunca se guarda la línea de comando.
    command
        .arg(format!("-Xmx{}M", settings.memory_mb.clamp(512, 32768)))
        .arg("-Xms512M");
    if metadata["arguments"]["jvm"].is_array() {
        command.args(arguments(&metadata["arguments"]["jvm"], &variables)?);
    } else {
        command
            .arg(format!("-Djava.library.path={}", natives.display()))
            .args(["-cp", &cp]);
    }
    if let Some(argument) = metadata["logging"]["client"]["argument"].as_str() {
        let id = model::safe_component(
            metadata["logging"]["client"]["file"]["id"]
                .as_str()
                .context("Log sin ID")?,
        )?;
        variables.insert(
            "path".into(),
            root.join("assets/log_configs")
                .join(id)
                .to_string_lossy()
                .into_owned(),
        );
        command.arg(expanded(argument, &variables)?);
    }
    command.arg(
        metadata["mainClass"]
            .as_str()
            .context("Esta versión no declara su clase principal")?,
    );
    if let Some(legacy) = metadata["minecraftArguments"].as_str() {
        // Se divide ANTES de sustituir rutas y nombres que pueden contener espacios.
        for arg in legacy.split_whitespace() {
            command.arg(expanded(arg, &variables)?);
        }
    } else {
        command.args(arguments(&metadata["arguments"]["game"], &variables)?);
    }
    fs::create_dir_all(root.join("logs"))?;
    let log = fs::File::create(root.join("logs/game-latest.log"))?;
    command
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log));
    let mut child = command.spawn().context("No se pudo iniciar Java")?;
    tx.send(Event::GameStarted)?;
    let status = child.wait()?;
    tx.send(Event::GameExited(status.code()))?;
    Ok(())
}

fn hide_console(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn evaluates_platform_and_feature_rules() {
        assert!(allows(None));
        assert!(!allows(Some(
            &json!([{"action":"allow","os":{"name":"linux"}}])
        )));
        assert!(allows(Some(
            &json!([{"action":"allow"},{"action":"disallow","os":{"name":"osx"}}])
        )));
        assert!(!allows(Some(
            &json!([{"action":"allow","features":{"is_demo_user":true}}])
        )));
        assert!(allows(Some(
            &json!([{"action":"allow","features":{"has_custom_resolution":true}}])
        )));
    }

    #[test]
    fn rejects_untrusted_paths() {
        assert!(path_under(Path::new("data"), "../escape").is_err());
        assert!(path_under(Path::new("data"), "C:\\escape").is_err());
        assert!(path_under(Path::new("data"), "org/lwjgl/file.jar").is_ok());
    }

    #[test]
    fn substitution_preserves_spaces_and_rejects_unknowns() {
        let vars = HashMap::from([("path".into(), "C:/My Games/Minecraft".into())]);
        assert_eq!(
            arguments(&json!(["-Dpath=${path}"]), &vars).unwrap(),
            vec!["-Dpath=C:/My Games/Minecraft"]
        );
        assert!(expanded("${unknown}", &vars).is_err());
    }

    fn fixture_dir(name: &str) -> PathBuf {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/test-data")
            .join(format!(
                "{name}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn native_zip(path: &Path, name: &str) {
        use std::io::Write;
        let mut writer = zip::ZipWriter::new(fs::File::create(path).unwrap());
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"test native").unwrap();
        writer.finish().unwrap();
    }

    #[test]
    fn zip_extraction_rejects_directory_traversal() {
        let root = fixture_dir("zip");
        let archive = root.join("unsafe.zip");
        native_zip(&archive, "../escape.dll");
        assert!(extract_zip(&archive, &root.join("destination"), &[]).is_err());
        assert!(!root.join("escape.dll").exists());
    }

    #[test]
    fn complete_offline_installation_checks_hashes_natives_and_legacy_assets() {
        let root = fixture_dir("offline");
        let version_dir = root.join("versions/test-1");
        let libraries = root.join("libraries/test");
        fs::create_dir_all(&version_dir).unwrap();
        fs::create_dir_all(&libraries).unwrap();
        fs::create_dir_all(root.join("assets/indexes")).unwrap();
        let jar = version_dir.join("test-1.jar");
        fs::write(&jar, b"client fixture").unwrap();
        let native = libraries.join("native.jar");
        native_zip(&native, "example.dll");
        let library = libraries.join("library.jar");
        fs::write(&library, b"library fixture").unwrap();
        let asset = root.join("asset-fixture");
        fs::write(&asset, b"sound fixture").unwrap();
        let hash = net::sha1_file(&asset).unwrap();
        let object = root.join("assets/objects").join(&hash[..2]).join(&hash);
        fs::create_dir_all(object.parent().unwrap()).unwrap();
        fs::copy(&asset, &object).unwrap();
        let index = root.join("assets/indexes/legacy.json");
        model::write_json(
            &index,
            &json!({"virtual":true,"objects":{"sound/test.ogg":{"hash":hash}}}),
        )
        .unwrap();
        let metadata = json!({
            "mainClass":"example.Main", "assets":"legacy", "javaVersion":{"majorVersion":8},
            "downloads":{"client":{"url":"https://invalid.example/client", "sha1":net::sha1_file(&jar).unwrap()}},
            "libraries":[{
                "downloads":{
                    "artifact":{"path":"test/library.jar", "url":"https://invalid.example/lib", "sha1":net::sha1_file(&library).unwrap()},
                    "classifiers":{"natives-windows":{"path":"test/native.jar", "url":"https://invalid.example/native", "sha1":net::sha1_file(&native).unwrap()}}
                },
                "natives":{"windows":"natives-windows"}
            }],
            "assetIndex":{"id":"legacy","url":"https://invalid.example/index", "sha1":net::sha1_file(&index).unwrap()}
        });
        let metadata_path = version_dir.join("test-1.json");
        model::write_json(&metadata_path, &metadata).unwrap();
        let version = Version {
            id: "test-1".into(),
            kind: "release".into(),
            url: "https://invalid.example/metadata".into(),
            sha1: net::sha1_file(&metadata_path).unwrap(),
            release_time: String::new(),
        };
        let settings = Settings {
            java: std::env::current_exe().unwrap().display().to_string(),
            ..Default::default()
        };
        let (tx, _rx) = std::sync::mpsc::channel();
        let installation = prepare(&root, &version, &settings, true, &tx).unwrap();
        assert_eq!(installation.classpath, vec![library, jar]);
        assert_eq!(
            fs::read(installation.natives.join("example.dll")).unwrap(),
            b"test native"
        );
        assert_eq!(
            fs::read(installation.assets.join("sound/test.ogg")).unwrap(),
            b"sound fixture"
        );
        assert!(version_dir.join("installed.json").is_file());
        fs::write(&object, b"corrupted").unwrap();
        let error = prepare(&root, &version, &settings, true, &tx)
            .err()
            .unwrap();
        assert!(format!("{error:#}").contains("modo offline"));
    }
}
