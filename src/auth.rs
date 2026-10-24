use crate::{
    model::{Event, Profile, Session},
    net,
};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    time::{Duration, Instant},
};

pub fn official_profiles() -> Vec<Profile> {
    let Some(appdata) = std::env::var_os("APPDATA") else {
        return Vec::new();
    };
    let path = std::path::PathBuf::from(appdata).join(".minecraft/launcher_accounts.json");
    let Ok(value) = crate::model::read_json::<Value>(&path) else {
        return Vec::new();
    };
    value["accounts"]
        .as_object()
        .into_iter()
        .flat_map(|accounts| accounts.values())
        .filter_map(|account| {
            serde_json::from_value::<Profile>(account["minecraftProfile"].clone()).ok()
        })
        .filter(|profile| !profile.name.is_empty() && profile.id.len() == 32)
        .collect()
}

fn post(client: &reqwest::blocking::Client, url: &str, body: Value) -> Result<Value> {
    let response = client.post(url).json(&body).send()?;
    let status = response.status();
    let value: Value = response
        .json()
        .context("El servicio de autenticación devolvió una respuesta inválida")?;
    anyhow::ensure!(status.is_success(), "La autenticación falló ({status}). Revisa la configuración de la aplicación y tu cuenta Xbox. Código: {}", value.get("XErr").or(value.get("error")).unwrap_or(&Value::Null));
    Ok(value)
}

pub fn configured_client_id(override_id: &str) -> &str {
    if override_id.trim().is_empty() {
        option_env!("LUMA_MICROSOFT_CLIENT_ID").unwrap_or("").trim()
    } else {
        override_id.trim()
    }
}

pub fn login(client_id: &str, tx: &Sender<Event>, cancel: &AtomicBool) -> Result<Session> {
    let client_id = configured_client_id(client_id);
    let (pending, code) = crate::oauth::authorize(client_id, tx, cancel)?;
    anyhow::ensure!(
        !cancel.load(Ordering::Relaxed),
        "Inicio de sesión cancelado"
    );
    let client = net::client()?;
    let response = client
        .post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .timeout(Duration::from_secs(30))
        .form(&[
            ("client_id", client_id),
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", pending.redirect.as_str()),
            ("code_verifier", pending.verifier.as_str()),
        ])
        .send()?;
    let status = response.status();
    let value: Value = response.json()?;
    anyhow::ensure!(status.is_success(), "Microsoft rechazó el acceso ({status}). Revisa el registro de Luma y su URI http://localhost. Código: {}", value["error"].as_str().unwrap_or("desconocido"));
    let ms_token = value["access_token"]
        .as_str()
        .context("Microsoft no entregó un token")?;
    anyhow::ensure!(
        !cancel.load(Ordering::Relaxed),
        "Inicio de sesión cancelado"
    );
    tx.send(Event::Status("Verificando tu cuenta de Minecraft…".into()))?;
    let xbox = post(
        &client,
        "https://user.auth.xboxlive.com/user/authenticate",
        json!({
            "Properties": {"AuthMethod":"RPS", "SiteName":"user.auth.xboxlive.com", "RpsTicket":format!("d={ms_token}")},
            "RelyingParty":"http://auth.xboxlive.com", "TokenType":"JWT"
        }),
    )?;
    let xbox_token = xbox["Token"]
        .as_str()
        .context("Xbox Live no entregó un token")?;
    let xsts = post(
        &client,
        "https://xsts.auth.xboxlive.com/xsts/authorize",
        json!({
            "Properties":{"SandboxId":"RETAIL", "UserTokens":[xbox_token]},
            "RelyingParty":"rp://api.minecraftservices.com/", "TokenType":"JWT"
        }),
    )?;
    let uhs = xsts["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .context("No se encontró tu identidad Xbox")?;
    let token = xsts["Token"].as_str().context("XSTS no entregó un token")?;
    let mc = post(
        &client,
        "https://api.minecraftservices.com/authentication/login_with_xbox",
        json!({
            "identityToken":format!("XBL3.0 x={uhs};{token}")
        }),
    )?;
    let token = mc["access_token"]
        .as_str()
        .context("Minecraft no entregó un token")?
        .to_owned();
    let response = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&token)
        .send()?;
    anyhow::ensure!(response.status().is_success(), "No se encontró un perfil de Minecraft Java. La cuenta debe tener el juego y un perfil creado. Las aplicaciones nuevas pueden necesitar aprobación de Minecraft para acceder a su API.");
    let profile: Profile = response.json()?;
    anyhow::ensure!(
        !cancel.load(Ordering::Relaxed),
        "Inicio de sesión cancelado"
    );
    Ok(Session {
        profile,
        token,
        expires: Instant::now()
            + Duration::from_secs(
                mc["expires_in"]
                    .as_u64()
                    .unwrap_or(86400)
                    .saturating_sub(60),
            ),
    })
}
