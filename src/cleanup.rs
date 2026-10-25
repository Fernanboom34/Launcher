use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Inventory {
    executables: Vec<PathBuf>,
    game_directories: Vec<GameDirectory>,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
struct GameDirectory {
    path: PathBuf,
    storage: PathBuf,
}
static INVENTORY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
#[derive(Clone, Default)]
pub struct UninstallPlan {
    pub directories: Vec<PathBuf>,
    pub files: Vec<PathBuf>,
    pub excluded: Vec<PathBuf>,
}
pub fn data_root() -> PathBuf {
    directories::ProjectDirs::from("dev", "Luma", "LumaLauncher")
        .expect("Directorio de usuario no disponible")
        .data_local_dir()
        .to_path_buf()
}
fn inventory_path() -> PathBuf {
    data_root().parent().unwrap().join("installation.json")
}
pub fn register_executable(path: &Path) -> Result<()> {
    let _lock = INVENTORY_LOCK
        .lock()
        .map_err(|_| anyhow::anyhow!("Inventario ocupado"))?;
    let path = path.canonicalize()?;
    let file = inventory_path();
    let mut inventory: Inventory = fs::read(&file)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    if !inventory.executables.contains(&path) {
        inventory.executables.push(path);
        fs::create_dir_all(file.parent().unwrap())?;
        let temporary = file.with_extension("tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(&inventory)?)?;
        fs::rename(temporary, file)?;
    }
    Ok(())
}
pub fn remember_game_directory(path: &Path, storage: &Path) -> Result<()> {
    let _lock = INVENTORY_LOCK
        .lock()
        .map_err(|_| anyhow::anyhow!("Inventario ocupado"))?;
    let file = inventory_path();
    let mut inventory: Inventory = if file.exists() {
        serde_json::from_slice(&fs::read(&file)?)?
    } else {
        Inventory::default()
    };
    let entry = GameDirectory {
        path: normalized(path)?,
        storage: normalized(storage)?,
    };
    if !inventory.game_directories.contains(&entry) {
        inventory.game_directories.push(entry);
        fs::create_dir_all(file.parent().unwrap())?;
        let temp = file.with_extension("tmp");
        fs::write(&temp, serde_json::to_vec_pretty(&inventory)?)?;
        fs::rename(temp, file)?;
    }
    Ok(())
}
fn app_directories() -> Vec<PathBuf> {
    let dirs = directories::ProjectDirs::from("dev", "Luma", "LumaLauncher").unwrap();
    let mut result = Vec::new();
    for path in [dirs.data_local_dir(), dirs.config_dir(), dirs.cache_dir()] {
        let path = path.parent().unwrap().to_path_buf();
        if !result.contains(&path) {
            result.push(path);
        }
    }
    result.push(crate::distribution::install_dir());
    result.push(crate::distribution::legacy_install_dir());
    result
}
fn normalized(path: &Path) -> Result<PathBuf> {
    anyhow::ensure!(
        path.is_absolute(),
        "La ruta debe ser absoluta: {}",
        path.display()
    );
    anyhow::ensure!(
        !path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir)),
        "Ruta no permitida"
    );
    if path.exists() {
        Ok(path.canonicalize()?)
    } else {
        Ok(path.to_path_buf())
    }
}
pub fn validate_private_directory(path: &Path, protected: &[PathBuf]) -> Result<PathBuf> {
    anyhow::ensure!(
        !fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()),
        "La carpeta es un enlace; abre su ubicación original para eliminarla"
    );
    let path = normalized(path)?;
    if let Some(windows) = std::env::var_os("SystemRoot") {
        let windows = normalized(Path::new(&windows))?;
        anyhow::ensure!(
            !path.starts_with(&windows),
            "No se pueden borrar carpetas de Windows"
        );
    }
    for name in [
        "LOCALAPPDATA",
        "APPDATA",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramData",
    ] {
        if let Some(base) = std::env::var_os(name) {
            let base = normalized(Path::new(&base))?;
            anyhow::ensure!(
                !base.starts_with(&path),
                "No se puede borrar una carpeta general del sistema"
            );
        }
    }
    if let Ok(current) = std::env::current_dir() {
        anyhow::ensure!(
            !normalized(&current)?.starts_with(&path),
            "No se puede borrar la carpeta de trabajo de la aplicación"
        );
    }
    anyhow::ensure!(
        path.parent().is_some() && path.components().count() > 2,
        "No se puede borrar una raíz del sistema"
    );
    if let Some(user) = directories::UserDirs::new() {
        for special in [
            Some(user.home_dir()),
            user.document_dir(),
            user.desktop_dir(),
            user.download_dir(),
        ]
        .into_iter()
        .flatten()
        {
            let special = normalized(special)?;
            anyhow::ensure!(
                !special.starts_with(&path),
                "No se puede borrar una carpeta general del usuario: {}",
                path.display()
            );
        }
    }
    for special in protected {
        let special = normalized(special)?;
        anyhow::ensure!(
            !special.starts_with(&path),
            "La carpeta está compartida o contiene datos comunes: {}",
            path.display()
        );
    }
    Ok(path)
}
// Walk explicitly and unlink junctions/symlinks without traversing their targets.
pub fn delete_verified_directory(path: &Path, expected: &Path) -> Result<()> {
    let actual = normalized(path)?;
    let expected = normalized(expected)?;
    anyhow::ensure!(actual == expected, "La carpeta cambió antes de borrarse");
    if !path.exists() {
        return Ok(());
    }
    fn walk(path: &Path, boundary: &Path) -> Result<()> {
        let metadata = fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() {
            #[cfg(windows)]
            let directory = {
                use std::os::windows::fs::MetadataExt;
                metadata.file_attributes() & 0x10 != 0
            };
            #[cfg(not(windows))]
            let directory = path.is_dir();
            if directory {
                fs::remove_dir(path)?
            } else {
                fs::remove_file(path)?
            }
            return Ok(());
        }
        let resolved = path.canonicalize()?;
        anyhow::ensure!(
            resolved.starts_with(boundary),
            "Un archivo salió de la carpeta confirmada"
        );
        if metadata.is_dir() {
            for entry in fs::read_dir(path)? {
                walk(&entry?.path(), boundary)?;
            }
            fs::remove_dir(path)?;
        } else {
            fs::remove_file(path)?;
        }
        Ok(())
    }
    walk(path, &expected)
}
fn ps_output(script: &str) -> Result<String> {
    crate::distribution::ps_output(script)
}
pub fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if let Some(p) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{p}")
    } else {
        text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned()
    }
}
fn quote(path: &Path) -> String {
    format!("'{}'", display_path(path).replace('\'', "''"))
}

pub fn uninstall_plan() -> Result<UninstallPlan> {
    let mut plan = UninstallPlan::default();
    for path in app_directories() {
        if path.exists() {
            let checked = validate_private_directory(&path, &[])?;
            if !plan.directories.iter().any(|p| checked.starts_with(p)) {
                plan.directories.retain(|p| !p.starts_with(&checked));
                plan.directories.push(checked);
            }
        }
    }
    let inventory: Inventory = if inventory_path().exists() {
        serde_json::from_slice(&fs::read(inventory_path())?)?
    } else {
        Inventory::default()
    };
    let mut targets = inventory.executables;
    targets.push(crate::distribution::install_dir().join("Luma.exe"));
    targets.push(crate::distribution::legacy_install_dir().join("Luma.exe"));
    for path in &targets {
        for candidate in [
            path.clone(),
            path.with_extension("exe.previous"),
            path.with_extension("download"),
        ] {
            if candidate.is_file() {
                let candidate = normalized(&candidate)?;
                if !plan.directories.iter().any(|d| candidate.starts_with(d))
                    && !plan.files.contains(&candidate)
                {
                    plan.files.push(candidate);
                }
            }
        }
    }
    let profile_file = data_root().join("profiles.json");
    let profiles: serde_json::Value = if profile_file.exists() {
        serde_json::from_slice(&fs::read(profile_file)?)?
    } else {
        serde_json::Value::Null
    };
    let mut game_dirs: Vec<PathBuf> = profiles
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| p["game_dir"].as_str())
        .map(PathBuf::from)
        .collect();
    let mut storage_dirs: Vec<PathBuf> = profiles
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| p["storage_dir"].as_str())
        .map(PathBuf::from)
        .collect();
    game_dirs.extend(inventory.game_directories.iter().map(|d| d.path.clone()));
    storage_dirs.extend(inventory.game_directories.iter().map(|d| d.storage.clone()));
    for path in game_dirs {
        if !path.exists() {
            continue;
        }
        let checked = normalized(&path)?;
        if plan.directories.iter().any(|d| checked.starts_with(d)) {
            continue;
        }
        if validate_private_directory(&path, &storage_dirs).is_ok() {
            if !plan.directories.contains(&checked) {
                plan.directories.push(checked);
            }
        } else {
            plan.excluded.push(checked);
        }
    }
    let paths = targets
        .iter()
        .filter_map(|p| normalized(p).ok())
        .map(|p| quote(&p))
        .collect::<Vec<_>>()
        .join(",");
    if !paths.is_empty() {
        let script=format!("[Console]::OutputEncoding=[Text.UTF8Encoding]::new();$targets=@({paths});$w=New-Object -ComObject WScript.Shell;$folders=@([Environment]::GetFolderPath('Desktop'),[Environment]::GetFolderPath('StartMenu'));$links=@(foreach($f in $folders){{Get-ChildItem -LiteralPath $f -Filter '*.lnk' -Recurse -ErrorAction SilentlyContinue | ForEach-Object {{$s=$w.CreateShortcut($_.FullName);$name=[IO.Path]::GetFileName($s.TargetPath);if(($targets -contains $s.TargetPath) -or ($name -match '^Luma(?:-[0-9]+\\.[0-9]+\\.[0-9]+)?\\.exe$') -or $name -eq 'luma-launcher.exe'){{@{{link=$_.FullName;target=$s.TargetPath}}}}}}}});ConvertTo-Json -InputObject $links -Compress");
        #[derive(Deserialize)]
        struct Shortcut {
            link: PathBuf,
            target: PathBuf,
        }
        let links: Vec<Shortcut> = serde_json::from_str(&ps_output(&script)?)
            .context("No se pudieron consultar los accesos directos")?;
        for link in links {
            plan.files.push(normalized(&link.link)?);
            if link.target.is_file() {
                let target = normalized(&link.target)?;
                if !plan.files.contains(&target)
                    && !plan.directories.iter().any(|d| target.starts_with(d))
                {
                    plan.files.push(target);
                }
            }
        }
    }
    plan.files.sort();
    plan.files.dedup();
    plan.directories.sort();
    plan.directories.dedup();
    plan.excluded.sort();
    plan.excluded.dedup();
    Ok(plan)
}
pub fn ensure_not_running(plan: &UninstallPlan) -> Result<()> {
    let mut protected = plan.directories.clone();
    protected.extend(plan.files.clone());
    let paths = protected
        .iter()
        .map(|p| quote(p))
        .collect::<Vec<_>>()
        .join(",");
    let script=format!("$ErrorActionPreference='Stop';$targets=@({paths});$running=@(Get-CimInstance Win32_Process | Where-Object {{$p=$_;$exe=$p.ExecutablePath;$line=$p.CommandLine;$p.ProcessId -ne {} -and @($targets | Where-Object {{($exe -and ($exe.Equals($_,[StringComparison]::OrdinalIgnoreCase) -or $exe.StartsWith($_.TrimEnd('\\')+'\\',[StringComparison]::OrdinalIgnoreCase))) -or (($p.Name -eq 'java.exe' -or $p.Name -eq 'javaw.exe' -or $p.Name -eq 'msedgewebview2.exe') -and $line -and $line.IndexOf($_,[StringComparison]::OrdinalIgnoreCase) -ge 0)}}).Count -gt 0}});if($running.Count){{throw 'Cierra Luma y Minecraft antes de desinstalar.'}}",std::process::id());
    ps_output(&script)?;
    Ok(())
}
pub fn uninstall_files(plan: &UninstallPlan) -> Result<()> {
    ensure_not_running(plan)?;
    // Check every final absolute target before making the first deletion.
    for dir in &plan.directories {
        anyhow::ensure!(
            validate_private_directory(dir, &[])? == *dir,
            "Una carpeta cambió desde la confirmación"
        );
    }
    for file in &plan.files {
        if file.exists() {
            anyhow::ensure!(
                file.is_file() && normalized(file)? == *file,
                "Un archivo cambió desde la confirmación"
            );
        }
    }
    for dir in &plan.directories {
        let checked = validate_private_directory(dir, &[])?;
        delete_verified_directory(dir, &checked)?;
    }
    for file in &plan.files {
        if file.exists() {
            anyhow::ensure!(file.is_file(), "El archivo cambió: {}", file.display());
            fs::remove_file(file)?;
        }
    }
    Ok(())
}
pub fn uninstall(plan: &UninstallPlan) -> Result<()> {
    ensure_not_running(plan)?;
    for path in &plan.directories {
        anyhow::ensure!(
            validate_private_directory(path, &[])? == *path,
            "La carpeta cambió desde la confirmación"
        );
    }
    for path in &plan.directories {
        if crate::distribution::is_program_installation(path) {
            crate::distribution::remove_program_installation(path)?;
        }
    }
    let mut remaining = plan.clone();
    remaining
        .directories
        .retain(|p| !crate::distribution::is_program_installation(p));
    uninstall_files(&remaining)?;
    ps_output("$ErrorActionPreference='Stop';$key='HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Luma';if(Test-Path -LiteralPath $key){Remove-Item -LiteralPath $key -Recurse}")?;
    for dir in &plan.directories {
        if let Some(parent) = dir.parent() {
            if parent.file_name().is_some_and(|n| n == "Luma")
                && parent.is_dir()
                && fs::read_dir(parent)?.next().is_none()
            {
                fs::remove_dir(parent)?;
            }
        }
    }
    Ok(())
}
pub fn register_installation(exe: &Path, setup: &Path, version: &str) -> Result<()> {
    register_executable(exe)?;
    let script=format!("$ErrorActionPreference='Stop';$k='HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Luma';New-Item -Path $k -Force|Out-Null;$values=@{{DisplayName='Luma Launcher';DisplayVersion='{}';Publisher='Luma';InstallLocation={};UninstallString=('\"'+{}+'\" --uninstall')}};foreach($key in $values.Keys){{New-ItemProperty -LiteralPath $k -Name $key -Value $values[$key] -Force|Out-Null}}",version,quote(exe.parent().context("Ruta inválida")?),quote(setup));
    ps_output(&script)?;
    Ok(())
}
pub fn relocate_setup() -> Result<Option<PathBuf>> {
    let current = std::env::current_exe()?.canonicalize()?;
    if app_directories()
        .iter()
        .filter_map(|p| normalized(p).ok())
        .any(|p| current.starts_with(p))
    {
        let folder = std::env::temp_dir().join(format!("Luma-uninstaller-{}", std::process::id()));
        fs::create_dir_all(&folder)?;
        let copy = folder.join("Luma-Setup.exe");
        fs::copy(&current, &copy)?;
        Command::new(&copy)
            .current_dir(&folder)
            .args([
                "--source-setup",
                &current.to_string_lossy(),
                "--wait-for",
                &std::process::id().to_string(),
                "--uninstall",
            ])
            .spawn()?;
        return Ok(Some(copy));
    }
    Ok(None)
}
pub fn delete_setup_after_exit(setup: &Path, temporary_parent: Option<&Path>) -> Result<()> {
    let checked = normalized(setup)?;
    anyhow::ensure!(
        checked.is_file()
            && checked
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("exe")),
        "Instalador inválido"
    );
    let mut script=format!("Wait-Process -Id {} -ErrorAction SilentlyContinue;Start-Sleep -Milliseconds 500;Remove-Item -LiteralPath {} -Force -ErrorAction SilentlyContinue;",std::process::id(),quote(&checked));
    if let Some(parent) = temporary_parent {
        let parent = normalized(parent)?;
        anyhow::ensure!(
            parent.starts_with(std::env::temp_dir().canonicalize()?)
                && parent
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("Luma-uninstaller-")),
            "Carpeta temporal inválida"
        );
        script.push_str(&format!("if(!(Get-ChildItem -LiteralPath {} -Force)){{Remove-Item -LiteralPath {} -ErrorAction SilentlyContinue}}",quote(&parent),quote(&parent)));
    }
    crate::distribution::powershell(&script, false)
}
pub fn temporary_setup_directory() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?.canonicalize().ok()?;
    let parent = exe.parent()?;
    (parent.starts_with(std::env::temp_dir().canonicalize().ok()?)
        && parent
            .file_name()?
            .to_string_lossy()
            .starts_with("Luma-uninstaller-"))
    .then(|| parent.to_path_buf())
}
pub fn wait_for_process(id: u32) -> Result<()> {
    ps_output(&format!(
        "Wait-Process -Id {id} -ErrorAction SilentlyContinue"
    ))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> PathBuf {
        let mut nonce = [0u8; 16];
        getrandom::getrandom(&mut nonce).unwrap();
        let nonce = nonce.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/cleanup-tests")
            .join(format!(
                "{}-{}-{nonce}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        fs::create_dir_all(&root).unwrap();
        root.canonicalize().unwrap()
    }
    #[test]
    fn deletion_is_bounded_and_needs_the_confirmed_absolute_directory() {
        let root = fixture();
        let game = root.join("instance");
        let sibling = root.join("other");
        fs::create_dir_all(game.join("saves/world")).unwrap();
        fs::create_dir_all(&sibling).unwrap();
        fs::write(game.join("saves/world/level.dat"), b"world").unwrap();
        fs::write(sibling.join("keep"), b"keep").unwrap();
        assert!(validate_private_directory(&root, std::slice::from_ref(&game)).is_err());
        assert!(delete_verified_directory(&game, &sibling).is_err());
        assert!(game.join("saves/world/level.dat").exists());
        let confirmed = validate_private_directory(&game, &[]).unwrap();
        delete_verified_directory(&game, &confirmed).unwrap();
        assert!(!game.exists());
        assert_eq!(fs::read(sibling.join("keep")).unwrap(), b"keep");
        delete_verified_directory(&root, &root).unwrap();
    }
    #[test]
    fn rejects_general_user_directories_and_relative_paths() {
        assert!(validate_private_directory(Path::new("relative"), &[]).is_err());
        if let Some(user) = directories::UserDirs::new() {
            assert!(validate_private_directory(user.home_dir(), &[]).is_err());
        }
        assert!(validate_private_directory(Path::new("C:\\"), &[]).is_err());
    }
    #[cfg(windows)]
    #[test]
    fn uninstall_removes_only_the_files_and_directories_in_its_plan() {
        let root = fixture();
        let data = root.join("data");
        fs::create_dir_all(data.join("instances/world")).unwrap();
        fs::write(data.join("instances/world/level.dat"), b"world").unwrap();
        let executable = root.join("Luma.exe");
        fs::write(&executable, b"test application").unwrap();
        let untouched = root.join("unrelated.txt");
        fs::write(&untouched, b"keep").unwrap();
        let plan = UninstallPlan {
            directories: vec![data.canonicalize().unwrap()],
            files: vec![executable.canonicalize().unwrap()],
            ..Default::default()
        };
        uninstall_files(&plan).unwrap();
        assert!(!data.exists());
        assert!(!executable.exists());
        assert_eq!(fs::read(&untouched).unwrap(), b"keep");
        delete_verified_directory(&root, &root).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn running_application_blocks_uninstallation_before_any_deletion() {
        use std::os::windows::process::CommandExt;
        let root = fixture();
        let powershell = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let mut child = Command::new(&powershell)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 20",
            ])
            .creation_flags(0x08000000)
            .spawn()
            .unwrap();
        let plan = UninstallPlan {
            directories: vec![root.clone()],
            files: vec![powershell.canonicalize().unwrap()],
            ..Default::default()
        };
        let blocked = ensure_not_running(&plan);
        let _ = child.kill();
        let _ = child.wait();
        assert!(blocked.is_err());
        assert!(root.exists());
        delete_verified_directory(&root, &root).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn junction_targets_are_never_traversed() {
        let root = fixture();
        let game = root.join("instance");
        let outside = root.join("outside");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("keep"), b"untouched").unwrap();
        ps_output(&format!("$ErrorActionPreference='Stop';New-Item -ItemType Junction -Path {} -Target {} | Out-Null",quote(&game.join("link")),quote(&outside))).unwrap();
        assert!(validate_private_directory(&game.join("link"), &[]).is_err());
        delete_verified_directory(&game, &game.canonicalize().unwrap()).unwrap();
        assert_eq!(fs::read(outside.join("keep")).unwrap(), b"untouched");
        delete_verified_directory(&root, &root).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn process_check_is_read_only_and_accepts_an_unused_target() {
        let root = fixture();
        let plan = UninstallPlan {
            directories: vec![root.clone()],
            ..Default::default()
        };
        ensure_not_running(&plan).unwrap();
        assert!(root.exists());
        delete_verified_directory(&root, &root).unwrap();
    }
}
