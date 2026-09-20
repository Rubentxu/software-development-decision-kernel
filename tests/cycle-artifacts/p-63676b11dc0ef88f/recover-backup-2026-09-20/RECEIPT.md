# RECEIPT — recover-backup-2026-09-20 — recuperar commits del branch backup pre-sync

> **Slice id:** `p-63676b11dc0ef88f/recover-backup-2026-09-20`
> **Operador:** orchestrator (auto, esta sesión)
> **Baseline (released):** v1.169.121 → `97fa81b`
> **Released:** v1.169.122 → `94f7488`
> **Estado:** **CLOSED** (23 commits publicados, release con doctor y binario coherente).

## §1 Motivación

El resumen de la sesión previa declaraba "todo lo ejecutable sin decisión humana está agotado y verificado". La investigación de alcance A6 (S7 closeout) reveló que un macro-ciclo A6 (Static Enhanced Readiness) estaba **parcialmente cerrado en un branch local** (`backup/pre-sync-local-main-2026-09-20`) que nunca viajó a `origin/main`. 28 commits con:

- S2 UAT C05/C07/C09 (`32ac759`)
- S3 AC10 bridge AnalysisResult → ObservationSet (`c917393`)
- S4 durability Option B (`3e37300`, `31dd60f`) — resolvía PR-UAT-024
- Re-export `runner_receipt` perdido del gateway (`ab9fe23`)
- 5 docs AIW crosswalk / decisiones operator
- AIW-S1b cancellation (`8272139`, `c0bcad7`)

El "exhausted" era falso: la memoria de sesión perdió visibilidad sobre el estado local divergente.

## §2 Reconciliación (3-way diff backup vs main)

| Concern | Backup (no en main) | Main (al momento del diff) | Resolución |
|---|---|---|---|
| A6 S2 UAT C05/C07/C09 | test file + receipt | sólo S1 / S4 / S7 | cherry-pick limpio (`32ac759`) |
| A6 S3 AC10 bridge | `evidence_source_static_provider.rs` + mod.rs | mod.rs sin bridge | cherry-pick limpio (`c917393`) |
| A6 S4 durability | `Deserialize` en observation/types + schema `observation.set.appended` v1 + `observation_set_durability.rs` | `Serialize` solo | cherry-pick limpio (`3e37300`) |
| AIW-S2 re-export | `pub use runner_receipt::*` en gateway/lib.rs | import path directo via `runner_receipt::` | cherry-pick + conflicto en e2e resuelto a favor de import module-path (más limpio) |
| Cycle replan fix | `state_after` en ambos events, `verify_cycle_snapshot` re-aplica | `fb1fc80` ya lo hacía por otro path (`.applied` event) | saltado: `6b97202` redundante |
| Workaround pins | `6532d3a`, `f349e69` | obsoletos por `fb1fc80` | saltados |
| AIW-S3 storage | `619f7e5` | ya en main vía v1.169.97 | saltado |
| AIW-S4 expansion | `2b02b4c` | ya en main vía v1.169.98 | saltado |

Resultado: **21 commits aplicados, 7 saltados por redundancia con main**. 2 conflictos (gateway e2e, Cargo.lock) resueltos a favor de la dirección de main (import module path, lockfile de main).

## §3 Gates (todos verdes)

| Gate | Resultado |
|---|---|
| `cargo fmt --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo test --workspace` | 248 ok / 0 fail (incluye los 4 nuevos binaries `a6_s2_uat_c05_c07_c09`, `a6_s3_ac10_verify_integration`, `observation_set_durability` y los `cycle_replan` pins) |

## §4 Release v1.169.122

| Paso | Resultado |
|---|---|
| 0 Preflight | git clean, branch main, bump commit presente |
| 3 Build | `cargo build --release --bin sddk` OK |
| 4 Manifest | `sddk dev manifest --root .` 377 files hashed |
| 5 Bundle tarball | ok |
| 6 BUNDLE.toml v2 | ok |
| 7 Unified tarball | exec bit presente (`sddk-v1.169.122-sddk-linux-x86_64-musl.tar.gz`) |
| 8 sha256 + CHECKSUMS + sbom | ok |
| 9 gh release create | 9 assets subidos |
| 9b Public release gate | polling 6 URLs HTTP 200 (sha256 vs CDN consistente) |
| 10 install real | `bash scripts/install.sh --version v1.169.122 --editor all` exit 0 |
| 11 doctor | `all_present: true`, `binary.bundle_coherence: present` con `--prefix /home/rubentxu` |
| 12 prune-only keep 1 | "removed 0, kept 1.169.122" |
| 13 final state | binario 1.169.122, bundle 1.169.122, current symlink 1.169.122 |

## §5 Completion guard

| Verificación | Resultado |
|---|---|
| `git rev-parse HEAD` | `94f7488cd51eb38463bfe9a9da099431f3814117` |
| `git ls-remote origin v1.169.122` | `94f7488cd51eb38463bfe9a9da099431f3814117 refs/tags/v1.169.122` |
| `HEAD == origin/main` | sí |
| Tag remoto apunta al SHA del release | sí |
| Binario instalado coherente con asset publicado | sí (`/home/rubentxu/.local/bin/sddk` v1.169.122, sha256=110ab65c6...) |

## §6 Hallazgos colaterales

1. **R18 stale receipt**: `/home/rubentxu/.local/share/sddk/sddk-install.json` quedó como v1.145.1 desde una instalación inicial (2026-09-09). El binario canónico usa `/home/rubentxu/.local/bin/sddk-install.json` (correcto), pero el recibo de la carpeta `share/sddk` queda stale. Sin impacto funcional: el script de release escribe el binario en `bin/sddk` y su receipt vecino. **Acción:** abrir ciclo para limpieza de recibos stale (no urgente, no bloquea).

2. **`sddk dev doctor --prefix`**: requiere el prefix padre de `bin/`, no el prefix completo (`/home/rubentxu`, no `/home/rubentxu/.local`). Ya documentado en el output de `--help`.

3. **Contextual false-exhausted**: la memoria de sesión previa perdió visibilidad del branch backup. Investigación profunda del alcance A6 reveló el gap. Refuerza la necesidad de `mem_session_end` antes de declarar agenda agotada.

## §7 Siguiente paso

Cerrada la recuperación, el backlog real es ahora:

1. **A6 S5 — Real CogniCode EXT** (`COGNICODE_MCP_BIN` ausente → NOT_EVALUATED en S7 closeout)
2. **A6 S6 — Fake relocation dev-deps + test-support** (Option A cheap: `pub(crate)` sin feature flag)
3. **A8 — FULLY_ENHANCED orchestration** (blocked_by A6 + A7, ahora ya cumplidos en main)
4. **Paperwork P2** — `INC-PUSH-DERIVED-METADATA-NO-ADMISSIBLE-PATH` (en progreso, hook fix ya en main vía `41b4bc1`)
5. **A8 release receipt** — el macro-cycle A8 ya tiene tests en main (v1.169.103-107) pero no veo un RECEIPT cerrado tipo `A8-CURRENT-ROADMAP.md`. Verificar.

## §8 Archivos del ciclo

- `tests/cycle-artifacts/p-63676b11dc0ef88f/recover-backup-2026-09-20/RECEIPT.md` (este archivo)
- 21 commits cherry-picked en `git log main` (hash arriba)
- Tag remoto `v1.169.122` → `94f7488`

**Zero archivos nuevos de src creados por este ciclo** — sólo se restauró trabajo preexistente.
