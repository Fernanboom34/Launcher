# Sustitucion del instalador: rama de trabajo

`Luma.iss` es un prototipo de Inno Setup. **No esta listo para distribuir** y no sustituye los archivos del release actual.

La instalacion de archivos, UAC, accesos directos, registro de desinstalacion y retirada de archivos de programa pasan al motor estandar de Inno Setup. El script no ejecuta PowerShell ni comandos codificados. Descarga el manifiesto del release estable y el EXE indicado, limita el manifiesto, valida version y URL exacta de GitHub y verifica SHA-256 antes de instalar. No incluye JAR ni perfiles. Bloquea releases anteriores a 0.7.1 para evitar instalar 0.7.0 por accidente al probarlo.

Pendiente antes de publicar:

- Compilacion Inno Setup y rechazo del release 0.7.0 comprobados en [Actions 40168081216](https://github.com/Fernanboom34/Launcher/actions/runs/40168081216). Esta comprobacion no equivale a una instalacion completa ni a una validacion de Defender en el equipo del usuario.
- Integrar la opcion de borrar datos del jugador, con confirmacion y limites de rutas, en el contexto del usuario correcto. Actualmente el desinstalador estandar retira solo los archivos de programa; no borra mundos ni cuentas.
- Migrar el registro de la instalacion antigua y la entrada HKCU de desinstalacion, evitando duplicados y conservando los datos.
- Revisar el actualizador de Luma para no seguir usando comandos genericos de PowerShell en Program Files. El nuevo Setup no cambia el actualizador del EXE.
- Incorporar metadatos de version/icono/firma publica cuando exista una identidad valida.
- Probar instalacion limpia, actualizacion y desinstalacion en VM con Defender activo, ademas del Windows LTSC del usuario. No afirmar que el cambio elimina una deteccion sin comprobarlo.
- Cambiar build.ps1 y workflow para publicar este Setup SOLO al terminar las comprobaciones; de momento siguen compilando el Setup Rust anterior.

Documentacion oficial utilizada: [DownloadTemporaryFile](https://jrsoftware.org/ishelp/topic_isxfunc_downloadtemporaryfile.htm). Las descargas HTTPS verifican certificados y el parametro SHA-256 rechaza contenido que no coincida. Las operaciones con datos de usuario se deben disenar teniendo en cuenta que [ExecAsOriginalUser no esta disponible durante la desinstalacion](https://jrsoftware.org/ishelp/topic_isxfunc_execasoriginaluser.htm).
