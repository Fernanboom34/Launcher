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

No se realizó un análisis completo de dependencias mediante una base de avisos de seguridad, ni una prueba nativa de Windows. No se afirma que se haya eliminado todo riesgo de contenido malicioso. Para esa protección hacen falta controles adicionales de reputación y aislamiento del proceso de juego, con diseño y validación propios.
