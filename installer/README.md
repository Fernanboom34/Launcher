# Instalador Windows: beta para comparar

`Luma.iss` utiliza Inno Setup y se distribuye para pruebas como pre-release, sin reemplazar el estable. La beta fija su payload a `v0.7.1-beta.2-windows` para que la comparacion sea reproducible; no consulta el estable 0.7.0.

La copia de archivos, UAC, accesos directos, registro y retirada de archivos de programa pasan al motor estandar de Inno Setup. El script no ejecuta PowerShell ni comandos codificados. Descarga el manifiesto de su beta y el EXE indicado, limita el manifiesto, valida version y URL exacta de GitHub y verifica SHA-256 antes de instalar. No incluye JAR ni perfiles.

El primer proceso del Setup conserva el contexto del usuario y muestra instalar/actualizar o desinstalar. Solo el motor `/ALLUSERS /LUMA_ENGINE=1` recibe elevacion para Program Files. Al terminar la instalacion se retira la antigua entrada HKCU desde el proceso original. Para migrar desde 0.7.0, primero instala esta actualizacion conservando los datos.

Al desinstalar desde el Setup se abre `Luma.exe --manage-uninstall-data` sin pedir elevacion: muestra las rutas, permite conservarlas o borrar con confirmacion e inclusion explicita de carpetas compartidas. Reutiliza las comprobaciones de procesos, limites de rutas y borrado de enlaces de `cleanup.rs`; Program Files queda fuera de ese plan. Despues se invoca el desinstalador nativo para retirar la aplicacion. Desde Aplicaciones de Windows, el desinstalador nativo conserva los datos. El archivo descargado del Setup permanece en Descargas.

Verificacion: [Actions 40173302914](https://github.com/Fernanboom34/Launcher/actions/runs/40173302914) completo formato, Clippy, pruebas Rust, build MSVC, compilacion Inno y ciclo de instalacion/desinstalacion en una carpeta temporal. Se verifico el hash instalado, migracion de un Setup viejo de fixture y conservacion de un marcador externo. La variante fixture contiene solo el EXE de prueba para ese ciclo, no se publica. El Setup de produccion descarga el app desde internet.

Limites y pruebas pendientes:

- Revisar el actualizador de Luma para no seguir usando comandos genericos de PowerShell en Program Files. El nuevo Setup no cambia el actualizador del EXE.
- Incorporar metadatos de version/icono/firma publica cuando exista una identidad valida.
- Probar instalacion limpia, actualizacion y desinstalacion en VM con Defender activo, ademas del Windows LTSC del usuario. No afirmar que el cambio elimina una deteccion sin comprobarlo.
- El build Rust aun genera el binario legado; el workflow de verificacion con include_installer=true lo reemplaza por el Inno probado y recalcula SHA256SUMS. No publicar el Setup Rust generado por build.ps1 sin esta fase.
- Verificar manualmente el menu del bootstrap, UAC con otra cuenta administradora y el flujo visual de confirmacion de datos. No se borro ningun mundo ni instalacion real del usuario durante las comprobaciones.

Documentacion oficial utilizada: [DownloadTemporaryFile](https://jrsoftware.org/ishelp/topic_isxfunc_downloadtemporaryfile.htm). Las descargas HTTPS verifican certificados y el parametro SHA-256 rechaza contenido que no coincida. Las operaciones con datos de usuario se deben disenar teniendo en cuenta que [ExecAsOriginalUser no esta disponible durante la desinstalacion](https://jrsoftware.org/ishelp/topic_isxfunc_execasoriginaluser.htm).
