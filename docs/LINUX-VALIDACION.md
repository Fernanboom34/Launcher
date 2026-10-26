# Validación de la adaptación Linux x64

Comprobaciones realizadas en un entorno Debian 13 x86_64 con Rust 1.99.1. Los datos de prueba se mantuvieron separados mediante `LUMA_DATA_DIR`; no se usaron cuentas Microsoft ni mundos del usuario.

- `cargo fmt --check` y `cargo clippy --locked --all-targets -- -D warnings`: aprobados.
- Suite local: 36 pruebas aprobadas, ninguna fallida y una prueba de red omitida en la ejecución habitual. El instalador de Windows no ejecuta sus pruebas específicas en Linux.
- La prueba omitida de Modrinth se ejecutó por separado: búsqueda y versiones, descarga de Mod Menu y sus dependencias para Fabric 1.21.1, comprobaciones de archivos y reinstalación sin duplicados; aprobada.
- Compilación optimizada del launcher y arranque de su interfaz X11 con OpenGL por software. Captura real en `artifacts/linux-home.png`.
- Instalación real de Minecraft 1.7.10 y Eclipse Temurin Java 8 Linux x64. SHA-256 del runtime verificado; preparación de 707 archivos y verificación posterior sin conexión aprobadas.
- Arranque real de Minecraft con jugador local: LWJGL 2.9.1, OpenAL y atlas de texturas inicializados, menú visible y cierre normal con código 0. Captura en `artifacts/linux-minecraft.png`. No se creó un mundo ni se probó multijugador.
- Paquetes `.deb` y `.tar.gz` generados con binario ELF x64 y sumas SHA-256. El paquete local declara glibc >= 2.35, según los símbolos obligatorios del ejecutable; no se confunden los símbolos débiles opcionales con requisitos mínimos.
- Prueba de paquete con HOME/XDG aislados y ruta con espacios, `%` y `$`: rechazo de un binario alterado, instalación, permisos de ejecución, acceso al menú y comando, detección de perfiles, reinstalación, conservación de datos, desinstalación y rechazo de comandos ajenos aprobados. Payload del `.deb` extraído y ejecutado correctamente. No se instaló el `.deb` sobre el sistema anfitrión con privilegios.

La integración de webs en Linux utiliza el navegador externo y la importación manual de archivos; el catálogo de Modrinth sigue integrado. No se verificaron una sesión Wayland real, otras distribuciones, los loaders jugando, autenticación Microsoft ni una actualización descargada de un release Linux público. La regresión Windows requiere ejecutar el workflow de Windows; sus scripts y manifiesto de publicación se conservan.

Durante la prueba real apareció un 404 al intentar descargar un JAR genérico de una biblioteca que solo publica clasificadores nativos. Se corrigió la selección del artefacto y se añadió cobertura al caso de instalación sin conexión. Las nativas Linux y los runtimes se guardan aparte de los de Windows; se mantiene la lectura de marcadores de Java Windows existentes.
