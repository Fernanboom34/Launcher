use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub id: String,
    #[serde(default)]
    pub local: bool,
}

impl Profile {
    pub fn offline(name: &str) -> Result<Self> {
        use md5::{Digest, Md5};
        anyhow::ensure!(
            !name.is_empty()
                && name.len() <= 16
                && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),
            "El nombre local debe tener entre 1 y 16 letras, números o guiones bajos."
        );
        let mut bytes = Md5::digest(format!("OfflinePlayer:{name}")).to_vec();
        bytes[6] = (bytes[6] & 0x0f) | 0x30;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        let id = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        Ok(Self {
            name: name.into(),
            id,
            local: true,
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub selected: String,
    pub memory_mb: u32,
    pub client_id: String,
    pub java: String,
    pub profile: Option<Profile>,
    pub local_name: String,
    pub active_installation: String,
    pub game_directory: Option<PathBuf>,
    pub accounts: Vec<Profile>,
    pub appearance: Appearance,
    pub behavior: Behavior,
    pub runtime: RuntimeOptions,
    pub quick_world: Option<String>,
    pub instance_styles: std::collections::BTreeMap<String, InstanceStyle>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            selected: String::new(),
            memory_mb: 4096,
            client_id: String::new(),
            java: String::new(),
            profile: None,
            local_name: "Jugador".into(),
            active_installation: String::new(),
            game_directory: None,
            accounts: Vec::new(),
            appearance: Default::default(),
            behavior: Default::default(),
            runtime: Default::default(),
            quick_world: None,
            instance_styles: Default::default(),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct InstanceStyle {
    pub cover_seed: u32,
    pub artwork: Option<PathBuf>,
    pub favorite: bool,
    pub last_played: u64,
    pub created_at: u64,
    pub hidden_home: bool,
    pub play_seconds: u64,
}

impl Settings {
    pub fn launch_identity(&self, session: Option<&Session>) -> Result<(Profile, String)> {
        let profile = match &self.profile {
            Some(profile) if !profile.local => profile.clone(),
            _ => Profile::offline(&self.local_name)?,
        };
        if !profile.local {
            if let Some(session) = session
                .filter(|s| s.profile.id == profile.id && std::time::Instant::now() < s.expires)
            {
                return Ok((session.profile.clone(), session.token.clone()));
            }
        }
        Ok((profile, "0".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_profile_uses_java_offline_uuid_and_validates_names() {
        let profile = Profile::offline("Notch").unwrap();
        assert_eq!(profile.id, "b50ad385829d3141a2167e7d7539ba7f");
        assert!(profile.local);
        for name in ["", "two words", "../escape", "abcdefghijklmnopq"] {
            assert!(Profile::offline(name).is_err());
        }
    }

    #[test]
    fn local_account_does_not_use_a_microsoft_session() {
        let settings = Settings::default();
        let session = Session {
            profile: Profile {
                name: "OnlinePlayer".into(),
                id: "online".into(),
                local: false,
            },
            token: "online-token".into(),
            expires: std::time::Instant::now() + std::time::Duration::from_secs(60),
        };
        for session in [None, Some(&session)] {
            let (profile, token) = settings.launch_identity(session).unwrap();
            assert!(profile.local);
            assert_eq!(profile.name, "Jugador");
            assert_eq!(token, "0");
        }
    }

    #[test]
    fn microsoft_profile_uses_online_session_when_available_and_cached_identity_otherwise() {
        let settings = Settings {
            profile: Some(Profile {
                name: "Player".into(),
                id: "0123456789abcdef0123456789abcdef".into(),
                local: false,
            }),
            ..Default::default()
        };
        assert_eq!(settings.launch_identity(None).unwrap().1, "0");
        assert_eq!(settings.launch_identity(None).unwrap().1, "0");
        let session = Session {
            profile: settings.profile.clone().unwrap(),
            token: "test-token".into(),
            expires: std::time::Instant::now() + std::time::Duration::from_secs(60),
        };
        assert_eq!(
            settings.launch_identity(Some(&session)).unwrap().1,
            "test-token"
        );
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Version {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub sha1: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

impl Version {
    pub fn local(id: &str) -> Self {
        Self {
            id: id.into(),
            kind: "custom".into(),
            url: String::new(),
            sha1: String::new(),
            release_time: String::new(),
        }
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Loader {
    #[default]
    Vanilla,
    Fabric,
    NeoForge,
    Forge,
}

impl Loader {
    pub const ALL: [Self; 4] = [Self::Vanilla, Self::Fabric, Self::NeoForge, Self::Forge];
    pub fn label(self) -> &'static str {
        match self {
            Self::Vanilla => "Vanilla",
            Self::Fabric => "Fabric",
            Self::NeoForge => "NeoForge",
            Self::Forge => "Forge",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameProfile {
    pub id: String,
    pub name: String,
    pub minecraft: String,
    pub version_id: String,
    pub loader: Loader,
    pub loader_version: String,
    pub storage_dir: PathBuf,
    pub game_dir: PathBuf,
    #[serde(default)]
    pub imported: bool,
    #[serde(default)]
    pub runtime: Option<RuntimeOptions>,
}

#[derive(Clone)]
pub struct ProfileRequest {
    pub name: String,
    pub minecraft: String,
    pub loader: Loader,
    pub loader_version: String,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    pub versions: Vec<Version>,
}

#[derive(Clone)]
pub struct Session {
    pub profile: Profile,
    pub token: String,
    pub expires: std::time::Instant,
}

pub enum Event {
    Status(String),
    SkinsChanged,
    SkinAccount(String, crate::skins::AccountAppearance),
    Progress(usize, usize),
    Catalog(Manifest, bool),
    AuthLink(String),
    SignedIn(Session),
    Installed(String),
    Profiles(Vec<GameProfile>),
    InstanceRemoved(String),
    ContentChanged(String),
    ProfileCreated(Box<GameProfile>),
    LoaderVersions(
        Loader,
        String,
        Vec<String>,
        std::collections::HashSet<String>,
    ),
    UpdateAvailable(crate::distribution::Release),
    UpdateReady(PathBuf),
    EditorLoaderVersions(Vec<String>),
    GameStarted,
    GameExited(Option<i32>),
    Done,
    Error(String),
}

pub fn root() -> PathBuf {
    if let Some(path) = std::env::var_os("LUMA_DATA_DIR") {
        return PathBuf::from(path);
    }
    directories::ProjectDirs::from("dev", "Luma", "LumaLauncher")
        .expect("No se encontró el directorio de datos del usuario")
        .data_local_dir()
        .to_path_buf()
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_slice(
        &std::fs::read(path).with_context(|| format!("No se pudo leer {}", path.display()))?,
    )
    .with_context(|| format!("JSON inválido: {}", path.display()))
}

pub fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    std::fs::create_dir_all(path.parent().context("Ruta sin directorio")?)?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let tmp = path.with_extension(format!("{}-{nonce}.tmp", std::process::id()));
    std::fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    // Windows permite reemplazar el archivo de configuración mediante rename.
    std::fs::rename(tmp, path)?;
    Ok(())
}

pub fn safe_component(value: &str) -> Result<&str> {
    anyhow::ensure!(
        !value.is_empty()
            && value.trim() == value
            && !value.ends_with('.')
            && value
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b" ._+-".contains(&c))
            && value != "."
            && value != "..",
        "Identificador de versión no válido"
    );
    Ok(value)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeOptions {
    pub memory_mb: u32,
    pub java: String,
    pub jvm_args: String,
    pub environment: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub before: String,
    pub after: String,
}
impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            memory_mb: 4096,
            java: String::new(),
            jvm_args: String::new(),
            environment: String::new(),
            width: 1280,
            height: 720,
            fullscreen: false,
            before: String::new(),
            after: String::new(),
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub theme: usize,
    pub advanced: bool,
    pub decorations: bool,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: 0,
            advanced: true,
            decorations: true,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Behavior {
    pub minimize: bool,
    pub refocus: bool,
    pub sidebar_button: bool,
    pub sidebar_visible: bool,
    pub compact: bool,
    pub playtime: bool,
}
impl Default for Behavior {
    fn default() -> Self {
        Self {
            minimize: false,
            refocus: true,
            sidebar_button: false,
            sidebar_visible: true,
            compact: false,
            playtime: true,
        }
    }
}
