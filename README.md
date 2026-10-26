# Luma

## Beta 0.8.0-beta.1-windows

El selector de Minecraft muestra las versiones públicas estables y permite mostrar todas desde el desplegable. Fabric añade filtros de estables y builds, tanto al crear como al editar instancias. Inicio concentra la biblioteca; se retiran las páginas redundantes de biblioteca, versiones/descargas y actividad. Los registros del juego siguen dentro de cada instancia.

La nueva página **Skins** guarda PNG de 64×64 o 64×32, permite importarlos o arrastrarlos, editar nombre/modelo/PNG, exportar y eliminar. Incluye vista previa estática con giro manual y modelos clásico/delgado. Las nueve skins predeterminadas se descargan desde los servidores de texturas de Minecraft; no se incluyen imágenes ni JAR en la distribución.

Con Microsoft se puede cargar la apariencia actual, aplicar/restablecer la skin y elegir entre las capas que posee la cuenta; requiere sesión activa. Para cuentas locales, **Instalar mod y aplicar skin** descarga [CustomSkinLoader](https://modrinth.com/mod/customskinloader) y sus dependencias compatibles desde Modrinth en la instancia elegida, y configura el PNG local con copias de seguridad. Es necesario un cargador compatible; Vanilla muestra una indicación para cambiarlo desde la configuración de la instancia. La skin local se ve en ese cliente; otros jugadores no reciben automáticamente el PNG. Desactivar restaura un PNG previo si existía y conserva el mod. Ninguna operación de las pruebas cambia una cuenta Microsoft real.

Esta rama integra los arreglos de interacción, el navegador en proceso separado y las descargas `blob:` de páginas HTTPS. El panel nativo de descargas permanece disponible para decisiones del navegador.

Incluye un Setup basado en Inno Setup con elevación estándar de Windows. Instala esta beta en Program Files y ofrece instalar/actualizar o desinstalar, también para copias portables registradas y versiones anteriores a Inno, sin exigir instalar primero. Descarga y verifica un helper temporal que confirma el borrado opcional de datos con el usuario original y retira las copias registradas del launcher. Después se retira la instalación de Program Files si existe. Desde Aplicaciones de Windows el desinstalador nativo conserva los datos; abre el Setup para elegir su eliminación completa. El Setup descargado permanece en Descargas. Instalar y desinstalar desde el Setup requieren conexión para obtener los archivos verificados.

Esta distribución es solo una pre-release de Windows; no modifica main ni se ofrece como actualización automática estable. El Setup 0.7.0 tuvo una detección de Defender en un equipo y no se reprodujo en la laptop del usuario; no es un diagnóstico de GitHub Actions. El cambio de instalador todavía requiere comparación con Defender activo. No se recomienda permitir amenazas ni restaurar cuarentena. Esta beta descarga su payload de su propio tag para que la prueba sea reproducible.

Launcher nativo de **Minecraft Java Edition para Windows x64**, escrito en Rust con egui. Biblioteca de instancias, temas y acentos menta, con catálogo de contenido y navegador web integrado.

## Linux x64

La adaptación Linux incluye paquetes `.deb` y `.tar.gz` con instalación por usuario, soporte X11/Wayland y Java/nativas Linux. Consulta [instalación, diferencias y compilación en Linux](docs/LINUX.md). Windows conserva su instalador y su proceso de publicación.

## Novedades 0.7.0

- Corregida la franja negra bajo la cabecera y los mensajes CLIXML de PowerShell: los errores se presentan como texto normal en español.
- El instalador usa `C:/Program Files/Luma Launcher`. Windows solicita permisos para copiar o retirar los archivos de esa carpeta; las cuentas, accesos directos y datos del jugador se gestionan con su usuario habitual. El actualizador también solicita permisos cuando la aplicación está en Program Files.
- Se retiró el interruptor **Modo sin conexión**. Una cuenta local permite jugar sin Microsoft; una cuenta Microsoft guardada sin sesión activa utiliza su identidad guardada. Una sesión Microsoft válida habilita sus funciones autenticadas. Las descargas se intentan cuando faltan archivos y las instalaciones completas siguen funcionando con los archivos locales.
- **Instancia → Contenido → Cargar archivos** ofrece archivos locales, catálogo de Modrinth o la web de Modrinth/CurseForge dentro de Luma.
- El catálogo de Modrinth filtra por Minecraft y loader, muestra versiones compatibles, descarga desde su CDN y comprueba las sumas de verificación. Instala las dependencias obligatorias y reemplaza los archivos de las versiones anteriores.
- El navegador web mantiene un destino fijo para la instancia que lo abrió. Mods van a `mods`, resource packs a `resourcepacks`, shaders a `shaderpacks` y datapacks al mundo elegido, en `saves/<mundo>/datapacks`. Las descargas se preparan aparte y solo pasan a su destino cuando se completan y son un JAR/ZIP válido. Cierra el navegador antes de iniciar el juego.

Para cambiar una instalación anterior a Program Files, cierra Luma y ejecuta el instalador nuevo. La actualización desde la aplicación reemplaza el ejecutable en su ubicación actual; no traslada la instalación. No desinstales borrando los datos si quieres conservar tus mundos y cuentas.

Modrinth permite consultas y descargas públicas sin una clave. La API de terceros de CurseForge exige solicitar una clave; esta versión utiliza su página web sin reutilizar claves ajenas. El navegador usa Microsoft Edge WebView2: si falta, se descarga el instalador de Microsoft y se valida su firma antes de ejecutarlo. El runtime compartido de Microsoft puede ser utilizado por otras aplicaciones; el desinstalador de Luma elimina su caché propia, no ese runtime del sistema. Fuentes: [Modrinth](https://docs.modrinth.com/api/) y [CurseForge](https://docs.curseforge.com/rest-api/).

## Aviso de Windows SmartScreen

Los releases actuales no tienen una firma pública de editor configurada. Windows puede mostrar **Windows protegió su PC** o **editor desconocido** al descargar el instalador. Este aviso no demuestra por sí solo que un archivo sea malicioso, y tampoco debe interpretarse esta documentación como una garantía de seguridad.

Descarga únicamente desde los [releases oficiales de Luma](https://github.com/Fernanboom34/Launcher/releases). Para comprobar los archivos usa `Get-FileHash -Algorithm SHA256` y compara el resultado con `SHA256SUMS.txt` publicado en ese release. Si verificaste el origen y decides ejecutarlo, Windows puede ofrecer **Más información → Ejecutar de todas formas**. En equipos administrados esta opción puede estar bloqueada. El aviso de permisos de administrador para Program Files es un paso distinto. Puede identificar a Windows PowerShell, que ejecuta la copia restringida de archivos; eso no significa que Luma esté firmado por Microsoft.

Firmar todas las versiones con una identidad pública validada ayuda a acumular reputación. Un certificado autofirmado no evita el aviso en equipos ajenos; los certificados EV tampoco garantizan confianza inmediata. Consulta [SmartScreen para desarrolladores](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation). Para publicar sin esos avisos, otra vía es una aplicación MSIX certificada y distribuida por Microsoft Store; eso requiere preparar y enviar ese paquete por separado.

La compilación admite `build.ps1 -SigningCertificateThumbprint HUELLA_DEL_CERTIFICADO`: firma ambos ejecutables con SHA-256 y sello de tiempo antes de generar el manifiesto. El certificado y su clave privada deben estar disponibles en `Cert:/CurrentUser/My`. En GitHub Actions se pueden configurar los secretos `LUMA_SIGNING_PFX_BASE64` y `LUMA_SIGNING_PFX_PASSWORD` si el proveedor permite ese tipo de certificado exportable; para un token/HSM o servicio de firma se necesita su integración correspondiente. La clave privada nunca debe subirse al repositorio. No se ha usado el certificado local de desarrollo para simular una firma pública.

## Novedades 0.6.1

El instalador permite **instalar/actualizar** o **desinstalar** Luma. Al instalar registra la aplicación en Windows y guarda una copia de `Luma-Setup.exe` junto a `Luma.exe`. También puedes abrir el instalador descargado y entrar en **Desinstalar**; funciona sin conexión.

La desinstalación muestra las rutas que borrará y pide confirmar la pérdida permanente de datos. Incluye la aplicación instalada, copias registradas del ejecutable, descargas, Java administrado por Luma, cuentas, ajustes, mundos, mods, capturas y accesos directos. Las carpetas externas compartidas con otros launchers requieren marcar una opción adicional, porque borrarlas también afecta a esos launchers. Se puede eliminar el propio instalador al cerrar. Cierra Luma y Minecraft antes de desinstalar.

En el menú de clic derecho de una instancia hay dos acciones: **Eliminar de la biblioteca** conserva los archivos y **Eliminar completamente…** borra permanentemente la carpeta de juego confirmada. Esta última requiere una carpeta independiente; si está compartida, se indica el motivo. Las bibliotecas y los recursos que usa más de una instancia se mantienen hasta desinstalar Luma.

Los directorios externos conocidos se recuerdan incluso después de retirar una instancia de la biblioteca. Las copias portátiles se registran cuando se abren con esta versión; también se detectan ejecutables de Luma enlazados desde el escritorio y el menú Inicio. El desinstalador no busca ejecutables desconocidos por todo el disco.

## Novedades 0.6.0

- Páginas de instancia con contenido activable, explorador de archivos, mundos y registros en vivo. Los menús de clic derecho incluyen acciones de biblioteca, carpetas, duplicación, portada y accesos directos.
- Configuración por instancia: versión del juego y loader, reparación, Java, memoria, argumentos, variables de entorno, ventana y comandos antes/después del juego.
- Inicio continúa el mundo más reciente usando Quick Play en Minecraft 1.20 o posterior. En versiones antiguas inicia la instancia; puedes entrar al mundo desde el juego.
- Temas oscuro, claro, OLED y del sistema; sombras/animaciones, marco de ventana y comportamiento del launcher. La memoria predeterminada se copia al crear cada instancia: modificarla no cambia las existentes.
- Selector de cuentas locales y Microsoft; galería de capturas de todas las instancias.
- Instalador descargable y aviso superior de actualizaciones mediante GitHub Releases. Las descargas verifican SHA-256 antes de ejecutarse.

## Instalador y actualizaciones

Descarga `Luma-Setup.exe` del [último release público](https://github.com/Fernanboom34/Launcher/releases/latest). Instala el ejecutable en `C:/Program Files/Luma Launcher` y permite crear un acceso directo al escritorio. El mismo instalador consulta siempre el release estable más reciente. Para una instalación sin interfaz: `Luma-Setup.exe --install-to "C:/Mi carpeta/Luma"`.

Luma consulta actualizaciones al abrir con conexión. Si encuentra una versión mayor, muestra **Nueva versión disponible · Actualizar** en la parte superior. Al pulsarlo descarga y verifica la nueva aplicación, cierra Luma, reemplaza el ejecutable y vuelve a abrirlo. Conserva una copia `.previous` para recuperar el ejecutable si falla el reemplazo. Los datos del jugador viven en otra carpeta.

Los archivos distribuidos son exclusivamente el launcher, el instalador y `luma-release.json`. **No incluyen JAR, Java, mundos, mods ni perfiles preconstruidos**. Todo el contenido de Minecraft y sus loaders se descarga de sus proveedores al crear o reparar una instancia. Una instalación limpia empieza sin instancias; detectar una instalación existente solo registra sus archivos locales.

Para publicar una nueva versión cambia `Cargo.toml`, actualiza `Cargo.lock`, confirma los cambios y crea el tag `vX.Y.Z`. `.github/workflows/release.yml` verifica, compila y publica ambos ejecutables y el manifiesto. También admite ejecución manual desde Actions. El secreto opcional `LUMA_MICROSOFT_CLIENT_ID` incorpora el registro Microsoft propio en las compilaciones. `build.ps1` genera los tres archivos localmente en `artifacts/`.

## Abrir

Ejecuta `artifacts/Luma.exe` si ya está compilado. No requiere instalar Rust para utilizarlo.

1. En **Inicio** o **Biblioteca**, pulsa **+ Nueva instancia**. Elige un nombre, el cargador y una versión de Minecraft. **Versiones** permite buscar un lanzamiento del catálogo y abrir la misma ventana de creación.
2. Pulsa **Crear instancia**. Se descargan el cliente, las bibliotecas, los recursos y el Java que requiere esa versión.
3. En el panel derecho, abre **Cambiar jugador**, escribe tu nombre y pulsa **Usar nombre**. No necesitas una cuenta Microsoft; al abrir por primera vez ya hay un jugador local llamado `Jugador`.
4. Pulsa **Jugar** en la tarjeta de la instancia. Si ya están instalados todos los archivos puedes jugar sin internet. Una cuenta local también puede descargar lo que falte cuando hay conexión; no hace falta un interruptor adicional.

El UUID local se genera a partir del nombre siguiendo el formato offline de Java. Mantén el mismo nombre para conservar tu identidad e inventario en los mundos y servidores que la usen.

El propio juego puede intentar acceder a servicios de red; no se modifican sus archivos para impedirlo. Los servidores que exigen autenticación, Realms y otras funciones en línea necesitan una sesión válida y conexión.

## Perfiles y mods

La pantalla **Inicio** reúne accesos rápidos y una biblioteca de tarjetas. El botón de cuadrícula del menú lateral abre la **Biblioteca** completa. Puedes buscar por nombre, versión o cargador, filtrar favoritos y ordenar por actividad, nombre o Minecraft.

Cada tarjeta permite jugar directamente con su botón verde o con doble clic. Un clic abre la página de la instancia con Contenido, Archivos, Mundos, Registros y Configuración. El menú de la tarjeta permite abrir sus carpetas, marcarla como favorita, renombrarla y personalizar su portada. Los iconos laterales también dan acceso a tus primeras instancias.

En **Nueva instancia** puedes crear instalaciones **Vanilla, Fabric, NeoForge y Forge**:

1. Escribe el nombre, elige el cargador y la versión de Minecraft. El nombre se completa automáticamente si lo dejas vacío.
2. Las versiones compatibles del cargador se consultan automáticamente. Puedes usar la **Última compatible** o activar **Elegir versión**.
3. Personaliza la portada con **Generar otra**, una paleta de colores o **Subir PNG**. Las imágenes se copian a la carpeta de datos de Luma y se ajustan al formato de la tarjeta.
4. Pulsa **Crear instancia**. Luma descarga el juego, Java y las bibliotecas; para Forge y NeoForge ejecuta el instalador oficial de cliente, incluidos sus parches.
5. Pulsa **Jugar** en su tarjeta. **Mods** o **Abrir carpeta de mods** abre la carpeta donde colocar los `.jar` compatibles con ese Minecraft y ese cargador. Fabric API se añade como un mod cuando lo requieran los mods que uses.

Cada perfil creado tiene su propia carpeta de juego: `mods`, `config`, `saves`, opciones y capturas. Puedes crear varios perfiles para el mismo Minecraft y cargador sin mezclar sus mundos o mods. Java, las bibliotecas y los recursos descargados se comparten. **Renombrar** cambia el nombre mostrado sin mover archivos.

También puedes usar **Importar → Instalador Forge / NeoForge .jar…** para seleccionar un instalador ya descargado. La versión de Minecraft se obtiene del propio instalador. Los instaladores antiguos de Forge con `versionInfo` se importan con su biblioteca universal; los modernos ejecutan el instalador oficial.

Luma revisa cada diez segundos sus clientes y los de `%APPDATA%\.minecraft`. Los perfiles de `launcher_profiles.json` y los clientes de cargadores en `versions/` aparecen automáticamente. **Importar → Detectar instalaciones** fuerza la revisión y **Desde carpeta .minecraft…** añade instalaciones de otra carpeta. Las instalaciones importadas conservan su ubicación; cuando el launcher de origen declara una carpeta de juego, Luma la respeta para usar los mods y mundos existentes. Las partidas de la antigua selección rápida Vanilla se muestran como instancias anteriores, conservando sus archivos en su ubicación original.

Si ejecutas un instalador externo, utiliza **Importar → Carpeta para instaladores** y selecciona esa ruta como destino en el instalador. Cuando termine, el cliente aparecerá en la biblioteca. La opción prepara el archivo de perfiles que esperan los instaladores.

Jugar sin conexión requiere haber completado primero la instalación; Luma utiliza automáticamente los archivos disponibles. Crear o instalar un cargador necesita conexión. Las versiones compatibles dependen de lo que publique cada proyecto; el launcher muestra un aviso cuando no existe un cargador para el Minecraft elegido.

### Cuenta Microsoft (opcional)

La importación del launcher oficial lee únicamente `name` e `id` de los perfiles de `%APPDATA%\.minecraft\launcher_accounts.json`. No importa tokens, no cambia ese archivo y habilita únicamente el uso offline. Si no aparece tu perfil, inicia sesión primero en el launcher oficial y vuelve a abrir Luma.

El acceso directo ahora abre automáticamente el navegador con OAuth y PKCE: pulsa **Continuar con Microsoft**, elige tu cuenta y vuelve a Luma. Si ya tienes una sesión en ese navegador, Microsoft puede aprovecharla. No se pide copiar un código ni escribir la contraseña dentro de Luma. La cuenta de Windows no se importa automáticamente.

**Configuración del desarrollador, una sola vez por launcher:** Luma necesita un registro de aplicación de Microsoft. Cada jugador utiliza después su propia cuenta, sin crear una aplicación ni introducir un Client ID.

1. Registra una aplicación que admita **cuentas Microsoft personales** en Microsoft Entra.
2. En Authentication, añade la plataforma **Mobile and desktop applications** con la URI de retorno `http://localhost`. Habilita los flujos de cliente público si lo requiere la configuración del registro. No crees un secreto de cliente para distribuirlo en el ejecutable.
3. Incluye su **Application (client) ID** en la compilación:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\build.ps1 -MicrosoftClientId "ID-DE-LA-APLICACION"
```

El ID es público y se integra en el ejecutable mediante `LUMA_MICROSOFT_CLIENT_ID`. Para desarrollo también puede introducirse en **Ajustes → Cuentas → Registro de Luma**. El botón queda deshabilitado y se explica el motivo si esta compilación no tiene ID configurado. La copia entregada no incluye un registro real: falta registrar Luma para activar el acceso Microsoft.

Las APIs de Minecraft pueden requerir aprobación de la aplicación por parte de Microsoft/Mojang; crear un Client ID no garantiza acceso. No se incluye un ID de otro launcher. El perfil local funciona sin configurar esta integración. Una cuenta con Minecraft Java y un perfil creado es necesaria para la autenticación directa.

Los tokens permanecen en memoria y se descartan al cerrar Luma. Para jugar en línea después de reiniciarlo hay que volver a conectar la cuenta; el perfil offline sí se conserva. El launcher no concede una licencia del juego.

## Compilar

Requisitos recomendados: Rust estable y **Visual Studio Build Tools con Desktop development with C++**, Windows SDK incluido.

```powershell
cargo build --release --locked
.\target\release\luma-launcher.exe
```

También puedes ejecutar:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\build.ps1
```

El script produce `artifacts/Luma.exe`, la copia con versión, `artifacts/Luma-Setup.exe` y `artifacts/luma-release.json`. Los releases públicos se compilan con MSVC y el runtime de C y el cargador WebView2 enlazados estáticamente. En desarrollo con GNU se copia también `WebView2Loader.dll`: conserva ese archivo junto al ejecutable local. Si existe `.tools/mingw64/bin/gcc.exe` y está instalado el toolchain Rust `stable-x86_64-pc-windows-gnu`, utiliza ese compilador portátil. En esta carpeta se preparó esa alternativa para no necesitar Visual Studio. `.tools` no forma parte del código fuente ni del ejecutable distribuido.

```powershell
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

Con la alternativa GNU, usa `cargo +stable-x86_64-pc-windows-gnu` y añade `.tools/mingw64/bin` al PATH del proceso, igual que hace `build.ps1`.

## Verificación sin interfaz

```powershell
.\artifacts\Luma.exe --install 1.7.10
.\artifacts\Luma.exe --verify 1.7.10
.\artifacts\Luma.exe --play-local 1.7.10 MiNombre
.\artifacts\Luma.exe --create-profile "Mi Fabric" 1.21.1 fabric
.\artifacts\Luma.exe --list-profiles
.\artifacts\Luma.exe --verify-profile ID_DEL_PERFIL
.\artifacts\Luma.exe --play-profile ID_DEL_PERFIL MiNombre
.\artifacts\Luma.exe --import-profiles "D:\Juegos\.minecraft"
```

`--install` descarga e instala una versión oficial. `--verify` comprueba todos sus archivos localmente y falla si falta alguno; no realiza descargas ni inicia el juego. `--play-local` abre una versión instalada con el nombre indicado y sin cuenta. En la compilación de desarrollo se muestra también la salida en consola.

## Datos y funcionamiento

Los archivos se guardan en `%LOCALAPPDATA%\Luma\LumaLauncher\data`, accesibles desde **Ajustes → Abrir carpeta**. La ruta mostrada por la aplicación es la referencia definitiva.

- `versions/`: metadatos, cliente, bibliotecas nativas y marca de instalación.
- `libraries/` y `assets/`: archivos compartidos, verificados mediante SHA-1 cuando los metadatos proporcionan una suma de verificación.
- `runtimes/`: Eclipse Temurin para la versión de Java indicada por Mojang, verificado con SHA-256 de Adoptium.
- `instances/<versión>/`: partidas de la selección rápida Vanilla anterior.
- `instances/<id del perfil>/`: mods, mundos y configuración de cada perfil creado.
- `profiles.json`: instalaciones registradas y sus carpetas; la identidad del jugador se guarda por separado en `settings.json`.
- `artwork/`: portadas PNG de las instancias. Los colores, favoritos y actividad reciente se guardan en `settings.json` por identificador de instancia.
- `installers/` y `logs/installer-<id>.log`: instaladores oficiales y su salida de diagnóstico.
- `logs/game-latest.log`: salida del último proceso de Minecraft.
- `manifest.json` y `settings.json`: catálogo guardado, preferencias y perfil offline; sin contraseñas ni tokens de sesión.

Las descargas se realizan en ocho tareas como máximo y se escriben primero en archivos temporales. Un archivo terminado y válido se reutiliza; una descarga incompleta se vuelve a descargar. Se filtran reglas de Windows, arquitectura, características y argumentos de la versión, y se reconstruyen los recursos con nombres que necesitan los clientes antiguos.

El catálogo abarca las versiones que **Mojang publique con un cliente descargable**. Los perfiles admiten Vanilla, Fabric, NeoForge y Forge; la descarga de paquetes de mods completos y Bedrock no están incluidos. Algunas versiones históricas pueden tener incompatibilidades con drivers o Windows actuales. Tener un lanzamiento en el catálogo no garantiza que todas las versiones se hayan probado jugando en este equipo.

## Fuentes técnicas

- [Catálogo oficial de versiones](https://piston-meta.mojang.com/mc/game/version_manifest_v2.json)
- [Flujo OAuth de autorización con PKCE de Microsoft](https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow)
- [Registro de aplicaciones de escritorio](https://learn.microsoft.com/en-us/entra/identity-platform/scenario-desktop-app-registration)
- [API de Eclipse Adoptium](https://api.adoptium.net/)
- [eframe](https://docs.rs/eframe/0.31.1/eframe/)
- [API oficial de Fabric](https://github.com/FabricMC/fabric-meta)
- [Instalación de clientes NeoForge](https://docs.neoforged.net/user/docs/client/)
- [Instalador oficial de Forge](https://github.com/MinecraftForge/Installer)

Proyecto independiente, sin afiliación con Mojang o Microsoft. Minecraft es una marca de sus respectivos titulares.

## Gestión de instancias

En **Contenido**, carga mods, resource packs, shaders o data packs ZIP. Para estos últimos elige el mundo de destino. El interruptor de resource packs modifica la selección en `options.txt`; mods y archivos ZIP desactivados se conservan con `.disabled`. Los packs en carpetas se guardan temporalmente fuera del directorio que examina el juego. Los shaders quedan disponibles o deshabilitados; su selección concreta se realiza con el mod de shaders dentro de Minecraft. No se cambia contenido mientras el juego está en ejecución.

**Quitar contenido** abre una confirmación y elimina permanentemente los archivos seleccionados. Antes puedes crear un respaldo que duplica la instancia completa. No se crea `.luma/removed`; los respaldos antiguos no se borran automáticamente. **Eliminar de la biblioteca** retira su registro y conserva los archivos del juego; **Eliminar completamente…** permite borrar la carpeta de juego, incluidos sus mundos, de una instancia independiente tras confirmar. **Duplicar** copia la carpeta de juego y mantiene bibliotecas y recursos compartidos.

Los registros del proceso están en `<instancia>/.luma/logs/game-latest.log`; los que genera Minecraft siguen en `logs/`. La página Registros admite búsqueda, filtros de nivel y archivos históricos `.log`/`.txt`. Se mantiene en pantalla una cola de hasta 512 KiB para no cargar registros enormes.

`LUMA_DATA_DIR` permite usar una carpeta de datos independiente para pruebas o instalaciones separadas. No modifica la ubicación habitual si no está definido.
