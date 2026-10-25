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
