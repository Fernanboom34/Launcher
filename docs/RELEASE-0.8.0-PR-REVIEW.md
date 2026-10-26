# Revisión de PR para v0.8.0

- #20: ya fusionado por el propietario; conserva la nueva interfaz, eliminación confirmada, colores y luna.
- #9 y #10: fusionados, actualización compatible de carga/descarga de artefactos. Windows, Linux y análisis aprobados.
- #12: válido, pero su rama quedó en conflicto al actualizar los artefactos. Se incorpora su pin de checkout 7.0.1 en la preparación conjunta, sin perder los cambios ya integrados.
- #15 y #17: actualizaciones compatibles de md-5 0.11 y base64 0.23. Se incorporan juntas en la preparación y se vuelve a probar el código combinado.
- #11 (sha2), #13 (getrandom), #14 (eframe), #16 (windows-core): cerrados y ramas retiradas. Sus cambios de API rompen las compilaciones; no se necesitan para resolver un aviso de seguridad actual y se posponen sus migraciones para después de estabilizar 0.8.0.
- #18 y #19: cerrados y sustituidos. Actualizar init/analyze por separado produce el error de configuración observado. Esta preparación actualiza ambos a CodeQL 4.39.1, con el mismo SHA, y configura Dependabot para agruparlos.
- #21: promoción anterior de RC a main; queda sustituida por la promoción de la preparación completa de v0.8.0 cuando terminen las comprobaciones.

Las ramas descartadas se eliminan; los PR se cierran conservando su historial de revisión. Los PR válidos sustituidos se cierran con explicación después de incorporar sus cambios. No se integra una migración de API que falla ni se ignora un aviso de seguridad para publicar.
