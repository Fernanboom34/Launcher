# Verificación de Luma

Realizada el 24 de octubre de 2026, en Windows x64 con Rust 1.99.1 y el toolchain GNU portátil.

- Compilación optimizada para Windows y análisis `cargo clippy --all-targets --locked -- -D warnings` sin advertencias.
- Doce pruebas automatizadas: las ocho comprobaciones del juego y del perfil local, más el vector de referencia PKCE de RFC 7636, rechazo de respuestas OAuth falsas o ambiguas, recepción real de una respuesta simulada en localhost y cancelación del acceso.
- Instalación real de **1.7.10**, incluido Java 8, seguida de verificación local de sus 709 archivos.
- Instalación real de **26.3**, incluido Java 25, seguida de verificación local de sus 5.223 archivos.
- Revisión visual de Inicio, Versiones y Ajustes mediante capturas del framebuffer de la aplicación. El modo de captura guarda el PNG y cierra el proceso.
- El ejecutable utiliza DLL de Windows; no necesita los DLL del compilador MinGW junto al archivo.
- Arranque real de Minecraft **26.3** con el perfil local `LumaSmoke`, sin cuenta: inicializó el renderizador, el audio y los atlas de texturas. Después se cerró normalmente con código 0. No se creó un mundo durante esta prueba.

No se probó la autenticación con una cuenta real: falta registrar Luma e incluir un Client ID autorizado. El nuevo flujo OAuth con navegador se probó localmente con una respuesta simulada; no equivale a una verificación con Microsoft/Xbox/Minecraft. El perfil local permite jugar sin esa integración. No se probaron jugando todas las versiones del catálogo. Las instalaciones de prueba quedan disponibles en la carpeta de datos del launcher.
