# Luma 0.8.1

Versión estable de mantenimiento para Windows y Linux x64.

- Esquinas redondeadas en modo ventana con el marco propio de Luma. Al maximizar o usar pantalla completa, se mantienen esquinas rectas. Requiere composición/transparencia del escritorio.
- Actualizaciones autenticadas con Ed25519, sin exigir un certificado Authenticode. Se conservan la verificación SHA-256 y la copia protegida del actualizador.
- Actualización de los diálogos de archivos y de CodeQL.
- Licencia GPLv3 y botón de apoyo Ko-fi en el README.

Windows puede mostrar avisos de editor desconocido. La primera instalación desde 0.8.0 debe hacerse manualmente: esa versión no contiene la clave pública de actualización. La clave privada se conserva fuera de GitHub; los manifiestos se firman localmente antes de publicar.
