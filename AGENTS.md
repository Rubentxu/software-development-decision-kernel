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

### 2.3. Workspace y alcance de verificación

- `Cargo.toml` `[workspace.package] version` = versión de desarrollo actual (puede ir
  ahead del último tag hasta `chore(release)`).
- **Durante `apply` no se exige `cargo test --workspace` después de cada cambio.**
  El agente sigue `prompts/sddk/change-scoped-testing.md`: ejecuta el lote mínimo
  justificado por el cambio/SUT y reserva el perfil completo para `verify`/release.
- Este repo es Rust, por eso sus adapters concretos usan Cargo; la política SDDK es
  **agnóstica de lenguaje/build/test runner** y debe funcionar igual en repos JVM,
  JS/TS, Python, Go, .NET, C/C++, Bazel o polyglot mediante adapters/capabilities.
- `cargo test --workspace`, clippy/fmt globales siguen siendo gates válidos en el
  **full verification profile** de este repositorio y en el flujo de release.

### 2.4. Memory + Engram
- Sesiones largas DEBEN cerrar con `engram_mem_session_summary` (goal, discoveries,
  accomplished, next steps, relevant files). Sobrevive compactaciones. Reglas en
  `~/.config/opencode/skills/...`.

### 2.5. CI local-first, cloud async

- **El gate de verificación sigue siendo LOCAL**, pero su amplitud depende de la fase:
  `apply` = scoped/progresivo; `verify`/release = perfil completo (`cargo test --workspace`
  + clippy + fmt y demás checks declarados para este repo).
- GitHub Actions cloud **NO bloquea**: sin required status checks, runs = evidencia asíncrona.
- **Prohibido** esperar runs de la nube (`gh pr checks --watch`, retrasar
  push/merge por CI) o "arreglar CI" sin reproducir en local primero.
- **Workflows en local**: `act` v0.2.89 (`/usr/local/bin/act`) + podman;
  `ubuntu-latest` mapeado a `catthehacker/ubuntu:rust-latest` vía
  `~/.config/act/actrc`. Ejemplo: `act pull_request -W .github/workflows/<wf>.yml`.
- Los minutos del plan free de GitHub están agotados — el cloud puede ni
  ejecutar; confía en el gate local.

### 2.6. Contrato de testing para agentes

- Autoridad de alcance durante implementación: `prompts/sddk/change-scoped-testing.md`.
- El agente razona en `ActiveChangeSet → ProjectTestTopology → SUT impact → verification batch`,
  no en comandos Cargo/Maven/pytest/etc.
- Los comandos concretos son mecanismos de adapter.
- Si el impacto no se puede mapear con seguridad: **fail closed y reportar la relación
  SUT/dependency/contract/test/capability que falta**. No ejecutar todo para ocultar incertidumbre.
- Tras `TEST-APPLY-001`, inventar manualmente el test scope cuando SDDK ofrece plan
  semántico es una violación de protocolo.

### 2.7. Semantic ownership (canonical authority per concept)

> Adopted from `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/`
> (ADR-001..005, SPEC-001..006). Cycle
> `p-63676b11dc0ef88f-architecture-adoption-m0-supersession`.

- **Una autoridad canónica por concepto.** Cada dominio tiene un único
  origen de verdad (e.g. una sola fact log, una sola Evidence model, un
  solo Semantic Graph, un solo Revision substrate).
- **Cuatro clases de estado.** Todo dato durable cae en exactamente una:
  *Fact* (evento append-only), *Object* (entidad con identidad durable),
  *Projection* (derivado reconstruible) o *Ephemeral* (runtime, no durable).
- **El Vault es fuente humana**, nunca autoridad del runtime. Proyecciones
  reconstruyen al runtime; el runtime no lee del Vault como autoridad.
- **Cadena explícita Goal → WorkItem → WorkflowDefinition → ExecutablePlan
  → Run.** Saltar niveles o reincorporar definiciones implícitas en los
  prompts es no conforme.

### 2.8. Agent Experience (prompt text ≠ architecture)

> Adopted from
> `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-013..018`
> (M7 milestones) y
> `07-AGENT-EXPERIENCE/OVERVIEW.md`.

- **El texto de prompt nunca es arquitectura.** Las `EffectiveInstructions`
  se compilan desde contratos tipados y versionados.
- **Skill ≠ Capability.** Una skill es un patrón de invocación; una
  capability es un permiso de side effect. Mezclar el significado rompe
  autoridad.
- **CLI docs desde una sola fuente** (`CommandRegistry`). Ningún agente
  escribe su propia tabla de comandos.
- **Ejemplos deben ser ejecutables.** Validación en compile-time o en el
  registry; nunca texto suelto en prompts.
- **Conocer un comando ≠ poder ejecutarlo.** Distinción entre `Skill`
  (declara) y `Capability` (admite).

### 2.9. Extension discipline (single architecture-decision surface)

> Adopted from
> `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/`
> (ADR-008, ADR-009, ADR-011).

- **Una surface canónica de ADRs** por proyecto: las decisiones de diseño
  de SDDK-framework viven en `docs/adr/` o
  `docs/sddk-decision-kernel-architecture/03-adrs/` o
  `~/.sddk-knowledge/<project>/adrs/`, no se mezclan.
- **ADRs históricos permanecen.** Al integrar un paquete, *mapea* sus IDs
  locales a la numeración nativa del repositorio, no renumeres la historia.
- **Una sola surface canónica por concepto extensible:** Pack extension
  boundary, Configuration conventions, etc., están definidas por una ADR
  única y el resto se ajusta a ella.

### 2.10. Consolidation rule (exactly one current roadmap)

> Adopted from
> `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/05-INTEGRATION/SUPERSESSION.md`.

- **Exactamente un documento es "Current Roadmap".** Por defecto es
  `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/03-ROADMAP/ROADMAP.md`.
- **Paquetes antiguos reciben banner `Historical / superseded`** en su
  README, apuntando a
  [`docs/architecture/README.md`](docs/architecture/README.md).
- **Adopción incompleta mientras convivan roadmaps.** Antes de importar
  un WorkItem de un paquete histórico, reconcilia con el actual.
- **Migración strangler, no flag-day.** Nueva autoridad sobre concepto
  compartido primero, adapter de compatibilidad durante ventana,
  remoción sólo al cerrar la ventana.

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
[ ] apply: ejecutar SOLO el lote scoped admitido por el cambio/SUT actual
[ ] apply: registrar qué SUT/capability/tests se ejecutaron y por qué
[ ] apply: si el impacto no es justificable, bloquear/reportar; NO lanzar todo
[ ] Si tocaste assets/: sddk dev install        # bundle runtime actualizado
[ ] Si tocaste el TUI de modelos: bash tests-e2e/tui/run.sh cuando el scope lo requiera
[ ] git status                                  # clean
[ ] git diff                                    # revisas lo que vas a commitear
[ ] commit mensaje: feat(uat): … o fix(uat): …
[ ] git push origin main                        # pusheas según el contrato de entrega activo
```

**Antes de `verify`/release** se aplica el perfil completo del repositorio:

```text
[ ] cargo build --release -p sddk-cli
[ ] cargo fmt --check
[ ] cargo clippy --workspace --all-targets -- -D warnings
[ ] cargo test --workspace
[ ] shellcheck tests/test_*.sh scripts/*.sh tests-e2e/tui/run.sh
```

El full profile no debe copiarse dentro de cada inner loop de `apply`.

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
- **Canonical architecture + roadmap:** `docs/architecture/README.md`
  (current normative; supersedes `docs/sddk-decision-kernel-architecture/`,
  `docs/sddk-2.0-architecture-consolidation/`, `docs/sddk-complete-evolution-2026-08-23/`,
  `docs/SDDK-Human-Agent-Collaboration-Evolution-Pack-2026-08-28/` and others
  listed in that entry point's "Repository historical packages (superseded)").
- **Package source (verbatim, 72 docs):**
  `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/`
- **Scoped testing contract:** `prompts/sddk/change-scoped-testing.md`
- **Scoped verification ADR:** `docs/architecture/adrs/ADR-0097-COMMON-REVISION-SUBSTRATE.md`
  (formerly `docs/sddk-decision-kernel-architecture/03-adrs/ADR-043-CHANGE-SCOPED-VERIFICATION.md`,
  now superseded; the canonical substrate ADR is ADR-0097).
- **Historial de regresiones resueltas:** `docs/history/AGENTS-history.md`
- **Estado actual del proyecto (handoff):** `docs/handoff/HANDOFF-2026-08-26-sddk-framework.md`

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
| 1 | Workspace green | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace` | exit code 0 |
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

---

## 9. Cycle supersede workflow

> Nuevo tras cycle-51 `kernel-cycle-51-supersede-first-class` (v1.66.6).

### Invocación canónica

```bash
sddk cycle supersede \
  --cycle <cycle-id> \
  --successor <successor-cycle-id> \
  --evidence-ref '["<artifact-ref>"]' \
  --lease-owner <actor> \
  --fencing-token <token>
```

Opcional con `--reason` (scope_invalid | goal_replaced | external_obsolete)
en lugar de `--successor` — exactamente uno de los dos.

### Ledger event invariant (3 events, GAP-BUG-1)

Un supersede atómico **siempre** emite **3 eventos** en el ledger (no 2):

1. `cycle.supersede.requested` — primer evento, con `successor_cycle_id`,
   `reason`, `evidence_refs`, `lease_owner`, `fencing_token`.
2. `lease.released` — efecto secundario de GAP-BUG-1 (lease release
   atómico dentro de `update_cycle_with_event(..., release_lease_on_phase_change=true)`).
3. `cycle.supersede.applied` — evento final, marca el supersede como
   persistente.

El orden importa: `lease.released` aparece entre `requested` y `applied`.
Este invariante está pinado por el test
`supersede_preserves_ledger_event_digests` en
`crates/sddk-engine/tests/cycle_supersede.rs` (N+3, asserted).

Ver `docs/sddk-decision-kernel-architecture/04-specs/SPEC-SUPERSEDE-001.md`
§5 ("Ledger invariants preserved") para el contrato completo.

### Reglas de validación

| Check | Nivel | Código de error | Mensaje |
|-------|-------|-----------------|---------|
| Lease fence | Fail-closed | `LeaseConflict` | Sin lease = rechazo inmediato |
| XOR successor/reason | CLI | Parse error | clap `conflicts_with` |
| evidence_refs no vacío | Engine | `SupersedeEvidenceRefsRequired` | MUST (SPEC §2 línea 87) |
| successor existe | Engine | `SupersedeSuccessorNotFound` | Crear el ciclo primero |
| Anti-self-supersede | Engine | `SupersedeSelfForbidden` | No superseder un ciclo consigo mismo |

### Flujo de commits (9 anchor commits)

| # | Tipo | Descripción |
|---|------|-------------|
| 1 | `fix(engine)` | GAP-BUG-1: libera lease atómicamente |
| 2 | `fix(engine)` | GAP-BUG-2/3: lease_owner y fencing_token del caller |
| 3 | `fix(cli)` | GAP-UX-1: validate_cycle_project antes de storage |
| 4 | `feat(engine)` | GAP-V-1/2/3: conflicts_with, successor exists, evidence_refs |
| 5 | `test(engine)` | 6 tests engine + 1 CLI test |
| 6 | `docs` | ADR-0079 accepted (en `~/.sddk-knowledge/`) |
| 7 | `docs` | SPEC-SUPERSEDE-001 promoted |
| 8 | `docs` | AGENTS.md §9 añadido |
| 9 | `chore(release)` | Release notes v1.66.6 |

### Notas de implementación

- `InMemoryLedger::acquire_cycle_lease` hardcodea `fencing_token=1` — tests deben usar `1`
- Receipt se escribe en `<cycle-artifacts>/<cycle-id>/supersede-receipt.json`
- Ellease se libera atómicamente en `update_cycle_with_event(..., release_lease_on_phase_change=true)`

---
Objetivos a alcanzar en esta aplicación, de lo más importante es la separación entre conocimiento, Alignment y Verification y Gobernanza.

Knowledge
────────────────────────
qué sabemos
por qué
desde cuándo
qué está stale
qué evidencia lo soporta

Software Alignment
────────────────────────
qué tensiones vemos
qué parece desalineado
qué tradeoffs existen
qué podría mejorarse
según qué lens

Verification
────────────────────────
cuándo evaluar
qué scope evaluar
cuánto profundizar
qué evidence falta
qué receipt deja verify/deb-verify

Governance
────────────────────────
qué está permitido
qué es obligatorio
qué bloquea
qué waiver existe
