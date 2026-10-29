# Luma 0.9.0

Nueva versión estable para Windows y Linux x64.

## Catálogo integrado en la instancia

- Explorar contenido abre el catálogo dentro de la pantalla de la instancia, con navegación para regresar a su contenido instalado.
- Resultados con iconos, autores, descripciones, búsqueda y paginación.
- Instalación rápida de la versión compatible: primero la release más reciente; si no existe, la beta más reciente; después la alpha más reciente.
- Selector manual de versiones para resolver incompatibilidades entre proyectos.
- Se mantienen los filtros de Minecraft y loader, la resolución de dependencias y las verificaciones de archivos antes de instalar.

Esta versión no incorpora el prototipo de servidor local de skins, authlib-injector ni el directorio web experimental. Las funciones de skins existentes en 0.8.1 se mantienen.

Las actualizaciones requieren manifiestos firmados con Ed25519 y verificación SHA-256. La clave privada permanece en el equipo del mantenedor y no se sube a GitHub. Los instaladores de Windows no tienen certificado Authenticode.
