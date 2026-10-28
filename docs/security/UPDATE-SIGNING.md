# Activación de actualizaciones autenticadas

## Estado actual

`security/update-trust.json` contiene la clave pública Ed25519 proporcionada por el propietario y se incorpora al binario al compilar. Los manifiestos de actualización deben estar firmados con la clave privada correspondiente. Authenticode queda pendiente y no es un requisito del actualizador en Windows: la autenticidad depende de Ed25519 y la integridad del ejecutable de su SHA-256 firmado. No se confía en una clave enviada por el manifiesto ni se generan claves privadas en CI. Las versiones ya publicadas conservan su comportamiento anterior: este cambio no puede proteger retroactivamente esos binarios.

## Manifiesto firmado

Añadir una o más claves públicas Ed25519 de 32 bytes en base64 a `ed25519_public_keys` mediante revisión. Distribuir esa primera versión verificada por un canal independiente: descargar la misma clave del release no establece confianza. Mantener la clave privada fuera del repositorio y fuera del CI ordinario de compilación.

El campo `signature` es una firma Ed25519 base64 de los bytes UTF-8 exactos, incluidos saltos LF y LF final:

```
Luma update manifest v1
<version>
<url>
<sha256>
```

El formato firma la versión, URL y hash; modificar cualquiera invalida la firma. Cada plataforma requiere su manifiesto firmado correspondiente. El archivo firmado se conserva junto a la descarga y se vuelve a verificar antes de aplicar el update.

Después de revisar/verificar el artefacto en un entorno de firma confiable, ejecutar `python3 scripts/sign-update-manifest.py MANIFEST ARTEFACTO --private-key RUTA_PEM`. El script no imprime la clave y utiliza OpenSSL. No volver a firmar automáticamente cualquier artefacto de un CI potencialmente comprometido. Publicar el manifiesto firmado en un release NUEVO; no sustituir assets de releases existentes.

## Authenticode en Windows (pendiente)

No se dispone de un certificado Authenticode y no se exige esta firma para descargar ni aplicar actualizaciones. Windows puede mostrar avisos de SmartScreen o de editor desconocido: Ed25519 autentica las actualizaciones para Luma, pero no proporciona reputación de editor en Windows.

Cuando se obtenga un certificado, se podrá integrar SignPath u otro proveedor. Firmar primero los ejecutables finales (incluido el instalador Inno `Luma-Setup.exe`), después calcular sus hashes y finalmente firmar los manifiestos con Ed25519. Cambiar un ejecutable después de generar el manifiesto invalida su hash. Cualquier futura exigencia adicional de certificado debe incorporarse explícitamente al código y distribuirse en una nueva versión.

## Orden de integración

Integrar también el PR de bloqueo del handle de Windows (#4) antes de distribuir: autenticar el hash no elimina por sí solo una carrera entre verificación y copia. La copia Linux comprueba el hash de los bytes efectivamente escritos antes del rename. La protección contra rollback persistente y la inmutabilidad de tags son controles distintos; no se promete resistencia a un administrador local que sustituya el propio launcher.

## Guía para el propietario

Consulta [Firmar y publicar en Windows](FIRMAR-RELEASE-WINDOWS.md). Desde 0.8.1 el workflow prepara un borrador: la firma local de los manifiestos es necesaria para finalizar la publicación estable.
