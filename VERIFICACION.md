# Verificación de Luma

Realizada el 24 de octubre de 2026, en Windows x64 con Rust 1.99.1 y el toolchain GNU portátil.

- Compilación optimizada para Windows y análisis `cargo clippy --all-targets --locked -- -D warnings` sin advertencias.
- Diecinueve pruebas automatizadas: las comprobaciones del juego, jugador local y OAuth, herencia de versiones, bibliotecas Maven, rechazo de herencia circular, detección del cargador, separación y renombrado de perfiles, importación sin duplicados y conservación de las partidas de la antigua selección rápida Vanilla.
- Instalación real de **1.7.10**, incluido Java 8, seguida de verificación local de sus 709 archivos.
- Instalación real de **26.3**, incluido Java 25, seguida de verificación local de sus 5.223 archivos.
- Rediseño 0.5.0: biblioteca de tarjetas, accesos rápidos, menú lateral compacto, panel contextual, filtros, favoritos y creación de instancias en una ventana modal. Las capturas de revisión se guardan en `artifacts/redesign-*.png`; el modo de captura no guarda preferencias ni cambia perfiles del usuario.
- El ejecutable utiliza DLL de Windows; no necesita los DLL del compilador MinGW junto al archivo.
- Arranque real de Minecraft **26.3** con el perfil local `LumaSmoke`, sin cuenta: inicializó el renderizador, el audio y los atlas de texturas. Después se cerró normalmente con código 0. No se creó un mundo durante esta prueba.
- Creación de perfiles independientes de **Vanilla 26.3**, **Fabric 0.20.1 para 1.21.1**, **Forge 52.1.20 para 1.21.1** y **NeoForge 21.1.264 para 1.21.1**, cada uno con su propia carpeta de juego.
- Instalación adicional de Forge desde un archivo `.jar` local ya descargado, usando el instalador oficial y registrando el perfil nuevo en Luma.
- Arranque real de Fabric, Forge y NeoForge: cargaron el cargador, el audio y los atlas de texturas; se cerraron normalmente con código 0. Se verificó el cliente heredado con el nombre que requiere el cargador para evitar módulos duplicados en NeoForge. No se crearon mundos ni se probaron paquetes completos de mods.
- Verificación final en modo offline de los cinco perfiles de ejemplo. La detección posterior mantuvo esos cinco perfiles sin añadir duplicados.

No se probó la autenticación con una cuenta real: falta registrar Luma e incluir un Client ID autorizado. El nuevo flujo OAuth con navegador se probó localmente con una respuesta simulada; no equivale a una verificación con Microsoft/Xbox/Minecraft. El perfil local permite jugar sin esa integración. No se probaron jugando todas las versiones del catálogo. Las instalaciones de prueba quedan disponibles en la carpeta de datos del launcher.

Las comprobaciones de instalación y arranque anteriores corresponden a la integración 0.4.0. El rediseño 0.5.0 conserva ese motor; se compiló, pasó las 19 pruebas y se revisó mediante capturas a 1360 × 880 y 1060 × 720, incluyendo la instancia seleccionada y el formulario de creación.


## Integración 0.6.0

- 24 pruebas del launcher y una del instalador; Clippy de todos los targets sin advertencias. Se añadieron pruebas de activación de mods y resource packs sin pérdida de datos, packs en carpetas, lectura NBT y orden de mundos, conservación de memoria por instancia, duplicación y retirada de registros sin borrar mundos.
- Revisión visual de páginas de instancia, configuración y temas mediante capturas del framebuffer de la propia aplicación. Las capturas y archivos de prueba quedan únicamente en carpetas ignoradas por Git.
- El instalador y el actualizador consultan el release estable público en `Fernanboom34/Launcher`, validan el origen HTTPS y el SHA-256. La instalación se distribuye sin archivos JAR ni perfiles.
- Quick Play requiere compatibilidad del juego (Minecraft 1.20 o posterior). No se inyectan mods ni se modifican JAR para forzar esta función en versiones anteriores.
- La integración Microsoft sigue pendiente de un Client ID propio autorizado; las cuentas locales no dependen de ella.

- Arranque real 0.6.0 de NeoForge 21.1.264 / Minecraft 1.21.1 con el jugador local LumaV06QA: audio y atlas inicializados, registro por instancia y cierre normal con código 0. El controlador emitió mensajes de depuración OpenGL durante la inicialización. No se creó un mundo.
- Reparación real del mismo cliente: ejecución del instalador oficial, regeneración de dependencias y verificación posterior de 3.996 archivos. Se conservaron el identificador y la carpeta de la instancia.
- Revisión de instalación limpia con LUMA_DATA_DIR independiente: biblioteca de cero instancias, sin perfiles precargados. Revisión del tema claro con panel lateral y controles legibles.
- Release público v0.6.0 publicado con tres archivos. El instalador descargó Luma desde GitHub sin credenciales, terminó con código 0 y produjo únicamente Luma.exe; el SHA-256 coincidió con el manifiesto publicado. No se probó un reemplazo completo de una versión anterior mediante la interfaz del actualizador.


## Integración 0.6.1

- 31 pruebas del launcher y 7 del instalador. Se añadieron comprobaciones de eliminación completa de instancias, bloqueo de carpetas compartidas y generales del usuario, límites de rutas, eliminación de enlaces sin seguirlos y bloqueo de la desinstalación con aplicaciones abiertas.
- La eliminación se ejecutó únicamente en carpetas de prueba aisladas. Se comprobó que archivos vecinos y destinos de enlaces permanecen intactos. No se ejecutó una desinstalación sobre la instalación ni los mundos reales del usuario.
- El instalador incorpora listado de rutas, confirmación de borrado, selección explícita de carpetas externas compartidas y eliminación del propio instalador al cerrar. Se registra un acceso de desinstalación por usuario en Windows y se conserva el instalador en la carpeta de programas.
- Revisión visual del instalador en modo desinstalación y del diálogo de borrado de instancia: las rutas son legibles, las acciones destructivas requieren confirmación y los botones quedan deshabilitados antes de aceptarla. Las vistas se cerraron sin ejecutar borrados sobre los datos del usuario.


## Integración 0.7.0

- Eliminada la franja negra: la altura del contenido de la cabecera cabe dentro del panel. Captura revisada y píxeles bajo la cabecera con el color del fondo, sin filas negras.
- PowerShell devuelve mensajes UTF-8 simples y suprime el progreso serializado. La prueba reproduce una excepción con acentos y comprueba que no contiene CLIXML.
- Instalación predeterminada en Program Files con elevación restringida a los archivos de programa. Registro, cuentas y datos permanecen en el contexto del usuario. Se reconoce también la ubicación anterior de instalación para desinstalarla.
- Pruebas de destinos por tipo e instancia/mundo, nombres de archivo, dependencias compatibles, ciclos y conflictos de versiones.
- Prueba real de API Modrinth: búsqueda y versiones de Mod Menu para Fabric 1.21.1, instalación de tres archivos incluyendo dependencias, verificación de JAR, reinstalación sin duplicados y búsqueda de datapacks/versiones ZIP de Terralith.
- Prueba real de navegador WebView2 oculto: descarga HTTPS de Fabric API, traslado a la carpeta mods de la instancia de prueba y comprobación de SHA-1. Toda la prueba se realizó en una carpeta aislada.
- Revisión visual del catálogo y del panel sin el interruptor de red. Las ventanas de prueba usan datos aislados y no crean perfiles dentro del ejecutable distribuido.
- SmartScreen: el Setup local está NotSigned; el certificado encontrado en el almacén local es autofirmado. Se preparó firma opcional y documentación; no se instaló confianza artificial ni se desactivó protección de Windows. Una firma pública y la reputación del editor requieren configuración real adicional.
- Los scripts de copia y retirada usados para Program Files se ejecutaron sin elevación en un destino de prueba: validación de hashes antes de copiar, copia de ambos EXE y retirada sin recorrer una unión hacia una carpeta vecina. No se ejecutó una instalación elevada sobre Program Files en el equipo del usuario.
- 38 pruebas regulares del launcher y 10 del instalador pasaron; las dos comprobaciones en vivo se ejecutaron por separado y pasaron. Se preparó MSVC con CRT y cargador WebView2 estáticos para los releases públicos; las compilaciones locales GNU incluyen el DLL del cargador junto al ejecutable.
