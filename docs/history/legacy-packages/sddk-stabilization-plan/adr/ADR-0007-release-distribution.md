# ADR-0007 — Distribución explícita y actualización side-by-side

**Estado:** aceptada
**Fecha:** 2026-08-03

## Contexto

El CLI debe ser fácil de instalar y actualizar sin introducir efectos secundarios durante `cargo build`.

## Decisión

- Desarrollo: `cargo xtask install-dev` instala en `~/.local/bin`.
- No usar `build.rs` como instalador.
- Release-plz gestiona versión y changelog.
- Dist genera binarios, checksums e instaladores.
- Los binarios incluyen receipts y procedencia.
- La actualización estable se realiza side-by-side y se promueve atómicamente.
- Nunca actualizar durante un ciclo activo o con efectos pendientes.

## Flujo de release del proyecto gestionado

```text
preflight → PR → checks → aprobación → merge → verificación
→ tag → publicación → ledger → cierre
```

## Consecuencias

La publicación es idempotente y reconciliable; el rollback conserva la versión anterior.
