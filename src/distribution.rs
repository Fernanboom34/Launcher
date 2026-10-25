use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc::Sender,
};
pub const REPOSITORY: &str = "Fernanboom34/Launcher";
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub sha256: String,
}
fn client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .user_agent("Luma-Launcher")
        .timeout(std::time::Duration::from_secs(300))
        .build()?)
}
fn trusted(url: &str) -> Result<()> {
    let u = url::Url::parse(url)?;
    anyhow::ensure!(
        u.scheme() == "https"
            && u.host_str() == Some("github.com")
            && u.path()
                .starts_with(&format!("/{REPOSITORY}/releases/download/")),
        "La descarga debe pertenecer a los releases HTTPS de Luma"
    );
    Ok(())
}
pub fn latest() -> Result<Release> {
    let c = client()?;
    let response: serde_json::Value = c
        .get(format!(
            "https://api.github.com/repos/{REPOSITORY}/releases/latest"
        ))
        .send()?
        .error_for_status()?
        .json()?;
    let manifest = response["assets"]
        .as_array()
        .context("Release sin archivos")?
        .iter()
        .find(|a| a["name"] == "luma-release.json")
        .and_then(|a| a["browser_download_url"].as_str())
        .context("El release aún no tiene luma-release.json")?;
    trusted(manifest)?;
    let release: Release = c.get(manifest).send()?.error_for_status()?.json()?;
    trusted(&release.url)?;
    let parts: Vec<_> = release.version.split('.').collect();
    anyhow::ensure!(
        parts.len() == 3
            && parts
                .iter()
                .all(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())),
        "Versión de release inválida"
    );
    anyhow::ensure!(
        response["tag_name"].as_str() == Some(&format!("v{}", release.version)),
        "El manifiesto no corresponde al tag del release"
    );
    anyhow::ensure!(
        release.sha256.len() == 64 && release.sha256.bytes().all(|x| x.is_ascii_hexdigit()),
        "SHA-256 inválido"
    );
    Ok(release)
}
pub fn newer(version: &str, current: &str) -> bool {
    fn numbers(s: &str) -> Option<Vec<u32>> {
        s.trim_start_matches('v')
            .split('.')
            .map(|n| n.parse().ok())
            .collect()
    }
    match (numbers(version), numbers(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}
pub fn download(release: &Release, target: &Path, status: &Sender<String>) -> Result<()> {
    trusted(&release.url)?;
    fs::create_dir_all(target.parent().context("Destino inválido")?)?;
    let part = target.with_extension("download");
    let result = (|| -> Result<()> {
        let mut response = client()?.get(&release.url).send()?.error_for_status()?;
        let mut file = fs::File::create(&part)?;
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        let mut size = 0u64;
        loop {
            let n = response.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            size += n as u64;
            anyhow::ensure!(size < 256 * 1024 * 1024, "Descarga demasiado grande");
            hash.update(&buffer[..n]);
            file.write_all(&buffer[..n])?;
            let _ = status.send(format!(
                "Descargando Luma · {:.1} MB",
                size as f64 / 1048576.
            ));
        }
        file.sync_all()?;
        drop(file);
        anyhow::ensure!(
            format!("{:x}", hash.finalize()).eq_ignore_ascii_case(&release.sha256),
            "La descarga no coincide con su SHA-256"
        );
        let mut executable = fs::File::open(&part)?;
        let mut magic = [0; 2];
        executable.read_exact(&mut magic)?;
        anyhow::ensure!(&magic == b"MZ", "El archivo no es un ejecutable Windows");
        fs::rename(&part, target)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(part);
    }
    result
}
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}
pub(crate) fn powershell(script: &str, wait: bool) -> Result<()> {
    use base64::Engine;
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut cmd = Command::new("powershell.exe");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-WindowStyle",
        "Hidden",
        "-EncodedCommand",
        &base64::engine::general_purpose::STANDARD.encode(bytes),
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    if wait {
        anyhow::ensure!(
            cmd.status()?.success(),
            "No se pudo completar la operación de Windows"
        );
    } else {
        cmd.spawn()?;
    }
    Ok(())
}
pub fn shortcut(executable: &Path, name: &str, arguments: &str) -> Result<()> {
    let script = format!("$w=New-Object -ComObject WScript.Shell;$s=$w.CreateShortcut((Join-Path ([Environment]::GetFolderPath('Desktop')) {}));$s.TargetPath={};$s.Arguments={};$s.WorkingDirectory={};$s.Save()", quote(&format!("{}.lnk", name.replace(['\\','/',':','*','?','\"','<','>','|'], "_"))), quote(&executable.to_string_lossy()), quote(arguments), quote(&executable.parent().context("Ruta inválida")?.to_string_lossy()));
    powershell(&script, true)
}
pub fn apply_update(staged: &Path) -> Result<()> {
    let exe = std::env::current_exe()?;
    let script = format!("$ErrorActionPreference='Stop';Wait-Process -Id {} -ErrorAction SilentlyContinue;Start-Sleep -Milliseconds 500;$target={};$source={};$backup=$target+'.previous';try {{Copy-Item -LiteralPath $target -Destination $backup -Force;Copy-Item -LiteralPath $source -Destination $target -Force;Start-Process -FilePath $target}} catch {{if(Test-Path -LiteralPath $backup){{Copy-Item -LiteralPath $backup -Destination $target -Force}};Add-Content -LiteralPath ($source+'.error.log') -Value $_}}", std::process::id(), quote(&exe.to_string_lossy()), quote(&staged.to_string_lossy()));
    powershell(&script, false)
}
#[allow(dead_code)]
pub fn install_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default()).join("Programs/Luma")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn releases_are_scoped_and_versions_numeric() {
        assert!(
            trusted("https://github.com/Fernanboom34/Launcher/releases/download/v1/Luma.exe")
                .is_ok()
        );
        assert!(
            trusted("http://github.com/Fernanboom34/Launcher/releases/download/v1/Luma.exe")
                .is_err()
        );
        assert!(trusted("https://example.com/Luma.exe").is_err());
        assert!(newer("v0.10.0", "0.6.0"));
        assert!(!newer("0.5.0", "0.6.0"));
    }
}
