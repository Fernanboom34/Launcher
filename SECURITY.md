# Seguridad de Luma

## Versiones y alcance

La línea 0.8.0 sigue siendo candidata a release. Usa los builds publicados más recientes; no hay mantenimiento de betas antiguas. Los PR de seguridad deben aprobar las comprobaciones nativas de Windows y Linux antes de integrarse.

## Reportar una vulnerabilidad

Usa «Report a vulnerability» en la pestaña Security del repositorio cuando esté disponible: https://github.com/Fernanboom34/Launcher/security/advisories/new . No publiques claves, tokens, datos de jugadores ni un exploit operativo en un issue público. Si GitHub no ofrece el reporte privado, solicita un canal privado al mantenedor sin incluir detalles sensibles. No se promete un plazo de respuesta que el proyecto no haya confirmado.

## Modelo de amenazas

Mods JAR, cargadores e instaladores ejecutan código con los permisos del jugador; las validaciones de ZIP y hashes no son un antivirus ni un sandbox. Consulta SECURITY-REVIEW.md para las limitaciones conocidas.

Los artefactos publicados no deben sustituirse. Una corrección requiere versión y tag nuevos. Los builds de release no restauran artefactos target/ desde caché, y los permisos de publicación están limitados al job que publica.

La autenticidad criptográfica de las actualizaciones requiere claves de firma y un editor confiable configurados fuera de una ejecución normal de CI. No confundir HTTPS ni SHA-256 con una firma del editor.
