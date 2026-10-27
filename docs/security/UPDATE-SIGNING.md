# Activación de actualizaciones autenticadas

## Estado actual

`security/update-trust.json` contiene la clave pública Ed25519 proporcionada por el propietario y se incorpora al binario al compilar. Los manifiestos de actualización deben estar firmados con la clave privada correspondiente. Sigue pendiente configurar un certificado Authenticode autorizado: en Windows las actualizaciones automáticas también requieren ese certificado y una firma válida del ejecutable. No se confía en una clave enviada por el manifiesto ni se generan claves privadas en CI. Las versiones ya publicadas conservan su comportamiento anterior: este cambio no puede proteger retroactivamente esos binarios.

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

## Authenticode en Windows

Configurar SignPath (requiere cuenta, organización, proyecto, artefacto y política de firma) o un certificado de firma válido bajo custodia del propietario. La firma del launcher y del instalador debe hacerse después de compilar; después se calculan hashes y se firma el manifiesto. Inno `Luma-Setup.exe` se produce en scripts/test-native-installer.ps1 después del antiguo ejecutable Rust: firmar también ese archivo final.

Añadir a `authenticode_certificate_sha256` el SHA-256 hexadecimal en minúsculas del certificado DER del firmante autorizado. El updater exige además que Windows determine `Get-AuthenticodeSignature.Status == Valid`; un certificado válido de otro editor no basta. La renovación de certificado/clave exige publicar previamente una versión que incluya ambos pines mediante una cadena de confianza revisada.

Este PR no da de alta SignPath ni simula una firma Authenticode. Sin los datos y credenciales reales, la activación queda pendiente. Las descargas/manuales y los instaladores anteriores no se convierten en autenticados por añadir este verificador.

## Orden de integración

Integrar también el PR de bloqueo del handle de Windows (#4) antes de distribuir: autenticar el hash no elimina por sí solo una carrera entre verificación y copia. La copia Linux comprueba el hash de los bytes efectivamente escritos antes del rename. La protección contra rollback persistente y la inmutabilidad de tags son controles distintos; no se promete resistencia a un administrador local que sustituya el propio launcher.
