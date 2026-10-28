# Catálogo e instancias: primera comparación

Referencia consultada: repositorio público [modrinth/code](https://github.com/modrinth/code). No hace falta un fork para consultarlo. Las licencias se comprueban por componente: `apps/app-frontend/LICENSE` y `apps/app/LICENSE` contienen GPLv3; eso no implica que todos los recursos del monorepo tengan la misma licencia. Esta primera entrega usa código propio de Luma, sin trasladar código ni assets de Modrinth.

## Catálogo

La referencia `apps/app-frontend/src/platform/adapters/browse/browse-page.vue` separa búsqueda, preferencias de instalación, destino y resolución del plan. Usa componentes Vue y servicios que no son intercambiables con egui. Luma ya usa la API oficial de Modrinth: el problema no es sustituir un catálogo ficticio, sino mejorar su interacción y sus estados.

Primera entrega: búsqueda automática con pausa breve, resultados separados del detalle y regreso sin perder consulta ni página, ventana adaptada al espacio disponible, estados de carga de versiones y bloqueo de instalación sobre una instancia ejecutándose. Se conserva la selección explícita de versión y la instalación de dependencias existente. Las respuestas antiguas no deben sobrescribir una búsqueda posterior.

Próximas entregas: iconos con caché y límites de descarga, ordenación y categorías, estado instalado/actualizable por proyecto, acción de instalación compatible desde resultados y confirmación clara del destino. Los data packs deben seguir eligiendo un mundo explícito; cambiar de tipo no puede reutilizar un destino inadecuado.

## Instancias

Referencias: `packages/app-lib/src/api/instance/{lifecycle,install,projects,paths}.rs`. Su separación de ciclo de vida, contenido y rutas es útil como guía; no demuestra por sí sola que Luma herede sus garantías.

Luma ya separa carpetas de juego, permite duplicar y conserva mundos al retirar un registro. Primera corrección: si falla la duplicación, retirar únicamente el destino nuevo incompleto y conservar la instancia original y su registro. Añadir una prueba de fallo real de copia.

Pendiente: revisar creación/reparación interrumpidas, consistencia entre registros y archivos, contenido instalado por identificador y versión, actualizaciones de proyectos con recuperación y conflictos entre operaciones sobre carpetas compartidas. Revisar y probar cada operación antes de afirmar que es transaccional. No se cambia automáticamente una versión de Minecraft o un cargador para acomodar un mod.

## Validación y alcance

Entregas en PR separados. No se modifica el release 0.8.1 ya publicado ni se publica otra versión. Esta comparación inicial no constituye una auditoría completa de seguridad de ninguno de los dos proyectos.
