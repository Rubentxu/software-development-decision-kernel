# ADR-0011 — Separación por responsabilidades: repo dev, bundle runtime, workspace de uso; cero intrusión en repos de proyectos

**Estado:** aceptada
**Fecha:** 2026-08-07
**Relacionados:** ADR-0001 (local-first), ADR-0006 (identity/paths XDG), ADR-0009/0010 (control plane)

## Contexto

`/var/home/rubentxu/.sddk-shared` concentra hoy tres responsabilidades incompatibles:

1. **Repo de desarrollo** del framework (crates, docs, CI, releases) — debe estar versionado y estable.
2. **Fuente del dev link** — `opencode.json` + symlinks apuntan a `agents/*.md` y `skills/*` de ese working tree; el editor carga el framework del repo, incluidos cambios sin commitear o checkouts de ramas.
3. **Workspace de uso/adopción** — el propio repo está adoptado (2 receipts duplicados del mismo workspace: `p-d9539a6a4bea1de0` scope `sddk-framework` y `p-52b95ef55999f9de` scope `.`), y los ciclos SDDK se ejecutan sobre él (9 ciclos, tags v0.21–v0.27).

Además, el framework escribe artefactos **dentro de los repos git de los proyectos** que lo usan:

- `workflow/workflow.yaml` plantado por `adopt apply` (`plant_workflow_manifest`).
- `sddk/{change}/...` artefactos de ciclo (proposal, spec, tasks, verify-report, release-report) del flujo legado.
- `docs/generated/inventory.md` + `workflow.md` — `sddk generate docs`.
- Directorios de bootstrap y cambios del flujo file-backed legado.

El usuario impone dos decisiones: separar los tres roles en directorios distintos **y** garantizar **cero intrusión del framework en los repos git de los proyectos** — todo el estado del framework vive en directorios de usuario (XDG), siguiendo la filosofía ya aplicada a vault/artifacts/ledger/cache/receipt y al knowledge vault (`~/.sddk-knowledge/{project}/`).

## Decisión

### 1. Tres directorios, tres responsabilidades

| Rol | Ubicación | Contenido | Adoptado | Linkado |
|-----|-----------|-----------|----------|---------|
| Repo de desarrollo | Clon canónico de `Rubentxu/software-development-decision-kernel` (p.ej. `~/Proyectos/agentesIA/sddk-framework` actualizado) | crates, docs, CI, releases | NO | NO |
| Bundle runtime | `~/.local/share/sddk/framework/` (XDG data root) | assets instalados del release (`agents/ skills/ prompts/ workflows/`) vía `sddk dev update --root` modo bundle | — | SÍ (fuente de symlinks del editor) |
| Workspace de uso | Directorios de proyectos reales del usuario | código del proyecto, sin nada del framework | SÍ | NO |

El dogfooding del framework se hace en un **clon de trabajo separado** (workspace de uso), nunca en el repo de desarrollo.

### 2. Política de cero intrusión (non-intrusive by default)

El framework **nunca escribe dentro de un repositorio git de un proyecto**. Todo estado operativo va a XDG:

| Artefacto | Antes | Ahora |
|-----------|-------|-------|
| Receipts de adopción | XDG (correcto) | se mantiene |
| Vault / artifacts CAS / ledger / cache | XDG (correcto) | se mantiene |
| Knowledge vault | `~/.sddk-knowledge/{project}/` (correcto) | se mantiene |
| Artefactos de ciclo | `sddk/{change}/...` en el repo | **XDG**: `~/.local/share/sddk/projects/<project_id>/cycle-artifacts/{cycle_id}/...` (o el CAS por hash) |
| workflow.yaml | plantado en el repo por `adopt apply` | **no se planta**: siempre se resuelve del manifest embebido o del bundle runtime |
| docs generados | `docs/generated/` en el repo | solo en el repo de desarrollo (dogfooding explícito); en proyectos, `sddk generate` escribe a XDG o se omite |

Regla de oro: **el único artefacto del framework legible/visible en el working tree de un proyecto es el código que el propio proyecto aporta**. Cualquier fichero creado por SDDK vive bajo `~/.local/share/sddk`, `~/.local/state/sddk`, `~/.cache/sddk` o `~/.sddk-knowledge`.

### 3. Cambios requeridos en el CLI

- `adopt apply`: **eliminar `plant_workflow_manifest`** (el engine ya cae al `CANONICAL_WORKFLOW` embebido cuando no hay manifest, `cycle.rs:108`). La adopción no crea ficheros en el repo.
- `cycle transition --artifact`: los agentes pasan rutas de artefactos; el CLI los registra tal cual. El cambio es en prompts/skills: los artefactos se escriben bajo `cycle-artifacts/` XDG (o vía `sddk artifact store` al CAS) y el path que se registra apunta ahí.
- `generate docs/inventory`: por defecto escribe a XDG (`~/.local/share/sddk/projects/<id>/generated/`); el repo de desarrollo puede usar `--in-repo` explícito para el dogfooding CI.
- `lint`: las comprobaciones que leen `workflow/workflow.yaml` del repo pasan a leer del manifest embebido/bundle; no exigen el fichero en el repo.

### 4. Cambios requeridos en prompts/skills

- `skills/_shared/persistence-contract.md` y `skills/knowledge-graph/SKILL.md`: los paths de artefactos pasan de rutas repo-locales a XDG y al vault canónico.
- `sddk-adopt.md` y `sddk-*.md`: actualizar los paths de artefactos y la instrucción "no escribir en el repo".
- `dev link`: los symlinks del editor apuntan al **bundle runtime** (`~/.local/share/sddk/framework/`), no al repo de desarrollo.

### 5. Limpieza del estado existente

- Eliminar los 2 receipts de adopción duplicados de `/var/home/rubentxu/.sddk-shared`.
- Mover/regenerar los artefactos de ciclo de `sddk/` del working tree a XDG (si se quieren conservar).
- Volver a linkear opencode/zcode contra el bundle runtime.

## Consecuencias positivas

- El editor es estable: los prompts/agents vienen del bundle versionado, no del working tree sucio.
- Los repos git de proyectos quedan limpios: cero ficheros SDDK, cero diff espurio.
- Los ciclos SDDK no contaminan el repo de desarrollo (sin ramas de ciclos mezcladas con el desarrollo del framework).
- Se elimina la duplicidad de receipts (identidad limpia para el control plane, ADR-0009).
- La filosofía ya existente (vault/artifacts/ledger en XDG, knowledge vault fuera del repo) se extiende al 100 % del estado operativo.

## Consecuencias negativas

- Los artefactos de ciclo ya no viajan en el repo (no son compartibles por git). Se mitiga con el CAS y con reportes HTML autocontenidos (ADR-0010) cuando se requiera compartir.
- Migración: prompts/skills y paths de artefactos existentes (9 ciclos en el proyecto actual) requieren actualización o re-derivación.
- `sddk generate docs` en el repo de desarrollo necesita un flag explícito (`--in-repo`) para el CI de dogfooding.

## Alternativas rechazadas

### Mantener el repo de desarrollo como fuente del dev link

Es el estado actual: inestable, dependiente del working tree, mezcla roles. Rechazada.

### Solo bundle runtime, sin repositorio de desarrollo separado

Imposible: el framework necesita un repo para desarrollarse a sí mismo (crates, CI, releases).

### Artefactos de ciclo dentro del repo pero gitignored

Ya ocurre hoy (`sddk/` gitignored) y sigue ensuciando el working tree, rompiendo la regla de "el repo solo contiene lo que el proyecto aporta". Rechazada.

## Disparadores de reevaluación

- Necesidad real de compartir artefactos de ciclo vía git por equipo → reevaluar un modo `in-repo` opcional explícito.
- Migración de dogfooding a un clon de trabajo que requiera acceso a artifacts del repo de desarrollo → documentar el flujo.

## Criterios de cumplimiento

- `git status` de un proyecto adoptado es idéntico antes y después de un ciclo completo (adopt → explore → propose → spec → design → tasks → apply → verify → archive).
- Ningún comando `sddk` crea ficheros bajo `WORKDIR` del proyecto salvo los que el agente crea como producto del cambio.
- Los symlinks de opencode/zcode apuntan bajo `~/.local/share/sddk/framework/`.
- No existen dos receipts de adopción para el mismo workspace.
- El control plane (ADR-0009) ingiere proyectos con identidad única.

## Multiplataforma (Linux / macOS / Windows)

La separación de responsabilidades no puede depender de convenciones Unix. Resolución por SO:

| SO | Data root (bundle, artifacts, cycle-artifacts, generated) | State root (ledger) | Cache | Editor config |
|----|-----------------------------------------------------------|---------------------|-------|---------------|
| Linux | `$XDG_DATA_HOME/sddk` o `~/.local/share/sddk` | `$XDG_STATE_HOME/sddk` o `~/.local/state/sddk` | `$XDG_CACHE_HOME/sddk` o `~/.cache/sddk` | `~/.config/opencode`, `~/.zcode` |
| macOS | `~/Library/Application Support/sddk` (o `$XDG_DATA_HOME/sddk` si se define) | `~/Library/Application Support/sddk` (state) | `~/Library/Caches/sddk` | `~/.config/opencode` (opencode usa XDG incluso en macOS), `~/.zcode` |
| Windows | `%APPDATA%\sddk` | `%LOCALAPPDATA%\sddk` | `%LOCALAPPDATA%\sddk\cache` | `%APPDATA%\opencode` (o el dir real del editor) |

Decisiones:

1. **Introducir el crate `dirs`** (o `directories`) en `sddk-engine/src/paths.rs`: `resolve_xdg_paths` mantiene los overrides explícitos `XDG_*` (determinismo en tests/CI) y cae a `dirs::data_dir()/state_dir()/cache_dir()` por SO cuando no hay override. Hoy el fallback es `HOME/.local/share`, que **falla en Windows estándar** (no existe `HOME`).
2. **macOS**: respetar la convención de Apple (`~/Library/...`) con overrides XDG aún posibles; `dev link` sigue apuntando a `~/.config/opencode` porque opencode resuelve así su config en todos los SO.
3. **Windows**: el framework hoy no publica builds Windows y `install.sh` es bash. Windows queda **fuera del alcance de releases** (disparador de reevaluación: primer consumidor Windows con necesidad real). No obstante, el código no debe contener `std::os::unix::` sin `#[cfg(unix)]` — el dual symlink/copy ya existe en `dev_cmd.rs` y se conserva.
4. **Bundle runtime path estable**: `FRAMEWORK_DIR` se normaliza a `dirs::data_dir()/sddk/framework` (Linux: `~/.local/share/sddk/framework`). `install.sh` ya define `SDDK_FRAMEWORK_DIR` con default `~/.sddk-shared/framework` — se alinea el default con el data root XDG y se documenta la variable de override.
5. **Knowledge vault** (`~/.sddk-knowledge/`): se mantiene en home por decisión del usuario (es su convención personal); se documenta que en equipos/CI puede apuntarse a un dir compartido vía variable.

Criterio multiplataforma: `cargo test` pasa en los 3 SO (o al menos linux + darwin, los targets publicados); ningún test asume paths Unix hardcoded (los tests de `paths.rs` usan overrides explícitos y deben extenderse con un caso `dirs`-fallback).

## Modelo inspirado en asdf-vm

asdf-vm resuelve el mismo problema de fondo (runtimes versionados **fuera** de los repos de proyectos, resolución por directorio, múltiples versiones conviviendo, shims como única superficie de integración). Se adoptan sus mecanismos:

### 1. Data dir único y configurable (equivalente a `ASDF_DATA_DIR`)

asdf: `$ASDF_DATA_DIR` (default `$HOME/.asdf`) aloja **todo** el estado: `plugins/`, `installs/<tool>/<version>/`, `shims/`.

SDDK: `$SDDK_DATA_DIR` (default `dirs::data_dir()/sddk`) aloja todo el estado operativo del framework:

```text
~/.local/share/sddk/                  # $SDDK_DATA_DIR (override env)
├── framework/                        # bundles instalados (equivalente a installs/)
│   └── 1.3.0/                        # una versión concreta
│       ├── agents/
│       ├── skills/
│       ├── prompts/
│       └── workflows/
├── projects/                         # estado por proyecto (ya existe: ledger, vault, artifacts, cycle-artifacts, generated, receipt)
├── control-plane/                    # ADR-0009: store central de telemetría
└── current -> framework/1.3.0        # versión activa global (symlink)
```

### 2. Múltiples versiones de bundle conviviendo (equivalente a `asdf install <tool> <version>`)

- `sddk dev install 1.3.0` y `sddk dev install 1.4.0` conviven en `framework/<version>/`.
- `sddk dev use 1.4.0` actualiza el symlink `current`.
- `dev link`/`dev doctor` operan siempre sobre `current` → el editor es estable aunque se instale una versión nueva.
- El repo de desarrollo del framework **deja de ser** fuente del dev link: se puede desarrollar `main` mientras el editor corre `1.3.0` (o `path:` — ver punto 4).

### 3. Resolución de versión por directorio (equivalente a `.tool-versions`)

asdf busca `.tool-versions` desde `$PWD` hacia arriba hasta `$HOME`. SDDK propone un fichero declarativo por proyecto:

```text
# .sddk-versions  (en la raíz del repo del proyecto — ÚNICA excepción a cero intrusión)
sddk 1.4.0
```

**Distinción crítica**: `.tool-versions`/`.sddk-versions` NO es "framework dentro del repo" — es **configuración declarativa del proyecto** (qué versión usar), igual que `package.json`, `go.mod` o `rust-toolchain.toml`. El framework nunca lo escribe (lo gestiona `sddk use` del desarrollador); el estado operativo sigue 100 % en XDG. Sin este fichero, la resolución cae a la versión global (`current`) — el proyecto funciona igual, solo sin pin por repositorio.

Mecánica de resolución (solo lectura del repo):
1. `$PWD/.sddk-versions` → 2. `.sddk-versions` en padres → 3. `$SDDK_DATA_DIR/current` (global).
4. Valor `path:<dir>` → apunta a un working tree local (dogfooding controlado: un desarrollador del framework puede apuntar al repo de desarrollo **explícitamente** y solo mientras lo necesita; es el mismo `path:` de asdf para language developers).

### 4. Plugins y hooks fuera del repo

asdf: los plugins viven en `$ASDF_DATA_DIR/plugins/`, no en el repo del proyecto. SDDK ya sigue esta filosofía: skills/agents/prompts se linkean desde el bundle (punto 2); no se copian al repo del proyecto. El knowledge vault ya está fuera (`~/.sddk-knowledge/{project}/`).

### Traducción de responsabilidades (tabla final)

| Concepto asdf | SDDK |
|---------------|------|
| `~/.asdf` (ASDF_DATA_DIR) | `~/.local/share/sddk` (SDDK_DATA_DIR) |
| `installs/<tool>/<version>/` | `framework/<version>/` (bundle) |
| `shims/` en PATH | symlinks de opencode/zcode → `current` |
| `.tool-versions` | `.sddk-versions` (declarativo, 1 línea, opcional) |
| `asdf set/install/use` | `sddk dev install/use` |
| `path:` version | `path:` para dogfooding del repo de desarrollo |
| `ASDF_CONFIG_FILE` (`.asdfrc`) | `SDDK_CONFIG_FILE` (futuro, si hace falta) |

## Referencias

- Spec: `docs/responsibility-separation/SPEC.md` (layout, resolución de versión, comandos, matriz de escritura, fases RS-1..RS-7, tests).
- Roadmap: milestone RS-2026-08 (R1-R9) en `docs/sddk-stabilization-plan/ROADMAP.md`.
- Backlog: épica E12 (SDDK-1201..1208) en `docs/sddk-stabilization-plan/BACKLOG.md`.
- PRD: RF-018 (no intrusión), RNF-008 (versionado de bundles), RNF-009 (portabilidad de paths).
- `crates/sddk-cli/src/lib.rs` (`plant_workflow_manifest`, `WORKFLOW_MANIFEST`), `crates/sddk-cli/src/cycle.rs` (`load_workflow` fallback embebido), `crates/sddk-cli/src/dev_cmd.rs` (`run_dev_update` checkout vs bundle, `link_editor`), `crates/sddk-engine/src/paths.rs` (`resolve_xdg_paths`).
- `skills/_shared/persistence-contract.md`, `skills/knowledge-graph/SKILL.md`, `agents/sddk-*.md`.
- ADR-0006 (paths XDG), ADR-0001 (local-first).
