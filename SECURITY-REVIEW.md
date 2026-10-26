# Revisión acotada de seguridad sobre v0.8.0-beta.3

Se revisaron las entradas de contenido del catálogo, del navegador y de la importación local, además de los lectores de metadatos y extracción ZIP. No constituye una auditoría exhaustiva ni una certificación antivirus.

## Hallazgos y cambios

- La importación local copiaba archivos sin validar el ZIP; el catálogo verificaba hashes sin validar su estructura. El navegador solo abría el directorio ZIP. Ahora los tres caminos validan las entradas antes de incorporarlas al destino.
- Se rechazan rutas absolutas, traversal, separadores Windows, unidades/ADS, caracteres de control, nombres repetidos sin distinguir mayúsculas y enlaces simbólicos.
- Se limitan tamaño comprimido (512 MiB), entradas (65536), tamaño por entrada (256 MiB), total descomprimido (1 GiB) y proporción de compresión (1000:1 después de 1 MiB). Se leen los datos para verificar tamaño y CRC. Algunos packs legítimos excepcionalmente grandes pueden ser rechazados.
- El cliente de descargas compartido y el de catálogo exigen HTTPS también al seguir redirecciones. La descarga compartida tiene un límite de 1 GiB y limpia el archivo parcial al fallar; antes no tenía límite de bytes.

## Pruebas reproducibles

`cargo test --locked --features eframe/x11 --all-targets`

En `content::hostile_archives`: traversal POSIX/Windows, ruta absoluta, unidad Windows, colisión de nombres, enlace simbólico, corrupción de datos y bomba de compresión controlada. También se admite un ZIP normal. Ningún payload ejecuta código ni toca datos de usuario.

## Límites y trabajo pendiente

Un hash comprueba integridad respecto al proveedor, no inocuidad. Un ZIP/JAR estructuralmente válido puede ser malicioso; Minecraft ejecuta mods con los permisos del jugador. Estos cambios no analizan bytecode, no aíslan Minecraft, no consultan antivirus/listas de reputación y no impiden que se coloquen archivos directamente en la carpeta de mods. Tampoco cubren todas las vulnerabilidades de parsers del juego ni ZIP anidados.

Se consultó OSV para las 593 dependencias de registro del lockfile inicial y se distinguieron vulnerabilidades de avisos de mantenimiento. No se realizó una prueba nativa de Windows. No se afirma que se haya eliminado todo riesgo de contenido malicioso. Para esa protección hacen falta controles adicionales de reputación y aislamiento del proceso de juego, con diseño y validación propios.


## Segunda revisión: rutas de escritura y dependencias

- `engine::path_under` validaba la sintaxis pero no los enlaces existentes. Un directorio enlazado dentro del destino permitía alcanzar un archivo externo durante la extracción. Ahora rechaza enlaces en los componentes y ancestros; en Windows también reparse points. El test `rejects_existing_links_in_extraction_destination` construye un ZIP y una carpeta enlazada, exige el rechazo y comprueba que no se creó el archivo externo. La prueba nativa de enlaces solo se ejecutó en Linux.
- La extracción ZIP del motor pasa por esa comprobación, rechaza los nombres originales peligrosos y limita entradas y bytes descomprimidos. No se restauran permisos SUID/SGID. La extracción sigue siendo no transaccional: puede dejar archivos previos a una entrada rechazada.
- La descarga usaba un `.part` de nombre fijo con apertura que seguía enlaces. Ahora usa un nombre aleatorio criptográfico y creación exclusiva. Esto evita el ataque de precolocar un enlace en el nombre temporal conocido.
- Se actualizó `zip` 2.6.1 a 9.1.2 para retirar RUSTSEC-2026-0367, RUSTSEC-2026-0368 y RUSTSEC-2026-0369. Esos avisos afectan a APIs de extracción de la biblioteca; Luma usa extracción propia, pero se actualiza igualmente. OSV no devolvió avisos para 9.1.2 al verificarla.
- El lockfile también contiene glib 0.18.5 con RUSTSEC-2024-0429. `cargo tree -i glib` no lo incluye en el build Linux de Luma; el grafo con todos los targets lo arrastra por la parte GTK de wry, declarado únicamente para Windows en este proyecto. No se afirma corregido el paquete del lockfile ni su eventual uso en configuraciones distintas.
- Quedan avisos de mantenimiento en paste, proc-macro-error, ttf-parser, zbus_names, zvariant y zvariant_derive. No se clasifican automáticamente como vulnerabilidades explotables.

Estas comprobaciones de rutas no son un sandbox ni proporcionan una garantía contra carreras de otro proceso local que cambie los directorios entre la comprobación y la escritura. Resolver ese adversario requiere operaciones relativas a handles de directorio y validación nativa en ambos sistemas. Las carpetas enlazadas intencionadamente pueden ser rechazadas ahora.
