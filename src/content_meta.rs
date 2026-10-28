use crate::{content::Content, model, net};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Info {
    pub title: String,
    pub version: String,
    pub author: String,
    pub icon: Option<PathBuf>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Record {
    size: u64,
    modified: u64,
    hash: String,
    checked: bool,
    info: Info,
}
fn stamp(path: &Path) -> (u64, u64) {
    fs::metadata(path)
        .map(|m| {
            (
                m.len(),
                m.modified()
                    .ok()
                    .and_then(|v| v.duration_since(UNIX_EPOCH).ok())
                    .map(|v| v.as_nanos() as u64)
                    .unwrap_or(0),
            )
        })
        .unwrap_or_default()
}
fn zip_bytes(zip: &mut zip::ZipArchive<fs::File>, path: &str, limit: u64) -> Option<Vec<u8>> {
    let file = zip.by_name(path).ok()?;
    if file.size() > limit {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes).ok()?;
    ((bytes.len() as u64) <= limit).then_some(bytes)
}
pub(crate) fn save_icon(folder: &Path, key: &str, bytes: &[u8]) -> Option<PathBuf> {
    if bytes.len() > 1024 * 1024 {
        return None;
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(1024);
    limits.max_image_height = Some(1024);
    reader.limits(limits);
    let image = reader.decode().ok()?.thumbnail(96, 96);
    let path = folder.join(format!("{key}.png"));
    image.save(&path).ok()?;
    Some(path)
}
fn text(value: &Value) -> String {
    if let Some(s) = value.as_str() {
        return s.to_owned();
    }
    if let Some(a) = value.as_array() {
        return a
            .iter()
            .map(text)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
    }
    value
        .get("name")
        .or_else(|| value.get("text"))
        .map(text)
        .unwrap_or_default()
}
fn local(item: &Content, icons: &Path, key: &str) -> Info {
    let mut info = Info {
        title: item
            .name
            .trim_end_matches(".jar")
            .trim_end_matches(".zip")
            .to_owned(),
        ..Default::default()
    };
    if item.path.is_dir() {
        if let Ok(bytes) = fs::read(item.path.join("pack.png")) {
            info.icon = save_icon(icons, key, &bytes);
        }
        return info;
    }
    let Ok(file) = fs::File::open(&item.path) else {
        return info;
    };
    let Ok(mut zip) = zip::ZipArchive::new(file) else {
        return info;
    };
    let mut icon = "pack.png".to_owned();
    if let Some(data) = zip_bytes(&mut zip, "fabric.mod.json", 2 * 1024 * 1024)
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
    {
        info.title = text(&data["name"]);
        info.version = text(&data["version"]);
        info.author = text(&data["authors"]);
        icon = data["icon"]
            .as_str()
            .map(str::to_owned)
            .or_else(|| {
                data["icon"]
                    .as_object()
                    .and_then(|m| m.values().find_map(|v| v.as_str().map(str::to_owned)))
            })
            .unwrap_or(icon);
    } else {
        for name in ["META-INF/neoforge.mods.toml", "META-INF/mods.toml"] {
            if let Some(toml) =
                zip_bytes(&mut zip, name, 2 * 1024 * 1024).and_then(|b| String::from_utf8(b).ok())
            {
                for (key, dest) in [
                    ("displayName", &mut info.title),
                    ("version", &mut info.version),
                    ("authors", &mut info.author),
                    ("logoFile", &mut icon),
                ] {
                    if let Ok(pattern) =
                        regex::Regex::new(&format!(r#"(?m)^\s*{key}\s*=\s*["']([^"']+)["']"#))
                    {
                        if let Some(c) = pattern.captures(&toml) {
                            *dest = c[1].to_owned();
                        }
                    }
                }
                if info.version.contains("${") {
                    info.version.clear();
                }
                break;
            }
        }
    }
    if info.title.is_empty() {
        info.title = item.name.clone();
    }
    if let Some(bytes) = zip_bytes(&mut zip, &icon, 1024 * 1024) {
        info.icon = save_icon(icons, key, &bytes);
    }
    info
}
pub fn load(
    game: &Path,
    items: &[Content],
    force: bool,
    updated: impl Fn(HashMap<PathBuf, Info>),
) -> Result<()> {
    anyhow::ensure!(game.is_dir(), "La instancia ya no existe");
    let folder = game.join(".luma/content-icons");
    fs::create_dir_all(&folder)?;
    let file = game.join(".luma/content-info.json");
    let old: HashMap<PathBuf, Record> = model::read_json(&file).unwrap_or_default();
    let mut records = HashMap::new();
    for (index, item) in items.iter().enumerate() {
        let (size, modified) = stamp(&item.path);
        let relative = item
            .path
            .strip_prefix(game)
            .context("Contenido fuera de la instancia")?
            .to_path_buf();
        let record = if let Some(cached) = old
            .get(&relative)
            .filter(|r| r.size == size && r.modified == modified)
        {
            let mut record = cached.clone();
            if force {
                record.checked = false;
            }
            record
        } else {
            Record {
                size,
                modified,
                info: local(item, &folder, &format!("local-{index}-{modified}")),
                ..Default::default()
            }
        };
        records.insert(relative, record);
    }
    let emit = |records: &HashMap<PathBuf, Record>| {
        updated(
            records
                .iter()
                .map(|(p, r)| (game.join(p), r.info.clone()))
                .collect(),
        )
    };
    emit(&records);
    let mut hashes = Vec::new();
    for (path, record) in &mut records {
        if !record.checked && game.join(path).is_file() && record.size < 512 * 1024 * 1024 {
            if record.hash.is_empty() {
                record.hash = net::sha1_file(&game.join(path))?;
            }
            hashes.push(record.hash.clone());
        }
    }
    if hashes.is_empty() {
        persist(&file, &records)?;
        return Ok(());
    }
    let client = net::client()?;
    for hashes in hashes.chunks(100) {
        let versions: HashMap<String, Value> = match client
            .post("https://api.modrinth.com/v2/version_files")
            .timeout(Duration::from_secs(15))
            .json(&serde_json::json!({"hashes":hashes,"algorithm":"sha1"}))
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.json())
        {
            Ok(v) => v,
            Err(_) => continue,
        };
        let ids = versions
            .values()
            .filter_map(|v| v["project_id"].as_str())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let projects: Vec<Value> = if ids.is_empty() {
            vec![]
        } else {
            client
                .get("https://api.modrinth.com/v2/projects")
                .query(&[("ids", serde_json::to_string(&ids)?)])
                .timeout(Duration::from_secs(15))
                .send()
                .and_then(|r| r.error_for_status())
                .and_then(|r| r.json())
                .unwrap_or_default()
        };
        let teams = projects
            .iter()
            .filter_map(|p| p["team"].as_str())
            .collect::<Vec<_>>();
        let members: Vec<Vec<Value>> = if teams.is_empty() {
            vec![]
        } else {
            client
                .get("https://api.modrinth.com/v2/teams")
                .query(&[("ids", serde_json::to_string(&teams)?)])
                .timeout(Duration::from_secs(15))
                .send()
                .and_then(|r| r.error_for_status())
                .and_then(|r| r.json())
                .unwrap_or_default()
        };
        let mut authors = HashMap::new();
        for team in members {
            if let Some(member) = team
                .iter()
                .find(|m| {
                    m["role"]
                        .as_str()
                        .is_some_and(|r| r.eq_ignore_ascii_case("owner"))
                })
                .or_else(|| team.first())
            {
                if let (Some(team), Some(name)) = (
                    member["team_id"].as_str(),
                    member["user"]["username"].as_str(),
                ) {
                    authors.insert(team.to_owned(), name.to_owned());
                }
            }
        }
        let mut icons = HashMap::new();
        for project in &projects {
            let Some(id) = project["id"].as_str() else {
                continue;
            };
            let path = folder.join(format!("{id}.png"));
            if path.is_file() {
                icons.insert(id.to_owned(), path);
                continue;
            }
            if !id.bytes().all(|b| b.is_ascii_alphanumeric()) {
                continue;
            }
            if let Some(url) = project["icon_url"]
                .as_str()
                .and_then(|u| url::Url::parse(u).ok())
                .filter(|u| u.scheme() == "https" && u.host_str() == Some("cdn.modrinth.com"))
            {
                if let Ok(response) = client
                    .get(url)
                    .timeout(Duration::from_secs(10))
                    .send()
                    .and_then(|r| r.error_for_status())
                {
                    let mut bytes = Vec::new();
                    if response
                        .take(1024 * 1024 + 1)
                        .read_to_end(&mut bytes)
                        .is_ok()
                    {
                        if let Some(path) = save_icon(&folder, id, &bytes) {
                            icons.insert(id.to_owned(), path);
                        }
                    }
                }
            }
        }
        for record in records.values_mut().filter(|r| hashes.contains(&r.hash)) {
            record.checked = true;
            if let Some(version) = versions.get(&record.hash) {
                record.info.version = text(&version["version_number"]);
                if let Some(project) = projects.iter().find(|p| p["id"] == version["project_id"]) {
                    record.info.title = text(&project["title"]);
                    if let Some(author) = project["team"].as_str().and_then(|t| authors.get(t)) {
                        record.info.author = author.clone();
                    }
                    if let Some(icon) = project["id"].as_str().and_then(|id| icons.get(id)) {
                        record.info.icon = Some(icon.clone());
                    }
                }
            }
        }
        emit(&records);
    }
    persist(&file, &records)?;
    Ok(())
}
fn persist(file: &Path, records: &HashMap<PathBuf, Record>) -> Result<()> {
    // A metadata task must not recreate an instance removed while HTTP was pending.
    if !file.parent().is_some_and(|p| p.is_dir()) {
        return Ok(());
    }
    fs::write(file, serde_json::to_vec(records)?)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn root() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "luma-meta-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn extracts_local_mod_identity_and_does_not_recreate_removed_directories() {
        let root = root();
        let icons = root.join("icons");
        fs::create_dir_all(&icons).unwrap();
        let path = root.join("sample.jar");
        let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        zip.start_file("fabric.mod.json", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(
            br#"{"name":"Example mod","version":"2.4.1","authors":["Alice",{"name":"Bob"}]}"#,
        )
        .unwrap();
        zip.finish().unwrap();
        let item = Content {
            path,
            name: "sample.jar".into(),
            kind: "Mods".into(),
            enabled: true,
        };
        let info = local(&item, &icons, "test");
        assert_eq!(
            (&*info.title, &*info.version, &*info.author),
            ("Example mod", "2.4.1", "Alice, Bob")
        );
        persist(&root.join("removed/cache.json"), &HashMap::new()).unwrap();
        assert!(!root.join("removed").exists());
        crate::cleanup::delete_verified_directory(&root, &root.canonicalize().unwrap()).unwrap();
    }
    #[test]
    #[ignore = "Identifica el pack real de Fresh Animations en Modrinth y comprueba caché sin red"]
    fn live_pack_metadata_and_icon() {
        let root = root();
        fs::create_dir_all(root.join("resourcepacks")).unwrap();
        let file = root.join("resourcepacks/FreshAnimations.zip");
        net::download(&net::client().unwrap(),"https://cdn.modrinth.com/data/50dA9Sha/versions/tvGVYMD4/FreshAnimations_v1.11.1_26.1%2B.zip",&file,Some("ae9175983c5109162c86daae1acff34ccf623faf"),false).unwrap();
        let items = crate::content::contents(&root);
        let result = std::cell::RefCell::new(HashMap::new());
        load(&root, &items, false, |info| *result.borrow_mut() = info).unwrap();
        let info = result.borrow();
        let info = info.get(&file).unwrap();
        assert_eq!(info.title, "Fresh Animations");
        assert!(!info.version.is_empty());
        assert!(!info.author.is_empty());
        assert!(info.icon.as_ref().is_some_and(|i| i.is_file()));
        println!("{} · {} · {}", info.title, info.version, info.author);

        load(&root, &items, false, |_| {}).unwrap();
        crate::cleanup::delete_verified_directory(&root, &root.canonicalize().unwrap()).unwrap();
    }
}
