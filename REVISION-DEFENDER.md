# Instalador 0.7.0: investigación de Defender

24/10/2026. Estado: detección confirmada, causa y clasificación pendientes de revisión. No hay un instalador nuevo validado. Este problema es distinto de SmartScreen.

## Evidencia
- Microsoft Defender: `Trojan:Win32/Bearfoos.A!ml`, ThreatID `2147731250`, al descargar y ejecutar `Luma-Setup.exe` desde el release de GitHub. Motor local `1.1.26090.9`, firmas `1.461.186.0`. Protección en tiempo real activada al consultar.
- SHA-256 publicado del Setup: `bbbcc77eeea9b1152704f21b2745d2e68b7652a07fc438e45fae46cc1da41d9c`.
- Compilación pública MSVC: Actions 40164781752, fuente `f76a99c3027683ae5cdee221c03313edd638a887`. El release no se firmó; solo existe soporte opcional para firma. No se aplicó certificado autofirmado.
- Se añadió un aviso visible al inicio de las notas del release 0.7.0. No se han desactivado protecciones, creado exclusiones ni restaurado cuarentena.

## Revisión de implementación y límites
El instalador propio descarga un EXE, lanza PowerShell con scripts codificados para instalar/desinstalar y elevar permisos, se copia a temporales para poder desinstalarse y programa su propia retirada. Estas funciones están en `src/distribution.rs`, `src/cleanup.rs` y `src/bin/luma-setup.rs`. Son comportamientos que merecen revisión; no se ha demostrado cuál motivó la detección. Una firma de editor no demuestra que la detección sea errónea ni corrige automáticamente una alerta de malware.

No basta con recompilar hasta obtener un hash no detectado. Hay que revisar el diseño, comprobar la procedencia de la compilación y obtener un dictamen sobre la muestra detectada. Para una distribución posterior conviene evaluar un instalador estándar (MSI/Inno Setup) o un ayudante nativo de instalación con acciones limitadas, reduciendo el uso de comandos genéricos, y probar instalación, actualización y desinstalación en una VM limpia con Defender activo.

## Siguiente trabajo
1. Enviar la muestra original a [Microsoft Security Intelligence](https://www.microsoft.com/en-us/wdsi/filesubmission) como proveedor de software, con su hash y los datos siguientes. **Todavía no se ha enviado**. Registrar ID del caso y respuesta; no afirmar falso positivo antes del resultado.
2. Revisar todas las acciones privilegiadas, descarga de EXE, scripts de eliminación y dependencias de la compilación. Hasta ahora no se ha completado una auditoría independiente ni una revisión de cadena de suministro.
3. Elegir y desarrollar la sustitución del instalador si procede, en una rama nueva. Conservar instalación en Program Files, datos por usuario, acceso directo opcional y desinstalación explícita de datos; sin JAR ni perfiles precargados.
4. Validar con Defender activo en un Windows limpio, además del equipo LTSC. No reutilizar una excepción o permiso de amenaza previamente creado por el usuario como evidencia de que está limpio.
5. Solo después publicar un reemplazo y actualizar las instrucciones públicas. Para firmar se necesita una identidad de editor validada; sigue pendiente.

## Texto preparado para Microsoft

Product: Microsoft Defender Antivirus (Windows 11)

Detection: Trojan:Win32/Bearfoos.A!ml

SHA256: bbbcc77eeea9b1152704f21b2745d2e68b7652a07fc438e45fae46cc1da41d9c

We maintain Luma, an open-source Minecraft launcher written in Rust. Microsoft Defender detects our 0.7.0 installer on download and execution. Please analyze the original sample and confirm whether the detection is correct. We are not treating it as a confirmed false positive. The unsigned installer downloads our application from GitHub Releases, checks SHA-256, requests UAC to copy application files into Program Files, creates a per-user uninstall entry and optional shortcut, and can remove user-confirmed application data. It currently uses encoded PowerShell commands and a temporary copy for self-uninstallation. Source: https://github.com/Fernanboom34/Launcher/tree/f76a99c3027683ae5cdee221c03313edd638a887 . Build: https://github.com/Fernanboom34/Launcher/actions/runs/40164781752 . Release: https://github.com/Fernanboom34/Launcher/releases/tag/v0.7.0 . We have warned users and have not advised bypassing Defender or disabling protection.
