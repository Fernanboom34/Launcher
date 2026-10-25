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
            .split('-')
            .next()?
            .split('.')
            .map(|n| n.parse().ok())
            .collect()
    }
    match (numbers(version), numbers(current)) {
        // The remote candidate is a stable release (validated by latest()).
        // A stable build also supersedes a beta of the same numeric version.
        (Some(a), Some(b)) => a > b || (a == b && current.contains('-') && !version.contains('-')),
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
fn ps_command(script: &str) -> Command {
    use base64::Engine;
    let script=format!("$ProgressPreference='SilentlyContinue';$ErrorActionPreference='Stop';[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false);try {{& {{ {script} }}}} catch {{[Console]::Error.WriteLine($_.Exception.Message);exit 1}};exit 0");
    let encoded = base64::engine::general_purpose::STANDARD.encode(
        script
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    let mut command = Command::new("powershell.exe");
    // Windows PowerShell must discover its own modules when launched by pwsh.
    command.env_remove("PSModulePath");
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-WindowStyle",
        "Hidden",
        "-EncodedCommand",
        &encoded,
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}
pub(crate) fn ps_output(script: &str) -> Result<String> {
    let output = ps_command(script).output()?;
    anyhow::ensure!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_start_matches('\u{feff}')
        .trim()
        .into())
}
pub(crate) fn powershell(script: &str, wait: bool) -> Result<()> {
    if wait {
        ps_output(script)?;
    } else {
        ps_command(script).spawn()?;
    }
    Ok(())
}
fn encoded_script(script: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(
        script
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
    )
}
fn elevate(script: &str) -> Result<()> {
    let wrapped=format!("$ProgressPreference='SilentlyContinue';$ErrorActionPreference='Stop';try{{{script};exit 0}}catch{{exit 1}}");
    let encoded = encoded_script(&wrapped);
    ps_output(&format!("try{{$p=Start-Process -FilePath powershell.exe -Verb RunAs -WindowStyle Hidden -ArgumentList @('-NoProfile','-NonInteractive','-WindowStyle','Hidden','-EncodedCommand','{encoded}') -Wait -PassThru}}catch{{throw 'Windows no autorizó los permisos de administrador.'}};if($p.ExitCode -ne 0){{throw 'Windows no pudo completar la operación en Program Files. Cierra Luma y vuelve a intentarlo.'}}"))?;
    Ok(())
}
pub fn legacy_install_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default()).join("Programs/Luma")
}
pub fn is_program_installation(path: &Path) -> bool {
    crate::cleanup::display_path(path)
        .trim_end_matches('\\')
        .eq_ignore_ascii_case(&crate::cleanup::display_path(&install_dir()))
}
#[allow(dead_code)]
pub fn install_application(release: &Release, staged: &Path, setup: &Path) -> Result<()> {
    elevate(&installation_script(
        release,
        staged,
        setup,
        &install_dir(),
    )?)
}
fn installation_script(
    release: &Release,
    staged: &Path,
    setup: &Path,
    folder: &Path,
) -> Result<String> {
    anyhow::ensure!(folder.is_absolute(), "Ruta de instalación inválida");
    let setup_hash = format!("{:x}", Sha256::digest(fs::read(setup)?));
    let script=format!("$folder={};$source={};$setup={};if((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -ne '{}'){{throw 'Descarga inválida'}};if((Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash -ne '{setup_hash}'){{throw 'Instalador inválido'}};New-Item -ItemType Directory -Path $folder -Force|Out-Null;Copy-Item -LiteralPath $source -Destination (Join-Path $folder 'Luma.exe') -Force;Copy-Item -LiteralPath $setup -Destination (Join-Path $folder 'Luma-Setup.exe') -Force",quote(&crate::cleanup::display_path(folder)),quote(&crate::cleanup::display_path(staged)),quote(&crate::cleanup::display_path(setup)),release.sha256);
    Ok(script)
}
pub fn remove_program_installation(path: &Path) -> Result<()> {
    anyhow::ensure!(
        is_program_installation(path),
        "La carpeta no es la instalación de Luma en Program Files"
    );
    elevate(&removal_script(path))
}
fn removal_script(path: &Path) -> String {
    let target = quote(&crate::cleanup::display_path(path));
    let script = format!(
        r#"$root={target};if(!(Test-Path -LiteralPath $root)){{exit 0}};$item=Get-Item -LiteralPath $root -Force;if(($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0){{throw 'La instalación es un enlace'}};$resolved=(Resolve-Path -LiteralPath $root).ProviderPath;if(![IO.Path]::GetFullPath($root).Equals($resolved,[StringComparison]::OrdinalIgnoreCase)){{throw 'La carpeta cambió'}};function Remove-LumaTree($path,$boundary){{$entry=Get-Item -LiteralPath $path -Force;$absolute=[IO.Path]::GetFullPath($entry.FullName);if(!$absolute.Equals($boundary,[StringComparison]::OrdinalIgnoreCase) -and !$absolute.StartsWith($boundary+'\',[StringComparison]::OrdinalIgnoreCase)){{throw 'Ruta fuera de la instalación'}};if(($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0){{if($entry.PSIsContainer){{[IO.Directory]::Delete($absolute)}}else{{[IO.File]::Delete($absolute)}};return}};if($entry.PSIsContainer){{foreach($child in Get-ChildItem -LiteralPath $absolute -Force){{Remove-LumaTree $child.FullName $boundary}};[IO.Directory]::Delete($absolute)}}else{{[IO.File]::Delete($absolute)}}}};Remove-LumaTree $resolved $resolved"#
    );
    script
}
pub fn shortcut(executable: &Path, name: &str, arguments: &str) -> Result<()> {
    let script = format!("$w=New-Object -ComObject WScript.Shell;$s=$w.CreateShortcut((Join-Path ([Environment]::GetFolderPath('Desktop')) {}));$s.TargetPath={};$s.Arguments={};$s.WorkingDirectory={};$s.Save()", quote(&format!("{}.lnk", name.replace(['\\','/',':','*','?','\"','<','>','|'], "_"))), quote(&executable.to_string_lossy()), quote(arguments), quote(&executable.parent().context("Ruta inválida")?.to_string_lossy()));
    powershell(&script, true)
}
pub fn apply_update(staged: &Path) -> Result<()> {
    let exe = std::env::current_exe()?;
    let result = staged.with_extension("update-result.txt");
    let digest = format!("{:x}", Sha256::digest(fs::read(staged)?));
    let replacement=format!("$ProgressPreference='SilentlyContinue';$ErrorActionPreference='Stop';Wait-Process -Id {} -ErrorAction SilentlyContinue;Start-Sleep -Milliseconds 500;$target={};$source={};$result={};$backup=$target+'.previous';try{{if((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -ne '{digest}'){{throw 'La actualización cambió antes de instalarse'}};Copy-Item -LiteralPath $target -Destination $backup -Force;Copy-Item -LiteralPath $source -Destination $target -Force;[IO.File]::WriteAllText($result,'OK')}}catch{{[IO.File]::WriteAllText($result,$_.Exception.Message);if(Test-Path -LiteralPath $backup){{Copy-Item -LiteralPath $backup -Destination $target -Force -ErrorAction SilentlyContinue}}}}",std::process::id(),quote(&exe.to_string_lossy()),quote(&staged.to_string_lossy()),quote(&result.to_string_lossy()));
    let start=format!("Start-Process -FilePath {} -WorkingDirectory {} -ArgumentList ('--update-report \"'+{}+'\"')",quote(&exe.to_string_lossy()),quote(&exe.parent().context("Ruta inválida")?.to_string_lossy()),quote(&result.to_string_lossy()));
    if exe.parent().is_some_and(is_program_installation) {
        let encoded = encoded_script(&replacement);
        // Only the replacement runs elevated. Restarting remains in the player's user context.
        let restart =
            format!("Wait-Process -Id __WORKER_PID__ -ErrorAction SilentlyContinue;{start}");
        let script=format!("try{{$worker=Start-Process -FilePath powershell.exe -Verb RunAs -WindowStyle Hidden -ArgumentList @('-NoProfile','-NonInteractive','-WindowStyle','Hidden','-EncodedCommand','{encoded}') -PassThru}}catch{{throw 'Windows no autorizó la actualización en Program Files.'}};$restart={};$restart=$restart.Replace('__WORKER_PID__',[string]$worker.Id);$bytes=[Text.Encoding]::Unicode.GetBytes($restart);$code=[Convert]::ToBase64String($bytes);Start-Process -FilePath powershell.exe -WindowStyle Hidden -ArgumentList @('-NoProfile','-NonInteractive','-EncodedCommand',$code)|Out-Null",quote(&restart));
        ps_output(&script)?;
    } else {
        powershell(&format!("{replacement};{start}"), false)?;
    }
    Ok(())
}
#[allow(dead_code)]
pub fn install_dir() -> PathBuf {
    PathBuf::from(
        std::env::var_os("ProgramW6432")
            .or_else(|| std::env::var_os("ProgramFiles"))
            .unwrap_or_else(|| r"C:\Program Files".into()),
    )
    .join("Luma Launcher")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    fn installation_payload_verifies_hashes_and_removal_stays_in_its_folder() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/install-payload-tests")
            .join(format!(
                "{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        fs::create_dir_all(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let app = root.join("app.exe");
        let setup = root.join("setup.exe");
        let target = root.join("installed");
        let outside = root.join("outside");
        fs::write(&app, b"MZtest application").unwrap();
        fs::write(&setup, b"MZtest setup").unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("keep"), b"untouched").unwrap();
        let mut release = Release {
            version: "0.7.0".into(),
            url: String::new(),
            sha256: "0".repeat(64),
        };
        assert!(ps_output(&installation_script(&release, &app, &setup, &target).unwrap()).is_err());
        assert!(!target.exists());
        release.sha256 = format!("{:x}", Sha256::digest(fs::read(&app).unwrap()));
        ps_output(&installation_script(&release, &app, &setup, &target).unwrap()).unwrap();
        assert_eq!(
            fs::read(target.join("Luma.exe")).unwrap(),
            b"MZtest application"
        );
        assert_eq!(
            fs::read(target.join("Luma-Setup.exe")).unwrap(),
            b"MZtest setup"
        );
        ps_output(&format!(
            "New-Item -ItemType Junction -Path {} -Target {} | Out-Null",
            quote(&crate::cleanup::display_path(&target.join("link"))),
            quote(&crate::cleanup::display_path(&outside))
        ))
        .unwrap();
        ps_output(&removal_script(&target)).unwrap();
        assert!(!target.exists());
        assert_eq!(fs::read(outside.join("keep")).unwrap(), b"untouched");
        crate::cleanup::delete_verified_directory(&root, &root).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn powershell_errors_are_plain_utf8_without_clixml() {
        let error=ps_output("Write-Progress -Activity 'Preparando módulos' -Status 'Primera vez';throw 'Cierra Luma y Minecraft antes de desinstalar. Operación cancelada.'").unwrap_err().to_string();
        assert_eq!(
            error,
            "Cierra Luma y Minecraft antes de desinstalar. Operación cancelada."
        );
        assert!(!error.contains("CLIXML"));
    }
    #[test]
    fn default_installation_is_program_files_and_legacy_is_detected() {
        assert_eq!(install_dir().file_name().unwrap(), "Luma Launcher");
        assert!(is_program_installation(&install_dir()));
        assert!(!is_program_installation(&legacy_install_dir()));
    }
    #[test]
    fn releases_are_scoped_and_versions_numeric() {
        assert!(newer("0.7.1", "0.7.1-beta.1"));
        assert!(!newer("0.7.0", "0.7.1-beta.1"));
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
