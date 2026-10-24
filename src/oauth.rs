use crate::model::Event;
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
    thread,
    time::{Duration, Instant},
};
use url::Url;

pub struct PendingLogin {
    listener: TcpListener,
    pub redirect: String,
    pub verifier: String,
    state: String,
    pub authorize: String,
}

fn random_secret() -> Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes)
        .map_err(|_| anyhow::anyhow!("No se pudo generar una clave segura para iniciar sesión"))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

impl PendingLogin {
    pub fn new(client_id: &str) -> Result<Self> {
        anyhow::ensure!(!client_id.is_empty(), "Esta compilación de Luma aún no tiene configurado el acceso Microsoft. El desarrollador debe registrar la aplicación una sola vez e incluir su Client ID. Puedes seguir usando el perfil local.");
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .context("No se pudo preparar el retorno local de Microsoft")?;
        listener.set_nonblocking(true)?;
        let redirect = format!("http://localhost:{}", listener.local_addr()?.port());
        let verifier = random_secret()?;
        let state = random_secret()?;
        let mut authorize =
            Url::parse("https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize")?;
        authorize.query_pairs_mut().extend_pairs([
            ("client_id", client_id),
            ("response_type", "code"),
            ("response_mode", "query"),
            ("redirect_uri", redirect.as_str()),
            ("scope", "XboxLive.signin offline_access"),
            ("state", state.as_str()),
            ("code_challenge", challenge(&verifier).as_str()),
            ("code_challenge_method", "S256"),
        ]);
        Ok(Self {
            listener,
            redirect,
            verifier,
            state,
            authorize: authorize.into(),
        })
    }

    pub fn wait_for_code(&self, cancel: &AtomicBool) -> Result<String> {
        let deadline = Instant::now() + Duration::from_secs(180);
        while Instant::now() < deadline {
            anyhow::ensure!(
                !cancel.load(Ordering::Relaxed),
                "Inicio de sesión cancelado"
            );
            match self.listener.accept() {
                Ok((mut stream, peer)) => {
                    if !peer.ip().is_loopback() {
                        continue;
                    }
                    let target = match read_request(&mut stream, &self.redirect) {
                        Ok(target) => target,
                        Err(_) => {
                            respond(&mut stream, "400 Bad Request", "Solicitud no válida.");
                            continue;
                        }
                    };
                    match callback(&target, &self.state) {
                        Ok(Some(code)) => {
                            respond(&mut stream, "200 OK", "Cuenta conectada a Microsoft. Puedes volver a Luma para terminar la verificación de Minecraft.");
                            return Ok(code);
                        }
                        Ok(None) => {
                            respond(
                                &mut stream,
                                "200 OK",
                                "Has cancelado el acceso. Puedes volver a Luma.",
                            );
                            bail!("Microsoft canceló o rechazó el inicio de sesión");
                        }
                        Err(_) => respond(
                            &mut stream,
                            "400 Bad Request",
                            "La respuesta no corresponde a este inicio de sesión. Vuelve a Luma.",
                        ),
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(100))
                }
                Err(e) => return Err(e.into()),
            }
        }
        bail!("Se agotó el tiempo para iniciar sesión. Pulsa Continuar con Microsoft para intentarlo de nuevo.")
    }
}

fn callback(target: &str, state: &str) -> Result<Option<String>> {
    anyhow::ensure!(target.starts_with("/?"), "Ruta no válida");
    let parsed = Url::parse(&format!("http://localhost{target}"))?;
    let values: Vec<_> = parsed.query_pairs().collect();
    let states: Vec<_> = values.iter().filter(|(key, _)| key == "state").collect();
    anyhow::ensure!(
        states.len() == 1 && states[0].1 == state,
        "Estado de autenticación no válido"
    );
    if values.iter().any(|(key, _)| key == "error") {
        return Ok(None);
    }
    let codes: Vec<_> = values.iter().filter(|(key, _)| key == "code").collect();
    anyhow::ensure!(
        codes.len() == 1 && !codes[0].1.is_empty() && codes[0].1.len() <= 4096,
        "Código de autenticación no válido"
    );
    Ok(Some(codes[0].1.to_string()))
}

fn read_request(stream: &mut TcpStream, redirect: &str) -> Result<String> {
    stream.set_read_timeout(Some(Duration::from_millis(500)))?;
    stream.set_write_timeout(Some(Duration::from_millis(500)))?;
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut request = Vec::new();
    let mut buf = [0u8; 1024];
    loop {
        anyhow::ensure!(
            Instant::now() < deadline && request.len() < 8192,
            "Solicitud demasiado grande o lenta"
        );
        let n = stream.read(&mut buf)?;
        anyhow::ensure!(n > 0, "Solicitud incompleta");
        request.extend_from_slice(&buf[..n]);
        if request.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    let text = std::str::from_utf8(&request)?;
    let mut lines = text.split("\r\n");
    let mut first = lines
        .next()
        .context("Solicitud sin cabecera")?
        .split_whitespace();
    anyhow::ensure!(first.next() == Some("GET"), "Método no válido");
    let target = first.next().context("Solicitud sin ruta")?;
    let expected_host = redirect.trim_start_matches("http://");
    let hosts: Vec<_> = lines
        .filter_map(|line| line.split_once(':'))
        .filter(|(name, _)| name.eq_ignore_ascii_case("host"))
        .collect();
    anyhow::ensure!(
        hosts.len() == 1 && hosts[0].1.trim() == expected_host,
        "Host no válido"
    );
    Ok(target.to_owned())
}

fn respond(stream: &mut TcpStream, status: &str, message: &str) {
    let html = format!("<!doctype html><html lang=es><meta charset=utf-8><title>Luma</title><body style=\"background:#0d1118;color:#e9eff5;font:20px system-ui;padding:12vh 12vw\"><h1 style=\"color:#81e8b5\">Luma</h1><p>{message}</p></body></html>");
    let response = format!("HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n{html}", html.len());
    let _ = stream.write_all(response.as_bytes());
}

pub fn authorize(
    client_id: &str,
    tx: &Sender<Event>,
    cancel: &AtomicBool,
) -> Result<(PendingLogin, String)> {
    let pending = PendingLogin::new(client_id)?;
    tx.send(Event::AuthLink(pending.authorize.clone()))?;
    if webbrowser::open(&pending.authorize).is_err() {
        tx.send(Event::Status(
            "Abre Microsoft desde la ventana de Luma para continuar".into(),
        ))?;
    }
    let code = pending.wait_for_code(cancel)?;
    Ok((pending, code))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_matches_rfc7636_vector() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn callback_rejects_forged_or_ambiguous_responses() {
        assert_eq!(
            callback("/?code=ok&state=expected", "expected").unwrap(),
            Some("ok".into())
        );
        assert_eq!(
            callback("/?error=access_denied&state=expected", "expected").unwrap(),
            None
        );
        for target in [
            "/?code=ok&state=wrong",
            "/?code=ok",
            "/?code=one&code=two&state=expected",
            "/?code=ok&state=expected&state=expected",
            "/favicon.ico",
        ] {
            assert!(callback(target, "expected").is_err());
        }
    }

    #[test]
    fn loopback_receives_code_and_releases_listener() {
        let pending = PendingLogin::new("test-client").unwrap();
        let address = pending.listener.local_addr().unwrap();
        let request = format!(
            "GET /?code=mock-code&state={} HTTP/1.1\r\nHost: localhost:{}\r\n\r\n",
            pending.state,
            address.port()
        );
        let client = thread::spawn(move || {
            let mut stream = TcpStream::connect(address).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            stream.write_all(request.as_bytes()).unwrap();
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            assert!(response.starts_with("HTTP/1.1 200"));
            assert!(!response.contains("mock-code"));
        });
        assert_eq!(
            pending.wait_for_code(&AtomicBool::new(false)).unwrap(),
            "mock-code"
        );
        client.join().unwrap();
        drop(pending);
        assert!(TcpListener::bind(address).is_ok());
    }

    #[test]
    fn login_can_be_cancelled_without_any_browser_response() {
        let pending = PendingLogin::new("test-client").unwrap();
        assert!(pending.wait_for_code(&AtomicBool::new(true)).is_err());
        assert!(PendingLogin::new("").is_err());
    }
}
