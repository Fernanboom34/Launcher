use super::*;
use std::{os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

fn data_home() -> Result<PathBuf> {
    Ok(directories::BaseDirs::new()
        .context("No hay directorio de usuario")?
        .data_dir()
        .to_path_buf())
}

fn desktop_quote(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$")
        .replace('%', "%%")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    format!("\"{escaped}\"")
}

pub fn shortcut(executable: &Path, name: &str, arguments: &str) -> Result<()> {
    let args = shell_words::split(arguments)?;
    let exec = std::iter::once(executable.to_string_lossy().into_owned())
        .chain(args)
        .map(|s| desktop_quote(&s))
        .collect::<Vec<_>>()
        .join(" ");
    let safe_name: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect();
    anyhow::ensure!(!safe_name.is_empty(), "Nombre de acceso directo vacío");
    let folder = data_home()?.join("applications");
    fs::create_dir_all(&folder)?;
    fs::write(folder.join(format!("luma-{safe_name}.desktop")), format!(
        "[Desktop Entry]\nType=Application\nName={}\nExec={exec}\nIcon=luma-launcher\nTerminal=false\nCategories=Game;\n",
        name.replace(['\n', '\r'], " ")
    ))?;
    Ok(())
}

pub fn apply_update(staged: &Path) -> Result<()> {
    let exe = std::env::current_exe()?.canonicalize()?;
    let installed = data_home()?.join("luma-launcher/bin/luma-launcher");
    anyhow::ensure!(
        installed.canonicalize().ok().as_ref() == Some(&exe),
        "Esta instalación se actualiza con su paquete. Instala el nuevo .deb o vuelve a ejecutar install.sh del paquete Linux."
    );
    super::validate_executable(staged)?;
    let temporary = exe.with_extension(format!("update-{}", std::process::id()));
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = (|| -> Result<()> {
        std::io::copy(&mut fs::File::open(staged)?, &mut output)?;
        output.set_permissions(fs::Permissions::from_mode(0o755))?;
        output.sync_all()?;
        fs::copy(&exe, exe.with_extension("previous"))?;
        fs::rename(&temporary, &exe)?;
        Command::new(&exe)
            .spawn()
            .context("No se pudo reiniciar Luma")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn desktop_exec_quotes_paths_and_field_codes() {
        assert_eq!(
            desktop_quote("/home/user/My Games/Luma%"),
            "\"/home/user/My Games/Luma%%\""
        );
        assert_eq!(desktop_quote("$HOME`x`\""), "\"\\$HOME\\`x\\`\\\"\"");
    }
}
