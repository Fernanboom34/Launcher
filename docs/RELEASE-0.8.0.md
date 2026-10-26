# Luma v0.8.0 Release

Versión estable para Windows x64 y Linux x64 con glibc, con todos los cambios del RC y la revisión final de dependencias y CI.

- Nueva identidad de luna y colores azul oscuro/menta, incluidos los iconos del launcher, EXE e instalador.
- Biblioteca de instancias Vanilla, Fabric, Forge y NeoForge; contenido, mundos, registros y configuración independientes.
- Tabla de proyectos con casillas individuales, filas más amplias, bordes corregidos y acciones flotantes. Instancias vacías muestran el personaje amarillo con fondo transparente y mensaje en es-MX/en-US según el idioma configurado.
- Configuración con barra de RAM visible, selección coherente y ventana redimensionable sin marco de sistema por defecto. Detener utiliza rojo. Loss permanece únicamente dentro del launcher.
- Quitar contenido requiere confirmación y elimina permanentemente los archivos seleccionados. Crear respaldo duplica la instancia completa antes de continuar. No se crea .luma/removed; los respaldos antiguos del usuario no se borran automáticamente.
- Validaciones estructurales de contenido, límites de descarga/descompresión, rechazo de rutas peligrosas y enlaces, corrección de la carrera de copia del actualizador Windows y retiro de glib 0.18.5 del lockfile.

## Descargas

**Windows:** `Luma-Setup.exe` instala/actualiza Luma; `Luma-0.8.0.exe` es el ejecutable portable. El instalador es nativo Inno Setup y conserva los datos del jugador salvo que se confirme su eliminación.

**Linux x64:** ejecutable, `.tar.gz` con `install.sh` o `.deb`:

```sh
sudo apt install ./luma-launcher_0.8.0_amd64.deb
```

Las sumas SHA-256 están en `SHA256SUMS.txt` y `SHA256SUMS-linux-x64.txt`. La aplicación no incluye Minecraft, Java, mods ni cuentas preconfiguradas: se descargan o agregan según la instancia.

## Estado de firma y actualizaciones

Todavía no se ha configurado certificado Authenticode/SignPath ni claves públicas Ed25519 de producción. Windows puede mostrar el aviso de editor desconocido. En esta versión las actualizaciones automáticas quedan bloqueadas hasta configurar esa confianza; las nuevas versiones se instalan manualmente. Los hashes comprueban integridad, no autentican por sí solos al editor. Véase `docs/security/UPDATE-SIGNING.md`.

Las validaciones no garantizan que un mod JAR válido sea inocuo ni aíslan Minecraft. Persisten avisos de mantenimiento en dependencias; la auditoría no los oculta y rechaza vulnerabilidades y avisos de tipo unsound. Detalles en SECURITY.md y SECURITY-REVIEW.md.

## Validación de publicación

La publicación exige formato, Clippy estricto, pruebas Rust, compilaciones nativas de Windows/Linux, prueba de instalación/desinstalación Inno con datos desechables, pruebas de paquetes Linux, capturas de interfaz, RustSec y CodeQL. Tres integraciones externas siguen siendo opt-in. No equivale a probar todas las versiones de Minecraft, distribuciones Linux o cuentas Microsoft reales.
