# Revisión para 0.8.1

| PR | Decisión | Motivo |
| --- | --- | --- |
| #23 rfd 0.17 | Integrado | API utilizada compatible; cinco checks correctos. Reduce dependencias del portal Linux. |
| #24 sha1 0.11 | Cerrado y rama eliminada | Falla la compilación con digest 0.11 (LowerHex en catalog.rs y usos de Digest). La migración necesita un cambio dedicado. |
| #25 CodeQL | Integrado | init y analyze actualizados juntos, fijados por SHA; cinco checks correctos. |
| #26 Ed25519 | Integrado | Quita Authenticode sin quitar Ed25519, SHA-256 ni el handle protegido; cinco checks correctos. |

La búsqueda de issues del conector devolvió una lista vacía incorrectamente; la revisión se contrastó con la lista directa de pull requests de GitHub.
