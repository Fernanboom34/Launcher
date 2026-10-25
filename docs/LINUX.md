# Luma para Linux x64

Requiere Linux **x86_64 con glibc**, OpenGL 3.3 y una sesión de escritorio X11 o Wayland. No incluye Minecraft, Java, mundos ni cuentas. Java se descarga de Eclipse Temurin y se verifica con SHA-256 al preparar una instancia.

## Instalar

En Debian/Ubuntu, descarga el `.deb` y ejecuta:

```sh
sudo apt install ./luma-launcher_0.7.0_amd64.deb
```

El menú de aplicaciones mostrará **Luma**. También puedes ejecutar `luma-launcher`. Retira el paquete con `sudo apt remove luma-launcher`; los datos de juego se conservan.

Para instalar solo para tu usuario, extrae `Luma-0.7.0-linux-x64.tar.gz` y, dentro de esa carpeta, ejecuta:

```sh
./install.sh
```

Necesita Python 3, no necesita sudo. Instala en `${XDG_DATA_HOME:-$HOME/.local/share}/luma-launcher`, registra Luma en el menú y crea `~/.local/bin/luma-launcher`. Puedes ejecutar esa ruta aunque `~/.local/bin` no esté en PATH. Comprueba las sumas publicadas con `sha256sum -c SHA256SUMS-linux-x64.txt`; el instalador comprueba también los archivos del paquete antes de copiarlos.

Para desinstalar la instalación por usuario:

```sh
python3 "${XDG_DATA_HOME:-$HOME/.local/share}/luma-launcher/uninstall.py" --uninstall
```

Se conservan mundos, cuentas, ajustes, Java y directorios importados. Para actualizar, instala el nuevo paquete. La actualización integrada admite la instalación por usuario; el `.deb` se actualiza con el gestor de paquetes.

El mínimo de glibc depende de dónde se compile. El `.deb` declara el mínimo real del ejecutable y `package.json` lo indica en el tar.gz. El workflow Linux utiliza Ubuntu 22.04 como base; un paquete compilado en otra distribución puede requerir una glibc más reciente. Alpine/musl, ARM y sistemas sin escritorio no están soportados por estos paquetes.

## Diferencias en Linux

- El catálogo integrado de Modrinth permite descargar contenido con el destino y las verificaciones habituales.
- Las webs de Modrinth y CurseForge se abren en el navegador del sistema. Para instalar sus descargas usa **Cargar archivos → Elegir archivos…**; las descargas del navegador no se trasladan automáticamente. WebView2 sigue siendo exclusivo de Windows.
- Importa las instalaciones del launcher oficial desde `~/.minecraft`. Los datos propios usan las rutas XDG de Luma; `LUMA_DATA_DIR` permite aislar pruebas.
- Las bibliotecas nativas y el Java administrado son Linux x64. Los comandos antes/después del juego se ejecutan mediante `/bin/sh`, en la carpeta de la instancia. No uses comandos de PowerShell o `cmd.exe` en esos campos.
- La autenticación Microsoft necesita el mismo registro autorizado que Windows. El jugador local funciona sin Client ID.
- Un escritorio con portales permite los diálogos de archivos. Si no abren, comprueba `xdg-desktop-portal` y el backend de tu escritorio. Necesita `xdg-open` para carpetas y navegador.

## Compilar y comprobar

Requiere Rust estable con rustfmt y Clippy, compilador C, pkg-config, binutils, dpkg-deb y Python 3. Las bibliotecas de escritorio anteriores deben estar instaladas. No se requieren GTK/WebKit de desarrollo.

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
bash scripts/build-linux.sh
python3 scripts/test-linux-package.py
```

Los productos quedan en `artifacts/`: `.deb`, `.tar.gz`, ejecutable ELF, manifiesto Linux separado y sumas SHA-256. El empaquetado no modifica el instalador, manifiesto o workflow de publicación de Windows y no publica ningún release. `luma-setup.exe` es el instalador de Windows; en Linux se distribuyen los paquetes descritos aquí.
