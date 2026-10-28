# Catálogo e instancias: primera comparación

Referencia consultada: repositorio público [modrinth/code](https://github.com/modrinth/code). No hace falta un fork para consultarlo. Las licencias se comprueban por componente: `apps/app-frontend/LICENSE` y `apps/app/LICENSE` contienen GPLv3; eso no implica que todos los recursos del monorepo tengan la misma licencia. Esta primera entrega usa código propio de Luma, sin trasladar código ni assets de Modrinth.

## Catálogo

La referencia `apps/app-frontend/src/platform/adapters/browse/browse-page.vue` separa búsqueda, preferencias de instalación, destino y resolución del plan. Usa componentes Vue y servicios que no son intercambiables con egui. Luma ya usa la API oficial de Modrinth: el problema no es sustituir un catálogo ficticio, sino mejorar su interacción y sus estados.

El catálogo ocupa el área de Contenido al pulsar Explorar contenido. Incluye búsqueda automática, tarjetas con iconos y autores, selección de tipo, resultados paginados y regreso a los archivos instalados. Los iconos HTTPS del CDN de Modrinth se limitan a 1 MiB y 1024×1024 antes de reducirlos y guardarlos; se admiten PNG, JPEG y WebP.

Instalar consulta y filtra versiones por Minecraft y loader, ordena release antes que beta y alpha y fecha descendente dentro de cada categoría. Elegir versión permite escoger manualmente otra versión compatible, mostrando categoría y fecha. La compatibilidad declarada con juego y loader no garantiza que todos los mods funcionen juntos. Se mantiene el instalador de dependencias y sus validaciones. La instalación se bloquea durante la ejecución de la instancia o con el navegador de contenido abierto.

Pendiente: ordenación y filtros avanzados, estado instalado/actualizable por identificador de proyecto y un resumen explícito de conflictos entre mods. Los data packs conservan su destino de mundo; cambiar a otro tipo descarta ese destino.

## Instancias

Referencias: `packages/app-lib/src/api/instance/{lifecycle,install,projects,paths}.rs`. Su separación de ciclo de vida, contenido y rutas es útil como guía; no demuestra por sí sola que Luma herede sus garantías.

Luma ya separa carpetas de juego, permite duplicar y conserva mundos al retirar un registro. Primera corrección: si falla la duplicación, retirar únicamente el destino nuevo incompleto y conservar la instancia original y su registro. Añadir una prueba de fallo real de copia.

Pendiente: revisar creación/reparación interrumpidas, consistencia entre registros y archivos, contenido instalado por identificador y versión, actualizaciones de proyectos con recuperación y conflictos entre operaciones sobre carpetas compartidas. Revisar y probar cada operación antes de afirmar que es transaccional. No se cambia automáticamente una versión de Minecraft o un cargador para acomodar un mod.

## Validación y alcance

Entregas en PR separados. No se modifica el release 0.8.1 ya publicado ni se publica otra versión. Esta comparación inicial no constituye una auditoría completa de seguridad de ninguno de los dos proyectos.
