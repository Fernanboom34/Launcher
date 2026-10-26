use anyhow::{Context, Result};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::{Child, ExitStatus},
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};
type OwnedChild = Arc<Mutex<(Child, bool)>>;
static GAMES: OnceLock<Mutex<HashMap<PathBuf, OwnedChild>>> = OnceLock::new();
fn games() -> &'static Mutex<HashMap<PathBuf, OwnedChild>> {
    GAMES.get_or_init(Default::default)
}
fn key(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}
pub struct Managed {
    key: PathBuf,
    child: OwnedChild,
}
impl Managed {
    pub fn register(path: &Path, mut child: Child) -> Result<Self> {
        let key = key(path);
        let mut games = games()
            .lock()
            .map_err(|_| anyhow::anyhow!("Registro de procesos ocupado"))?;
        if games.contains_key(&key) {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("La instancia ya está en ejecución");
        }
        let child = Arc::new(Mutex::new((child, false)));
        games.insert(key.clone(), child.clone());
        Ok(Self { key, child })
    }
    pub fn wait(&self) -> Result<ExitStatus> {
        loop {
            if let Some(status) = self
                .child
                .lock()
                .map_err(|_| anyhow::anyhow!("Proceso ocupado"))?
                .0
                .try_wait()?
            {
                return Ok(status);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    pub fn was_stopped(&self) -> bool {
        self.child.lock().is_ok_and(|p| p.1)
    }
}
impl Drop for Managed {
    fn drop(&mut self) {
        if let Ok(mut games) = games().lock() {
            if games
                .get(&self.key)
                .is_some_and(|child| Arc::ptr_eq(child, &self.child))
            {
                games.remove(&self.key);
            }
        }
    }
}
pub fn stop(path: &Path) -> Result<bool> {
    let child = games()
        .lock()
        .map_err(|_| anyhow::anyhow!("Registro de procesos ocupado"))?
        .get(&key(path))
        .cloned();
    let Some(child) = child else {
        return Ok(false);
    };
    let mut child = child
        .lock()
        .map_err(|_| anyhow::anyhow!("Proceso ocupado"))?;
    if child.0.try_wait()?.is_some() {
        return Ok(false);
    }
    child
        .0
        .kill()
        .context("No se pudo detener esta instancia")?;
    child.1 = true;
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stop_only_controls_the_registered_child() {
        #[cfg(windows)]
        let make = || {
            use std::os::windows::process::CommandExt;
            let mut c = std::process::Command::new("powershell.exe");
            c.args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 30",
            ])
            .creation_flags(0x08000000);
            c.spawn().unwrap()
        };
        #[cfg(not(windows))]
        let make = || {
            std::process::Command::new("sleep")
                .arg("30")
                .spawn()
                .unwrap()
        };
        let root = std::env::temp_dir().join(format!("luma-process-test-{}", std::process::id()));
        let first = Managed::register(&root.join("first"), make()).unwrap();
        let second = Managed::register(&root.join("second"), make()).unwrap();
        assert!(!stop(&root.join("unknown")).unwrap());
        assert!(stop(&root.join("first")).unwrap());
        first.wait().unwrap();
        assert!(second.child.lock().unwrap().0.try_wait().unwrap().is_none());
        assert!(stop(&root.join("second")).unwrap());
        second.wait().unwrap();
        assert!(second.was_stopped());
        drop(first);
        drop(second);
        assert!(!stop(&root.join("first")).unwrap());
    }
}
