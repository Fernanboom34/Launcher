use crate::model::GameProfile;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct Content {
    pub path: PathBuf,
    pub name: String,
    pub kind: String,
    pub enabled: bool,
}
pub fn contents(game: &Path) -> Vec<Content> {
    let mut out = Vec::new();
    for (dir, kind) in [
        (game.join("mods"), "Mods"),
        (game.join("resourcepacks"), "Resource packs"),
        (game.join("shaderpacks"), "Shaders"),
    ] {
        scan(&dir, kind, &mut out);
    }
    for world in dirs(&game.join("saves")) {
        scan(&world.join("datapacks"), "Data packs", &mut out);
    }
    let selected = resource_packs(game);
    for item in &mut out {
        if item.kind == "Resource packs" {
            item.enabled = item.enabled
                && selected
                    .iter()
                    .any(|x| x == &format!("file/{}", item.name) || x == &item.name);
        }
    }
    out.sort_by_key(|x| x.name.to_lowercase());
    out
}
fn scan(dir: &Path, kind: &str, out: &mut Vec<Content>) {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        if e.file_type().is_ok_and(|t| t.is_symlink()) {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let disabled = name.ends_with(".disabled");
        if e.path().is_file() && !(name.ends_with(".jar") || name.ends_with(".zip") || disabled) {
            continue;
        }
        out.push(Content {
            path: e.path(),
            name: name.trim_end_matches(".disabled").into(),
            kind: kind.into(),
            enabled: !disabled,
        });
    }
    // Directory packs are parked outside the game's scanned folder when disabled.
    for e in fs::read_dir(dir.join(".luma-disabled"))
        .into_iter()
        .flatten()
        .flatten()
    {
        if e.file_type().is_ok_and(|t| t.is_symlink()) {
            continue;
        }
        out.push(Content {
            path: e.path(),
            name: e.file_name().to_string_lossy().into(),
            kind: kind.into(),
            enabled: false,
        });
    }
}
pub fn toggle(game: &Path, item: &Content) -> Result<()> {
    let base = game.canonicalize()?;
    let canonical = item.path.canonicalize()?;
    anyhow::ensure!(
        canonical.starts_with(base),
        "Contenido fuera de la instancia"
    );
    if item.kind == "Resource packs" {
        let target = game.join("resourcepacks").join(&item.name);
        if !item.enabled && item.path != target {
            anyhow::ensure!(!target.exists(), "Ya existe el paquete");
            fs::rename(&item.path, &target)?;
        }
        let mut packs = resource_packs(game);
        packs.retain(|x| x != &item.name && x != &format!("file/{}", item.name));
        if !item.enabled {
            packs.push(format!("file/{}", item.name));
        }
        let file = game.join("options.txt");
        let old = fs::read_to_string(&file).unwrap_or_default();
        let mut lines: Vec<String> = old
            .lines()
            .filter(|l| !l.starts_with("resourcePacks:"))
            .map(str::to_owned)
            .collect();
        lines.push(format!("resourcePacks:{}", serde_json::to_string(&packs)?));
        let temp = file.with_extension("luma-tmp");
        fs::write(&temp, lines.join("\n") + "\n")?;
        fs::rename(temp, file)?;
        return Ok(());
    }
    let target = if item
        .path
        .parent()
        .is_some_and(|p| p.ends_with(".luma-disabled"))
    {
        item.path
            .parent()
            .unwrap()
            .parent()
            .context("Ruta inválida")?
            .join(&item.name)
    } else if !item.enabled {
        item.path.with_file_name(&item.name)
    } else if item.path.is_dir() {
        item.path
            .parent()
            .unwrap()
            .join(".luma-disabled")
            .join(&item.name)
    } else {
        item.path.with_file_name(format!("{}.disabled", item.name))
    };
    anyhow::ensure!(!target.exists(), "Ya existe {}", target.display());
    fs::create_dir_all(target.parent().context("Ruta inválida")?)?;
    fs::rename(&item.path, target)?;
    Ok(())
}
pub fn bulk(
    game: &Path,
    paths: &std::collections::HashSet<PathBuf>,
    enabled: Option<bool>,
) -> Result<usize> {
    let mut changed = 0;
    for item in contents(game)
        .into_iter()
        .filter(|item| paths.contains(&item.path))
    {
        if let Some(enabled) = enabled {
            if item.enabled != enabled {
                toggle(game, &item)?;
                changed += 1;
            }
        } else {
            archive(game, &item.path)?;
            changed += 1;
        }
    }
    Ok(changed)
}
pub fn archive(game: &Path, path: &Path) -> Result<()> {
    let base = game.canonicalize()?;
    let from = path.canonicalize()?;
    anyhow::ensure!(
        from.starts_with(&base) && from != base,
        "Archivo fuera de la instancia"
    );
    let folder = base.join(".luma/removed").join(format!(
        "{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    fs::create_dir_all(&folder)?;
    fs::rename(
        &from,
        folder.join(from.file_name().context("Archivo inválido")?),
    )?;
    Ok(())
}
fn dirs(path: &Path) -> Vec<PathBuf> {
    fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir() && !t.is_symlink()))
        .map(|e| e.path())
        .collect()
}
#[derive(Clone)]
pub struct World {
    pub folder: String,
    pub name: String,
    pub last_played: i64,
    pub mode: String,
    pub path: PathBuf,
}
#[derive(Deserialize)]
struct Level {
    #[serde(rename = "Data")]
    data: LevelData,
}
#[derive(Deserialize, Default)]
#[serde(default)]
struct LevelData {
    #[serde(rename = "LevelName")]
    name: String,
    #[serde(rename = "LastPlayed")]
    last: i64,
    #[serde(rename = "GameType")]
    mode: i32,
}
pub fn worlds(game: &Path) -> Vec<World> {
    let mut worlds = Vec::new();
    for path in dirs(&game.join("saves")) {
        let parsed = (|| -> Result<Level> {
            let file = fs::File::open(path.join("level.dat"))?;
            let mut bytes = Vec::new();
            flate2::read::GzDecoder::new(file)
                .take(32 * 1024 * 1024)
                .read_to_end(&mut bytes)?;
            Ok(fastnbt::from_bytes(&bytes)?)
        })();
        if let Ok(level) = parsed {
            let folder = path.file_name().unwrap().to_string_lossy().into_owned();
            worlds.push(World {
                folder: folder.clone(),
                name: if level.data.name.is_empty() {
                    folder
                } else {
                    level.data.name
                },
                last_played: level.data.last,
                mode: match level.data.mode {
                    1 => "Creativo",
                    2 => "Aventura",
                    3 => "Espectador",
                    _ => "Supervivencia",
                }
                .into(),
                path,
            });
        }
    }
    worlds.sort_by_key(|w| std::cmp::Reverse(w.last_played));
    worlds
}
#[derive(Clone)]
pub struct Screenshot {
    pub path: PathBuf,
    pub instance: String,
    pub modified: std::time::SystemTime,
}
pub fn screenshots(profiles: &[GameProfile]) -> Vec<Screenshot> {
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for p in profiles {
        for e in fs::read_dir(p.game_dir.join("screenshots"))
            .into_iter()
            .flatten()
            .flatten()
        {
            let path = e.path();
            if path
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("png"))
                && seen.insert(path.clone())
            {
                result.push(Screenshot {
                    path,
                    instance: p.name.clone(),
                    modified: e
                        .metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::UNIX_EPOCH),
                });
            }
        }
    }
    result.sort_by_key(|s| std::cmp::Reverse(s.modified));
    result
}
pub fn tail(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let len = file.metadata()?.len();
    file.seek(SeekFrom::Start(len.saturating_sub(512 * 1024)))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn resource_packs(game: &Path) -> Vec<String> {
    fs::read_to_string(game.join("options.txt"))
        .ok()
        .and_then(|text| {
            text.lines().find_map(|l| {
                l.strip_prefix("resourcePacks:")
                    .and_then(|s| serde_json::from_str(s).ok())
            })
        })
        .unwrap_or_else(|| vec!["vanilla".into()])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bulk_actions_set_a_state_instead_of_flipping_mixed_rows() {
        let game = fixture();
        fs::create_dir_all(game.join("mods")).unwrap();
        fs::write(game.join("mods/a.jar"), b"a").unwrap();
        fs::write(game.join("mods/b.jar.disabled"), b"b").unwrap();
        let paths = contents(&game).into_iter().map(|i| i.path).collect();
        assert_eq!(bulk(&game, &paths, Some(false)).unwrap(), 1);
        assert!(contents(&game).iter().all(|i| !i.enabled));
        let paths = contents(&game).into_iter().map(|i| i.path).collect();
        assert_eq!(bulk(&game, &paths, Some(true)).unwrap(), 2);
        assert!(contents(&game).iter().all(|i| i.enabled));
        let paths = contents(&game).into_iter().map(|i| i.path).collect();
        assert_eq!(bulk(&game, &paths, None).unwrap(), 2);
        assert!(contents(&game).is_empty());
        assert!(game.join(".luma/removed").is_dir());
        crate::cleanup::delete_verified_directory(&game, &game.canonicalize().unwrap()).unwrap();
    }
    fn fixture() -> PathBuf {
        let mut nonce = [0u8; 16];
        getrandom::getrandom(&mut nonce).unwrap();
        let nonce = nonce.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let p = std::env::temp_dir().join(format!(
            "luma-content-{}-{}-{nonce}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn content_toggles_preserve_bytes_and_resource_selection() {
        let root = fixture();
        fs::create_dir_all(root.join("mods")).unwrap();
        fs::write(root.join("mods/example.jar"), b"user mod bytes").unwrap();
        let item = contents(&root).remove(0);
        toggle(&root, &item).unwrap();
        assert_eq!(
            fs::read(root.join("mods/example.jar.disabled")).unwrap(),
            b"user mod bytes"
        );
        let item = contents(&root).remove(0);
        assert!(!item.enabled);
        toggle(&root, &item).unwrap();
        assert!(root.join("mods/example.jar").is_file());
        fs::create_dir_all(root.join("resourcepacks")).unwrap();
        fs::write(root.join("resourcepacks/pack.zip"), b"pack").unwrap();
        fs::write(
            root.join("options.txt"),
            "music:0.5\nresourcePacks:[\"vanilla\"]\n",
        )
        .unwrap();
        let item = contents(&root)
            .into_iter()
            .find(|i| i.kind == "Resource packs")
            .unwrap();
        assert!(!item.enabled);
        toggle(&root, &item).unwrap();
        assert!(resource_packs(&root).contains(&"file/pack.zip".into()));
        assert!(fs::read_to_string(root.join("options.txt"))
            .unwrap()
            .contains("music:0.5"));
        let item = contents(&root)
            .into_iter()
            .find(|i| i.kind == "Resource packs")
            .unwrap();
        assert!(item.enabled);
        toggle(&root, &item).unwrap();
        assert!(!resource_packs(&root).contains(&"file/pack.zip".into()));
        assert!(root.join("resourcepacks/pack.zip").is_file());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn directory_datapacks_and_archive_stay_recoverable() {
        let root = fixture();
        let path = root.join("saves/world/datapacks/pack");
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("pack.mcmeta"), b"{}").unwrap();
        let item = contents(&root).remove(0);
        toggle(&root, &item).unwrap();
        assert!(!path.exists());
        let item = contents(&root).remove(0);
        assert!(!item.enabled);
        toggle(&root, &item).unwrap();
        assert!(path.join("pack.mcmeta").is_file());
        archive(&root, &path).unwrap();
        assert!(!path.exists());
        assert!(root.join(".luma/removed").is_dir());
        assert!(archive(&root, root.parent().unwrap()).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn worlds_read_nbt_names_modes_and_order() {
        use std::io::Write;
        let root = fixture();
        for (folder, name, last, mode) in [
            ("older", "Mi mundo", 10i64, 0i32),
            ("newer", "Mi creativo", 20, 1),
        ] {
            let p = root.join("saves").join(folder);
            fs::create_dir_all(&p).unwrap();
            let bytes = fastnbt::to_bytes(
                &serde_json::json!({"Data":{"LevelName":name,"LastPlayed":last,"GameType":mode}}),
            )
            .unwrap();
            let mut gz = flate2::write::GzEncoder::new(
                fs::File::create(p.join("level.dat")).unwrap(),
                flate2::Compression::default(),
            );
            gz.write_all(&bytes).unwrap();
            gz.finish().unwrap();
        }
        let worlds = worlds(&root);
        assert_eq!(worlds.len(), 2);
        assert_eq!(worlds[0].folder, "newer");
        assert_eq!(worlds[0].name, "Mi creativo");
        assert_eq!(worlds[0].mode, "Creativo");
        fs::remove_dir_all(root).unwrap();
    }
}

/// Structural validation, not a malware verdict. JAR bytecode remains executable.
pub fn validate_archive(path: &Path) -> Result<()> {
    anyhow::ensure!(
        fs::metadata(path)?.len() <= 512 * 1024 * 1024,
        "Archivo de contenido demasiado grande"
    );
    let mut zip = zip::ZipArchive::new(fs::File::open(path)?)?;
    anyhow::ensure!(
        !zip.is_empty() && zip.len() <= 65536,
        "Cantidad de entradas ZIP inválida"
    );
    let mut total = 0u64;
    let mut names = std::collections::HashSet::new();
    for i in 0..zip.len() {
        let entry = zip.by_index(i)?;
        let name = entry.name().to_owned();
        anyhow::ensure!(
            entry.enclosed_name().is_some()
                && !name.contains('\\')
                && !name.contains(':')
                && !name.chars().any(char::is_control),
            "Ruta ZIP peligrosa"
        );
        anyhow::ensure!(names.insert(name.to_lowercase()), "Ruta ZIP duplicada");
        anyhow::ensure!(
            entry.unix_mode().is_none_or(|m| m & 0o170000 != 0o120000),
            "Enlace simbólico en ZIP"
        );
        anyhow::ensure!(
            entry.size() <= 256 * 1024 * 1024,
            "Entrada ZIP demasiado grande"
        );
        total = total
            .checked_add(entry.size())
            .context("Tamaño ZIP inválido")?;
        anyhow::ensure!(
            total <= 1024 * 1024 * 1024,
            "Contenido descomprimido demasiado grande"
        );
        anyhow::ensure!(
            entry.size()
                <= entry
                    .compressed_size()
                    .saturating_mul(1000)
                    .max(1024 * 1024),
            "Compresión ZIP excesiva"
        );
        let expected = entry.size();
        let actual = std::io::copy(&mut entry.take(expected + 1), &mut std::io::sink())?;
        anyhow::ensure!(actual == expected, "Tamaño ZIP inconsistente");
    }
    Ok(())
}

#[cfg(test)]
mod hostile_archives {
    use super::*;
    use std::io::Write;
    fn check(entries: &[(&str, &[u8])]) -> Result<()> {
        let mut nonce = [0u8; 8];
        getrandom::getrandom(&mut nonce).unwrap();
        let path =
            std::env::temp_dir().join(format!("luma-archive-{:x}.zip", u64::from_le_bytes(nonce)));
        let mut writer = zip::ZipWriter::new(fs::File::create(&path)?);
        for (name, data) in entries {
            writer.start_file(*name, zip::write::SimpleFileOptions::default())?;
            writer.write_all(data)?;
        }
        writer.finish()?;
        let result = validate_archive(&path);
        fs::remove_file(path)?;
        result
    }
    #[test]
    fn rejects_traversal_windows_paths_and_case_collisions() {
        for name in ["../escape", "/absolute", "C:/escape", "a\\..\\escape"] {
            assert!(check(&[(name, b"hostile")]).is_err(), "{name}");
        }
        assert!(check(&[("A", b"a"), ("a", b"b")]).is_err());
        assert!(check(&[("pack.mcmeta", b"{}"), ("assets/example/test", b"hello")]).is_ok());
    }
    #[test]
    fn rejects_symlinks_and_corrupted_payloads() {
        let path = std::env::temp_dir().join(format!("luma-corrupt-{}.zip", std::process::id()));
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        writer
            .add_symlink(
                "link",
                "../../outside",
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        fs::write(&path, bytes).unwrap();
        assert!(validate_archive(&path).is_err());
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        writer
            .start_file(
                "payload",
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(b"unique-payload-bytes").unwrap();
        let mut bytes = writer.finish().unwrap().into_inner();
        let offset = bytes
            .windows(20)
            .position(|w| w == b"unique-payload-bytes")
            .unwrap();
        bytes[offset] ^= 1;
        fs::write(&path, bytes).unwrap();
        assert!(validate_archive(&path).is_err());
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn rejects_compression_bomb() {
        assert!(check(&[("bomb", &vec![0; 2 * 1024 * 1024])]).is_err());
    }
}
