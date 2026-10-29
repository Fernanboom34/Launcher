# Prototipo de skins locales mediante agente

Experimento aislado para evaluar un cliente Minecraft sin Fabric/Forge, con `authlib-injector` cargado como agente Java. No es Vanilla intacto. No se publica un release con este directorio: si se distribuye como versión del launcher, el canal propuesto es **0.9.0-alpha.1**, pre-release.

## Alcance

- Servicio Python de solo lectura, escuchando exclusivamente en `127.0.0.1` con puerto efímero y ruta aleatoria.
- Un perfil offline y una skin PNG cargada en memoria. Sin contraseñas, sesiones Microsoft ni base de datos remota.
- UUID compatible con `OfflinePlayer:<nombre>`, metadatos Yggdrasil, consultas por nombre/UUID y propiedad `textures` firmada con una clave RSA temporal por ejecución. SHA1withRSA viene impuesto por ese protocolo; la clave **no es la Ed25519 de actualizaciones**.
- PNG de hasta 1 MiB, cabecera y dimensiones 64×64 o 64×32 comprobadas. Eso no constituye validación completa de todos los chunks PNG. No se modifica el archivo original.
- Host y Origin restringidos, máximo ocho conexiones simultáneas y límites de lectura. No hay CORS, carga de archivos ni endpoints authenticate/refresh/join/hasJoined.
- El proceso acompaña al launcher CLI y se cierra cuando termina el juego. El lanzador experimental utiliza su propio grupo de procesos para detener sus hijos al interrumpirse.

No proporciona skins entre equipos ni acceso de perfiles locales a servidores que exigen Microsoft. No sustituye la autenticación oficial. Solo debe usarse en una carpeta experimental vacía: el script escribe sus opciones de lanzamiento allí y rechaza una instalación ajena. No apuntarlo a `.minecraft` ni a los datos habituales de Luma.

## Dependencias

Python 3.10+, OpenSSL (Git para Windows lo incluye), un ejecutable de Luma con su CLI y el agente oficial. Luma descarga la versión de Java necesaria en los datos experimentales; no hace falta incorporar otro lenguaje al launcher estable. La integración definitiva deberá portar el servicio a Rust y conectar su ciclo de vida con el del proceso de Minecraft.

Agente fijado para esta prueba:

- Versión: authlib-injector **1.2.8**, build **56**.
- URL: https://authlib-injector.yushi.moe/artifact/56/authlib-injector-1.2.8.jar
- SHA-256: `9c7f4343e6c82034958ffb48c14a2cb0c85928be7283103ce17da00c6d5a7b10`
- Fuente y licencia AGPLv3: https://github.com/yushijinhun/authlib-injector . No se incluye el JAR ni código de ese proyecto en este repositorio.
- Especificación: https://yushijinhun.github.io/authlib-injector/zh/Yggdrasil-服务端技术规范.html

El script rechaza otro archivo aunque tenga el mismo nombre. El hash está fijado en el código revisable, no se toma del mismo manifiesto remoto en cada arranque.

## Ejecución en Windows

Descarga el agente oficial de la URL anterior. Desde la raíz del código revisado:

```powershell
py experiments/local-skins/prototype.py --name Jugador --skin "C:\Mis skins\skin.png" --agent "C:\Herramientas\authlib-injector-1.2.8.jar" --launcher "C:\Program Files\Luma Launcher\Luma.exe" --data-dir "$env:LOCALAPPDATA\LumaLocalSkinExperiment" --version 1.21.1 --prepare
```

`--prepare` instala los archivos del juego exclusivamente en la carpeta experimental. Se puede omitir en las siguientes ejecuciones. `--slim` elige brazos delgados. El script no instala mods ni activa el agente en la configuración habitual del launcher. La ruta del ejecutable depende de la instalación de cada usuario.

Para ejecutar únicamente el servicio y obtener el argumento de Java, omitir `--launcher` y `--data-dir`. La URL impresa contiene una capacidad local temporal; no es una contraseña de cuenta. El servicio debe seguir abierto mientras se ejecuta Minecraft.

## Comprobaciones

```text
python3 experiments/local-skins/test_prototype.py
```

Se verifica la firma con OpenSSL de forma independiente, el rechazo de una propiedad alterada, el UUID offline, la búsqueda por nombre, los perfiles desconocidos, el rechazo de Host/Origin ajenos y los intentos de autenticación/lectura de rutas no expuestas.

La prueba visual debe documentar versión, plataforma, skin y ausencia de mods. No se extrapola un resultado en 1.21.1 a versiones distintas ni a servidores multijugador. Windows requiere una validación propia del ciclo de vida antes de distribuir.

## Modelo de brazos y dependencia PNG

Antes de ejecutar el prototipo o sus pruebas:

```powershell
py -m pip install -r experiments/local-skins/requirements.txt
```

El modelo se detecta automáticamente cuando el PNG tiene las columnas transparentes
propias de brazos delgados. En ese caso `--classic` se rechaza. No necesitas
`--slim` para esas imágenes. Para una imagen moderna ambigua se conserva la
opción explícita `--slim`; un PNG no lleva un campo universal que declare el
modelo. Las skins antiguas 64×32 son clásicas. El decodificador verifica el PNG
completo y no modifica sus píxeles.

El inicio muestra modelo y nombres disponibles. Importar una skin local NO la
publica ni la transfiere al otro cliente LAN. El directorio de prueba contiene
únicamente LumaWebA y LumaWebB; Jugador no está incluido. Los contadores finales
incluyen unknown_profile para diagnosticar consultas sin coincidencia. Los
perfiles enviados por un servidor pueden contener propiedades firmadas por otra
clave o URLs loopback de otro proceso: compartir un directorio no demuestra que
Minecraft reconsulte automáticamente esas propiedades. La visibilidad mutua
sigue pendiente de una prueba real y no se considera corregida por estos tests.
