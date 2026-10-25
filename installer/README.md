# Instalador Windows: beta para comparar

`Luma.iss` utiliza Inno Setup y se distribuye para pruebas como pre-release, sin reemplazar el estable. La beta fija su payload a `v0.8.0-beta.1-windows` para que la comparacion sea reproducible; no consulta el estable 0.7.0.

La copia de archivos, UAC, accesos directos, registro y retirada de archivos de programa pasan al motor estandar de Inno Setup. El script no ejecuta PowerShell ni comandos codificados. Descarga el manifiesto de su beta y el EXE indicado, limita el manifiesto, valida version y URL exacta de GitHub y verifica SHA-256 antes de instalar. No incluye JAR ni perfiles.

Inno solicita sus permisos habituales para Program Files y muestra instalar/actualizar o desinstalar. Se elimino el relanzamiento de Setup desde InitializeSetup, que Inno no admite. Las operaciones con datos se lanzan con ExecAsOriginalUser. No hay que instalar una actualizacion antes de desinstalar una copia portable o anterior a Inno.

Al desinstalar, Setup descarga un helper temporal del mismo release, valida su SHA256 y lo ejecuta como el usuario original con `--manage-uninstall-data`. Permite conservar mundos, mods, cuentas y ajustes o eliminarlos con confirmacion; retira las copias registradas del launcher y sus accesos directos en ambos casos. Reutiliza comprobaciones de procesos y limites de rutas de `cleanup.rs`. Despues se invoca el desinstalador nativo, si existe; para instalaciones legadas solo se retiran archivos de programa de nombres conocidos, conservando contenido desconocido. Desde Aplicaciones de Windows, el desinstalador nativo conserva los datos. El Setup descargado permanece en Descargas. Este flujo necesita conexion para obtener el helper.

Verificacion: [Actions 40173302914](https://github.com/Fernanboom34/Launcher/actions/runs/40173302914) completo formato, Clippy, pruebas Rust, build MSVC, compilacion Inno y ciclo de instalacion/desinstalacion en una carpeta temporal. Se verifico el hash instalado, migracion de un Setup viejo de fixture y conservacion de un marcador externo. La variante fixture contiene solo el EXE de prueba para ese ciclo, no se publica. El Setup de produccion descarga el app desde internet.

Beta.3: [Actions 40221754955](https://github.com/Fernanboom34/Launcher/actions/runs/40221754955) pasa 52 pruebas regulares y el ciclo nativo por la entrada normal del instalador. Tambien comprueba la retirada legada preservando un archivo desconocido. El EXE MSVC final guarda el mismo Fresh Animations del reporte con SHA1 correcto en una instancia desechable mediante scripts/check-web-download.py.

Limites y pruebas pendientes:

- Revisar el actualizador de Luma para no seguir usando comandos genericos de PowerShell en Program Files. El nuevo Setup no cambia el actualizador del EXE.
- Incorporar metadatos de version/icono/firma publica cuando exista una identidad valida.
- Probar instalacion limpia, actualizacion y desinstalacion en VM con Defender activo, ademas del Windows LTSC del usuario. No afirmar que el cambio elimina una deteccion sin comprobarlo.
- El build Rust aun genera el binario legado; el workflow de verificacion con include_installer=true lo reemplaza por el Inno probado y recalcula SHA256SUMS. No publicar el Setup Rust generado por build.ps1 sin esta fase.
- Verificar manualmente el menu normal, UAC con otra cuenta administradora y el flujo visual de confirmacion de datos. No se borro ningun mundo ni instalacion real del usuario durante las comprobaciones.

Documentacion oficial utilizada: [DownloadTemporaryFile](https://jrsoftware.org/ishelp/topic_isxfunc_downloadtemporaryfile.htm). Las descargas HTTPS verifican certificados y el parametro SHA-256 rechaza contenido que no coincida. Las operaciones con datos de usuario se deben disenar teniendo en cuenta que [ExecAsOriginalUser no esta disponible durante la desinstalacion](https://jrsoftware.org/ishelp/topic_isxfunc_execasoriginaluser.htm).
