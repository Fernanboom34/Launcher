# Luma 0.8.0-beta.3 — Windows y Linux x64

- Corregida la proyección de la vista 3D de skins: las caras superiores, frontales y las capas usan la misma orientación de cámara. Se mantiene la detección de brazos clásicos/delgados y la normalización de skins antiguas ya incluida en beta 2.
- Verificado el recorrido PNG → importación → archivo de CustomSkinLoader: los píxeles se conservan, incluidas las caras asimétricas de los brazos/piernas normalizados desde 64×32. Esta corrección visual no reescribe las skins guardadas.
- Pestañas de Contenido/Archivos/Mundos/Registros y filtros con selección redondeada. El encabezado y las filas seleccionadas de la tabla respetan su contenedor curvo, también al desplazarse.
- Se conservan las funciones de beta 2: skins locales, metadatos de proyectos, selección múltiple, configuración y control independiente de instancias.

## Descargar e instalar

Windows: `Luma-0.8.0-beta.3.exe` o el instalador nativo `Luma-Setup.exe`. Linux x64: ejecutable, `.deb` o `.tar.gz` con `install.sh`. Las sumas de Windows están en `SHA256SUMS.txt`; las de Linux, en `SHA256SUMS-linux-x64.txt`.

```sh
sudo apt install ./luma-launcher_0.8.0-beta.3_amd64.deb
```

La retirada de los paquetes Linux conserva los datos del jugador. En Linux las webs se abren en el navegador del sistema; el catálogo Modrinth sigue integrado. No se incluyen JAR, Java ni instancias precargadas.

La publicación exige formato, Clippy estricto, pruebas de ambos sistemas, instalación/desinstalación nativa Windows con datos desechables, pruebas de los paquetes Linux y capturas de la interfaz. Las capturas emplean fixtures identificadas, sin datos del usuario. No equivale a comprobar una cuenta Microsoft real, todas las distribuciones Linux ni todas las versiones del juego. Windows mantiene sus advertencias existentes de editor sin firma pública. No se modifica la protección del sistema.

Esta versión es una pre-release; no sustituye el canal estable ni integra la beta en `main`.
