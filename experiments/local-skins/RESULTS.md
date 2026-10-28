# Primera prueba del prototipo

Entorno: Linux x64, Java 21, Minecraft Java **1.21.1**, authlib-injector **1.2.8 / build 56** con el SHA-256 fijado en el prototipo. Ejecutable CLI de Luma 0.8.1. Carpeta de datos experimental independiente.

Se descargó el cliente oficial y se creó un mundo superplano de prueba. No existe carpeta `mods` en esa instancia ni se instaló Fabric/Forge. El argumento Java incorpora el agente, por lo que el cliente no es Vanilla intacto.

La skin utilizada es un atlas de diagnóstico con cabeza marrón, brazos de distintos colores y pantalones azulados, para distinguirla de cualquier skin predeterminada. Una captura real del juego en tercera persona muestra esa textura. La imagen se captura desde la ventana de Minecraft; no se genera con el visor 3D del launcher.

Siete pruebas locales de protocolo comprueban firma válida y alterada, UUID offline, consulta por nombre, perfiles desconocidos, Host/Origin, rutas de lectura no autorizadas, ausencia de autenticación multijugador, capacidad local y límite del cuerpo de las solicitudes. La clave temporal es RSA para Yggdrasil; la clave Ed25519 de publicación no participa.

Este resultado valida una combinación de versión/plataforma y una sesión de un jugador. Todavía no valida Windows dentro del juego, otras versiones, multijugador, sincronización entre equipos ni una integración definitiva en Rust. El prototipo no se activa en la configuración habitual de Luma.

Al terminar la sesión, Minecraft devolvió código 0. El servicio registró exactamente una consulta de metadatos, una de perfil y una de textura. Se comprobó que su puerto ya no aceptaba conexiones después de salir.
