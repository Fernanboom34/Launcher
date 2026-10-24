# Luma

Launcher nativo de **Minecraft Java Edition para Windows x64**, escrito en Rust con egui. Tema oscuro, acentos menta y un paisaje original dibujado con vectores, sin navegador integrado.

## Abrir

Ejecuta `artifacts/Luma.exe` si ya está compilado. No requiere instalar Rust para utilizarlo.

1. En **Inicio** o **Biblioteca**, pulsa **+ Nueva instancia**. Elige un nombre, el cargador y una versión de Minecraft. **Versiones** permite buscar un lanzamiento del catálogo y abrir la misma ventana de creación.
2. Pulsa **Crear instancia**. Se descargan el cliente, las bibliotecas, los recursos y el Java que requiere esa versión.
3. En el panel derecho, abre **Cambiar jugador**, escribe tu nombre y pulsa **Usar nombre**. No necesitas una cuenta Microsoft; al abrir por primera vez ya hay un jugador local llamado `Jugador`.
4. Pulsa **Jugar** en la tarjeta de la instancia. Activa **Modo sin conexión** en el panel derecho si ya tienes todos los archivos instalados y quieres que el launcher no descargue nada. Con la opción desactivada, un jugador local también puede jugar sin cuenta y descargar automáticamente lo que falte.

El UUID local se genera a partir del nombre siguiendo el formato offline de Java. Mantén el mismo nombre para conservar tu identidad e inventario en los mundos y servidores que la usen.

El propio juego puede intentar acceder a servicios de red; no se modifican sus archivos para impedirlo. Los servidores que exigen autenticación, Realms y otras funciones en línea necesitan una sesión válida y conexión.

## Perfiles y mods

La pantalla **Inicio** reúne accesos rápidos y una biblioteca de tarjetas. El botón de cuadrícula del menú lateral abre la **Biblioteca** completa. Puedes buscar por nombre, versión o cargador, filtrar favoritos y ordenar por actividad, nombre o Minecraft.

Cada tarjeta permite jugar directamente con su botón verde o con doble clic. Un clic selecciona la instancia y muestra sus opciones en el panel derecho. El menú de la tarjeta permite abrir sus carpetas, marcarla como favorita, renombrarla y personalizar su portada. Los iconos laterales también dan acceso a tus primeras instancias.

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

La opción **Modo sin conexión** requiere haber completado primero la instalación. Crear o instalar un cargador necesita conexión. Las versiones compatibles dependen de lo que publique cada proyecto; el launcher muestra un aviso cuando no existe un cargador para el Minecraft elegido.

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

El ID es público y se integra en el ejecutable mediante `LUMA_MICROSOFT_CLIENT_ID`. Para desarrollo también puede introducirse en **Ajustes → Configuración del desarrollador**. El botón queda deshabilitado y se explica el motivo si esta compilación no tiene ID configurado. La copia entregada no incluye un registro real: falta registrar Luma para activar el acceso Microsoft.

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

El script produce `artifacts/Luma.exe`. Si existe `.tools/mingw64/bin/gcc.exe` y está instalado el toolchain Rust `stable-x86_64-pc-windows-gnu`, utiliza ese compilador portátil. En esta carpeta se preparó esa alternativa para no necesitar Visual Studio. `.tools` no forma parte del código fuente ni del ejecutable distribuido.

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
