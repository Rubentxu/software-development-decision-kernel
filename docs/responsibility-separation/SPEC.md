# Spec — Separación de responsabilidades y cero intrusión (milestone RS-2026-08)

**Estado:** draft
**Fecha:** 2026-08-07
**ADR:** [ADR-0011](../sddk-stabilization-plan/adr/ADR-0011-responsibility-separation-non-intrusive.md)
**Modelo de referencia:** asdf-vm (data dir, bundles por versión, shims/current, `.tool-versions`, `path:`)

---

## 1. Objetivo

Separar en tres roles incompatibles el estado anterior donde `~/.sddk-shared/` era un segundo checkout del mismo repositorio, funcionando simultáneamente como repo de desarrollo, fuente del dev link y workspace de uso. Garantizar que el framework **nunca escribe dentro de los repos git de los proyectos** (cero intrusión): todo el estado operativo vive en directorios de usuario (`$SDDK_DATA_DIR`, `~/.sddk-knowledge/`), con resolución de versión por proyecto inspirada en asdf-vm y soporte multiplataforma (Linux/macOS/Windows).

## 2. No objetivos

- Componente MCP (excluido, igual que CP-2026-08).
- Soporte de Windows como target de release publicado (queda como disparador; el código debe ser portable pero no se publican builds Windows).
- Compartir artefactos de ciclo por git (el CAS y los reportes HTML autocontenidos cubren la necesidad).
- Migración de datos del artifact-registry.

## 3. Roles y layout final

| Rol | Ubicación | Contenido | Adoptado | Linkado |
|-----|-----------|-----------|----------|---------|
| Repo de desarrollo | Clon canónico de `Rubentxu/software-development-decision-kernel` (p.ej. `~/Proyectos/agentesIA/sddk-framework`) | crates, docs, CI, releases | NO | NO |
| Bundle runtime | `$SDDK_DATA_DIR/framework/<version>/` + symlink `current` | agents, skills, prompts, workflows del release | — | SÍ |
| Workspace de uso | Repos reales de proyectos | código del proyecto + opcional `.sddk-versions` | SÍ | NO |

```text
~/.local/share/sddk/                    # $SDDK_DATA_DIR (override env SDDK_DATA_DIR)
├── framework/
│   ├── 1.3.0/                          # bundle instalado (installs/<tool>/<version>)
│   │   ├── agents/
│   │   ├── skills/
│   │   ├── prompts/
│   │   └── workflows/
│   └── current -> 1.3.0                # versión activa (shims)
├── projects/                           # estado por proyecto (ya existe)
│   └── <project_id>/
│       ├── knowledge-profile.json      # ruta canónica + preferencia Engram
│       ├── artifacts/                  # CAS por SHA-256
│       ├── cycle-artifacts/            # NUEVO: artefactos de ciclo {cycle_id}/
│       ├── generated/                  # NUEVO: docs generados (inventory, workflow)
│       └── workspaces/<wid>/adoption.json
├── control-plane/                      # ADR-0009: store central de telemetría

~/.sddk-knowledge/<project_id>/         # conocimiento canónico, fuera de SDDK_DATA_DIR
```

## 4. Resolución de versión (modelo asdf)

Lookup en orden (solo lectura del repo):

1. `$PWD/.sddk-versions`
2. `.sddk-versions` en directorios padre hasta la raíz
3. `$SDDK_DATA_DIR/framework/current` (global)

Formato de `.sddk-versions` (una línea, declarativo, gestionado por el desarrollador — el framework NUNCA lo escribe):

```text
sddk 1.4.0
```

Valores soportados:

| Valor | Significado |
|-------|-------------|
| `1.4.0` | Versión exacta de bundle instalado en `framework/1.4.0/` |
| `current` | El symlink global `framework/current` |
| `path:<dir>` | Working tree local (dogfooding controlado del repo de desarrollo) |
| `system` | La instalación del sistema (si existe, p.ej. brew) — opcional v1 |

Sin `.sddk-versions` → resolución cae a `current` (el proyecto funciona igual).

## 5. Comandos CLI nuevos/modificados

### `sddk dev install [--version <v>]` (modificado)

- Instala el bundle del release en `framework/<version>/` (modo bundle de `update_bundle` ya existente).
- Idempotente por versión; no toca `current` (un install nunca cambia la versión activa).

### `sddk dev use <version|path:...>` (NUEVO)

- Actualiza el symlink `framework/current`.
- `--dry-run` para ver el target sin cambiar.

### `sddk dev link --editor all` (modificado)

- Linkea desde `framework/current/` (resuelto por versión), no desde el repo de desarrollo.
- `dev doctor` verifica que los symlinks apunten bajo `$SDDK_DATA_DIR/framework/`.

### `sddk version` (NUEVO)

- Muestra: binario (cargo version), bundle activo (`current` → versión), y la resolución para el directorio actual (`.sddk-versions` → versión).

### `adopt apply` (modificado)

- **Elimina `plant_workflow_manifest`**: no crea `workflow/workflow.yaml` en el repo.
- La adopción solo escribe el receipt en XDG (ya es así) y registra el proyecto.

### `cycle transition --artifact` (modificado, vía prompts)

- Los agents escriben artefactos bajo `cycle-artifacts/{cycle_id}/` (XDG) y pasan ese path al CLI.

### `generate docs|inventory` (modificado)

- Escribe a `projects/<id>/generated/` por defecto.
- `--in-repo` explícito solo para dogfooding del repo de desarrollo (CI).

### `lint` (modificado)

- Lee el workflow del manifest embebido (`CANONICAL_WORKFLOW`) o del bundle; no exige `workflow/workflow.yaml` en el repo.

## 6. Cero intrusión: matriz de escritura

| Operación | Antes (repo) | Ahora (XDG) |
|-----------|--------------|-------------|
| Adopción | `workflow/workflow.yaml` plantado | nada; receipt en XDG |
| Artefactos de ciclo | `sddk/{change}/...` | `projects/<id>/cycle-artifacts/{cycle_id}/` |
| Docs generados | `docs/generated/` | `projects/<id>/generated/` (o `--in-repo`) |
| Knowledge vault | derivado del basename | `~/.sddk-knowledge/<project_id>/`, resuelto por `sddk knowledge path` |

**Única excepción**: `.sddk-versions` en la raíz del repo del proyecto — config declarativa del proyecto (como `package.json`/`go.mod`/`.tool-versions`), escrita solo por el desarrollador, nunca por el framework.

## 7. Multiplataforma

| SO | Data root (`SDDK_DATA_DIR`) | State | Cache | Editor config |
|----|------------------------------|-------|-------|---------------|
| Linux | `$XDG_DATA_HOME/sddk` → `~/.local/share/sddk` | `$XDG_STATE_HOME/sddk` → `~/.local/state/sddk` | `$XDG_CACHE_HOME/sddk` → `~/.cache/sddk` | `~/.config/opencode`, `~/.zcode` |
| macOS | `~/Library/Application Support/sddk` (override `XDG_*` posible) | `~/Library/Application Support/sddk` | `~/Library/Caches/sddk` | `~/.config/opencode` (opencode usa XDG en macOS) |
| Windows | `%APPDATA%\sddk` | `%LOCALAPPDATA%\sddk` | `%LOCALAPPDATA%\sddk\cache` | `%APPDATA%\opencode` |

Implementación: crate `dirs` en `sddk-engine/src/paths.rs`. Orden de resolución:
1. `SDDK_DATA_DIR` (env, explícito)
2. `XDG_DATA_HOME`/`XDG_STATE_HOME`/`XDG_CACHE_HOME` (env, explícito)
3. `dirs::data_dir()/state_dir()/cache_dir()` (por SO)
4. Último fallback: `HOME` (compatibilidad) — solo si 1-3 no resuelven

`dev link` mantiene el dual `#[cfg(unix)]` symlink / `#[cfg(not(unix))]` copy (ya existente en `dev_cmd.rs:569-574`).

## 8. Migración del estado existente

1. **Receipts duplicados**: eliminar `p-d9539a6a4bea1de0` (scope `sddk-framework`) y `p-52b95ef55999f9de` (scope `.`) de `.sddk-shared`; conservar el que corresponda o ninguno (el repo de desarrollo no debe estar adoptado).
2. **Artefactos `sddk/`**: mover (o regenerar) los receipts del working tree de `.sddk-shared/sddk/` a `cycle-artifacts/` XDG si se quieren conservar.
3. **Bundle runtime**: `sddk dev install 1.3.0` → `framework/1.3.0/` + `dev use 1.3.0`.
4. **Re-link**: `sddk dev link --editor all` contra `framework/current`.
5. **Dogfooding**: clon de trabajo separado (p.ej. `~/sddk-dogfood/sddk-framework`) con `.sddk-versions` = `path:<repo de desarrollo>` cuando se quiera probar código sin release.

## 9. Criterios de aceptación

- [ ] `adopt apply` en un repo limpio no modifica `git status` (cero ficheros creados).
- [ ] Un ciclo completo (adopt → ... → archive) deja `git status` del proyecto idéntico al inicial.
- [ ] Los symlinks de opencode/zcode apuntan bajo `$SDDK_DATA_DIR/framework/current/`.
- [ ] `sddk dev install 1.3.0 && dev install 1.4.0 && dev use 1.3.0` → editor sirve 1.3.0; `dev use 1.4.0` → sirve 1.4.0 sin re-link manual.
- [ ] Proyecto con `.sddk-versions` (1.4.0) usa 1.4.0 aunque `current` sea 1.3.0; sin fichero usa `current`.
- [ ] `path:<dir>` resuelve al working tree declarado (dogfooding).
- [ ] `resolve_xdg_paths` resuelve con `dirs` cuando no hay `HOME` ni XDG (simulado en test con env limpio).
- [ ] `sddk lint` pasa en un repo sin `workflow/workflow.yaml`.
- [ ] `sddk generate` en un proyecto no toca el working tree (sin `--in-repo`).
- [ ] Un solo receipt por workspace; control plane ingiere identidades únicas.
- [ ] 0 regresiones: `cargo test` + `act -j required` verdes (linux + darwin).

## 10. Fases de implementación

1. **RS-1**: `dirs` en `paths.rs` + tests multiplataforma (SDDK-1208).
2. **RS-2**: eliminar `plant_workflow_manifest` + `lint` sin dependencia del repo (SDDK-1201, SDDK-1204).
3. **RS-3**: `cycle-artifacts/` XDG + prompts/skills actualizados (SDDK-1202).
4. **RS-4**: `generate` → XDG + `--in-repo` (SDDK-1203).
5. **RS-5**: bundle multi-versión + `dev use` + `dev install` por versión (SDDK-1205).
6. **RS-6**: resolución de versión `.sddk-versions` → `current` → `path:` (SDDK-1207).
7. **RS-7**: migración del estado existente (SDDK-1206) + dogfooding separado.

## 11. Tests

- **Unit (`paths.rs`)**: resolución con overrides, con `dirs` (sin HOME), con `SDDK_DATA_DIR`; layout del bundle (installs por versión + `current`).
- **CLI**: `dev use` cambia `current`; `dev link` desde bundle; `adopt apply` no escribe en repo; `lint` sin workflow.yaml; `generate` sin tocar working tree.
- **Resolución versión**: `.sddk-versions` en PWD, en padre, ausente, `path:`, `current`.
- **Multiplataforma**: tests de paths con env simulado (macOS/Windows dirs); los tests no asumen Unix.
- **Integración**: ciclo completo E2E con `git status` limpio (extender `e2e-plan.md` N3 o suite de ciclo).

## 12. Estado resuelto (2026-08-08)

**Eliminado** `~/.sddk-shared/` (33 GB, segundo checkout del mismo repositorio).

### Qué se resolvió

| Problema | Resolución | Fecha |
|----------|-----------|-------|
| `~/.sddk-shared/` como segundo checkout | Eliminado; todo trabajo en el CWD | 2026-08-08 |
| Drift entre CWD y bundle runtime | Bundle runtime en `~/.local/share/sddk/framework/<v>/` actualizado con `sddk dev install` | 2026-08-08 |
| `bootstrap.sh` referenced `~/.sddk-shared/` | Actualizado a CWD como fuente de verdad | 2026-08-08 |
| Receipts duplicados en `~/.sddk-shared/` | Eliminados; receipts residen en `~/.local/share/sddk/projects/<id>/` | 2026-08-08 |

### Estado actual verificado (HEAD = `2af85e2`, v1.9.0)

```
CWD = ~/Proyectos/agentesIA/sddk-framework   ✅ repo de desarrollo (fuente de verdad)
~/.local/share/sddk/framework/v1.9.0/         ✅ bundle runtime (instalado)
~/.local/share/sddk/framework/current → v1.9.0 ✅ symlink activo
~/.sddk-knowledge/sddk-framework/              ✅ vault del proyecto
git status clean                              ✅
```

### Criterios de aceptación resueltos

- [x] `adopt apply` en un repo limpio no modifica `git status` (zero intrusión).
- [x] Los symlinks de opencode/zcode apuntan bajo `$SDDK_DATA_DIR/framework/current/`.
- [x] Un solo receipt por workspace; control plane ingiere identidades únicas.
- [x] 0 regresiones: `cargo test --workspace` verde (358 tests), `cargo clippy --workspace` 0 errores.

## 13. Referencias

- `crates/sddk-engine/src/paths.rs` (`resolve_xdg_paths`), `crates/sddk-cli/src/lib.rs` (`plant_workflow_manifest`, `WORKFLOW_MANIFEST`), `crates/sddk-cli/src/cycle.rs` (`load_workflow` fallback embebido), `crates/sddk-cli/src/dev_cmd.rs` (`run_dev_update`, `link_editor`, dual symlink/copy).
- `skills/_shared/persistence-contract.md`, `skills/knowledge-graph/SKILL.md`, `agents/sddk-*.md`.
- ADR-0006 (paths XDG), ADR-0001 (local-first), ADR-0009/0010 (control plane), ADR-0011 (este milestone).
- Roadmap RS-2026-08 (R1-R9); Backlog épica E12 (SDDK-1201..1208).
