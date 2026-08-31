# Cadencia de mantenimiento

## Disparadores de una nueva edición
1. **Release mayor** del framework principal (ej. Bevy 0.19 → 0.20).
2. **Breaking change** en un crate clave del libro.
3. **MSRV** de Rust sube por encima del declarado.
4. Acumulación de drift menor (revisión trimestral).

## Proceso
1. `version-drift-detector` → impacto.
2. Decidir severidad: ¿minor patch, o nueva edición?
3. Minor patch: actualizar versiones + re-verificar, mismo `edition`.
4. Nueva edición: actualizar + añadir capítulo "migración" + subir `edition`.

## Cambios a documentar siempre
- Versiones de framework/crate que cambiaron.
- APIs que se renombraron/eliminaron.
- Ejemplos que hubo que reescribir.
- Claims que pasaron a `verified` con nueva fuente.

## Anti-patrón
No publicar una "reedición" que solo cambia el número de versión sin re-verificar ejemplos. Eso reintroduce exactamente el problema que `code-example-verifier` y `hallucination-auditor` previenen.
