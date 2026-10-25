# Estado para retomar Luma

Actualizado: 25/10/2026 (México). Solicitud actual del usuario: limpiar el proyecto (~8 GB), investigar Defender que detecta Setup como Trojan:Win32/Bearfoos.A!ml, corregir crash al abrir webs en Windows 11 Enterprise LTSC, Play de mundos y controles de contenido (switch y tres puntos). Mantener este archivo actualizado para retomar si se acaba la cuota.

## Reglas y contexto
- Proyecto: C:\Users\PCERDITA\Documents\Launcher. Rust/egui, Windows. Repositorio público Fernanboom34/Launcher.
- Último release 0.7.0. No firmamos con certificado: el instalador está NotSigned; se preparó firma opcional. Defender es una detección de malware, distinta de SmartScreen. No desactivar Defender, crear exclusiones ni restaurar la cuarentena para probar.
- Preservar mundos, cuentas y datos reales (LocalAppData/Luma/LumaLauncher/data). No desinstalar ni limpiar datos del jugador.
- Compilación local GNU: PATH .tools/mingw64/bin y ~/.cargo/bin; cargo +stable-x86_64-pc-windows-gnu. Main GNU necesita WebView2Loader.dll al lado; releases MSVC estáticos via GitHub Actions. No publicar GNU suelto.
- No subagentes. No publicar otro instalador sin investigar el bloqueo.
- NUEVA INSTRUCCIÓN DEL USUARIO: una rama por cambio, no trabajar ni integrar en main automáticamente. Ramas desde main, sin mezclar arreglos entre sí.

## En curso
1. Crear este registro, inspeccionar tamaños y rutas antes de eliminar compilados/pruebas obsoletos.
2. Revisar código de interacción de contenido y mundos.
3. Revisar navegador Wry/TAO, capturar errores del sistema y aislar el navegador del proceso principal si procede.
4. Revisar instalador y Defender: inventario de detección de solo lectura, hashes y documentación oficial; no asumir falso positivo sin revisión.

## Pendientes
- Verificar el navegador con compilación MSVC estática de release (solo se probó GNU local hasta ahora), tanto Modrinth como CurseForge. No publicar un release desde la rama de pruebas.
- Verificar Quick Play de extremo a extremo con una COPIA de un mundo, sin modificar los mundos reales. El arreglo de clics ya pasa la prueba de entrada de egui; todavía no se arrancó un mundo para verificarlo.
- Defender sigue sin resolver: obtener análisis de muestra original y revisar/reemplazar diseño de instalador antes de afirmar que está limpio. Leer `REVISION-DEFENDER.md` en rama `security/defender-review` (`git show security/defender-review:REVISION-DEFENDER.md`). No se envió muestra aún.
- DECISIÓN POSTERIOR DEL USUARIO: NO enviar a Microsoft; prefiere solucionarlo en el proyecto. No enviar muestra ni informe. Preparar sustitución del Setup propio por un instalador estándar y probar con Defender activo; no prometer que un cambio de empaquetado elimina por sí solo la detección.
- Revisar ramas e integrar solo después de validación. Usuario pidió ramas por cambio, no cambios directos a main. No se creó tag ni release nuevo.

## Completado en esta sesión
- Registro creado antes de hacer cambios.
- Limpieza terminada: inicialmente target 6,869 GiB y .tools 0,902 GiB. `cargo clean --release` retiró 1,1 GiB; después de las pruebas `scripts/clean-project.ps1 -Artifacts` retiró otros 6,1 GiB de compilados y los temporales de artifacts. Target y artifacts ya no existen. El compilador .tools/mingw64 se conserva porque se necesita localmente. No se tocaron datos reales, Git ni código.
- Registros Windows: el Luma.exe instalado falla al abrir web con 0xc0000005 y 0xc000041d en offset 0x772e0f. Se está aislando WebView2/TAO en proceso separado ejecutado en su hilo principal; el launcher supervisará el cierre y conservará un log de errores.
- Defender confirmó detección de Setup en Descargas (ThreatID 2147731250, Bearfoos.A!ml). No se ha alterado Defender ni restaurado cuarentena. No se puede afirmar todavía que sea un falso positivo. Sigue sin firma pública.
- Rama `fix/instance-controls`: commit `dbf0919`, subida a GitHub. Prueba de interacción que hace clic izquierdo real (RawInput egui) sobre switch, Jugar y tres puntos: pasó. La causa era aplicar context_menu a Frame.response después de crear los botones, registrando una capa clicable encima. El nuevo fondo se registra antes que los controles. Incluye arreglos de filas de Contenido y Mundos.
- Rama `fix/content-browser-process`: commit `46fd356`, subida a GitHub. Ejecuta el MISMO EXE como `--content-browser` en proceso separado; envía solicitud JSON por stdin y eventos JSON por stdout; errores en `<datos Luma>/browser/browser-latest.log`. WebView2 se ejecuta en hilo principal independiente, y la navegación de popups se difiere fuera del callback COM. Si falla el hijo, el padre conserva el launcher y muestra error.
- Navegador: compilación GNU local y Clippy all-targets sin advertencias; 38 pruebas main y 10 setup pasaron. Se abrieron las webs reales Modrinth y CurseForge en procesos hijo ocultos, datos aislados, runtime ya instalado; cerraron con código 0 y sin stderr. Script reproducible guardado en esa rama: `scripts/check-browser-process.py ruta/al/Luma.exe`. Env LUMA_BROWSER_PROBE cierra tras 35 s y no instala runtime. Pendiente MSVC/flujo completo desde botones.
- Rama `security/defender-review`: commit `349f5d8`, subida a GitHub. README diferencia malware de SmartScreen y REVISION-DEFENDER.md contiene evidencia, hash, hipótesis sin afirmar causa, plan y texto en inglés para Microsoft. Se añadió advertencia al inicio del release público 0.7.0 mediante API; no se reemplazaron binarios ni se alteró Defender.
- Rama `chore/project-cleanup`: configuración dev/test sin símbolos ni incremental para reducir tamaño futuro; script de limpieza con comprobación de rutas y rechazo de enlaces. El script se ejecutó correctamente, y cargo metadata aceptó Cargo.toml. El ajuste del perfil cambia cómo se recompila, no lógica de juego.
- Tamaño final medido tras limpiar: **0,903 GiB**, casi todo .tools. No se volvió a compilar localmente después de limpiarlo.
- Rama `ci/branch-verification`: commits `2e34860` y `a945113`, subidos. Extiende el workflow release.yml con input source_ref y publish=false por defecto, y artefacto de prueba que excluye Setup. Únicamente en ESTA RAMA el dispatch manual no publica por defecto; el workflow de main sigue publicando, precaución.
- Verificación MSVC del navegador solicitada sin publicar: [Actions 40167680299](https://github.com/Fernanboom34/Launcher/actions/runs/40167680299), workflow ref `ci/branch-verification`, input source_ref=`fix/content-browser-process`, publish=false. Estaba en curso al registrar este punto. Revisar estado/artefacto `Luma-verificacion-40167680299`, probar EXE MSVC con el script de la rama del navegador y anotar resultado. **No descargar/ejecutar un Setup nuevo**: está excluido del artefacto.
- RESULTADO POSTERIOR: Actions 40167680299 terminó SUCCESS; publicación SKIPPED. Descargado artefacto autenticado (sin pasar token al redirect firmado), solo EXE/JSON/checksums, comprobado SHA-256 y probado EXE MSVC real en LTSC. Modrinth y CurseForge abrieron en procesos hijo y ambos cerraron con código 0, sin stderr. Temporales de las pruebas retirados automáticamente. No se probó aún el flujo manual completo desde Cargar archivos ni descargas a instancia desde el hijo, aunque la rutina de descarga conserva la implementación previa.
- Rama `fix/standard-installer`: commit `15fa200`, subida. Prototipo `installer/Luma.iss` (Inno Setup) sin PowerShell: UAC y archivos de programa nativos, accesos directos, descarga del último manifiesto, URL exacta del repo/version, SHA-256, bloqueo de releases anteriores a 0.7.1. NO LISTO PARA PUBLICAR: faltan borrado de datos con confirmación/contexto correcto, migración de instalaciones antiguas, integración con updater y validación Defender. El README de esa rama lista pendientes.
- `ci/branch-verification` añadió commit `986c16b`: modo installer_only que compila Inno en Windows y prueba, en directorio temporal, que el prototipo rechaza el estable 0.7.0 antes de instalar. [Actions 40168081216](https://github.com/Fernanboom34/Launcher/actions/runs/40168081216): compilación y prueba ya pasaron; no sube ni publica el Setup.
- NUEVA INSTRUCCIÓN: cualquier versión de prueba que se publique debe ser **beta/pre-release**. No convertirla en latest estable ni activar actualizaciones de usuarios estables. No se ha publicado una beta todavía; solo se han subido ramas y artefacto de verificación del launcher.

## Comandos para retomar
```powershell
Get-Content CONTINUAR.md -Encoding UTF8
git status --short
git branch -vv
# Elegir la rama del arreglo que se vaya a continuar; no main.
git switch fix/content-browser-process
$env:PATH="$PWD\.tools\mingw64\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
cargo +stable-x86_64-pc-windows-gnu test --locked
```
Después de la limpieza habrá que recompilar dependencias. El EXE GNU necesita WebView2Loader.dll (el script de smoke añade su carpeta del build al PATH); no distribuirlo solo. Los helpers antiguos en artifacts se eliminaron a propósito: si hace falta GitHub API, usar git credential fill solo en memoria, nunca imprimir credenciales. El workflow actual de main PUBLICA al ejecutarlo manualmente: no dispararlo para verificar una rama sin proteger el paso de publicación.

`main` permanece en `1057f3a8e4d099d1a922ca79c15c6274f3ffb0c9`, sin ninguno de estos arreglos todavía.

Rama que debe quedar seleccionada para encontrar este registro: `chore/project-cleanup`. Los cambios de otras ramas NO están incorporados a ella. El archivo se guarda en disco y Git; actualizarlo antes de cambiar de rama o cerrar la sesión.
