# Firmar y publicar en Windows

La compilación de GitHub prepara un **borrador**, no una versión pública. El último paso se realiza en tu computadora con la clave privada PEM. No compartas la clave ni la copies dentro del repositorio.

## Preparación (una sola vez)

1. Instala [Python](https://www.python.org/downloads/windows/), [Git para Windows](https://git-scm.com/downloads/win) (incluye OpenSSL) y [GitHub CLI](https://cli.github.com/). Cierra y abre PowerShell después de instalar.
2. En PowerShell ejecuta `gh auth login`, selecciona GitHub.com, HTTPS y acceso mediante navegador. Usa tu cuenta con permiso para publicar releases de Luma.
3. Descarga el código actual de `main` desde **Code → Download ZIP** y extrae el ZIP. También puedes usar Git para clonar el repositorio.
4. Guarda tu clave, por ejemplo, en `C:\Claves-Luma\actualizaciones.pem`, fuera de esa carpeta de código. Conserva una copia de seguridad privada.

## Para 0.8.1

1. Comprueba en GitHub Actions que las validaciones Windows, Linux, RustSec y CodeQL del release terminaron correctamente. Revisa los cambios y los artefactos que vas a autorizar: firmar sin revisarlos no protege de un build malicioso.
2. Abre la carpeta extraída del repositorio en el Explorador. Haz clic en la barra donde aparece la ruta, escribe `powershell` y pulsa Enter. Así abres PowerShell en la carpeta que contiene `Cargo.toml` y `scripts`.
3. Copia este comando y sustituye la ruta entre comillas por la ubicación real de **tu** clave:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\publish-signed-release.ps1 -PrivateKey "C:\Claves-Luma\actualizaciones.pem" -Version "0.8.1"
```

El permiso de ejecución se aplica solo a ese proceso; no cambia la política permanente del equipo. Si el PEM está cifrado, OpenSSL puede pedir su contraseña en tu terminal; no la escribas en el comando ni en GitHub.

El script descarga el launcher Windows y Linux y sus manifiestos desde el borrador. Comprueba sus hashes, la versión, las URLs y que la clave privada corresponde a la pública de Luma. Firma los dos manifiestos, actualiza la suma del manifiesto Linux, sube **solo los JSON firmados y las sumas Linux** y publica 0.8.1 como estable. Nunca sube la clave privada. No modifica los ejecutables ni permite reemplazar assets de un release ya publicado.

Si algo falla, conserva el mensaje de error, sin incluir claves ni contraseñas. El script no publica si falla una firma. Si la subida queda incompleta, se puede repetir mientras el release siga siendo borrador.

Para releases posteriores usa el código revisado correspondiente y cambia el número de versión. La primera instalación de 0.8.1 desde 0.8.0 es manual porque 0.8.0 no incorpora la clave pública.
