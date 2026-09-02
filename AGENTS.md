# AGENTS.md — sddk-framework

> Convenciones, layout y reglas que todo agente (humano o IA) debe respetar.
> Léelo antes de hacer cambios — la separación de directorios es **estructural**,
> no cosmética, y romperla contamina el bundle runtime.

---

## 1. Contexto del proyecto

`sddk-framework` es el **repo de desarrollo** (NO adoptado) del framework SDDK.
Contiene crates, docs, CI, releases, agents/skills/prompts **fuente**. Todo cambio,
commit, push y release se hace desde `~/Proyectos/agentesIA/sddk-framework/` (CWD).
El proyecto **nunca escribe dentro de otros repos de proyectos** (regla "cero
intrusión", ver `docs/responsibility-separation/SPEC.md`). El bundle runtime
vive en `$SDDK_DATA_DIR/framework/<version>/` (`~/.local/share/sddk/framework/<v>/`)
y se actualiza con `sddk dev install`.

---

## 2. Convenciones duras (no negociables)

### 2.0. Frontera de namespace

- Gentle AI SDD y SDDK son sistemas distintos. Sus agentes, skills, prompts y
  contratos de persistencia no se mezclan.
- El nombre historico "SDD-kernel" queda normalizado a **SDDK**.
- La superficie activa es `orchestrator`, `sddk-*` y `prompts/sddk/`. Sin aliases.

### 2.1. Commits

- **Conventional Commits** en español: `feat(uat): …`, `fix(uat): …`, `chore(release): …`.
  Sin `Co-Authored-By` ni atribución a IA.
- Una concernencia por commit. Si un cambio toca docs + código, un solo commit con la
  concernencia explicada en el body.
- Commits a `main` via `git push origin main` (no PRs — proyecto lineal con tags `vX.Y.Z`).
- **Activación del hook de prevención**: `git config core.hooksPath githooks` (local por checkout;
  el hook rechaza push a main sin commit `chore(release): bump version`).

### 2.2. Branch model

- `main` es la rama única de desarrollo + releases. No hay `develop`, `release/*` ni hotfix.
- Cualquier feature se commitea directo a `main` (o se squash-margea en PRs externos).

### 2.3. Workspace

- `Cargo.toml` `[workspace.package] version` = versión de desarrollo actual (puede ir
  ahead del último tag hasta `chore(release)`).
- `cargo test --workspace` verde + `cargo clippy --workspace` sin errores antes de commitear.

### 2.4. Memory + Engram
- Sesiones largas DEBEN cerrar con `engram_mem_session_summary` (goal, discoveries,
  accomplished, next steps, relevant files). Sobrevive compactaciones. Reglas en
  `~/.config/opencode/skills/...`.

### 2.5. CI local-first, cloud async

- **El gate de verificación es LOCAL**: `cargo test --workspace` + `cargo clippy
  --workspace` + fmt antes de commitear (ver checklist §5). GitHub Actions cloud
  **NO bloquea**: sin required status checks, runs = evidencia asíncrona.
- **Prohibido** esperar runs de la nube (`gh pr checks --watch`, retrasar
  push/merge por CI) o "arreglar CI" sin reproducir en local primero.
- **Workflows en local**: `act` v0.2.89 (`/usr/local/bin/act`) + podman;
  `ubuntu-latest` mapeado a `catthehacker/ubuntu:rust-latest` vía
  `~/.config/act/actrc`. Ejemplo: `act pull_request -W .github/workflows/<wf>.yml`.
- Los minutos del plan free de GitHub están agotados — el cloud puede ni
  ejecutar; confía en el gate local.

---

## 4. Reglas de oro

### 4.1. Trabajar SIEMPRE desde el CWD (`sddk-framework/`)

- ✅ `cd ~/Proyectos/agentesIA/sddk-framework && git … && cargo …`
- ❌ `cd ~/.sddk-shared/ && …` — viola la regla "single source of truth en el CWD".
  **No crear nuevos checkouts en `~/.sddk-shared/`.**

### 4.2. El bundle runtime vive en `~/.local/share/sddk/framework/<v>/`

- Se actualiza con `sddk dev install` (o `sddk dev update`).
- **No es un checkout de git.** Es un snapshot publicado.
- **No edites directamente `~/.local/share/sddk/...`** — se sobrescribe en el próximo install.

### 4.3. El bundle runtime NO es un checkout del repo

- `agents/`, `skills/`, `prompts/` son **copias**, no symlinks. `bootstrap.sh`
  los symlinkea a los directorios de cada editor.

### 4.4. Las decisiones de diseño tienen una superficie canónica por alcance

- `docs/adr/` (este repo) — ADRs del proyecto público.
- `docs/sddk-decision-kernel-architecture/03-adrs/` — ADRs de la arquitectura
  objetivo activa; su roadmap vive en `02-roadmap/`.
- `~/.sddk-knowledge/<project>/adrs/` — ADRs de proyectos adoptados.
- Specs del plan en `~/.sddk-knowledge/<project>/specs/`.
- **Debt lifecycle** (ADR-0047): severity taxonomy in [`docs/debt/SEVERITY.md`](docs/debt/SEVERITY.md), priority taxonomy in [`docs/debt/PRIORITY.md`](docs/debt/PRIORITY.md), index in [`docs/debt/README.md`](docs/debt/README.md). Cycle-7b will add JSON Schema + INC template + agent/workflow/prompt integrations.

---

## 5. Checklist antes de commitear

```text
[ ] cargo build --release -p sddk-cli            # compila
[ ] cargo test --workspace                       # verde
[ ] cargo clippy --workspace --all-targets -- -D errors    # 0 errores (gate duro, cycle-17+)
[ ] shellcheck tests/test_*.sh scripts/*.sh tests-e2e/tui/run.sh    # parity con just ci (Phase C gated — stable)
[ ] Si tocaste assets/: sddk dev install        # bundle runtime actualizado
[ ] Tras release: sddk dev doctor | grep bundle_coherence (binario == bundle)
[ ] Si tocaste el TUI de modelos: bash tests-e2e/tui/run.sh
[ ] git status                                  # clean
[ ] git diff                                    # revisas lo que vas a commitear
[ ] commit mensaje: feat(uat): … o fix(uat): …
[ ] git push origin main                        # pusheas
```

---

## 6. Resumen en una línea

> **El proyecto es el CWD** (`sddk-framework/`). El bundle runtime vive
> en `~/.local/share/sddk/framework/<v>/` (instalado por `sddk dev install`).
> Todo cambio de código va al CWD; todo cambio de contenido publicable se
> copia al bundle con `sddk dev install`.

---

## 7. See also

- **Release & distribution:** `docs/RELEASING.md`
- **Architecture model:** `docs/ARCHITECTURE-MODEL.md`
- **Historial de regresiones resueltas:** `docs/history/AGENTS-history.md`
- **Estado actual del proyecto (handoff):** `docs/handoff/HANDOFF-2026-08-26-sddk-framework.md`
- **Roadmap de arquitectura:** `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md`

---

## 8. Flujo canónico de release (auto-install local)

> Estandarizado tras cycle-46 (install coherence) y cycle-47 (install
> consolidation). **Cada release que se cree debe completar todo el flujo
> completo y autoinstalarse en local después de distribuirse por los
> mecanismos normalizados** (GitHub Releases). Hacer un release parcial
> (binario sin bundle, GH Release sin install, install sin prune) deja
> el local desincronizado de lo que ven los usuarios — prohibido.

**Entry point canónico:** `bash scripts/release.sh`.

Ese script ejecuta los 13 pasos abajo en orden, gateados por el previo.
Cualquier paso que falle aborta con código no-cero. Si el script no puede
correr en tu entorno, el equivalente manual está en `docs/RELEASING.md`
sección "Manual fallback" — pero la regla es: **si no puedes correr el
script, abre un ciclo para arreglar lo que sea que te lo impide, no
hagas un release a medias**.

### Pipeline (13 pasos)

| # | Paso | Gate | Cómo se verifica |
|---|------|------|------------------|
| 0 | Preflight | `gh auth status`, branch `main`, tree limpio, HEAD = `chore(release): bump version` | `git log -1 --format=%s` matchea regex |
| 1 | Workspace green | `cargo fmt --check`, `cargo clippy -D errors`, `cargo test --workspace` | exit code 0 |
| 2 | Read version | de `Cargo.toml` workspace.package.version | regex `^v?[0-9]+\.[0-9]+\.[0-9]+` |
| 3 | Build binary | `cargo build --release --bin sddk` | `$BIN --version` |
| 4 | Manifest | `$BIN dev manifest --root .` + `--verify` | `verify_manifest` sin mismatches (RDI) |
| 5 | Bundle tarball | `tar czf` con prefix `software-development-decision-kernel/` | `bundle.tar.gz.sha256` |
| 6 | BUNDLE.toml (v2) | `schema_version=2`, `bundle.{version,binary_min_version,binary_max_version}`, `contents.manifest_sha256` | `sddk dev install` lo valida (fail-closed) |
| 7 | Unified tarball | `bin/sddk` + `framework/` con `chmod 0755` defensivo sobre el binario | `tar tvzf …` muestra `-rwxr-xr-x` |
| 8 | sha256 + CHECKSUMS + sbom | CycloneDX 1.5 mínimo | existe `sddk.sha256`, `CHECKSUMS`, `sbom.json` |
| 9 | `gh release create` | assets en un solo comando | `gh release view $TAG --repo …` |
| 10 | Install desde URL real | `bash scripts/install.sh --version $TAG --editor all` (sin `SDDK_BASE_URL`) | exit 0, `bin/sddk` extraído con exec bit |
| 11 | `sddk dev doctor` | `--prefix $SDDK_PREFIX` | `binary.bundle_coherence: present` + `all_present: true` |
| 12 | `sddk dev update --prune-only --keep 1` | elimina `<version>/` stale | "removed N, kept 1.X.Y" |
| 13 | Final state | print binary version, bundle version, current symlink, framework layout | output legible |

### Flags del script

```bash
bash scripts/release.sh               # flujo completo
bash scripts/release.sh --dry-run     # solo pasos 0-8 (no publica)
bash scripts/release.sh --skip-tests  # asume que ya corriste los gates
bash scripts/release.sh --skip-install # pasos 0-9 (no toca local)
bash scripts/release.sh --force       # sobrescribe release existente en GH
```

### CD y CDN cache: por qué step 10 espera

GitHub Releases se sirve desde una CDN edge. `gh release upload --clobber`
sube el nuevo asset al storage de GH pero el CDN puede servir el viejo
hasta ~5 minutos después (verificado durante cycle-47 D4: `curl` desde
`/releases/download/...` devolvía el binario viejo aunque `gh release
download` reportase el nuevo). El script hace **poll al sha256 del binario
contra la URL pública** antes de invocar `install.sh`. Si la CDN sigue
sirviendo stale tras 5 min, aborta con error claro. No intentar saltarse
este poll con `--skip-install`; en su lugar, esperar a que la CDN refresque.

### Por qué dos commits (`feat` + `chore(rerelease)`)

El pre-push hook (`githooks/pre-push`) rechaza cualquier push a `main`
que no contenga al menos un commit cuyo subject matchee
`^chore\(release\): bump version`. Mezclar el bump dentro del commit de
feature pasa el `git commit` pero falla en el `git push`, y deshacer es
engorroso. Convención: commit de feature primero, commit de bump después,
push único.

### Después del release: ciclo

Una vez publicado, el ciclo se cierra escribiendo
`archive-manifest.md` en el directorio del ciclo con la lista de commits,
URL del GH Release, y estado local final (`binary`, `bundle`, `current`,
`receipt`, `doctor`). El cierre formal del ciclo vía CLI es operativo
desde v1.66.1 (`validate_cycle_project`, `sddk cycle lock acquire`) y
v1.66.2 (`Storage::cycle_exists`, INC-DEBT-017). Seguimos usando
`archive-manifest.md` como ground-truth durable del cierre (rol no
reemplazado). **F4 gotcha:** bare slugs sin `project_id/` prefix (p.ej.
`p-63676b11dc0ef88f/gap6-lock-repair` sin el prefijo de proyecto) disparan
`STORAGE_NOT_FOUND` aunque la fila exista — la causa viva es la falta de
normalización del bare-slug en `validate_cycle_project`.

