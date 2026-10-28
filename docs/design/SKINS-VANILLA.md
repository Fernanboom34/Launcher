# Skins: perfiles locales y Vanilla

Referencias públicas consultadas en modrinth/code: `packages/app-lib/src/api/minecraft_skins.rs` y `packages/app-lib/src/state/minecraft_skins/mojang_api.rs`. La biblioteca local guarda skins; Mojang decide cuál equipa la cuenta. Para cambiarla, Modrinth envía la imagen a `https://api.minecraftservices.com/minecraft/profile/skins` con el token de acceso de Minecraft.

Luma ya implementa ese endpoint en `skins::apply_account`. También guarda perfiles locales sin contraseñas en `settings.json`, con nombre y UUID offline, y una biblioteca de skins en `skins/library.json` y `skins/textures/`. Crear otra base de datos no proporciona a Vanilla un mecanismo para cargar una skin por perfil local.

Una skin personal arbitraria en un cliente Vanilla sin modificar se obtiene mediante los servicios oficiales para una cuenta Minecraft. Las cuentas locales no pueden publicar propiedades de texturas firmadas por Mojang. Luma mantiene CustomSkinLoader como opción local en instancias compatibles con mods.

Un paquete de recursos podría sustituir una textura predeterminada sin mod, pero no equivale a asignar una skin exclusiva: afecta a jugadores que utilicen esa misma textura, depende de la versión y no proporciona sincronización multijugador. Por eso no se presenta como una solución equivalente ni se implementa sin elegir explícitamente ese comportamiento. No se reemplazan UUIDs por los de otras cuentas ni se manipula la caché de autenticación.

La primera mejora visual normaliza la proyección de la vista previa, ajusta el grosor de las capas de cabeza/cuerpo, separa ligeramente los brazos y permite girar, inclinar, centrar y ocultar la segunda capa. Es una vista previa local; esos controles no alteran la apariencia de la cuenta.
