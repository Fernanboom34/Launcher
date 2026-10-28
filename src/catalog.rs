use crate::{
    model::{GameProfile, Loader},
    net,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Mod,
    ResourcePack,
    Shader,
    DataPack,
}
impl Kind {
    pub const ALL: [Self; 4] = [Self::Mod, Self::ResourcePack, Self::Shader, Self::DataPack];
    pub fn label(self) -> &'static str {
        match self {
            Self::Mod => "Mods",
            Self::ResourcePack => "Resource packs",
            Self::Shader => "Shaders",
            Self::DataPack => "Data packs",
        }
    }
    pub fn extension(self) -> &'static str {
        if self == Self::Mod {
            "jar"
        } else {
            "zip"
        }
    }
    fn project_type(self) -> &'static str {
        match self {
            Self::Mod => "mod",
            Self::DataPack => "datapack",
            Self::ResourcePack => "resourcepack",
            Self::Shader => "shader",
        }
    }
}
#[derive(Clone, Deserialize)]
pub struct Project {
    #[serde(default)]
    pub icon_url: Option<String>,
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub slug: String,
    pub downloads: u64,
}
#[derive(Deserialize)]
pub struct Search {
    pub hits: Vec<Project>,
    pub total_hits: usize,
}
#[derive(Clone, Deserialize)]
pub struct Version {
    #[serde(default)]
    pub version_type: String,
    #[serde(default)]
    pub date_published: String,
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub files: Vec<File>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}
#[derive(Clone, Deserialize)]
pub struct File {
    pub url: String,
    pub filename: String,
    pub hashes: BTreeMap<String, String>,
    #[serde(default)]
    pub primary: bool,
}
#[derive(Clone, Deserialize)]
pub struct Dependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub dependency_type: String,
}
fn loader(profile: &GameProfile) -> &'static str {
    match profile.loader {
        Loader::Vanilla => "minecraft",
        Loader::Fabric => "fabric",
        Loader::Forge => "forge",
        Loader::NeoForge => "neoforge",
    }
}
fn client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .https_only(true)
        .user_agent(concat!(
            "Fernanboom34/LumaLauncher/",
            env!("CARGO_PKG_VERSION"),
            " (https://github.com/Fernanboom34/Launcher)"
        ))
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(90))
        .build()?)
}
fn id(value: &str) -> Result<&str> {
    anyhow::ensure!(
        !value.is_empty()
            && value.len() <= 100
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "ID de proyecto inválido"
    );
    Ok(value)
}
pub fn search(profile: &GameProfile, kind: Kind, query: &str, offset: usize) -> Result<Search> {
    let mut facets = vec![
        vec![format!("all_project_types:{}", kind.project_type())],
        vec![format!("versions:{}", profile.minecraft)],
    ];
    if kind == Kind::Mod {
        facets.push(vec![format!("categories:{}", loader(profile))]);
    }
    Ok(client()?
        .get("https://api.modrinth.com/v2/search")
        .query(&[
            ("query", query),
            ("facets", &serde_json::to_string(&facets)?),
            ("limit", "20"),
            ("offset", &offset.to_string()),
        ])
        .send()?
        .error_for_status()?
        .json()?)
}
pub fn versions(profile: &GameProfile, kind: Kind, project: &str) -> Result<Vec<Version>> {
    let url = format!(
        "https://api.modrinth.com/v2/project/{}/version",
        id(project)?
    );
    let mut query = vec![(
        "game_versions",
        serde_json::to_string(&[&profile.minecraft])?,
    )];
    if kind == Kind::Mod {
        query.push(("loaders", serde_json::to_string(&[loader(profile)])?));
    }
    if kind == Kind::DataPack {
        query.push(("loaders", serde_json::to_string(&["datapack"])?));
    }
    let versions: Vec<Version> = client()?
        .get(url)
        .query(&query)
        .send()?
        .error_for_status()?
        .json()?;
    let mut versions: Vec<_> = versions
        .into_iter()
        .filter(|v| compatible(v, profile, kind))
        .collect();
    sort_versions(&mut versions);
    Ok(versions)
}
fn sort_versions(versions: &mut [Version]) {
    versions.sort_by(|a, b| {
        channel_rank(&a.version_type)
            .cmp(&channel_rank(&b.version_type))
            .then_with(|| b.date_published.cmp(&a.date_published))
    });
}
fn channel_rank(channel: &str) -> u8 {
    match channel {
        "release" => 0,
        "beta" => 1,
        "alpha" => 2,
        _ => 3,
    }
}
fn compatible(v: &Version, p: &GameProfile, kind: Kind) -> bool {
    v.game_versions.contains(&p.minecraft)
        && (match kind {
            Kind::Mod => v.loaders.iter().any(|l| l == loader(p)),
            Kind::DataPack => v.loaders.iter().any(|l| l == "datapack"),
            _ => true,
        })
}
pub fn destination(profile: &GameProfile, kind: Kind, world: Option<&Path>) -> Result<PathBuf> {
    let dir = match kind {
        Kind::Mod => profile.game_dir.join("mods"),
        Kind::ResourcePack => profile.game_dir.join("resourcepacks"),
        Kind::Shader => profile.game_dir.join("shaderpacks"),
        Kind::DataPack => {
            let world = world
                .context("Elige el mundo para este data pack")?
                .canonicalize()?;
            anyhow::ensure!(
                world.starts_with(profile.game_dir.join("saves").canonicalize()?)
                    && world.join("level.dat").is_file(),
                "Mundo inválido"
            );
            world.join("datapacks")
        }
    };
    fs::create_dir_all(&dir)?;
    let dir = dir.canonicalize()?;
    anyhow::ensure!(
        dir.starts_with(profile.game_dir.canonicalize()?),
        "La carpeta de contenido está fuera de la instancia"
    );
    Ok(dir)
}
pub fn safe_filename(name: &str, kind: Kind) -> Result<&str> {
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .split('.')
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    anyhow::ensure!(
        !name.is_empty()
            && name.len() <= 220
            && !name.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|', '\0'])
            && !name.ends_with(['.', ' '])
            && path.file_name().is_some_and(|n| n == name)
            && path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(kind.extension()))
            && !matches!(
                stem.as_str(),
                "CON"
                    | "PRN"
                    | "AUX"
                    | "NUL"
                    | "COM1"
                    | "COM2"
                    | "COM3"
                    | "COM4"
                    | "COM5"
                    | "COM6"
                    | "COM7"
                    | "COM8"
                    | "COM9"
                    | "LPT1"
                    | "LPT2"
                    | "LPT3"
                    | "LPT4"
                    | "LPT5"
                    | "LPT6"
                    | "LPT7"
                    | "LPT8"
                    | "LPT9"
            ),
        "Nombre o tipo de archivo no permitido: {name}"
    );
    Ok(name)
}
fn download(file: &File, target: &Path) -> Result<()> {
    use sha2::{Digest, Sha512};
    let url = url::Url::parse(&file.url)?;
    anyhow::ensure!(
        url.scheme() == "https" && url.host_str() == Some("cdn.modrinth.com"),
        "Origen de descarga de Modrinth inválido"
    );
    if let Some(hash) = file.hashes.get("sha1") {
        return net::download(&client()?, &file.url, target, Some(hash), false);
    }
    let expected = file
        .hashes
        .get("sha512")
        .context("Modrinth no publicó una suma de verificación")?;
    let mut response = client()?.get(url).send()?.error_for_status()?;
    let mut output = fs::File::create(target)?;
    let mut hash = Sha512::new();
    let mut bytes = [0u8; 65536];
    let mut size = 0usize;
    loop {
        let n = response.read(&mut bytes)?;
        if n == 0 {
            break;
        }
        size += n;
        anyhow::ensure!(size <= 512 * 1024 * 1024, "Archivo demasiado grande");
        hash.update(&bytes[..n]);
        output.write_all(&bytes[..n])?;
    }
    anyhow::ensure!(
        format!("{:x}", hash.finalize()).eq_ignore_ascii_case(expected),
        "La descarga no pasó la verificación SHA-512"
    );
    output.sync_all()?;
    Ok(())
}
#[derive(Serialize, Deserialize, Default)]
struct Installed {
    #[serde(default)]
    entries: HashMap<String, Record>,
}
#[derive(Serialize, Deserialize)]
struct Record {
    version: String,
    filename: String,
}
fn resolve_versions(
    profile: &GameProfile,
    kind: Kind,
    root_version: Version,
    mut fetch: impl FnMut(&Dependency) -> Result<Version>,
) -> Result<Vec<Version>> {
    let mut queue = vec![root_version];
    let mut resolved = Vec::new();
    let mut seen = HashMap::new();
    while let Some(version) = queue.pop() {
        if let Some(previous) = seen.get(&version.project_id) {
            anyhow::ensure!(
                previous == &version.id,
                "Dos dependencias requieren versiones distintas del mismo proyecto"
            );
            continue;
        }
        anyhow::ensure!(resolved.len() < 64, "Demasiadas dependencias");
        seen.insert(version.project_id.clone(), version.id.clone());
        for dependency in version
            .dependencies
            .iter()
            .filter(|d| d.dependency_type == "required")
        {
            let dependency = fetch(dependency)?;
            anyhow::ensure!(
                compatible(&dependency, profile, kind),
                "La dependencia {} no es compatible",
                dependency.name
            );
            queue.push(dependency);
        }
        resolved.push(version);
    }
    Ok(resolved)
}
pub fn install(
    profile: &GameProfile,
    kind: Kind,
    root_version: Version,
    dest: &Path,
    status: impl Fn(String),
) -> Result<usize> {
    anyhow::ensure!(
        compatible(&root_version, profile, kind),
        "La versión no es compatible con esta instancia"
    );
    let client = client()?;
    let resolved = resolve_versions(profile, kind, root_version, |dependency| {
        if let Some(version_id) = &dependency.version_id {
            Ok(client
                .get(format!(
                    "https://api.modrinth.com/v2/version/{}",
                    id(version_id)?
                ))
                .send()?
                .error_for_status()?
                .json::<Version>()?)
        } else if let Some(project) = &dependency.project_id {
            versions(profile, kind, project)?
                .into_iter()
                .next()
                .context("Una dependencia no tiene versión compatible")
        } else {
            anyhow::bail!("Una dependencia requiere instalación manual")
        }
    })?;
    let dest = dest.canonicalize()?;
    anyhow::ensure!(
        dest.starts_with(profile.game_dir.canonicalize()?),
        "Destino fuera de la instancia"
    );
    use sha1::{Digest, Sha1};
    let manifest_path = profile.game_dir.join(".luma").join(format!(
        "modrinth-{}-{:x}.json",
        kind.label().replace(' ', "-").to_ascii_lowercase(),
        Sha1::digest(dest.to_string_lossy().as_bytes())
    ));
    let mut manifest: Installed = if manifest_path.exists() {
        crate::model::read_json(&manifest_path)?
    } else {
        Installed::default()
    };
    if kind == Kind::Mod {
        let known: HashSet<_> = manifest
            .entries
            .values()
            .map(|r| r.filename.clone())
            .collect();
        let mut hashes = Vec::new();
        for entry in fs::read_dir(&dest)?.filter_map(Result::ok) {
            if entry
                .file_type()
                .is_ok_and(|t| t.is_file() && !t.is_symlink())
                && entry
                    .path()
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("jar"))
                && !known.contains(&entry.file_name().to_string_lossy().to_string())
            {
                hashes.push((
                    entry.file_name().to_string_lossy().to_string(),
                    net::sha1_file(&entry.path())?,
                ));
            }
        }
        if !hashes.is_empty() {
            status("Identificando mods ya instalados…".into());
            for batch in hashes.chunks(200) {
                let found:HashMap<String,Version>=client.post("https://api.modrinth.com/v2/version_files").json(&serde_json::json!({"hashes":batch.iter().map(|(_,h)|h).collect::<Vec<_>>(),"algorithm":"sha1"})).send()?.error_for_status()?.json()?;
                for (name, hash) in batch {
                    if let Some(version) = found
                        .get(hash)
                        .filter(|v| resolved.iter().any(|r| r.project_id == v.project_id))
                    {
                        if let Some(existing) = manifest.entries.get(&version.project_id) {
                            anyhow::ensure!(existing.filename==*name||!dest.join(&existing.filename).exists(),"Ya hay varias copias del mismo mod. Revisa {name} antes de instalar");
                        }
                        manifest.entries.insert(
                            version.project_id.clone(),
                            Record {
                                version: version.id.clone(),
                                filename: name.clone(),
                            },
                        );
                    }
                }
            }
        }
    }
    let stage = profile.game_dir.join(".luma/staging").join(format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    fs::create_dir_all(&stage)?;
    let result = (|| -> Result<usize> {
        let mut files = Vec::new();
        let mut names = HashSet::new();
        for version in resolved {
            let file = version
                .files
                .iter()
                .find(|f| f.primary)
                .or_else(|| version.files.first())
                .context("La versión no tiene archivos")?
                .clone();
            safe_filename(&file.filename, kind)?;
            anyhow::ensure!(
                names.insert(file.filename.clone()),
                "Las dependencias contienen nombres de archivo repetidos"
            );
            status(format!(
                "Descargando {} {}…",
                version.name, version.version_number
            ));
            let path = stage.join(&file.filename);
            download(&file, &path)?;
            crate::content::validate_archive(&path)?;
            files.push((version, file, path));
        }
        // Validate all collisions before replacing any file; only replace files managed by this catalog.
        for (version, file, _) in &files {
            let target = dest.join(&file.filename);
            if target.exists() {
                let owned = manifest
                    .entries
                    .get(&version.project_id)
                    .is_some_and(|r| r.filename == file.filename);
                let identical = file
                    .hashes
                    .get("sha1")
                    .is_some_and(|h| net::sha1_file(&target).is_ok_and(|v| v == *h));
                anyhow::ensure!(
                    owned || identical,
                    "Ya existe {}. Revisa el archivo antes de reemplazarlo",
                    file.filename
                );
            }
        }
        let count = files.len();
        for (version, file, path) in files {
            if let Some(old) = manifest.entries.get(&version.project_id) {
                safe_filename(&old.filename, kind)?;
                let old_path = dest.join(&old.filename);
                if old_path.is_file() {
                    crate::content::remove_permanently(&profile.game_dir, &old_path)?;
                }
            }
            let target = dest.join(&file.filename);
            fs::rename(path, &target)?;
            manifest.entries.insert(
                version.project_id,
                Record {
                    version: version.id,
                    filename: file.filename,
                },
            );
        }
        crate::model::write_json(&manifest_path, &manifest)?;
        Ok(count)
    })();
    let _ = crate::cleanup::delete_verified_directory(&stage, &stage.canonicalize()?);
    result
}

#[cfg(test)]
pub(crate) fn test_profile() -> GameProfile {
    let mut nonce = [0u8; 16];
    getrandom::getrandom(&mut nonce).unwrap();
    let nonce = nonce.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/content-tests")
        .join(format!(
            "{}-{}-{nonce}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
    fs::create_dir_all(root.join("game")).unwrap();
    GameProfile {
        id: "test".into(),
        name: "Prueba aislada".into(),
        minecraft: "1.21.1".into(),
        version_id: "1.21.1".into(),
        loader: Loader::Fabric,
        loader_version: String::new(),
        storage_dir: root.canonicalize().unwrap(),
        game_dir: root.join("game").canonicalize().unwrap(),
        imported: false,
        runtime: Some(Default::default()),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installation_prefers_release_then_beta_then_alpha_and_newest_within_channel() {
        let mut versions = vec![
            version("old-release", "p", None),
            version("new-release", "p", None),
            version("beta", "p", None),
            version("alpha", "p", None),
        ];
        versions[0].date_published = "2025-01-01T00:00:00Z".into();
        versions[1].date_published = "2026-01-01T00:00:00Z".into();
        versions[2].version_type = "beta".into();
        versions[2].date_published = "2027-01-01T00:00:00Z".into();
        versions[3].version_type = "alpha".into();
        versions[3].date_published = "2028-01-01T00:00:00Z".into();
        sort_versions(&mut versions);
        assert_eq!(
            versions.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(),
            vec!["new-release", "old-release", "beta", "alpha"]
        );
        versions.retain(|v| v.version_type != "release");
        sort_versions(&mut versions);
        assert_eq!(versions[0].id, "beta");
        versions.retain(|v| v.version_type != "beta");
        assert_eq!(versions[0].id, "alpha");
    }
    fn version(name: &str, project: &str, dependency: Option<&str>) -> Version {
        Version {
            version_type: "release".into(),
            date_published: String::new(),
            id: name.into(),
            project_id: project.into(),
            name: name.into(),
            version_number: "1".into(),
            game_versions: vec!["1.21.1".into()],
            loaders: vec!["fabric".into()],
            files: vec![],
            dependencies: dependency
                .map(|id| {
                    vec![Dependency {
                        version_id: Some(id.into()),
                        project_id: None,
                        dependency_type: "required".into(),
                    }]
                })
                .unwrap_or_default(),
        }
    }
    #[test]
    fn content_filenames_reject_traversal_streams_and_wrong_types() {
        for name in [
            "../escape.jar",
            "C:\\escape.jar",
            "test.jar:stream",
            "CON.jar",
            "NUL.zip",
            "file.exe",
            "other.zip",
        ] {
            assert!(safe_filename(name, Kind::Mod).is_err());
        }
        assert!(safe_filename("mi-mod+1.21.1.jar", Kind::Mod).is_ok());
        assert!(safe_filename("Texturas bonitas.zip", Kind::ResourcePack).is_ok());
    }
    #[test]
    fn destinations_are_per_instance_type_and_world() {
        let p = test_profile();
        let mods = destination(&p, Kind::Mod, None).unwrap();
        let resources = destination(&p, Kind::ResourcePack, None).unwrap();
        let shaders = destination(&p, Kind::Shader, None).unwrap();
        assert_ne!(mods, resources);
        assert_ne!(mods, shaders);
        assert_eq!(mods, p.game_dir.join("mods"));
        let world = p.game_dir.join("saves/Uno");
        fs::create_dir_all(&world).unwrap();
        fs::write(world.join("level.dat"), b"world").unwrap();
        assert_eq!(
            destination(&p, Kind::DataPack, Some(&world)).unwrap(),
            world.join("datapacks").canonicalize().unwrap()
        );
        assert!(destination(&p, Kind::DataPack, Some(&p.storage_dir)).is_err());
        crate::cleanup::delete_verified_directory(&p.storage_dir, &p.storage_dir).unwrap();
    }
    #[test]
    fn required_dependencies_are_compatible_and_cycles_terminate() {
        let p = test_profile();
        let a = version("a", "A", Some("b"));
        let b = version("b", "B", Some("a"));
        let resolved = resolve_versions(&p, Kind::Mod, a.clone(), |d| {
            Ok(if d.version_id.as_deref() == Some("b") {
                b.clone()
            } else {
                a.clone()
            })
        })
        .unwrap();
        assert_eq!(resolved.len(), 2);
        let mut wrong = b.clone();
        wrong.loaders = vec!["forge".into()];
        assert!(resolve_versions(&p, Kind::Mod, a, |_| Ok(wrong.clone())).is_err());
        crate::cleanup::delete_verified_directory(&p.storage_dir, &p.storage_dir).unwrap();
    }
    #[test]
    fn conflicting_pinned_dependencies_fail_before_download() {
        let p = test_profile();
        let mut root = version("a", "A", Some("b"));
        root.dependencies.push(Dependency {
            version_id: Some("c".into()),
            project_id: None,
            dependency_type: "required".into(),
        });
        assert!(resolve_versions(&p, Kind::Mod, root, |d| Ok(version(
            d.version_id.as_deref().unwrap(),
            "same-project",
            None
        )))
        .is_err());
        crate::cleanup::delete_verified_directory(&p.storage_dir, &p.storage_dir).unwrap();
    }
    #[test]
    #[ignore = "Accede a Modrinth y descarga contenido real en una carpeta aislada"]
    fn live_modrinth_search_versions_and_installation() {
        let p = test_profile();
        let found = search(&p, Kind::Mod, "Mod Menu", 0).unwrap();
        let project = found.hits.iter().find(|p| p.slug == "modmenu").unwrap();
        let version = versions(&p, Kind::Mod, &project.project_id)
            .unwrap()
            .remove(0);
        let dest = destination(&p, Kind::Mod, None).unwrap();
        let count = install(&p, Kind::Mod, version.clone(), &dest, |s| println!("{s}")).unwrap();
        assert!(count >= 1);
        let first = fs::read_dir(&dest).unwrap().count();
        for file in fs::read_dir(&dest).unwrap().flatten() {
            zip::ZipArchive::new(fs::File::open(file.path()).unwrap()).unwrap();
        }
        install(&p, Kind::Mod, version, &dest, |s| println!("{s}")).unwrap();
        assert_eq!(first, fs::read_dir(&dest).unwrap().count());
        let packs = search(&p, Kind::DataPack, "Terralith", 0).unwrap();
        let pack = packs.hits.iter().find(|p| p.slug == "terralith").unwrap();
        let versions = versions(&p, Kind::DataPack, &pack.project_id).unwrap();
        assert!(!versions.is_empty());
        assert!(versions[0]
            .files
            .iter()
            .any(|f| f.filename.ends_with(".zip")));
        crate::cleanup::delete_verified_directory(&p.storage_dir, &p.storage_dir).unwrap();
    }
}
