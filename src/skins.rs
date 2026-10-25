use crate::{
    catalog::{self, Kind},
    model::{self, GameProfile, Loader, Session},
    net,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
pub struct Skin {
    pub id: String,
    pub name: String,
    pub slim: bool,
    #[serde(default)]
    pub builtin: bool,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub skins: Vec<Skin>,
    pub selected: BTreeMap<String, String>,
}
impl Library {
    pub fn load(root: &Path) -> Result<Self> {
        let file = root.join("skins/library.json");
        if !file.exists() {
            return Ok(Self::default());
        }
        model::read_json(&file)
    }
    pub fn save(&self, root: &Path) -> Result<()> {
        model::write_json(&root.join("skins/library.json"), self)
    }
    pub fn import(
        &mut self,
        root: &Path,
        bytes: &[u8],
        name: &str,
        slim: bool,
        builtin: bool,
    ) -> Result<String> {
        let image = decode(bytes)?;
        let mut png = Cursor::new(Vec::new());
        image.write_to(&mut png, image::ImageFormat::Png)?;
        let bytes = png.into_inner();
        let id = format!("{:x}", Sha256::digest(&bytes));
        if !self.skins.iter().any(|s| s.id == id) {
            std::fs::create_dir_all(root.join("skins/textures"))?;
            std::fs::write(texture_path(root, &id)?, bytes)?;
            self.skins.push(Skin {
                id: id.clone(),
                name: name.chars().take(60).collect(),
                slim,
                builtin,
            });
            self.save(root)?;
        }
        Ok(id)
    }
    pub fn remove(&mut self, root: &Path, id: &str) -> Result<()> {
        let file = texture_path(root, id)?;
        anyhow::ensure!(self.skins.iter().any(|s| s.id == id), "Skin no encontrada");
        self.skins.retain(|s| s.id != id);
        self.selected.retain(|_, value| value != id);
        self.save(root)?;
        if file.exists() {
            std::fs::remove_file(file)?;
        }
        Ok(())
    }
}
pub fn texture_path(root: &Path, id: &str) -> Result<PathBuf> {
    anyhow::ensure!(
        id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()),
        "Identificador de skin inválido"
    );
    Ok(root.join("skins/textures").join(format!("{id}.png")))
}
pub fn decode(bytes: &[u8]) -> Result<image::RgbaImage> {
    anyhow::ensure!(bytes.len() <= 1024 * 1024, "La skin supera 1 MB");
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(64);
    limits.max_image_height = Some(64);
    reader.limits(limits);
    let source = reader
        .decode()
        .context("El archivo no es un PNG de skin válido")?
        .to_rgba8();
    anyhow::ensure!(
        source.width() == 64 && [32, 64].contains(&source.height()),
        "Usa una skin PNG de 64×64 o 64×32 píxeles"
    );
    if source.height() == 64 {
        return Ok(source);
    }
    let mut full = image::RgbaImage::new(64, 64);
    image::imageops::overlay(&mut full, &source, 0, 0);
    for (sx, sy, dx, dy, w, h) in [
        (4, 16, 20, 48, 4, 4),
        (8, 16, 24, 48, 4, 4),
        (0, 20, 24, 52, 4, 12),
        (4, 20, 20, 52, 4, 12),
        (8, 20, 16, 52, 4, 12),
        (12, 20, 28, 52, 4, 12),
        (44, 16, 36, 48, 4, 4),
        (48, 16, 40, 48, 4, 4),
        (40, 20, 40, 52, 4, 12),
        (44, 20, 36, 52, 4, 12),
        (48, 20, 32, 52, 4, 12),
        (52, 20, 44, 52, 4, 12),
    ] {
        let face = image::imageops::flip_horizontal(
            &image::imageops::crop_imm(&source, sx, sy, w, h).to_image(),
        );
        image::imageops::overlay(&mut full, &face, dx, dy);
    }
    Ok(full)
}
pub fn download_texture(url: &str) -> Result<Vec<u8>> {
    let mut url = url::Url::parse(url)?;
    anyhow::ensure!(
        url.host_str() == Some("textures.minecraft.net")
            && url.path().starts_with("/texture/")
            && url.username().is_empty()
            && url.password().is_none(),
        "Origen de textura no permitido"
    );
    url.set_scheme("https")
        .map_err(|_| anyhow::anyhow!("URL de textura inválida"))?;
    let mut bytes = Vec::new();
    net::client()?
        .get(url)
        .send()?
        .error_for_status()?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    decode(&bytes)?;
    Ok(bytes)
}
pub const DEFAULTS: &[(&str, &str, bool)] = &[
    (
        "Steve",
        "31f477eb1a7beee631c2ca64d06f8f68fa93a3386d04452ab27f43acdf1b60cb",
        false,
    ),
    (
        "Alex",
        "46acd06e8483b176e8ea39fc12fe105eb3a2a4970f5100057e9d84d4b60bdfa7",
        true,
    ),
    (
        "Ari",
        "4c05ab9e07b3505dc3ec11370c3bdce5570ad2fb2b562e9b9dd9cf271f81aa44",
        false,
    ),
    (
        "Efe",
        "daf3d88ccb38f11f74814e92053d92f7728ddb1a7955652a60e30cb27ae6659f",
        false,
    ),
    (
        "Kai",
        "e5cdc3243b2153ab28a159861be643a4fc1e3c17d291cdd3e57a7f370ad676f3",
        false,
    ),
    (
        "Makena",
        "dc0fcfaf2aa040a83dc0de4e56058d1bbb2ea40157501f3e7d15dc245e493095",
        false,
    ),
    (
        "Noor",
        "90e75cd429ba6331cd210b9bd19399527ee3bab467b5a9f61cb8a27b177f6789",
        false,
    ),
    (
        "Sunny",
        "a3bd16079f764cd541e072e888fe43885e711f98658323db0f9a6045da91ee7a",
        false,
    ),
    (
        "Zuri",
        "f5dddb41dcafef616e959c2817808e0be741c89ffbfed39134a13e75b811863d",
        false,
    ),
];
fn authenticated(session: &Session) -> Result<()> {
    anyhow::ensure!(
        !session.profile.local && std::time::Instant::now() < session.expires,
        "Inicia sesión con Microsoft para cambiar la skin de tu cuenta"
    );
    Ok(())
}
#[derive(Clone, Deserialize)]
pub struct Cape {
    pub id: String,
    pub alias: String,
    pub url: String,
    pub state: String,
}
#[derive(Deserialize)]
pub struct AccountAppearance {
    #[serde(default)]
    pub skins: Vec<AccountSkin>,
    #[serde(default)]
    pub capes: Vec<Cape>,
}
#[derive(Deserialize)]
pub struct AccountSkin {
    pub url: String,
    pub variant: String,
    pub state: String,
}
pub fn account_appearance(session: &Session) -> Result<AccountAppearance> {
    authenticated(session)?;
    Ok(net::client()?
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&session.token)
        .send()?
        .error_for_status()?
        .json()?)
}
pub fn apply_account(root: &Path, skin: &Skin, session: &Session) -> Result<()> {
    authenticated(session)?;
    let bytes = std::fs::read(texture_path(root, &skin.id)?)?;
    decode(&bytes)?;
    let form = reqwest::blocking::multipart::Form::new()
        .text("variant", if skin.slim { "slim" } else { "classic" })
        .part(
            "file",
            reqwest::blocking::multipart::Part::bytes(bytes)
                .file_name("skin.png")
                .mime_str("image/png")?,
        );
    net::client()?
        .post("https://api.minecraftservices.com/minecraft/profile/skins")
        .bearer_auth(&session.token)
        .multipart(form)
        .send()?
        .error_for_status()?;
    Ok(())
}
pub fn reset_account(session: &Session) -> Result<()> {
    authenticated(session)?;
    net::client()?
        .delete("https://api.minecraftservices.com/minecraft/profile/skins/active")
        .bearer_auth(&session.token)
        .send()?
        .error_for_status()?;
    Ok(())
}
pub fn set_cape(session: &Session, cape: Option<&str>) -> Result<()> {
    authenticated(session)?;
    let client = net::client()?;
    let request = if let Some(id) = cape {
        client
            .put("https://api.minecraftservices.com/minecraft/profile/capes/active")
            .json(&serde_json::json!({"capeId":id}))
    } else {
        client.delete("https://api.minecraftservices.com/minecraft/profile/capes/active")
    };
    request
        .bearer_auth(&session.token)
        .send()?
        .error_for_status()?;
    Ok(())
}
pub fn apply_local(
    root: &Path,
    skin: &Skin,
    name: &str,
    profile: &GameProfile,
    status: impl Fn(String),
) -> Result<()> {
    model::Profile::offline(name)?;
    anyhow::ensure!(profile.loader != Loader::Vanilla, "Esta instancia Vanilla necesita un cargador compatible con CustomSkinLoader. Cámbialo desde Configuración > Instalación.");
    let version = catalog::versions(profile, Kind::Mod, "customskinloader")?
        .into_iter()
        .next()
        .context("CustomSkinLoader no publica una versión compatible con esta instancia")?;
    let dest = catalog::destination(profile, Kind::Mod, None)?;
    catalog::install(profile, Kind::Mod, version, &dest, &status)?;
    configure_local(root, skin, name, &profile.game_dir)?;
    status("Skin local aplicada. Reinicia la instancia para verla; otros jugadores no reciben tu PNG automáticamente.".into());
    Ok(())
}
pub fn configure_local(root: &Path, skin: &Skin, name: &str, game: &Path) -> Result<()> {
    model::Profile::offline(name)?;
    let bytes = std::fs::read(texture_path(root, &skin.id)?)?;
    decode(&bytes)?;
    let folder = game.join("CustomSkinLoader");
    std::fs::create_dir_all(folder.join("LocalSkin/skins"))?;
    anyhow::ensure!(
        folder.canonicalize()?.starts_with(game.canonicalize()?),
        "La carpeta de skins está fuera de la instancia"
    );
    let file = folder.join("CustomSkinLoader.json");
    let mut config: serde_json::Value = if file.exists() {
        model::read_json(&file)?
    } else {
        serde_json::json!({"loadlist":[{"name":"Mojang","type":"MojangAPI"}]})
    };
    anyhow::ensure!(
        config.is_object(),
        "Configuración de CustomSkinLoader inválida; no se modificó"
    );
    let entry = serde_json::json!({"name":"LumaLocal","type":"Legacy","skin":"LocalSkin/skins/{USERNAME}.png","model":if skin.slim{"slim"}else{"default"}});
    if config.get("loadlist").is_none() {
        config["loadlist"] = serde_json::json!([]);
    }
    let list = config["loadlist"]
        .as_array_mut()
        .context("La lista de CustomSkinLoader no es válida")?;
    list.retain(|e| e["name"] != "LumaLocal");
    list.insert(0, entry);
    if file.exists() {
        std::fs::copy(&file, folder.join("CustomSkinLoader.json.luma-backup"))?;
    }
    let target = folder.join("LocalSkin/skins").join(format!("{name}.png"));
    let marker = folder.join(format!(".luma-{name}.json"));
    if target.exists() && !marker.exists() {
        std::fs::copy(&target, folder.join(format!(".luma-before-{name}.png")))?;
    }
    model::write_json(&file, &config)?;
    std::fs::write(&target, &bytes)?;
    model::write_json(
        &marker,
        &serde_json::json!({"sha256":format!("{:x}",Sha256::digest(&bytes))}),
    )
}
pub fn disable_local(name: &str, game: &Path) -> Result<()> {
    model::Profile::offline(name)?;
    let folder = game.join("CustomSkinLoader");
    let marker = folder.join(format!(".luma-{name}.json"));
    anyhow::ensure!(
        marker.is_file(),
        "Luma no registró una skin local para este jugador en esta instancia"
    );
    let value: serde_json::Value = model::read_json(&marker)?;
    let file = folder.join("LocalSkin/skins").join(format!("{name}.png"));
    if file.exists() {
        anyhow::ensure!(
            value["sha256"].as_str()
                == Some(format!("{:x}", Sha256::digest(std::fs::read(&file)?)).as_str()),
            "La skin fue modificada fuera de Luma; se conserva"
        );
        std::fs::remove_file(&file)?;
    }
    let backup = folder.join(format!(".luma-before-{name}.png"));
    if backup.is_file() {
        std::fs::rename(&backup, &file)?;
    }
    std::fs::remove_file(marker)?;
    if !std::fs::read_dir(&folder)?.any(|entry| {
        entry.is_ok_and(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name.starts_with(".luma-") && name.ends_with(".json")
        })
    }) {
        let file = folder.join("CustomSkinLoader.json");
        if file.is_file() {
            let mut config: serde_json::Value = model::read_json(&file)?;
            if let Some(list) = config["loadlist"].as_array_mut() {
                list.retain(|e| e["name"] != "LumaLocal");
                model::write_json(&file, &config)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "luma-skin-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }
    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut file = Cursor::new(Vec::new());
        image::RgbaImage::from_pixel(width, height, image::Rgba([200, 120, 70, 255]))
            .write_to(&mut file, image::ImageFormat::Png)
            .unwrap();
        file.into_inner()
    }
    #[test]
    fn validates_dimensions_normalizes_legacy_and_deduplicates_imports() {
        assert!(decode(b"not a PNG").is_err());
        assert!(decode(&png(32, 32)).is_err());
        assert!(decode(&png(128, 128)).is_err());
        assert_eq!(decode(&png(64, 32)).unwrap().dimensions(), (64, 64));
        let root = fixture();
        let mut library = Library::default();
        let bytes = png(64, 64);
        let id = library.import(&root, &bytes, "Skin", true, false).unwrap();
        assert_eq!(
            id,
            library.import(&root, &bytes, "Otra", true, false).unwrap()
        );
        assert_eq!(library.skins.len(), 1);
        assert!(texture_path(&root, "../../escape").is_err());
        assert_eq!(Library::load(&root).unwrap().skins[0].id, id);
        crate::cleanup::delete_verified_directory(&root, &root.canonicalize().unwrap()).unwrap();
    }
    #[test]
    fn local_skin_preserves_config_and_restores_the_previous_file() {
        let root = fixture();
        let game = root.join("game");
        let folder = game.join("CustomSkinLoader");
        std::fs::create_dir_all(folder.join("LocalSkin/skins")).unwrap();
        let previous = png(64, 32);
        let target = folder.join("LocalSkin/skins/Player.png");
        std::fs::write(&target, &previous).unwrap();
        let original = serde_json::json!({"enableCape":false,"loadlist":[{"name":"Existing","type":"MojangAPI"}]});
        model::write_json(&folder.join("CustomSkinLoader.json"), &original).unwrap();
        let mut l = Library::default();
        l.import(&root, &png(64, 64), "New", true, false).unwrap();
        configure_local(&root, &l.skins[0], "Player", &game).unwrap();
        configure_local(&root, &l.skins[0], "Player", &game).unwrap();
        let config: serde_json::Value =
            model::read_json(&folder.join("CustomSkinLoader.json")).unwrap();
        assert_eq!(config["enableCape"], false);
        assert_eq!(config["loadlist"].as_array().unwrap().len(), 2);
        assert_eq!(config["loadlist"][0]["name"], "LumaLocal");
        assert_eq!(config["loadlist"][0]["model"], "slim");
        assert!(configure_local(&root, &l.skins[0], "../escape", &game).is_err());
        disable_local("Player", &game).unwrap();
        assert_eq!(std::fs::read(target).unwrap(), previous);
        crate::cleanup::delete_verified_directory(&root, &root.canonicalize().unwrap()).unwrap();
    }
    #[test]
    #[ignore = "Descarga un mod compatible real y una skin oficial en una instancia desechable"]
    fn live_local_skin_install() {
        let root = fixture();
        let mut profile = crate::catalog::test_profile();
        profile.minecraft = "26.3".into();
        profile.loader = Loader::NeoForge;
        let mut l = Library::default();
        let (name, key, slim) = DEFAULTS[0];
        let bytes =
            download_texture(&format!("https://textures.minecraft.net/texture/{key}")).unwrap();
        l.import(&root, &bytes, name, slim, true).unwrap();
        let result = apply_local(&root, &l.skins[0], "Player", &profile, |s| println!("{s}"));
        assert!(
            profile
                .game_dir
                .join("CustomSkinLoader/LocalSkin/skins/Player.png")
                .is_file(),
            "{result:?}"
        );
        result.unwrap();
        assert!(std::fs::read_dir(profile.game_dir.join("mods"))
            .unwrap()
            .any(|f| f.unwrap().path().extension().is_some_and(|e| e == "jar")));
        crate::cleanup::delete_verified_directory(&root, &root.canonicalize().unwrap()).unwrap();
        crate::cleanup::delete_verified_directory(&profile.storage_dir, &profile.storage_dir)
            .unwrap();
    }
}
