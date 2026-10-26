# Luma 0.8.0-rc.1 — Windows y Linux x64

- Instancias sin contenido: se oculta la tabla y se muestra el personaje amarillo con fondo transparente, mensaje y botón según el idioma configurado (es-MX o en-US). Las búsquedas sin resultados muestran un mensaje diferente.
- Tabla de contenido continua con filas de altura uniforme (58 px), iconos de 34 px, tonos alternos, casillas individuales visibles y cierre inferior redondeado. Seleccionar conserva el color de la fila. Papelera en lugar de X.
- Panel lateral simplificado y ancho automático; el selector de cuenta se adapta al espacio disponible.
- Referencia a Loss en la esquina inferior derecha del launcher. Mientras hay una instancia en ejecución se abre una pequeña superposición del escritorio, sin decoración, sin tomar el foco y dejando pasar los clics. Se retira al terminar la instancia o cerrar Luma. No modifica Minecraft ni instala mods.
- Validación estructural compartida para archivos del catálogo, navegador e importación local: rutas peligrosas, enlaces, nombres duplicados, CRC y límites de descompresión. Descargas temporales aleatorias con creación exclusiva, HTTPS en redirecciones, y límites de tamaño.
- Extracción ZIP con rechazo de destinos enlazados y límites de entradas y bytes. Actualización de zip a 9.1.2 para retirar tres avisos de seguridad. Detalles y riesgos pendientes en SECURITY-REVIEW.md.

## Descargas

Windows: `Luma-0.8.0-rc.1.exe` o `Luma-Setup.exe`. Linux x64 con glibc: ejecutable, `.deb` o `.tar.gz` con `install.sh`. Las sumas de comprobación acompañan cada plataforma. Los instaladores no incluyen Minecraft, Java ni instancias.

## Límites de este candidato

La superposición depende del compositor: no se garantiza visibilidad sobre pantalla completa exclusiva ni en todos los escritorios Wayland; para esos casos usar Minecraft en ventana o pantalla completa sin bordes. Su ubicación corresponde al monitor del launcher. No se afirma compatibilidad universal ni una prueba manual de la superposición sobre un juego real en Windows.

Las validaciones no son un antivirus: un JAR válido puede ejecutar código malicioso con los permisos del jugador. No hay aislamiento de Minecraft ni protección completa contra carreras de otro proceso local durante escrituras. Persisten avisos de mantenimiento en dependencias y un aviso de glib en el grafo de targets no activado en la compilación Linux actual; véase el informe.

La publicación exige formato, Clippy, pruebas y paquetes de Windows y Linux, incluyendo instalación/desinstalación nativa Windows con datos desechables. Tres pruebas de integración de servicios externos siguen siendo opcionales. Este candidato a release no sustituye el canal estable ni modifica main. Windows conserva sus advertencias de editor sin firma pública.
