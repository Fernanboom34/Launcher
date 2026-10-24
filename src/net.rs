use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use sha1::{Digest, Sha1};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    time::Duration,
};

pub fn client() -> Result<Client> {
    Ok(Client::builder()
        .user_agent("LumaLauncher/0.1.0")
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(600))
        .build()?)
}

pub fn sha1_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha1::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        digest.update(&buf[..n]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub fn download(
    client: &Client,
    url: &str,
    target: &Path,
    sha1: Option<&str>,
    offline: bool,
) -> Result<()> {
    if target.is_file()
        && sha1
            .map(|hash| sha1_file(target).is_ok_and(|s| s == hash))
            .unwrap_or(true)
    {
        return Ok(());
    }
    if offline {
        bail!("Falta un archivo o está dañado: {}. Desactiva el modo offline y vuelve a instalar la versión.", target.display());
    }
    anyhow::ensure!(
        url.starts_with("https://"),
        "El archivo {} necesita que se vuelva a ejecutar el instalador del cargador, o su URL no utiliza HTTPS.", target.display()
    );
    fs::create_dir_all(target.parent().context("Ruta de descarga inválida")?)?;
    let part = target.with_extension("part");
    let result = (|| -> Result<()> {
        let mut response = client.get(url).send()?.error_for_status()?;
        let mut file = fs::File::create(&part)?;
        std::io::copy(&mut response, &mut file)?;
        file.flush()?;
        if let Some(expected) = sha1 {
            anyhow::ensure!(
                sha1_file(&part)? == expected,
                "La descarga no pasó la verificación SHA-1: {url}"
            );
        }
        fs::rename(&part, target)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(part);
    }
    result.with_context(|| format!("No se pudo descargar {url}"))
}
