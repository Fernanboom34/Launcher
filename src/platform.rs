use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub fn path_key(path: &Path) -> String {
    let text = path.to_string_lossy();
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text.into_owned()
    }
}

pub const OS: &str = if cfg!(windows) { "windows" } else { "linux" };
pub const JAVA_BINARY: &str = if cfg!(windows) { "java.exe" } else { "java" };
pub const EXECUTABLE_SUFFIX: &str = if cfg!(windows) { ".exe" } else { "" };
pub const CLASSPATH_SEPARATOR: &str = if cfg!(windows) { ";" } else { ":" };
pub const FILE_MANAGER: &str = if cfg!(windows) {
    "explorer.exe"
} else {
    "xdg-open"
};
pub const HOOK_DESCRIPTION: &str = if cfg!(windows) {
    "Se ejecutan en la carpeta de la instancia mediante cmd.exe."
} else {
    "Se ejecutan en la carpeta de la instancia mediante /bin/sh."
};

pub fn official_minecraft_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join(".minecraft"))
    }
    #[cfg(not(windows))]
    {
        directories::UserDirs::new().map(|u| u.home_dir().join(".minecraft"))
    }
}

pub fn hook_command(text: &str) -> Command {
    #[cfg(windows)]
    let mut command = Command::new("cmd.exe");
    #[cfg(windows)]
    command.args(["/D", "/S", "/C", text]);
    #[cfg(not(windows))]
    let mut command = Command::new("/bin/sh");
    #[cfg(not(windows))]
    command.args(["-c", text]);
    command
}

pub fn system_java() -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|p| p.join(JAVA_BINARY))
        .find(|p| p.is_file())
}

pub fn os_version() -> &'static str {
    static VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    VERSION.get_or_init(|| {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            Command::new("cmd")
                .args(["/C", "ver"])
                .creation_flags(0x08000000)
                .output()
                .ok()
                .and_then(|output| {
                    regex::Regex::new(r"\d+\.\d+\.\d+")
                        .ok()?
                        .find(&String::from_utf8_lossy(&output.stdout))
                        .map(|v| v.as_str().to_owned())
                })
                .unwrap_or_else(|| "10.0.0".into())
        }
        #[cfg(not(windows))]
        {
            std::fs::read_to_string("/proc/sys/kernel/osrelease")
                .map(|s| s.trim().to_owned())
                .unwrap_or_default()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classpath_and_java_match_host() {
        let joined = std::env::join_paths([PathBuf::from("a"), PathBuf::from("b")]).unwrap();
        assert_eq!(joined.to_str().unwrap(), format!("a{CLASSPATH_SEPARATOR}b"));
        assert_eq!(JAVA_BINARY.ends_with(".exe"), cfg!(windows));
        assert_eq!(
            path_key(Path::new("World")) == path_key(Path::new("world")),
            cfg!(windows)
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn hooks_execute_in_the_instance_directory_and_report_failure() {
        let root = std::env::temp_dir().join(format!("luma-hook-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        assert!(hook_command("printf 'hook works' > hook.txt")
            .current_dir(&root)
            .status()
            .unwrap()
            .success());
        assert_eq!(std::fs::read(root.join("hook.txt")).unwrap(), b"hook works");
        assert!(!hook_command("exit 7").status().unwrap().success());
        std::fs::remove_dir_all(root).unwrap();
    }
}
