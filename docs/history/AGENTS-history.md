# AGENTS-history.md — archived regressions from AGENTS.md

> **STATUS:** ARCHIVED — frozen at commit `13f6028` (v1.9.8).
> **DO NOT APPEND** new regressions here. File a new incidence at:
> `~/.sddk-knowledge/sddk-framework/incidences/INC-XXX.md`
> Current AGENTS.md (stable surface): `AGENTS.md` in repo root.
> Forward pointer updated at each cycle close.

---

## 5. Regresiones detectadas (a resolver en futuras sesiones)

Estas cosas **no se arreglaron todavía** pero violan el spec. Documentadas
para no perderlas de vista.

### 5.1. `~/.sddk-shared/` (REGRESIÓN RESUELTA — 2026-08-08)

`~/.sddk-shared/` era un **segundo checkout** del mismo repo. Todo el
trabajo de desarrollo debe ocurrir en el CWD (`sddk-framework/`).

**Resuelto**: eliminado con `rm -rf /var/home/rubentxu/.sddk-shared/`
previa verificación de que los 3 commits y los 4 cambios uncommitted
estaban en el CWD / `origin/main` (ver commit `98b20d7` que documenta
esta regresión retrospectivamente).

**Prevención**: no vuelvas a crear un segundo checkout. Si necesitas
iterar, usa el CWD. El bundle runtime (`~/.local/share/sddk/framework/v1.5.3/`)
se actualiza con `sddk dev install` (ver §4.2).

### 5.2. `bootstrap.sh` referencia `~/.sddk-shared/` (DRIFT)

```
$ grep -c sddk-shared bootstrap.sh
1
```

**Problema**: el bootstrap dice *"make `~/.sddk-shared/` the single source
of truth"*, pero el spec RS-2026-08 cambió la fuente de verdad al CWD
(`sddk-framework/`) + bundle runtime en `~/.local/share/sddk/framework/`.

**Acción**: actualizar `bootstrap.sh` para:
- Usar `$(cd "$(dirname "$0")" && pwd)` (que YA es lo que hace para
  `SHARED_DIR`) como source de agents/skills/prompts.
- Apuntar los symlinks a `~/.local/share/sddk/framework/current/`
  (no a `~/.sddk-shared/`).
- Renombrar/desinstalar la variable `SDDK_SHARED_DIR`.

### 5.3. `Cargo.toml` v1.5.3 sin tag público (DRIFT menor)

```
[workspace.package]
version = "1.5.3"
```

Los últimos tags en `origin` son `v1.5.0`, `v1.5.2`, `v1.5.3`. El HEAD
actual está ahead del último tag público porque incluye commits de
desarrollo sin taggear. **No es un bug** — es el flujo normal —
pero requiere `chore(release): bump to vX.Y.Z` cuando se quiera
publicar.
