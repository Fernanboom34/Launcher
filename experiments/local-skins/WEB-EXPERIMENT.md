# Directorio web compartido: prueba controlada

El servicio de cada PC puede leer una misma instantánea HTTPS de perfiles y texturas. El directorio de diagnóstico se publica como archivos estáticos del repositorio, sin backend, tokens de escritura ni cuentas reales:

https://raw.githubusercontent.com/Fernanboom34/Launcher/12474d171b810af1db9c7c8b1f38bf985217f018/experiments/local-skins/web-fixture/registry.json

Contiene `LumaWebA` (rojo) y `LumaWebB` (azul). Las imágenes de prueba fueron generadas para este experimento; no se publicó la skin del usuario. El esquema incluye nombre, archivo, SHA-256 y modelo. Los nombres no representan reservas de identidad globales.

## Uso

Descarga la textura correspondiente desde la misma carpeta del directorio (red.png o blue.png). Añade a la invocación del prototipo:

```text
--name LumaWebA --skin RUTA/red.png --registry https://raw.githubusercontent.com/Fernanboom34/Launcher/12474d171b810af1db9c7c8b1f38bf985217f018/experiments/local-skins/web-fixture/registry.json
```

Para el segundo cliente usa `LumaWebB`, `blue.png` y otra carpeta experimental. El servicio sigue exponiendo solo URLs de loopback al juego: descarga las texturas del directorio antes de arrancar, verifica los hashes y firma las propiedades con su clave RSA temporal. Los dos agentes confían en sus respectivos servicios locales. No se abren puertos del servicio de skins a otras computadoras.

El prototipo acepta exclusivamente el directorio de este repositorio en un commit de 40 caracteres. Rechaza ramas mutables, redirecciones, otro origen, nombres repetidos, rutas no previstas, registros excesivos y texturas alteradas. También rechaza una skin local que contradiga el registro compartido de su nombre. No existe subida desde el cliente. Si la web no responde, el inicio experimental falla; todavía no hay caché offline del directorio.

## Resultado observado

- Doce pruebas locales pasan, incluyendo dos servicios independientes que resuelven ambos perfiles y descargan bytes idénticos, además de los rechazos de seguridad.
- Dos procesos reales de Minecraft 1.21.1 en Linux descargaron la instantánea HTTPS. El primero cargó su skin roja y abrió un mundo LAN; el segundo usó el perfil azul.
- El intento de conexión al mundo LAN terminó con **Failed to log in: Status: 403**. El servicio no implementa join/hasJoined: el catálogo de skins no reemplaza la validación de sesión. No se implementó una respuesta de autenticación ficticia para superar ese rechazo.
- **No quedó verificada la visualización mutua de dos jugadores en un mundo compartido.** Las pruebas de consulta de perfiles no deben describirse como una prueba multijugador completa.

La siguiente prueba multijugador requiere un servidor de prueba cuya política permita esos perfiles locales, configurado expresamente por su administrador, o una arquitectura que preserve las sesiones oficiales. También faltan dos equipos físicos, Windows dentro del juego, fallos de red, actualización de texturas y recuperación de caché.

## Publicación de skins por usuarios

Alojar archivos estáticos resuelve la distribución, no quién puede cambiar cada registro. Un sistema público necesita comprobar la propiedad del perfil y autorizar cada modificación. No se debe guardar un token compartido de GitHub/R2 en Luma. Para un grupo cerrado puede empezarse con cambios revisados por el mantenedor; una publicación automática requerirá un flujo de autorización específico. Las claves Ed25519 de las actualizaciones no se reutilizan para este directorio.

El experimento sigue en alpha. No se publica una beta ni se modifica el canal estable.
