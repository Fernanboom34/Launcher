# Auditoría de ramas y RC

Base comprobada: `c890f48c4694e1ef986958b8716e698098f6ab09` (`release/0.8.0-rc.1`).

`main` avanzó por fast-forward desde `1057f3a8e4d099d1a922ca79c15c6274f3ffb0c9` hasta ese RC. No se reescribió su historia ni se movió el tag publicado.

Se comprobaron `git log origin/release/0.8.0-rc.1..<rama>` y `git cherry` por cada rama. Estar por delante del antiguo main no significa tener trabajo fuera del RC. En particular, empty-content-mascot y beta3-content-table-security tienen cero commits fuera del RC.

## Ramas íntegramente en RC: retiradas

- `feat/skin-library` — `c27605b9af991ccacbbd0e26dc0e113136da0fe5` — YA EN RC.
- `feature/linux-beta2-integration` — `7be7dbf013c258975c9a2e9cafe092c5fb02e0e6` — YA EN RC.
- `feature/linux-x64` — `0d2a95f28ab74f91c08cc5df4c44ba73be0a1bb6` — YA EN RC.
- `fix/beta3-checksum-lines` — `e9b4465c2327129897e9fe03c8efd2dff7729d95` — YA EN RC.
- `fix/beta3-content-table-security` — `adeec637d2bcbe58aa4f1b5ac36c3c5223bd75ee` — YA EN RC.
- `fix/beta3-draft-publication` — `cd6aae5722d2f3327107807d9301d9e2eda6a7b9` — YA EN RC.
- `fix/content-rounded-surfaces` — `6e317535988599c0ebbad8c05afa5e4e6a9116c2` — YA EN RC.
- `fix/game-process-lifecycle` — `25ddf2e372becde4f065e1a19802656baf2020e8` — YA EN RC.
- `fix/instance-version-filters` — `754eafaf511e2e8db53d1854397562e8ba304c92` — YA EN RC.
- `fix/linux-beta-asset-names` — `e0c23b777a87ff16472c5eb4fd535e8a8653dee9` — YA EN RC.
- `fix/skin-model-and-rotation` — `49bdc5a81a12231c8f20ad6a50401090d48fc082` — YA EN RC.
- `fix/skin-preview-projection` — `e3b409d4454b97164088cc4bf10440a96fd60364` — YA EN RC.
- `fix/version-selection` — `6b3075668698eaea4ecc326b3b2b6ffd9ef454a7` — YA EN RC.
- `release/0.7.1-beta.1` — `00e7983c1756f7cdf1c534d4d502ef9a5b07e9ae` — YA EN RC.
- `release/0.7.1-beta.2-windows` — `d9921b32a9525acdb68495a4abfce24f695a5216` — YA EN RC.
- `release/0.7.1-beta.3-windows` — `948341fe52f3b534e095a3beca610a54676e2ec0` — YA EN RC.
- `release/0.8.0-beta.1-windows` — `098cc17f56a3d5c79b238874cbe43920c3f34bb8` — YA EN RC.
- `release/0.8.0-beta.2-windows` — `e70db1a12e94ba0eea9eef0f6d75f177fc850732` — YA EN RC.
- `release/0.8.0-beta.3` — `80b6ecf7d0252b55e672a7d0a14623749b3eb692` — YA EN RC.
- `release/0.8.0-beta.4` — `dc2d8a5570561734ef73f9caaffc572233da5fae` — YA EN RC.
- `ui/empty-content-mascot` — `7a32e5b983f8844e46f42cbde469773df23c5537` — YA EN RC.
- `ui/instance-content-table` — `e123d73e14c7491a3260f6a5028e1edc589ea86e` — YA EN RC.
- `ui/instance-layout-polish` — `3270fb2b756e38d85fa334caad826633f0484042` — YA EN RC.
- `ui/instance-modal-dialogs` — `85dfcd0c2f46254bf27b972b5a622444d9bfc9d4` — YA EN RC.
- `ui/simplify-navigation` — `29a7a79d2cf9d57905c9d8e810028f95065e7fc3` — YA EN RC.

Se eliminaron 25 refs remotas usando leases con su SHA observado para evitar borrar una rama si avanzaba concurrentemente. Sus commits siguen siendo alcanzables desde main y RC.

## Decisión individual para las restantes

| Rama | Commits fuera del RC | Decisión |
|---|---:|---|
| `chore/project-cleanup` | 8 | Conservar notas históricas y decisiones del usuario; no aplicar planes viejos al RC. |
| `ci/branch-verification` | 5 | Conservar como histórico; sustituido por la verificación actual y el PR de CI #6. No integrar workflows antiguos. |
| `fix/browser-downloads` | 1 | Conservar referencia histórica; todos sus parches tienen equivalentes exactos en RC (`git cherry` muestra solamente `-`). No hay funcionalidad que integrar. |
| `fix/content-browser-process` | 1 | Conservar referencia histórica; todos sus parches tienen equivalentes exactos en RC (`git cherry` muestra solamente `-`). No hay funcionalidad que integrar. |
| `fix/instance-controls` | 1 | Conservar referencia histórica; todos sus parches tienen equivalentes exactos en RC (`git cherry` muestra solamente `-`). No hay funcionalidad que integrar. |
| `fix/legacy-skin-normalization` | 1 | Conservar referencia histórica; todos sus parches tienen equivalentes exactos en RC (`git cherry` muestra solamente `-`). No hay funcionalidad que integrar. |
| `fix/setup-bootstrap-elevation` | 2 | Conservar referencia histórica; todos sus parches tienen equivalentes exactos en RC (`git cherry` muestra solamente `-`). No hay funcionalidad que integrar. |
| `fix/stable-updates-from-beta` | 1 | Conservar referencia histórica; todos sus parches tienen equivalentes exactos en RC (`git cherry` muestra solamente `-`). No hay funcionalidad que integrar. |
| `fix/standard-installer` | 2 | Conservar referencia histórica; todos sus parches tienen equivalentes exactos en RC (`git cherry` muestra solamente `-`). No hay funcionalidad que integrar. |
| `fix/uninstall-portable-and-legacy` | 1 | Conservar como histórico: el RC ya contiene la función y una versión posterior parametrizada/testeada de application_plan. No revertirla. |
| `fix/webview-download-paths` | 1 | Conservar referencia histórica; todos sus parches tienen equivalentes exactos en RC (`git cherry` muestra solamente `-`). No hay funcionalidad que integrar. |
| `fix/windows-installer-flow` | 7 | Conservar como histórico: seis parches equivalentes; la diferencia restante documenta un instalador beta antiguo. No incorporar documentación obsoleta. |
| `release/linux-beta2-assets` | 2 | Conservar como histórico; adjunta paquetes a beta 2, no es una funcionalidad pendiente. No modificar un release publicado. |
| `security/defender-review` | 2 | Conservar decisiones y antecedentes; no subir muestras ni cambiar protecciones. No es una corrección de producto pendiente. |

Estas 14 ramas se mantienen sin integrar: conservar una referencia no implica que sea trabajo pendiente. Ninguna corrección nueva reutiliza una de estas ramas antiguas.

## PR históricos en rojo

Al iniciar esta auditoría #1 ya estaba CLOSED y #2 MERGED contra release/0.8.0-beta.2-windows. No había PR abiertos. Sus ramas eran ancestros del RC y se retiraron con el resto. No se fusionó ninguno de esos PR en rojo durante esta sesión ni se alteró su historial.

## Nuevas correcciones, cada una en su PR contra RC

- #4: bloqueo del handle durante hash y copia Windows; Windows y Linux aprobados.
- #5: retirar glib 0.18.5 del lockfile y migrar el event loop de WebView2 a winit; requiere validación nativa Windows.
- #6: CI por SHA, sin cachés de compilación, permisos mínimos, publicaciones sin clobber, Dependabot, RustSec, CodeQL y SECURITY.md. La auditoría niega dependencias unsound; depende de integrar #5 para resolver el aviso de glib, no de ignorarlo.
- #7: autenticidad Ed25519 y pines Authenticode, bloqueo por defecto mientras el propietario no configure confianza. Requiere también #4 antes de distribuir.

## Ajustes administrativos pendientes

La integración devolvió HTTP 403 al crear reglas de tags y al habilitar reportes privados. No se aplicaron ni se afirma que estén activos. Un administrador con permisos de administración deberá aplicar `docs/security/version-tag-rules.json` a través de Settings > Rules > Rulesets (tags `v*`, prohibir cambios y borrado; permitir crear nuevas versiones), y activar Settings > Security > Private vulnerability reporting.

La firma real Authenticode/SignPath y las claves Ed25519 siguen pendientes: el propietario confirmó que aún no dispone de ellas y pidió dejar las actualizaciones sin firma bloqueadas. No se generaron claves privadas de producción en CI.

## Commits exclusivos observados

### chore/project-cleanup

```text
e607b4e Guarda estado final de beta publicada y plan concreto para continuar el instalador
8bf513f Registra laptop de prueba y limitación de la regla local de Defender
b572484 Actualiza prioridades de continuidad y enlace de compilación de beta
f13b1f4 Registra integración y compilación de beta sin incluir instalador pendiente
70105fa Registra prueba MSVC en LTSC, prototipo Inno y política de pre-releases
b4adeb2 Registra decisión de no enviar muestra y sustituir el diseño del instalador
fc7f392 Registra limpieza final y ejecución de verificación MSVC sin publicación
45f1c68 Limpia generados, limita cachés de desarrollo y registra continuidad del trabajo
```

### ci/branch-verification

```text
b076641 Conserva la compilación verificada aunque falle la prueba del instalador
15522d9 Prueba instalación y desinstalación Inno aisladas antes de guardar un nuevo Setup
986c16b Verifica el compilador Inno y el bloqueo de releases antiguos sin publicar instaladores
a945113 Excluye el instalador pendiente de revisión de los artefactos de prueba
2e34860 Permite verificar ramas en Windows sin publicar ni reemplazar releases
```

### fix/browser-downloads

```text
5ebc8a8 Permite descargas blob HTTPS y conserva el panel nativo y los motivos de interrupción
```

### fix/content-browser-process

```text
46fd356 Aísla el navegador de contenido en su propio proceso e hilo principal
```

### fix/instance-controls

```text
dbf0919 Corrige clics en controles de contenido y mundos sin taparlos con el menú contextual
```

### fix/legacy-skin-normalization

```text
b1e0fda Normalize opaque legacy skin hat regions before preview and upload
```

### fix/setup-bootstrap-elevation

```text
c57b49f Use standard Inno elevation instead of unsupported self-execution
c3371f7 Fix explicit UAC launch of installer engine
```

### fix/stable-updates-from-beta

```text
f0fc7c9 Permite a una beta detectar la siguiente actualización estable
```

### fix/standard-installer

```text
a622925 Documenta compilación y prueba del prototipo Inno sin instalar el estable anterior
15fa200 Prepara prototipo de instalador estándar con descargas verificadas sin PowerShell
```

### fix/uninstall-portable-and-legacy

```text
1415907 Uninstall registered portable and legacy copies without installing first
```

### fix/webview-download-paths

```text
980ae83 Normalize WebView2 download paths and reproduce persisted-profile failure
```

### fix/windows-installer-flow

```text
77849b6 Documenta el Setup beta nativo, pruebas y límites de los contextos de usuario
7abafdb Adapta el diálogo al constructor de Inno Setup 6.6 o posterior
5849195 Añade prueba aislada de copia, migración del Setup viejo y desinstalación nativa
b4d6490 Registra flujo nativo de instalación y corrección de la nota pública
22cac64 Integra instalación nativa y confirmación de datos en el contexto original del jugador
7509fe1 Documenta compilación y prueba del prototipo Inno sin instalar el estable anterior
df1735e Prepara prototipo de instalador estándar con descargas verificadas sin PowerShell
```

### release/linux-beta2-assets

```text
ae96efe Align Linux checksum filenames with GitHub asset naming
e5db028 Attach validated Linux beta 2 assets using native Actions authentication
```

### security/defender-review

```text
99ba40c Respeta decisión del usuario de resolver el diseño sin enviar muestras
349f5d8 Documenta la detección de Defender y prepara la solicitud de revisión del instalador
```

