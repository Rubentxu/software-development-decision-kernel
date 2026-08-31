# ADR-0006 — Identidad estable de proyecto y paths XDG

**Estado:** aceptada
**Fecha:** 2026-08-03

## Contexto

Usar el basename del repositorio produce colisiones, falla con worktrees y vincula el conocimiento a una ruta local mutable.

## Decisión

Separar:

- `project_id`: hash de remote normalizado y scope del proyecto.
- `workspace_id`: hash de `project_id` y ruta canónica del checkout.
- `display_name`: nombre legible.

Cuando no exista remote, generar un UUID estable almacenado en el registro de adopción.

Paths por defecto:

```text
$XDG_DATA_HOME/sddk/projects/<project-id>/vault
$XDG_DATA_HOME/sddk/projects/<project-id>/artifacts
$XDG_STATE_HOME/sddk/projects/<project-id>/ledger.sqlite
$XDG_CACHE_HOME/sddk
```

## Consecuencias

- Dos repositorios `backend` no colisionan.
- Un proyecto puede tener varios worktrees.
- Renombrar el directorio no pierde el conocimiento.
