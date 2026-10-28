# Instalador nativo de Luma

`Luma.iss` utiliza Inno Setup y descarga el payload del tag estable `v0.8.1`. La versión y la URL deben coincidir con ese tag; se comprueba el SHA-256 antes de instalar. No incluye Java, mods ni perfiles.

`./scripts/test-native-installer.ps1` compila el instalador y una variante aislada con un ejecutable de prueba para verificar instalación, desinstalación y conservación de datos ajenos. La variante de prueba nunca se publica. El script recalcula los hashes del instalador final.

El actualizador integrado autentica sus manifiestos con Ed25519. El instalador descargado manualmente usa HTTPS y SHA-256; no incluye verificación Ed25519 del manifiesto ni firma Authenticode. Descárgalo del release oficial verificado. Véase [firma de actualizaciones](../docs/security/UPDATE-SIGNING.md).
