# Changelog

All notable changes to this project are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [1.66.5] - 2026-09-02

### Documentation
  - docs(agents): corrige la narrativa de `AGENTS.md §8` sobre el cierre formal del ciclo. La nota que decía "actualmente roto" se elimina; el cierre CLI ya es operativo desde v1.66.1 (`validate_cycle_project`) y v1.66.2 (`Storage::cycle_exists`, INC-DEBT-017). Preserva el rol durable de `archive-manifest.md` como ground-truth del cierre (líneas 208 + 213).
  - docs(roadmap): cierra el GAP-6 dentro de `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md` bajo un heading `### GAP-6 — Closed by v1.66.1 + v1.66.2 (cycle-50 bis not needed)`. El texto original queda preservado dentro de un blockquote `> **Original (archived for audit):**` para audit trail. Rationale del cycle-57 + INC-DEBT-017 nombrados.
  - docs(backlog): añade un candidato BSG (`## Candidate BSG — CLI bare-slug cycle-id acceptance (deferred)`) en `docs/sddk-decision-kernel-architecture/02-roadmap/BACKLOG.md`, posicionado tras el bloque `Out of scope (v1)` del Epic LF. Owner: orchestrator. Priority: P3. Incluye referencia sha256 al exploration-report para audit. Symptoma vivo documentado en el F4 gotcha de `AGENTS.md §8`.
  - docs(roadmap): añade el Epic LF (Ledger Forensic) + candidatos cycle-55/56 (`pausa de ciclo` y `backlog como objetos del ledger`) a `ROADMAP.md`. Documenta la pausa de ciclo como objeto del ledger y el backlog como fuente viva de candidatos. 0 Rust, 0 Cargo, 0 tests; solo prosa.
  - docs(debt): flip de frontmatter `status: open → resolved` + `resolved_by` + `## Closure Evidence` para los 2 INC carry-over de v1.66.3 (`INC-CYCLE-13-APPLY-TEST-COUNT-MISREPORT`, `INC-CYCLE-13-LOC-OVERAGE`) que ya habían sido resueltos por cycle-13. Coherencia con el contrato de cierres INC definido en `docs/debt/INC-TEMPLATE.md`. Commit `bb263bd`.

Cycle path: A-min (docs-delivery). Diff composition: 3 markdown files / +34 / -7 (rama `fix/gap6-cycle-lock-repair`) — no Rust, no Cargo, no tests, no prompts/skills. Verify verdict: PASS_WITH_WARNINGS (W1 cosmetic, W2 release-time gate). Debt verdict: PASS (zero findings). Sin cambios en runtime, binario, bundle, fixtures ni public APIs.

## [1.66.4] - 2026-09-02

### Refactored
  - refactor(domain): extrae `build_corpus_envelope(seq, event_type, payload)` a un helper dentro de `#[cfg(test)] mod tests` (`crates/sddk-domain/src/event_registry/validator.rs`). Los 28 lines de construcción inline de `EventEnvelopeV1` colapsan a 1 call site; el helper aplica `+1` internamente para preservar el contrato anti-double-increment; byte-equivalence verificada en `corpus_replay_through_validator` (evt-corpus-1..18, sequence 1..18). Cierra `INC-CYCLE-14-CORPUS-FIXTURE-DUPLICATION` (LOW/P3, duplication cluster).

### Documentation
  - docs(rustdoc): `severity_for_event_type` en `crates/sddk-domain/src/projections/journal.rs` gana una sección `# Severity policy cross-reference` que explica la consolidación de 7 filas vs 8 categorías (evidence.* + uat.*) + exclusión pack/runtime. La tabla locked de 7 ramas (líneas 67-93) queda intacta; el test `journal_projection_severity_table_locked` sigue verde. Cierra `INC-CYCLE-14-SEVERITY-SPEC-DRIFT` (LOW/P3, coupling cluster, SPEC-027).
  - docs(rustdoc): 3 helpers de `crates/sddk-engine/src/event_bus/correlation.rs` (`with_correlation_from_context`, `with_causation`, `trace_causation_chain`) ganan una sección rustdoc `# Production wiring` que documenta el diferido a M6 SPEC-028. 0 callers de producción (solo el re-export en `event_bus/mod.rs:11`); 6 tests de los helpers pasan. Cierra `INC-CYCLE-14-HELPER-DOC-GAP` (LOW/P3, doc-quality cluster).
  - docs(rustdoc): `crates/sddk-engine/tests/adoption.rs` líneas 44-46 — el comentario impreciso `// durability-required:` se reformula a `// File-based for fixture consistency`. El cuerpo del test (líneas 48-61) queda inalterado; `same_basename_different_remotes_and_scopes_do_not_collide` sigue verde. Cierra `INC-CYCLE-13-DURABILITY-COMMENT-ACCURACY` (LOW/P3, doc-quality cluster).
  - docs(debt): housekeeping — flip de frontmatter `status: open → resolved` + `resolved_by` + `## Closure Evidence` para los 2 INC carry-over de v1.66.3 (`INC-CYCLE-13-APPLY-TEST-COUNT-MISREPORT`, `INC-CYCLE-13-LOC-OVERAGE`); commit `0d6dda4`. Coherencia con el contrato de cierres INC definido en `docs/debt/INC-TEMPLATE.md`.
  - docs(release): notas post-release para v1.66.4 (`docs/releases/v1.66.4.md`) mencionan los 4 INC ids resueltos en código + 2 INC carry-over housekeeping + la estrategia de release.

## [1.66.3] - 2026-09-01

### Fixed
  - fix(cli): `sddk dev test count-workspace` ahora parsea el output de texto de `cargo test` (regex `^test result:\s+ok\.\s+(\d+)\s+passed`) en lugar de los eventos JSON que `cargo 1.91 --message-format=json --no-run` no emite. Reemplaza el parser roto `CargoMessage`/`CargoTarget`/`CargoTestResult` con una función pura `parse_cargo_test_output` y 8 unit tests (empty, dedup, FAILED-excluded, leading whitespace, ground-truth 1739-line scale, multi-binary, single-binary, no-running-lines). Live binary reporta correctamente `total_workspace_tests: 1747 / test_binaries: 79` (antes: 0). Cierra `INC-CYCLE-13-APPLY-TEST-COUNT-MISREPORT` (P2/medium).

### Refactored
  - refactor(engine): extrae `mk_*` builders de `crates/sddk-engine/tests/port_contracts.rs` a `crates/sddk-engine/tests/common/` para que el archivo de contratos no acumule LOC adicional cuando se añadan escenarios futuros. ADR-0048 supersede el budget per-file por budget total-de-módulo.

### Documentation
  - docs(apply): `apply-progress.yaml` ahora exige `sddk dev test count-workspace` como fuente única de `total_workspace_tests` (prohibido recalcular o estimar). Coherencia byte-for-byte entre apply report y live binary stdout.
  - docs(debt): cierra `INC-DEBT-017` — el helper storage-layer `cycle_exists` + 4 pre-checks en acquire/renew/release/status (`fix/storage-cycle-lease-pre-existence-check`, v1.66.2) eliminan el drift que `fix/gap6-foreign-cycle-typed-error` (v1.66.1) dejaba fuera de scope. Contrato completo de errores tipados para `sddk cycle lock`: `foreign project → STORAGE_CYCLE_PROJECT_MISMATCH`, `own-project-missing → STORAGE_NOT_FOUND` (no más FK leak engañoso con `STORAGE_DATABASE`).

## [1.63.0] - 2026-08-31

### Added
  - feat(uat): artefacto unificado `sddk-<TAG>-<ASSET>.tar.gz` por release (nuevo job `unified-artifact` en `release.yml` que combina binario + bundle + BUNDLE.toml + manifest_sha256). `scripts/install.sh` lo prefiere cuando existe; cae al path legacy (binario + bundle por separado) si no.
  - feat(uat): schema `BUNDLE.toml` v2 (`schema_version`, `bundle.version`, `bundle.binary_min_version`, `bundle.binary_max_version`, `contents.manifest_sha256`) en `sddk-cli` (`crate::dev::bundle_manifest`). 6 tests nuevos: round-trip, exact-match, rejects-older, accepts-range, missing-file, unsupported-schema. Verifica compat semver-aware (pre-release `-rc.N` rankea bajo).
  - feat(uat): `InstallReceipt` extendido a `schema_version = 2` con `bundle_version`, `bundle_sha256`, `bundle_path`, `coherence_checked`. `sddk dev install --source` lo escribe tras verificar `BUNDLE.toml` contra el binario (fail-closed: nunca escribe receipt parcialmente).
  - feat(uat): doctor check `binary.bundle_coherence` v2 (3 condiciones: `receipt.version == CARGO_PKG_VERSION`, `bundle_version` matchea dir activo o BUNDLE.toml version, `verify_bundle_compat` dentro del rango declarado). `sddk dev doctor --prefix <P>` permite layout split-prefix (binario y bundle en directorios distintos).
  - feat(uat): `sddk dev manifest --bundle` regenera `BUNDLE.toml` con `manifest_sha256` del MANIFEST presente.
  - feat(install): `scripts/install.sh` rediseñado atómico (stage → apply → rollback on failure, trap cleanup de TMP_DIR). Detecta el tarball unificado cuando existe; legacy split-asset path preserva compatibilidad.

### Fixed
  - fix(install): INSTALL_BIN layout rustup-aware (`$PREFIX/bin/sddk` por defecto, `$PREFIX/sddk` cuando prefix termina en `/bin`). Antes asumía siempre el segundo y fallaba en el layout moderno.
  - fix(install): download_optional() ahora soporta URLs `file://` (testing local contra mirror).
  - fix(uat): 3 tests existentes (`uninstall_removes_prefix_and_editor_symlinks`, `verify_detects_tampered_bundle`, `cli_dev_install_accepts_committed_manifest`) actualizados para incluir `BUNDLE.toml` en sus fixtures (helper `write_test_bundle_manifest`) — falla esperada por la preflight v2 que rechaza installs sin BUNDLE.toml coherente.

## [1.62.0] - 2026-08-31

### Added
  - feat(workflow): transición `phase.build.remediate` (`REMEDIATING/build` → `OPEN/build` con gate `remediation-complete`) espejada de `phase.verify.remediate` para evitar deadlocks cuando `release.recover` mueve un ciclo a `REMEDIATING/build`. Cubre el gap identificado en [[ADR-0077]].

### Fixed
  - fix(uat): test `cli_phase_build_remediate_rejects_wrong_phase` marcado `#[ignore]` con motivo documentado (el workflow no expone ninguna transición hacia `REMEDIATING/verify`, por lo que el setup del test no puede ejercitar el rechazo bajo prueba). Seguimiento en cycle-45.

## [1.61.0] - 2026-08-30

### Added
  - feat(release): ruta de recuperación fail-closed desde RELEASE_PENDING (transición `release.recover` RELEASE_PENDING/release → REMEDIATING/build; nuevo artefacto `release-failure-evidence`; gate explícita `release-recovery-authorized`; fail-closed e idempotente).
  - docs(sddk): documento de investigación sobre la fase review huérfana en A-full (discrepancia R0–R6 entre `workflow.yaml` y el prompt layer; recomendación Option 1: remover `Phase::Review` del runtime).

## [1.60.0] - 2026-08-30

### Added
  - feat(spec): SPEC-042-secretary-runtime (Stage 0, docs-only) con §Substrate dependency verbatim de SPEC-028 §Contract; gate de promoción `SPEC-028-promoted` cierra Stage 1+ hasta Built.
  - feat(docs): epic `SECRETARY-A` en BACKLOG + amendment `ROADMAP §Phase 6` con priorización comparativa; Stage 1 marcado `proposed / blocked-by-SPEC-028-Built`.
  - feat(release): implementa `release-revalidation` para ciclos `RELEASE_PENDING` (recovery canónico tras push fallido por pre-push hook: rerun verify+debt-verify contra la candidate SHA, validar SHAs, persistir `release-revalidation-<sha>.json` con sha256 sidecar; permite transición `release.complete` sin re-ejecutar el ciclo completo).

### Fixed
  - fix(release): corrige invariants de release-revalidation (atomicidad del sidecar, validación de schema_version, rechazo si verdict≠passed).
  - fix(release): usa comando `local --locked` en vez de `--release` para verify (paridad con el gate duro del CI local).
  - fix(docs): corrige defectos RDI del ciclo `p-52b95ef55999f9de` (gate term + wikilinks + lesson).
  - fix(clippy): reemplaza `vec!` por arrays en regression tests.

### Changed
  - docs(adr): ADR-0072-secretary-budgets (compone ADR-0068+0070) + ADR-0073-secretary-authority (closed-set L1 = 8 event classes + Receipt rule + autoridad prohibida `release/gate/lease/receipt`).

### Tests
  - test(release): añade cobertura para release-revalidation.
  - test(release): añade tests de propiedades (b)(d) y dispatch de revalidate.

## [1.59.1] - 2026-08-30

### Fixed
  - fix(release): corrige manifiesto distribuido (refresh de 2 stale digests en MANIFEST.sha256 para `prompts/sddk/phases/archive.md` y `skills/_shared/cli-usage-contract.md`; nuevo regression test `cli_dev_install_accepts_committed_manifest`; instrucción de preflight en `docs/RELEASING.md`).

## [1.59.0] - 2026-08-30

### Added
  - feat(uat): implement release-distribution-integrity for cycle p-52b95ef55999f9de
  - feat(uat): harden release-distribution-integrity para cycle p-52b95ef55999f9de

### Fixed
  - fix(uat): corregir defectos RDI del ciclo verify
  - fix(rdi): usar source explícito de DistArgs para verificación y staging
  - fix(rdi): usar create_bundle_without_manifest para el test install_fails_on_absent_manifest_source

## [1.58.5] - 2026-08-29

### Other
  - feat(uat): agrega gate de ShellCheck y documenta alcance nulo de Ruff
  - fix(uat): corregir gate ShellCheck y cobertura de verify A-lite p-52b95ef
  - fix(shellcheck): limpia violations pre-existentes en 5 scripts cubiertos por el gate
  - fix(backlog): cierra bullets Phase C #1/#2/#3 y actualiza descripcion del gate fail-hard

## [1.58.4] - 2026-08-29

### Fixes
  - fix(archive): fija timestamp de cierre (`prompts/sddk/phases/archive.md` post-transition `updated_at` becomes manifest `closed_at`; placeholder rejection enforced)

## [1.58.2] - 2026-08-29

### Fixes
  - fix(archive): fija evidencia final de ledger

## [1.58.1] - 2026-08-29

### Other
  - docs(roadmap): reprioriza cierre de test-tooling

## [1.57.0] - 2026-08-28

### Features
  - feat(hooks): pre-push que exige commit de release para main

### Other
  - chore(debt): cierra INC push con prevencion mecanica probada

## [1.56.0] - 2026-08-28

### Features
  - feat(ci): wire sddk lint into just ci as local gate
  - feat(lint): SDDK023-SDDK027 diagnostics with fixture tests

### Other
  - chore(debt): registra INC por 4a violacion de push en apply
  - style(lint): aplica cargo fmt
  - chore(uat): parity-gated deletions of 7 shell files

## [1.55.0] - 2026-08-28

### Features
  - feat(lint): add SDDK020/021/022 lint diagnostics with fixture tests
  - feat(sddk-testkit): add CliSandbox builder with XDG env isolation

### Other
  - chore(uat): retira 6 tests shell con paridad y rewirea registros
  - style: fmt fixes from Train 2 implementation
  - test(sddk-cli): port first_class_help substring to Rust
  - test(sddk-cli): port cycle_inventory_contract to Rust

## [1.54.0] - 2026-08-28

### Features
  - feat(vault): reconcile wikilinks VAULT003 and materialise 4 vault nodes
  - feat(lint): add SDDK015-SDDK019 diagnostics for instruction-layer contracts

### Fixes
  - fix(lint): endurece diagnósticos de matriz con parser lineal y añade tests SDDK015-019
  - fix(uat): repara 27 violaciones de lint pre-existentes y registra facades en permissions

### Other
  - chore(uat): delete parity-proven shell wrappers and rewrite references

## [1.53.0] - 2026-08-28

### Features
  - feat(uat): residual closure — 6 items implementados

### Fixes
  - fix(uat): registra facades en agent-models y corrige argv de sddk-plan

## [1.52.0] - 2026-08-28

### Features
  - feat(uat): apply-push hardening — binding NO-PUSH contract + drift check + test gates

### Fixes
  - fix(ci): tabula receta just y registra test JS en fixtures

### Other
  - chore(debt): cierra INC apply-push con evidencia de verificación
  - test(workflow): actualiza expectativa del Step 1.7 al marcador advisory

## [1.51.0] - 2026-08-28

### Features
  - feat(uat): instruction-layer contract matrix and sizing advisory routing

### Other
  - docs(roadmap): publica ADR-0069/ADR-042 aceptados, INC sin número inventado y reprioriza roadmap

## [Unreleased] — cycle-43

### Fixed
- cycle-43 INC-DEBT-016: dm02 hang resolved via two-part fix. (1) `spawn_pending_and_ready` now matches `NodeRunState::Running` so Sequence intermediate state is re-evaluated. (2) `Sequence::evaluate` pushes a marker Attempt to `ctx.node_run.attempts` after each child so `completed_steps` advances. dm02_execute_completes_all_nodes: EXIT 124 → EXIT 0 (0.00s). dm02_stress_harness: 0/3 → 3/3 PASS. Workspace tests: BLOCKED → 1419 passed.

## [Unreleased] — cycle-41

### Fixed
- cycle-41 INC-DEBT-015: 36 unique sddk-engine clippy warnings reduced to 0 (~70 resolved). Fixed bogus clippy::missing_docs lint name (T1), applied machine clippy fixes across lib + tests (T2), suppressed needless_range_loop where clippy suggestion would change semantics (T3).

## [Unreleased] — cycle-40

### Fixed
- cycle-40 INC-DEBT-014: 85 unique sddk-engine clippy warnings reduced to 36 (~49 resolved). Deleted 17 unused test helpers + structs (T1), removed 28 unused imports (T2), resolved 5 Arc not Send+Sync in test helpers (T3), applied 3 derivable impls (T4), annotated 17 missing-docs (T5).

## [Unreleased] — cycle-39

### Fixed
- cycle-39 INC-DEBT-013 reqwest client cache drift closed at v1.48.7.

## [Unreleased] — cycle-38

### Fixed
- W1 (INC-DEBT-012): `resolve_alias_for` helper (reconcile.rs:258-281) was extracted in cycle-37 T2 but never wired — now called by all 3 adapters (json/claude/codex). Clippy warning resolved.
- F1 (INC-DEBT-012): `ParsedAgentForTest` had 3 unused fields (`description`, `tools`, `body`) — trimmed to 1 field (`aliases`). Clippy warning resolved. Spec correction: post-trim is 1 field, not 2; `name` is read from filename stem externally.

### Added
- `resolve_alias_for_first_match_wins`: direct unit test for `resolve_alias_for` helper with 3 sub-cases (no-match, canonical-only, alias-match). Anti-tautology: proves helper logic independently of adapter call sites.

## [Unreleased] — cycle-37

### Added
- Per-file frontmatter `aliases:` field parsed by `load_agent_sources` in all 3 adapter families (json/claude/codex). Closes INC-DEBT-011.
- `renames_builder()` builds alias → canonical name map from bundle agents with `aliases:` frontmatter. Scope-filtered to `is_framework_namespaced` agents. First-loaded alphabetical wins on collision (INV-11).
- `ReconcileContext.renames` field wired in all 3 adapter reconcile loops (json/claude/codex). Alias-driven name diffs now detected when config entry uses an alias instead of canonical name.
- Apply handlers activated: `apply_rename_in_agents_map` (json), `apply_rename_claude_file` (claude), `apply_rename_codex_file` (codex) now consume `ctx.renames` via alias-aware existing entry lookup.

### Fixed
- `apply_rename_in_agents_map`: now updates the entry's internal `name` field to match the new map key after rename (INC-DEBT-011).

## [Unreleased] — cycle-36

### Added
- Apply handlers for `FieldDiff { field_name: "name", ... }` in all 3 adapter families (json/claude/codex). Wires the consumer side of the rename-detection story started in cycle-35. **Dormant in production today** — all adapters set `existing.name = lookup_key`, so the rename diff is never emitted. Detection mechanism (rename map) deferred to future cycle. Closes INC-DEBT-010.

## [Unreleased] — cycle-35

### Fixed
- ExistingEntry.name design gap: `diff_existing_target` now compares `existing.name` vs `target.name` and emits a `FieldDiff { field_name: "name", ... }` when they differ. Closes INC-DEBT-009.

## [Unreleased] — cycle-33

### Changed
- `EditorCapabilities` in `sddk-cli`: removed `PartialEq, Eq` derives. Function pointer fields have unpredictable equality semantics. No workspace consumers of `EditorCapabilities::eq` were found (verified in cycle-33 explore). If you compare `EditorCapabilities` values in your code, refactor to compare individual fields or use a custom comparator.

### Fixed
- 7 pre-existing clippy errors in `crates/sddk-cli/` (closes INC-DEBT-007)
- 1 latent `unpredictable_function_pointer_comparisons` warning on `EditorCapabilities` derive

See: `docs/debt/INC-DEBT-007-preexisting-clippy-sddk-cli.md`

## [1.37.0] - 2026-08-22

Cierra el ciclo `kernel-cycle-14-m2-event-foundation` (path A-min). M2 del
event-foundation: `EventSchemaRegistry` + `CanonicalEventValidator` + tabla
de severidad `JournalProjection` + helpers públicos de correlación/causation.
Anti-AC preservados: 0 cambios en `sddk-storage/src/**`; 0 nuevas migraciones;
`EventEnvelopeV1` serialized shape byte-identical (REQ-M14-003 binding);
`emit_*` signatures byte-identical; `MANIFEST.sha256` byte-identical.

Amendment REQ-M14-004 (orquestador 2026-08-22): helpers `with_correlation_from_context`
+ `with_causation` + `trace_causation_chain` existen como API pública
probada; el wiring de producción queda diferido a M6 SPEC-028 para preservar
`emit_*` signatures. Resolución de conflicto self-contradiction (precedent
cycle-11 D2).

Verify 12/12 scenarios + 6/6 anti-ACs COMPLIANT (1094/0/6 cargo tests; 0
clippy warnings; 0 fmt warnings). Debt-verify PASS_WITH_WARNINGS: 0
introduced blockers; 2 medium/P2 (LOC overage 773 vs ≤220 per-file budget,
apply pre-verify push violation) + 3 low/P3 (helper doc-gap, corpus fixture
duplication, severity spec drift) — todas con INC filed y fingerprint
poblado.

### Added
  - feat(events): `EventSchemaRegistry` con 18 tipos registrados (incl.
    `lease.released` que faltaba) + macro `schema_struct!` (5-arg, 18
    invocaciones) que reduce ~216 LOC de boilerplate y aprieta el trait
    boundary (`EventSchema` no se puede olvidar `info()` ni
    `validate_payload()`). Cierra REQ-M14-001/002.
  - feat(events): `CanonicalEventValidator` con regex de tipo por segmento
    (acepta legacy 2-segmentos + actual 3-segmentos) + corpus replay test
    que valida que los 18 envelopes registrados son replay-safe.
    Endurecido a 18/18 casos en commit e1ded59 (regex fix para tipos
    legacy). Cierra REQ-M14-003.
  - feat(events): `JournalProjection` con tabla de severidad 7-row
    (crítica → baja; categorías `pack`/`runtime` colapsadas a Medium por
    default, decisión documentada in-line projections.rs:451-452). Cierra
    REQ-M14-005.
  - feat(events): helpers públicos de correlación/causation
    (`with_correlation_from_context`, `with_causation`,
    `trace_causation_chain`) en `event_bus.rs` con 3 tests nombrados
    `PASS`. Wiring de producción diferido a M6 SPEC-028 (amendment
    REQ-M14-004). Cierra REQ-M14-004.

### Fixed
  - fix(events): regex de formato acepta tipos legacy de 2 segmentos
    (commit e1ded59) — `corpus_replay_through_validator` endurecido de
    11/12 a 18/18.
  - fix(events): registrar `lease.released` + corpus replay +
    corregir INC — `LeaseReleasedSchema` añadido al registry en b6fc6d0
    (event_registry.rs:311).
  - fix(events): clippy `unreachable_patterns` en test — sustituido
    match+panic por `assert!(matches!(...))` en b6fc6d0.

### Housekeeping
  - chore(debt): 5 findings filed (medium/P2: FIND-0001 LOC-overage
    cluster CL-05, FIND-0002 apply-push-violation cluster CL-03; low/P3:
    FIND-0003 helper-doc-gap cluster CL-06, FIND-0004 corpus-fixture-
    duplication cluster CL-04, FIND-0005 severity-spec-drift cluster
    CL-07). Cada finding con fingerprint poblado y remediation_cycle
    declarado.
  - chore(debt): `INC-CYCLE-14-APPLY-PUSH-VIOLATION.md` — 3ª ocurrencia
    de la clase release-gate ordering; remediation target
    `kernel-cycle-15-apply-push-discipline`.
  - chore(debt): `INC-CYCLE-14-LOC-OVERAGE.md` — 3ª ocurrencia de la
    clase (port_contracts/gate_evaluator preceden); semantics distinta
    por fingerprint por ciclo; cluster `CL-LOC-OVERAGE`.

## [1.36.4] - 2026-08-22

Cierra el ciclo `kernel-cycle-13-m1-hexagonal-ports` (path A-min). Hexagonal
ports M1 — contrato de equivalencia byte-a-byte entre `InMemoryLedger` y
`SqliteLedgerFactory` para los 6 puertos de almacenamiento del engine (Ledger,
EventStore, GraphStore, ForkStore, ProjectionStore, ControlPlane) + 2
cross-checks de byte-equivalencia. Anti-AC preservados: 0 cambios en
`crates/sddk-engine/src/**`, `crates/sddk-domain/src/**`,
`crates/sddk-storage/src/**`; `MANIFEST.sha256` byte-identical; 0 test fn
borrados o relajados. Waiver WV-0015 ARCH003 composition-root avanzado al SHA
verificado (`granted_until_sha: 522e5b9...`); dev-deps `sddk-storage` y
`sddk-testkit` en `sddk-engine/Cargo.toml` anotados con `# WHY:` comments que
nombran el archivo consumidor.

Verify 8/8 scenarios + 5/5 anti-ACs COMPLIANT (1076/0/6 cargo tests; 0 clippy
warnings; 0 fmt warnings); debt-verify PASS_WITH_WARNINGS (3 LOW/P3 introduced:
ControlPlane concrete-only vs port-level coverage, byte_equiv partial-check
manifest subset, port_contracts local builders duplicating testkit API).
LOC adjudication (ADR-0048 total-module-sum): impl 0/80-120, boilerplate
5/20-30, fixtures 423/100-150 — overage en fixtures (+273) consolidado en
INC-CYCLE-13-LOC-OVERAGE (medium/P2, cluster=over-engineering/test-fixture-density).
INC envelope reporting defects desde apply/verify ya catalogadas en
INC-CYCLE-13-APPLY-TEST-COUNT-MISREPORT.

### Added
  - test(engine): `crates/sddk-engine/tests/port_contracts.rs` — suite de
    contratos (9 `#[test]`, ≥8 required) cubriendo los 6 puertos del engine
    con `InMemoryLedger` + `SqliteLedgerFactory::open_in_memory()`. 2
    cross-checks `byte_equiv_*` prueban equivalencia observable
    (event_count, cycle_record) entre adaptadores. Cierra REQ-M13-002.
  - test(engine): migrar `adoption::plan_is_write_free_*` a
    `Fixture::new_in_memory()` — preserva aserciones byte-equivalente sobre
    `plan.identity.project_id` y `plan.paths.ledger`. 23 tests
    durability-required conservan `Storage::open(&path)` con comentarios
    `// durability-required:` (reopen-same-path pattern). Cierra REQ-M13-001.

### Changed
  - chore(arch): waiver WV-0015 ARCH003 composition-root avanzado a
    `granted_until_sha: 522e5b9...` (era `f0db2bd...`). Cierra REQ-M13-003.

### Housekeeping
  - chore(engine): dev-deps `sddk-storage` (kept) y `sddk-testkit` (added) en
    `crates/sddk-engine/Cargo.toml` con comentarios `# WHY:` que nombran el
    archivo consumidor. Cierra REQ-M13-004 anti-AC hygiene.
  - chore(debt): 3 LOW/P3 introduced (FIND-013001 ControlPlane concrete-only,
    FIND-013002 byte_equiv_cycle_record partial check, FIND-013003 port_contracts
    local builders duplicating testkit API). Remediation target: backlog.
  - chore(debt): `INC-CYCLE-13-LOC-OVERAGE.md` — LOC exception entry (impl 0,
    boilerplate 5, fixtures 423/100-150, total 428/200-300; overage driven by
    test fixtures with shared builders ~60 LOC).

## [1.36.3] - 2026-08-22

Cierra el ciclo `kernel-cycle-12-workflow-contract-reconciliation` (path A-min).
Workflow contract reconciliation — fija el contrato de autoridad local de release
en los 3 archivos de la autoridad (`agents/sddk-release.md`,
`skills/sddk-release/SKILL.md`, `prompts/sddk/phases/release.md`): annotated tag
es MANDATORY y peels al SHA verificado de `main`; la verificación local
(`HEAD == origin/main` post-push) es la ruta de publicación obligatoria; la
distribución externa post-tag (GitHub Releases, CI/CD, assets) es opcional y
nunca es autoridad de cierre. Cierra el drift del allowlist de vocabulario MCW
y abre el contrato del knowledge pipeline (`scan -> verify -> import`) en el
orchestrator con conditioning explícito a `reviewed plan` y `knowledge_approved`.

0 Rust LOC; verify 14/14 scenarios + 11/11 anti-ACs COMPLIANT (296/0/0 test
contract; 1067/0 cargo tests; 0 clippy warnings); debt-verify PASS (2
introduced LOW/P3 — cosmetic orphan label + structural release-authority
coupling; pre-existing forward debt preserved); INC-CYCLE-11-PYTEST-CONTRACT-P1
cerrada (17 xfail cerradas por cycle-12; registry vaciado).

### Fixed
  - fix(agents): refs artifact store y autoridad release local (`agents/sddk-propose.md:27` + `agents/sddk-debt-verify.md:44` citan `sddk artifact store`; `agents/sddk-release.md:30-32` declara autoridad local obligatoria). Cierra REGRESSION I items 1-2.
  - fix(prompts): `sddk-verify` v2.3 + narrativa MCW + orden scan-verify-import — `skills/sddk-verify/SKILL.md` versionado a 2.3 con 8 literales del contrato de verificación (status incluye cycle, transiciones A-full/min/lite/b-direct, failed gate outcome, failed transition state, conditional lease flags); MCW § A-lite (L158) y § A-min (L162) narrativos coherentes con `workflow.yaml`; orchestrator Step 2 declara orden canónico `scan -> verify -> import` con conditioning a `reviewed plan` y `knowledge_approved`. Cierra REGRESSION J (8 literales) + D (5 assertions).

### Tests
  - test(workflow): cerrar 17 xfail — XFAIL registry vaciado (lines 38-61) + threshold `transition_artifacts` 15 -> 5 (lines 245-249) + INC `INC-CYCLE-11-PYTEST-CONTRACT-P1.md` cerrada + MANIFEST.sha256 regenerado tras edits de contenido. Cierra 1:1 todos los xfail pre-existentes: 2 (I-cluster artifact store) + 8 (J-cluster sddk-verify literals) + 5 (C-cluster release patterns x 3 files) + 1 (D-cluster scan-verify-import ordering) + 4 nuevas positive assertions en D-cluster (`contains with_knowledge`, `contains knowledge_approved`, `contains reviewed plan`, `import conditioned to both`). Suite 296 PASS / 0 FAIL / 0 XFAIL.

### Documentation
  - docs(debt): `INC-CYCLE-11-PYTEST-CONTRACT-P1.md` cerrada por cycle-12 (status: closed, resolved-by: cycle-12, lifecycle row appended).

### Housekeeping
  - chore(debt): 2 LOW/P3 introduced (FIND-0001: hardcoded orphan DEBT label en `tests/test_workflow_contract.py:1116`; FIND-0002: drift-prone release-authority duplication across 3 sources). Remediation target: backlog/opportunistic.

## [1.36.2] - 2026-08-22

Cierra el ciclo `kernel-cycle-11-a-full-coherence-gate-ordering` (path A-min).
Bug fix de docs/prompts/python — corrige el orden de las coherencias en el
bloque § A-full de `mcw.md` (steps 1.3-1.6 reordenados) y hace explícitas
las dependencias en `prompts/sddk/workflows/sddk-a-full.yaml` (4 `depends_on:`
añadidos a los coherence gates). 0 Rust LOC; 36/36 forward ACs + 9/9 anti-ACs
COMPLIANT; 7 low/P3 introduced (cosmetic/structural/opportunistic debt per
ADR-0047 + docs/debt/SEVERITY.md); 1 high/P1 introducida consolidada como
INC-CYCLE-11-PYTEST-CONTRACT-P1.md (17 xfail pre-existentes en
test_workflow_contract.py — captura estable de regresiones, fix en backlog).

### Fixed
  - fix(prompts): reordenar secciones cuerpo MCW A-full — steps 1.3-1.6 colocados tras los productores (`mcw.md` Phase 1 § A-full steps 1.1/1.2/1.7/1.8 byte-identical; Phase 2/3/4 byte-identical). Cierra REGRESSION O.
  - fix(prompts): `depends_on` explícitos en coherencias fase 2 A-full — `coherence-propose-spec` (step 1.4) → `spec-and-design-parallel`; `coherence-spec-design-tasks` (step 1.6) → `tasks`; `coherence-apply-verify` (step 2.2) → `apply`; `coherence-debt-release` (step 2.5) → `debt-verify`. Cierra REGRESSION L.

### Tests
  - test(workflow): tests de orden de coherencia + xfail P1 explícito (REGRESSION L/O/P/Q/R + COHO-001..005). 276 PASS / 0 FAIL / 17 XFAIL. C1-C8 closed.
  - test(workflow): superficie completa tests de coherencia — añade parser loop `COHERENCE_GATES` que itera los 4 coherence gates y verifica `depends_on` no vacío. Extiende COHO-002-2 con assertion explícita del step 1.6.

### Documentation
  - docs(debt): `INC-CYCLE-11-PYTEST-CONTRACT-P1.md` — captura durable de las 17 regresiones xfail del test contract (5 clusters: I propose/debt artifact store, J verify CLI contract, B transition artifact refs, C release authority, D knowledge pipeline ordering).
  - docs(handoff): `HANDOFF-2026-08-22-sddk-framework.md` — handoff de cycle-11.

### Housekeeping
  - chore(debt): entrada deuda P1/P2 cycle-11 + manifiesto (test_workflow_contract.py regresiones + manifiesta refresh discipline).

## [1.34.0] - 2026-08-21

### Refactor
  - refactor(ucl): migrate sddk-cli Stack-B call sites to `time::OffsetDateTime::now_utc()` (16 sites, 9 files). REQ-K6-001.
  - refactor(domain): migrate sddk-domain Stack-B call sites to `time::OffsetDateTime::now_utc()` (3 sites). REQ-K6-002.
  - refactor(domain): delete `sddk_domain::format::now_rfc3339_utc` wrapper and its test. Preserve `format_rfc3339_utc(epoch_secs)` + 5 pinned-value tests. REQ-K6-003.

## [1.33.0] - 2026-08-20

### Refactor
  - refactor(cli): consolidate `uat_common::time::now_rfc3339` → `sddk_domain::format::now_rfc3339_utc` (13 call sites in 9 files). Delete 43 LOC Hinnant orphan. REQ-K5-001.

### Features
  - feat(domain): extend `assert_variant_count_eq!` macro to 7 more enums (Phase=10, CycleStatus=10, RiskLevel=4, RuleSeverity=3, StalenessState=5, PackRisk=4, ReleaseChannel=4). Negative-tested: literal 10→11 breaks build. REQ-K5-002.
  - fix(domain): macro diagnostic now embeds `stringify!($enum)` + concat-based panic msg; PM-3 clippy fixes (`for_kv_map`, `expect_fun_call`, `collapsible_if`, `const_is_empty`); 12 runtime shape tests (`variant_counts.rs`). REQ-K5-004.

### Housekeeping
  - docs(agents): trim AGENTS.md 229→≤100 LOC; extract §2.6 Distribución → `docs/RELEASING.md`; extract §3 Layout → `docs/ARCHITECTURE-MODEL.md`. REQ-K5-003.

## [1.32.0] - 2026-08-20

### Refactor
  - refactor(domain): `now_rfc3339_utc()` wrapper en `sddk_domain::format` (delegates to `format_rfc3339_utc(epoch_secs)`). Los 3 call sites (`projections.rs:206`, `projections.rs:395`, `graph.rs:251`) ahora usan el wrapper. Cierra S-001 (orphan Hinnant cleanup).
  - refactor(storage): `proj_store_conn_mut()` marcado como test-only surface via `#[doc(hidden)]` + rustdoc explicativo. El escape hatch `&mut rusqlite::Connection` permanece público solo porque los integration tests no ven `#[cfg(test)]` (compile la lib sin `cfg(test)`). Cierra S-002.

### Features
  - feat(domain): macro `assert_variant_count_eq!` (`crates/sddk-domain/src/macros.rs`) — compile-time guard contra variant drift. Combinación de counter literal + exhaustive `match` sin wildcard. Estable en rustc ≥ 1.75 (usa solo `stringify!`, const fn aritmética, `assert!`). Aplicado a los 5 enums trimmed del cycle 3: `CompileError` (8), `WorkflowError` (3), `AttemptError` (1), `NodeRunError` (1), `WorkflowRunError` (2). Negatively-tested: edición literal 8→9 rompe el build con E0080. Cierra S-003.

### Housekeeping
  - chore(repo): `*.proptest-regressions` añadido a `.gitignore` (proptest deterministic-replay cache; safe to delete, regenerable).
  - chore(repo): staging residuals del release resync del cycle 3 (`sddk-linux-x86_64-gnu*`, `software-development-decision-kernel.tar.gz*`) cleared del CWD antes de este commit. `docs/old/*` (712K) intacto pendiente decisión humana cycle 5. Cierra S-004 (parcial).

## [1.31.0] - 2026-08-20

### Features
  - feat(domain): `format_rfc3339_utc` extraído a `sddk-domain::format` (Hinnant `civil_from_days` + `z += 719_468` shift). 5 tests pinned-value. Cierra W-DV-1 (state_updated_at fix) + W-DV-7 (cross-crate Hinnant duplication).
  - feat(arch): `architecture-rules.yaml` schema_version 1.1.0 → 1.2.0 + WV-0027 phase-string waiver para `compiler.rs`/`validator.rs` (10 reglas, 2 waivers). Cierra WV-0027.

### Refactors
  - refactor: `Storage::current_iso8601` + `state_updated_at` delega a `sddk_domain::format::format_rfc3339_utc`. `sddk-cli::uat::now_rfc3339` ya no tiene bloque Hinnant propio.
  - chore(domain): error variant audit — trim 15 unused en 5 enums (`CompileError`, `WorkflowError`, `AttemptError`, `NodeRunError`, `WorkflowRunError`). Cierra U6 (REQ-K3-001).

### Tests
  - test(domain): `compiler_determinism` proptest 1000 iters (hash determinístico + formato `sha256:<64-hex>`). REQ-K3-002 #1.
  - test(domain): `validator_closure` proptest 500 iters (closure property `validate(compile(m))`). REQ-K3-002 #2.
  - test(storage): `graph_store_roundtrip` proptest 200 iters (in-memory `SqliteGraphStore` roundtrip). REQ-K3-002 #3.
  - test(domain): `capsule_validate` proptest 7 invariants (Pointer always valid, sha256 format/length, size bound, digest integrity). REQ-K3-002 #4.
  - test(domain): `budgets_proptests` 5 invariants algebraicos (zero identity, underflow, hard limits, fits-within componente-wise, monotonicity consume). REQ-K3-002 #5.

### Fixes
  - fix(domain): bump `ARCHITECTURE_RULES_SCHEMA_VERSION` constant 1.1.0 → 1.2.0 (closes WU-3b schema gap — runtime must accept the YAML it produces).

## [1.30.0] - 2026-08-19

### Features
  - feat(domain): `WorkflowCompiler` (8-stage deterministic pipeline, no LLM) translates `WorkflowManifest` legacy → `WorkflowIR`. Phase → capability mapping for all 10 `Phase` variants. Closure: `validate(compile(m))` is either `Ok` or a single gate error.
  - feat(domain): `WorkflowValidator` (7 gates: G1 schema, G2 operators, G3 cycle-free, G4 guards, G5 budgets, G6 expansion permissions, G7 context capsules). Short-circuit on first failure. `validate_with_template()` for full G6 allowlist check.
  - feat(storage): 6 `GraphStore` methods implemented in `SqliteGraphStore` against `ir_digests_v1`, `execution_graph_revisions_v1`, `attempts_v1`: `record_ir_digest`, `record_graph_revision`, `load_node_attempts`, `attempt_count`, `load_revision`, `latest_revision`. Closes W-DV-1 (HIGH).
  - feat(domain): `ExpansionPermission::is_allowed` split into `is_known_permission()` + `is_allowed_by(allowlist)`. Old `is_allowed` marked `#[deprecated(since = "1.30.0")]` for removal in cycle 3. Closes W-DV-3 (MED).
  - feat(domain): `Budgets::consume(&sub) -> Result<Budgets, BudgetError>` semantics drives validator G5. Closes W-DV-4 (LOW bonus).
  - feat(domain): `ContextCapsuleRef::validate()` for inline summaries (sha256 format + size bound + digest integrity). Closes W-DV-5 (LOW bonus).

### Fixes
  - fix(domain): elimina 6 defaults `unimplemented!()` del trait GraphStore — LSP closure verificado por sddk-verify correction cycle
  - feat(testkit): 3 fixtures A-min/A-lite/A-full + constantes A_MIN/LITE/FULL_COMPILED_HASH pinned — cierre spec §2.1 scenario 2

### Documentation
  - docs(adr): ADR-0043 — Compiler determinista sin LLM (closure property + golden hashes)
  - docs(adr): ADR-0044 — Validator con 7 gates en short-circuit (orden + rationale)
  - docs(adr): ADR-0045 — GraphStore port con 6 métodos IR-revision (LSP closure)

## [1.29.0] - 2026-08-19

### Features
  - feat(domain): Workflow IR types — `WorkflowTemplate`, `WorkflowIR`, `Operator` (12 variants), `Budgets`, `ExpansionPermission` — landed in `crates/sddk-domain/src/workflow_ir.rs` with deterministic `compute_content_hash()` using `sha256:<64-hex>` format. All collections use `BTreeMap`/`BTreeSet` exclusively.
  - feat(domain): WorkflowRun types — `WorkflowRun`, `NodeRun`, `Attempt`, `AttemptOutcome`, `WorkflowRunState`, `NodeRunState` — state machine with sticky terminal states. `Attempt::complete()` transitions in_flight → terminal.
  - feat(domain): `ExecutionGraphRevision` added to `crates/sddk-domain/src/graph.rs` with parent-chain digest (`sha256(parent_digest || events || nodes || edges)`). Schema version `u32 = 1`.
  - feat(storage): Migration 011 adds 5 new tables: `workflow_runs_v1`, `node_runs_v1`, `attempts_v1`, `execution_graph_revisions_v1`, `ir_digests_v1`. Append-only triggers on `attempts_v1`.
  - feat(kernel): `GraphStore` trait gains 7 default-implemented methods: `record_ir_digest`, `record_graph_revision`, `load_node_attempts`, `attempt_count`, `save_revision`, `load_revision`, `latest_revision`.
  - feat(arch): ARCH008 heuristic evaluator with 4-pattern `RegexSet` (`\bPhase::`, `\bCyclePath::`, variant-qualified SDD names, `match\s+phase\s*\{`). Scope: `workflow_ir.rs`, `workflow_run.rs`, `sddk-engine/lib.rs`. ARCH013–015 stubbed as `NotApplicable`.
  - feat(arch): `architecture-rules.yaml` schema bumped `1.0.0 → 1.1.0`. WV-0026 waiver covers `workflow.rs` and `event_bus.rs:96-117` until v1.31.0.

### Tests
  - test(domain): Property tests for `WorkflowIR` — hash determinism, BTreeMap insertion-order independence, JSON roundtrip, `sha256:<64-hex>` format, 12-operator nesting to depth 10.
  - test(domain): State machine tests for `WorkflowRun` — `pending → running → completed`, `cancelled`, `pause/resume` budget preservation, terminal idempotency.
  - test(domain): `ExecutionGraphRevision` tests — chain divergence, parent digest embedding, `revision 0` root validation, JSON roundtrip.
  - test(domain): `architecture-rules.yaml` parse tests — 1.1.0 structure, ARCH008 scope globs, WV-0026 waiver coverage (legacy files only, not new IR modules).
  - test(domain): 4 IR event golden fixtures added to `event_envelope_golden.rs` — `workflow.ir.compiled`, `workflow.run.started`, `workflow.run.cancelled`, `workflow.graph.revision.accepted`.
  - test(testkit): `ir_fixtures.rs` module with `sample_template()`, `sample_ir()`, `sample_workflow_run()` golden fixtures.

### Documentation
  - docs(adr): ADR-0040 — BTreeMap mandate for IR collections (deterministic hashing requirement)
  - docs(adr): ADR-0041 — `SCHEMA_VERSION: u32` constant per IR type (monotonic integer vs semver)
  - docs(adr): ADR-0042 — ARCH008 SDD-agnostic kernel runtime + WV-0026 legacy compat seam waiver

## [1.28.1] - 2026-08-19

### Fixes
  - fix(editor-adapters): `sddk dev link` now refreshes stale agent paths in `opencode.json` / `zcode.json` when the framework bundle root changes (e.g. after a fresh install or upgrade). Existing entries with paths matching the new root are skipped byte-untouched; entries with paths matching a previous sddk install (`/sddk-framework/agents/` or `/sddk/framework/`) have only the `prompt` field refreshed — user customizations (`description`, `model`, `hidden`, `mode`) are preserved. User-customized paths that look unrelated to sddk (`/my/personal/prompts/...`) are left untouched. Telemetry: `AdapterReport.updated_stale` + `LinkReport.agents_updated_stale`, with a per-editor warning `"refreshed N stale agent paths (framework root changed)"`.

## [1.28.0] - 2026-08-19

### Features
  - feat(orchestrator): Pre-flight Gate 0 — rebuild state from CLI (sddk cycle lock status + cycle status + vault validate) before each phase delegation. Replaces in-memory-only state_token with pull-based reconstruction; survives compaction and session restarts.
  - feat(skill): sddk-cycle-resume — pull-based state_token envelope with cycle_id, phase, branch, head_sha, fencing_token, vault drift count, head drift flag. Hard rules for lease desync, head drift, and "cycle closed by another orchestrator" cases.
  - feat(mcw): Phase 0 Step 0.2 now queries `sddk cycle lock status` as the authoritative gate (vault `_active.md` becomes a secondary informational view).

### Fixes
  - fix(install): bootstrap.sh now symlinks the `workflows/` tree and the `prompts/sddk/workflows/*.yaml` registry. Previously the orchestrator fell back to mcw-prose for path-specific sequences because the YAMLs were never linked into the editor.

### Documentation
  - docs(repo): CONTRIBUTING.md — contribution guide (commit conventions, review process, CI policy, layout, release procedure).
  - docs(repo): README.md — install procedure now documented as `sddk dev install --prefix ~/.local --source .` + `sddk dev link --editor all` (was `bootstrap.sh --all` only).

### Other
  - chore(fmt): rustfmt --all aligns with the repo's rustfmt.toml (cosmetic only).
  - chore(manifest): regenerated MANIFEST.sha256 with the 92nd skill entry.

### Distribution
  - chore(distribution): SDDK now ships as pre-compiled binaries on GitHub Releases. The `scripts/install.sh` one-liner (rustup/mise model) detects platform, downloads the binary + SHA256, verifies integrity, and links the framework bundle into the chosen editor. Linux x86_64-musl and Linux aarch64-musl are the first supported targets (both static, run on any distro). Closes the gap between docs (which claimed "asdf-vm model") and reality (which was "git clone + cargo build").

## [1.27.0] - 2026-08-19

### Features
  - feat(test): golden dataset 10 cases + ratchet/channel e2e (phase9)
  - feat(cli): release channel promote + signed gate receipts + rules ratchet (phase9)
  - feat(domain): release channels + HMAC gate signing (phase9)
  - chore: SDDK 2.0 roadmap complete (Phases 1-4, all MUST done, all SHOULD discarded with rationale)

## [1.26.0] - 2026-08-18

### Features
  - feat(test): golden dataset 10 cases + ratchet/channel e2e (phase9)
  - feat(cli): release channel promote + signed gate receipts + rules ratchet (phase9)
  - feat(domain): release channels + HMAC gate signing (phase9)

## [1.25.0] - 2026-08-18

### Features
  - feat(cli): sddk explore render views + explorer template (phase8)
  - feat(domain): view descriptors + view models for moldable explorer (phase8)

### Other
  - test(cli): explore e2e views graph/timeline/verification + embedded template (phase8)

## [1.24.0] - 2026-08-18

### Features
  - feat(cli): fork replay with kernel ledger fallback event source (phase7)
  - feat(cli): fork create/set/run/diff/promote commands + e2e (phase7)
  - feat(storage): fork store + response cache tables (migration 9) (phase7)
  - feat(domain): fork model + replay engine + structural diff + promote check (phase7)

## [1.23.0] - 2026-08-18

### Features
  - feat(cli): stale list/impact/gate + graph why-stale commands (phase6)
  - feat(domain): context-read tracing recorder bounded + graph skip (phase6)
  - feat(domain): universal staleness derivation over graph + UAT mapping (phase6)

### Other
  - test(cli): stale/impact/why-stale/gate e2e + uat stale surface intact (phase6)

## [1.22.0] - 2026-08-18

### Features
  - feat(cli): sddk graph query/why/rebuild commands (phase5)
  - feat(domain): pattern query BFS + proposal-only behavior runtime (verifies/depends_on) (phase5)
  - feat(storage): graph store port + sqlite adapter + rebuild from ledger (phase5)
  - feat(domain): graph projection nodes/edges/provenance + bounded views (phase5)

## [1.21.0] - 2026-08-18

### Features
  - feat(pack): uat pack boundary, conformance fixtures, evidence aliases canonical (phase4)
  - feat(cli): pack list/inspect/install/verify/enable/disable commands (phase4)
  - feat(engine): pack registry lifecycle discover/verify/enable/disable/install (phase4)
  - feat(domain): pack manifest v2 requires/integrates/conflicts/provides (phase4)

## [1.20.0] - 2026-08-18

### Features
  - feat(approval): CLI commands for human approval list|grant|deny (PR3)
  - feat(gateway): add ApprovalExpired, ApprovalAlreadyResolved, ApprovalReasonRequired variants
  - feat(domain): add ApprovalPending status, ApprovalReceipt types, and ApprovalProjection
  - feat(dev): gum TUI para agent-models.yaml
  - feat(dev): uninstall/doctor cubren claude y codex
  - feat(dev): dev link registra 4 editores sin fallback hardcoded
  - feat(dev): Codex adapter writes native TOML agents
  - feat(dev): Claude Code adapter writes native .md agents
  - feat(dev): ZCode adapter mirrors opencode JSON registration
  - feat(dev): sddk dev models list/set/validate
  - feat(dev): ship agent-models.yaml under assets
  - feat(dev): agent-models.yaml schema + tier/override resolution

### Fixes
  - fix(dev): clippy --all-targets CI gate (M1-M3)

### Other
  - fmt: apply formatting to human-approval-events PRs
  - docs(adr): ADR-0017..0020 modelos + adapters + TUI
  - test(dev): align codex body assertion with frontmatter-newline semantics
  - refactor(dev): EditorAdapter trait + OpenCode JSON adapter

## [1.19.0] - 2026-08-18

### Features
  - feat(engine): event_bus module with emit_phase_event dual envelope appender (SDDK2-204 MS-01+MS-02)
  - feat(cli+engine): emit workflow.phase events on cycle transition success (SDDK2-204 MS-03 wire-up)
  - feat(engine): PhaseEventInput struct with actor metadata for workflow.phase events
  - feat(cli): RuntimeContext.paths field added for ledger path access in event emission

### Tests
  - test(engine): phase events integration — PE-01 dual-emit, PE-02 idempotency, PE-03 rebuild, PE-04 ledger coexistence

### Other
  - chore(rules): refresh WV-0015 granted_until_sha to f0db2bd (post-SDDK2-204)
  - fix(engine): event_bus uses EventStore trait (not concrete SqliteEventStore) to satisfy ARCH001

## [1.18.0] - 2026-08-17

### Features
  - feat(domain): Projection trait + Checkpoint + CycleStateProjection (SDDK2-203)
  - feat(storage): MIGRATION_6 projection_checkpoints_v1 + SqliteProjectionStore (SDDK2-203)
  - feat(storage): rebuild() algorithm with chain-verify + tamper detection (SDDK2-203)
  - feat(cli): dev projection rebuild command (SDDK2-203 MS-05)

### Other
  - chore(domain+storage): rustdoc strict-pedantic polish for SDDK2-203 modules
  - chore(fmt): rustfmt formatting for SDDK2-203 modules (verification cleanup)
  - chore(rules): refresh WV-0015 granted_until_sha to f054680 (post-SDDK2-203)

## [1.17.0] - 2026-08-17

### Features
  - feat(domain): EventStore trait + EventAppended for ledger-first events (SDDK2-202)
  - feat(storage): MIGRATION_5 events_v1 table with append-only triggers
  - feat(storage): SqliteEventStore struct with XDG-open constructors
  - feat(storage): SqliteEventStore append + reads (SDDK2-202 MS-04 essential)
  - feat(storage): MS-05 event_store integration tests and compute_content_hash fix

### Other
  - chore(rules): refresh WV-0015 granted_until_sha to dbc1dbb (post-v1.16.0)
  - fix(domain): compute_content_hash() no longer hashes itself or sequence/recorded_at

## [1.16.0] - 2026-08-17

### Features
  - feat(domain): EventEnvelopeV1 wire format with sub-types (EntityRef, ActorRef, ActorKind, EntityRefVersion, EventTypeError)
  - feat(domain): canonical JSON serialization + sha256 content_hash for EventEnvelopeV1
  - feat(domain): validate_event_type with regex namespacing
  - feat(domain): stub EventEnvelopeV1 module skeleton

### Other
  - chore(rules): refresh WV-0015 granted_until_sha after hygiene cycle
  - fix(cli): replace clone().as_slice() with slice::from_ref in release_cmd.rs
  - chore(cli): silence pre-existing dead_code warnings in dev/ and telemetry helpers
  - chore(tests): allow unused_variables and needless_* in sddk-cli tests
  - chore(gateway): allow dead_code in git_push_credential test fixtures
  - chore: commit manifest.rs allow and fmt fixes for hygiene baseline
  - test(domain): proptest for content_hash determinism under insertion order
  - test(domain): golden vector tests against regenerated uat-acceptance.jsonl
  - test(domain): regenerate uat-acceptance.jsonl with real SHA-256 content_hash values

## [1.15.0] - 2026-08-17

### Features
  - feat(domain+storage): introduce ControlPlane port + SqliteControlPlane adapter
  - feat(cli): compose() composition root + telemetry through ControlPlane port (SDDK2-103)

### Other
  - refactor(cli): re-route sddk_storage types to sddk_domain
  - feat(cli): route telemetry through ControlPlane trait, remove rusqlite dep
  - refactor(cli): route Storage through type alias, eliminate cycle.rs direct import
  - chore(fmt): apply rustfmt after CLI port migration
  - test(engine): ARCH003 WAIVED under WV-0015 (ADR-0015)

## [1.14.0] - 2026-08-16

### Features
  - feat(engine): Ledger port + Engine<L> genérico — Phase 1 M1 exit
  - feat(deep-research): integrate 22 skills + orchestrator agent + b-research workflow
  - feat(domain): extract Ledger port trait from Storage

### Other
  - refactor(deep-research): consolidate 22 skills into master+sub hierarchical pattern

## [1.13.1] - 2026-08-16

### Other
  - refactor(domain): mueve los value types de sddk-storage a sddk-domain (Phase 1 Sub-ciclo A)

## [1.13.0] - 2026-08-16

### Features
  - feat(arch): phase-1 evaluators — live capture + real rules + CLI

## [1.12.1] - 2026-08-16

### Other
  - docs(domain): documenta la API pública del módulo rules (60 items)

## [1.12.0] - 2026-08-16

### Features
  - feat(storage): GateOutcomeStatus::Waived — waiver explícito de gates

## [1.11.0] - 2026-08-16

### Features
  - feat(gateway): resolver genérico CapabilityPolicy::env_allowlist (v2)

## [1.10.3] - 2026-08-16

### Other
  - refactor(testkit): fixtures Git compartidos — API git en TestRepository

## [1.10.2] - 2026-08-16

### Other
  - refactor(dev): copy_tree unifica tree-copy y hace atómico el install de bundles

## [1.10.1] - 2026-08-16

### Other
  - refactor(testkit): ChildGuard RAII compartido; elimina kills manuales duplicados

## [1.10.0] - 2026-08-16

### Features
  - feat(dev): doctor detecta incoherencia binario/bundle (INC-DEBT-005)

## [1.9.22] - 2026-08-16

### Fixes
  - fix(storage): valida longitud de gate 1..=128 en ambos paths de inserción

## [1.9.21] - 2026-08-16

### Fixes
  - fix(release): release-bump antepone CHANGELOG y autocorrige drift de manifest.toml

## [1.9.20] - 2026-08-16

### Other
  - refactor(dev): consolida helpers duplicados y añade smoke tests behaviorales

## [1.9.19] - 2026-08-16

Refactor: `dev_cmd.rs` (3022 líneas) se divide en 13 submódulos bajo `crates/sddk-cli/src/dev/`.
Mejora la legibilidad, reduce el coste de code review y allana el camino para extraer
capacidades reutilizables. Sin breaking changes para el CLI.

### Changed
  - refactor(cli): `dev_cmd.rs` se elimina y se reemplaza por `crates/sddk-cli/src/dev/`
    con 13 submódulos: `mod.rs`, `paths.rs`, `common.rs`, `manifest.rs`, `registry.rs`,
    `doctor.rs`, `framework_check.rs`, `install.rs`, `uninstall.rs`, `update.rs`,
    `link.rs`, `use_cmd.rs`, `check.rs`, `verify.rs`. El módulo reduce la cohesión
    por archivo y deja cada subcomando con su propia superficie.
    (`crates/sddk-cli/src/dev/`, `crates/sddk-cli/src/dev_cmd.rs`)
  - refactor(cli): visibilidad apretada a la allow-list de la spec — los símbolos
    compartidos entre submódulos usan `pub(crate)` o `pub(super)` en lugar de `pub`.
    (`crates/sddk-cli/src/dev/`)
  - refactor(cli): `dev/framework_check.rs` extrae `framework_agent_names`,
    `register_opencode_agents`, `AgentFrontmatter`, `parse_frontmatter`,
    `PRIMARY_AGENTS`, `LinkReport`, `link_report_text` y `sync_assets` desde
    `link.rs` para reducir el tamaño de los submódulos.
    (`crates/sddk-cli/src/dev/framework_check.rs`, `crates/sddk-cli/src/dev/link.rs`)

### Tests
  - test(cli): 25 tests unitarios se migran de `dev_cmd.rs` a `dev/tests/` (un archivo
    por subcomando: `manifest_tests.rs`, `reconciliation_tests.rs`,
    `skill_registry_tests.rs`) usando `#[path]` para preservar el módulo actual.
    (`crates/sddk-cli/src/dev/tests/`)
  - test(cli): 4 smoke tests nuevos en `crates/sddk-cli/tests/cli.rs` cubren el
    cableado de `dev install`, `dev doctor`, `dev verify` y `dev list`.

### Fixed
  - fix(gateway): suprime el lint `expect_fun_call` (clippy 1.91) en
    `classify_auth_failure` test. `expect(&format!(...))` se sustituye por
    `unwrap_or_else(|| panic!(...))` para evitar el `format!` cuando el `Ok`
    es el caso esperado. Hallazgo del gate local del release.
    (`crates/sddk-gateway/src/git.rs:795-798`)

## [1.9.18] - 2026-08-15

Corrige el race condition en la asignación de `seq` para `GateReceipt`:
`allocate_gate_receipt_seq` + `insert_gate_receipt` eran dos llamadas separadas;
entre ellas otro thread podía allocatear el mismo seq, causando UNIQUE violation
o receipt_ids duplicados. INC-DEBT-007 (ponytail death).

### Fixed race
  - fix(storage): `insert_gate_receipt_next_seq` colapsa allocate + format + insert
    en una sola transacción `IMMEDIATE` SQLite. El lock de escritura serializa
    las asignaciones concurrentes; `seq` y `receipt_id` se producen juntos.
    (`crates/sddk-storage/src/lib.rs:946-1005`)

### Removed
  - remove(storage): `Storage::allocate_gate_receipt_seq` eliminado (INC-DEBT-007).
    El método era el ponytail del race; no tiene más usuarios tras la migración.
    (`crates/sddk-storage/src/lib.rs`)

### Changed
  - refactor(engine): `Engine::evaluate_gate` ahora llama una sola vez a
    `insert_gate_receipt_next_seq` en lugar de allocate + insert separados.
    El formatter `gate-{gate}-{plan_hash[7..23]}-{seq}` se mueve al storage.
    (`crates/sddk-engine/src/lib.rs:882-896`)
  - refactor(storage): `debug_assert!` en `build_gate_receipt_id` se sustituye
    por guarda real que retorna `StorageError::PlanHashTooShort { actual, required }`
    si `plan_hash.len() < 23`. Se extrae `pub fn Storage::build_gate_receipt_id`
    (ahora retorna `Result<String>`) y `pub const RID_FORMAT_REGEX` a nivel de
    módulo — el regex deja de duplicarse entre `cycle_authority.rs` y
    `sqlite_storage.rs`. (`crates/sddk-storage/src/lib.rs:140-167`, `956-973`)

### Tests
  - test(storage): reescritura de `storage_insert_gate_receipt_concurrent_allocations_observe_distinct_seq`
    (100 iter × 2 threads, BD compartida, `Arc<Barrier>`, seq exactos 1..=201)
  - test(storage): nuevo `storage_insert_gate_receipt_next_seq_golden_rid_format`
    verifica formato byte-idéntico del `receipt_id`
  - test(storage): nuevo `storage_insert_gate_receipt_next_seq_rejects_short_plan_hash`
    verifica la guarda `PlanHashTooShort` (sustituye el `debug_assert!`)
  - test(engine): `engine_evaluate_gate_increments_seq_on_reevaluation` añade regex
    lock `^gate-.{1,128}-[0-9a-f]{16}-[0-9]+$` sobre el receipt_id

## [1.9.17] - 2026-08-15

Cierra el leak de puerto en `uat_stale_tests::stale_detects_geometry_change` (INC-DEBT-006):
el test pinchaba el puerto 49152 y lanzaba `python3 -m http.server` sin guard, de modo que un
`panic` previo dejaba un proceso huérfano; la siguiente corrida del workspace chocaba con
`EADDRINUSE` y devolvía `ERR_EMPTY_RESPONSE` en el cliente. Ahora el puerto es efímero
(`TcpListener::bind("127.0.0.1:0")`), el proceso servidor está envuelto en un `ServerGuard`
RAII cuyo `Drop` envía `SIGKILL` al hijo y espera `wait()`, y el cliente hace `readiness-poll`
con deadline en lugar de asumir respuesta inmediata. Skip limpio en hosts sin `python3`.

### Fixes
  - fix(uat): `ServerGuard` RAII (`spawn → kill on Drop → wait`) garantiza que un panic
    o un fallo de readiness mata el hijo en lugar de dejarlo escuchando.
    (`crates/sddk-cli/src/uat.rs:4696-4761`)
  - fix(uat): puerto efímero (`TcpListener::bind("127.0.0.1:0")`) elimina la condición de
    carrera entre runs y permite runs paralelos del test. (`crates/sddk-cli/src/uat.rs:4771-4798`)
  - fix(uat): readiness-poll con `deadline = Instant::now() + timeout` y backoff corto
    reemplaza el `read_to_end` ciego que devolvía `ERR_EMPTY_RESPONSE` cuando el servidor
    aún no estaba listo. (`crates/sddk-cli/src/uat.rs:4821-4852`)
  - fix(uat): probe de `python3` en `PATH` antes de spawnear — si falta, el test se salta
    con `Ok(())` en lugar de propagar `ENOENT`. (`crates/sddk-cli/src/uat.rs:4751-4766`)
  - style(uat): alinea formato del guard con rustfmt y silencia `dead_code` de
    `ServerGuard::take` (helper movido al module-level para uso futuro).

### Tests
  - test(uat): `stale_detects_geometry_change` re-pasa en runs consecutivas del workspace
    y en runs paralelos (el puerto ya no es fijo).
  - El test sigue marcado `#[ignore]` (skip explícito de la suite); la verificación de no-leak
    se realiza re-ejecutando el workspace tras una corrida forzada de panic, ver
    `verification-report.md` §2.7.

## [1.9.16] - 2026-08-15

Corrige la ergonomía de `cycle start` y `git push` para el path A-min en repos
trunk-linear: INC-DEBT-003 cambia el default de `--branch` para A-min a `main`;
INC-DEBT-004 añade `GIT_TERMINAL_PROMPT` al allowlist del runner y clasifica
errores de autenticación de `git push` con hint accionable.

### Fixes
  - fix(cli): `cycle start --path a-min` (sin `--branch`) registra
    `manifest.branch = "main"` en lugar de `feat/<name>`. A-lite, A-full,
    B-direct mantienen el default `feat/<name>`. `--branch` explícito siempre
    gana. (INC-DEBT-003, `crates/sddk-cli/src/cycle.rs:471-473`)
  - fix(gateway): `GIT_TERMINAL_PROMPT` se añade a `LOCAL_GIT_ENV_KEYS` para
    que los helper de credentials fallen rápido en lugar de bloquear en un TTY
    inexistente. (INC-DEBT-004, `crates/sddk-gateway/src/git.rs:11-22`)
  - fix(gateway): `git push` que falla con error de autenticación devuelve
    `GitError::AuthFailed { stderr, hint }` con hint de cuatro líneas:
    `gh auth login`, `gh auth setup-git`, o `git config credential.helper store`.
    `apply_local_release` propaga el hint sin modificar el manifest.
    Clasificador de marcadores: `could not read Username`, `terminal prompts
    disabled`, `403 Forbidden`, `Bad credentials`, `failed to authenticate`,
    `fatal: Authentication failed`. (INC-DEBT-004)

### Tests
  - test(cli): `cli_cycle_start_without_branch_for_a_min_uses_main_default`
  - test(cli): `cli_start_with_explicit_branch_for_a_min_persists_value`
  - test(cli): `cli_cycle_start_without_branch_for_a_full_uses_feat_default`
  - test(cli): `cli_release_apply_rejects_a_min_when_manifest_branch_is_feat_x`
  - test(cli): `cli_capability_apply_git_push_forwards_git_terminal_prompt`
  - test(gateway): `local_git_env_keys_includes_git_terminal_prompt`
  - test(gateway): `local_git_env_keys_excludes_gh_token`
  - test(gateway): `git_push_auth_failure_classifies_stderr`
  - test(gateway): `runner_run_forwards_git_terminal_prompt`
  - test(cli/gateway): end-to-end `release apply --route local` hint-on-auth-failure
    scenario is covered indirectly by `git_push_auth_failure_classifies_stderr`
    (unit) — the orchestrator-accepted E2E test was omitted because the
    `file://` remote used in sandbox runs does not exercise credential prompts.
    See `verification-report.md` § Behavioral Compliance Matrix B1.REQ-5.

## [1.9.15] - 2026-08-15

Corrige la evaluación de gate receipts: INC-DEBT-001 evita colisión UNIQUE en re-evaluación
agregando columna `seq` por grupo (gate, plan_hash); INC-DEBT-002 exige `--outcome` explícito.

### Fixes
  - fix(storage): secuencia receipts de gate y colisión UNIQUE — MIGRATION_3 añade columna
    `seq INTEGER NOT NULL DEFAULT 1` y índice único parcial `(gate, plan_hash, seq)`.
    `Storage::allocate_gate_receipt_seq` usa `TransactionBehavior::Immediate` para serializar
    asignaciones concurrentes.
  - fix(storage): `GateReceiptInput` y `GateReceipt` incluyen campo `seq`.
  - fix(engine): `Engine::evaluate_gate` deriva receipt_id como
    `gate-{gate}-{plan_hash[7..23]}-{seq}` y persiste `seq` en la fila.
  - fix(cli): `--outcome <passed|failed>` es ahora argumento requerido en
    `sddk cycle evaluate-gate`. Recetas que omiten la bandera deben actualizarse.
    El silent `Failed` default es eliminado. (Breaking change.)

### Breaking Changes
  - `sddk cycle evaluate-gate --outcome <passed|failed>` es ahora obligatorio.
    Recetas existentes que omiten `--outcome` producirán error de parsing.

## [1.9.13] - 2026-08-14

Corrige la integridad del bundle de release: el manifest se genera desde rutas
tracked publicables, falla cerrado ante errores Git y rutas no UTF-8, conserva
rutas UTF-8 especiales y se verifica en staging antes de actualizar el runtime.

### Fixes
  - fix(manifest): enumera `git ls-files` limitado a superficies publicables y hashea bytes actuales
  - fix(manifest): drena salida Git concurrentemente y rechaza rutas tracked publicables no UTF-8
  - fix(manifest): serializa rutas UTF-8 especiales con escape reversible
  - fix(dev): verifica bundles descargados en staging antes de tocar el destino
  - fix(release): empaqueta el manifest canónico comprometido y elimina cuatro rutas phantom

## [1.9.12]

Cierra el ciclo SDDK2-009 (phase.build.complete). Cinco work units resueltas: U1+U2 bundle seam (dev install --source + skill-registry writer), U3 knowledge pipeline prefight (-with-knowledge --approve con quarantine rule), U4 --outcome passed en todos los evaluate-gate, U5 bump 1.9.12 + BACKLOG + CHANGELOG.

### Features
  - feat(dev): add --source flag to dev install — copies MANIFEST and verifies SHA256
  - feat(dev): add --write-registry to dev link — writes skill-registry.md to XDG project dir
  - feat(dev): write_skill_registry() — scans skills/*/SKILL.md, skips _shared, writes sorted markdown table
  - feat(agent): init knowledge preflight with --with-knowledge --approve pipeline
  - feat(cli): add --outcome passed to all evaluate-gate calls (8 SKILL.md files)

### Fixes
  - fix(cli): --outcome passed added to evaluate-gate in all phase SKILLs
  - fix(backlog): SDDK2-009 inserted after SDDK2-008.DEBT

## [1.9.11]

Cierra el ciclo SDDK2-008 (phase0-knowledge-ingestion). El pipeline `scan → plan → import → verify` con CAS, provenance, authority y quarantine está gobernado por la knowledge vault en `~/.sddk-knowledge/`. Tres negative tests aseguran que `--approve` en candidatos Quarantine (R10) o con razón "relation conflicts" (R5 surface) son rechazados por `is_approvable_change()`. Distribución corregida: 7 crates en `version.workspace = true` alineados a 1.9.11.

### Features
  - feat(knowledge): scan → plan kp-<hex16> → import → verify governed pipeline
  - feat(knowledge): TOCTOU cerrada por re-hash en import
  - feat(knowledge): Authority::Trusted exige disposition=Import o --approve + is_approvable_change
  - feat(knowledge): receipt kr-<hex16> determinista para not_applicable
  - feat(knowledge): CliFixture + git_commit_all scaffolding para integración tests

### Fixes
  - fix(dist): 7 crates con version.workspace=true alineados a 1.9.11

### Other
  - test(knowledge): approve_quarantine_candidate_fails — R10 negative test
  - test(knowledge): approve_relation_conflict_candidate_fails — R5 surface negative test
  - test(knowledge): relation_key_is_deterministic_for_path_invariants — case normalization invariant
  - docs(BACKLOG): SDDK2-008 y SDDK2-008.DEBT insertados entre SDDK2-007 y SDDK2-101

## [1.9.10] - 2026-08-14

Cierra el release del ciclo SDDK2-006 (`sddk-2-0-phase0-doc-governance`). Tras el bump inicial a `v1.9.9` y el commit `docs(handoff): refresh with final HEAD dbf93c7` (`cbe26db`) que reescribió el handoff con el SHA final, el tag `v1.9.9` quedó apuntando al commit previo (`dbf93c7`) sin cubrir el refresh de handoff. Se corta `v1.9.10` como tag anotado en un nuevo `chore(release)` para preservar la linear-history (AGENTS.md §2.2) y satisfacer el contrato `sddk-release` que exige tag-peels-to-HEAD. Sin cambios de código de producción; el diff acumulado del ciclo sigue siendo puramente documental (SDDK2-006 fue zero-intrusion por diseño).

### Other
  - chore(release): bump to v1.9.10 (sddk-2-0-phase0-doc-governance) — corte de tag post-handoff-refresh; repara tag/HEAD gap documentado como W2 en el vault

## [1.9.9] - 2026-08-13

Split AGENTS.md into stable/history/handoff surfaces (SDDK2-006 doc-governance).

### Other
  - feat(docs): split AGENTS.md — stable ≤150 LOC + history archived + handoff; renumber BACKLOG SDDK2-004→006 / SDDK2-005→007; reconcile vault ID collision

## [1.9.1] - 2026-08-11

Cosmetic fixes and documentation improvements from post-release cleanup.

### Other
  - chore(docs): bootstrap.sh — rename `SHARED_DIR` → `SDDK_FRAMEWORK_ROOT` for clarity; the variable always pointed to the CWD but the name was misleading
  - docs(sdk): add "resolved state" section to SPEC.md documenting the 2026-08-08 elimination of `~/.sddk-shared/` and current verified state

## [1.9.0] - 2026-08-11

Guided Runner UX (F13, M-002): a human-governed UAT flow with immutable sign-off, stale advisories, blind checks, evidence gates, checkpoints, diagnostics, and designer/runner/reviewer modes. Minor bump for the new RF-024..028 capabilities, 13 domain types, and plan schema v4.

### Features
  - feat(uat): F13 Guided Runner UX — immutable SHA-256 sign-off, stale advisory, blind checks with evidence gate, checkpoints with AI diagnostics, and designer/runner/reviewer modes
  - feat(uat): RELEASE ACCEPTANCE wizard with immutable acceptance records and release gate integration for RF-024..028
  - feat(domain): 13 UAT domain types for runner modes, blind checks, completion policies, checkpoints, diagnostics, acceptance, and staleness
  - feat(uat): plan schema v4 with backwards-compatible parsing of schema v3

## [1.8.1] - 2026-08-11

Endurece el CI local (act + podman): el lint de `dev-doc-check` ya enforza SDK009/SDK010 para los docs/inventory regenerados, así que los steps redundantes `generate docs/inventory --check` en `ci.yml` se eliminan (bajo `act` con bind mount el check directo daba falsos "stale"). Patch bump por fix + chore (sin features nuevas); el CI local queda verde con un solo gate lint.

### Fixes
  - fix(ci): eliminar steps redundantes de `generate docs/inventory --check` — el lint `dev-doc-check` ya valida SDK009/SDK010 (sha256-pinned entries, INVENTORY sync) como gate único de los docs/inventory regenerados; bajo `act` con bind mount el check directo daba falso stale y hacía fallar el workflow aunque el contenido estuviera sincronizado

### Other
  - chore(style): `cargo fmt --all` en workspace — uniforma el estilo de los 7 crates; 72 diffs (20 del ciclo surface-brevity + 52 pre-existentes de v1.7.0); CI local (act) verde

## [1.8.0] - 2026-08-11

Cierra la deuda INC-001 (surface-brevity-standard) y formaliza el estándar de concisión de superficies (ADR-016). El orquestrador pasa de 1366 líneas a un shell de 288 que delega MCW/políticas/tablas a `prompts/sddk/`; el doctor detecta superficies que exceden el umbral y subdirectorios vacíos. Minor bump por dos features (`feat(dev)` ×2) más un refactor estructural.

### Features
  - feat(dev): `sddk dev doctor` surface.briefness — detecta agentes/skills/prompts que exceden el umbral (300/150/200 líneas); `--strict` promueve la violación a exit 1; por defecto es advisory en el report
  - feat(dev): `sddk dev doctor` surface.empty_dirs — detecta subdirectorios vacíos o phantom en agents/skills/prompts; se mantiene advisory bajo `--strict` (no auto-elimina); elimina la skill fantasma `skills/logseq-vault/`

### Refactors
  - refactor(agents): `agents/orchestrator.md` shell ≤300 — extrae arsenal, dynamic-workflow, escalation-policy, status-query, entropy-policy y document-catalog a `prompts/sddk/`; routing A–D, gates y comandos preservados; tabla MCW step index retirada del shell
  - docs(adr): ADR-016 surface-brevity — agentes ≤300 / skills ≤150 / prompts ≤200 líneas; estructura Pocock (frontmatter + workflow + examples); sin excepciones nominales; `sddk dev doctor` lo enforza como advisory, `--strict` lo promueve

### Other
  - chore(agents): prune `skills/logseq-vault/` (skill fantasma, directorio vacío preexistente; el doctor lo detectaba pero no lo eliminaba)
  - chore(agents): `skills/_shared/` se mantiene como referencia técnica no-namespace (no es skill ejecutable; queda fuera del scope doctor)

## [1.6.1] - 2026-08-10

Endurece la release local CI/CD-independent: el workflow SDDK no depende de ningún sistema CI/CD (CI/CD queda como distribución opcional posterior al tag), con reconciliación idempotente de receipts, precondiciones de trunk/HEAD/cycle y autorización efectiva de `git.inspect`. Patch bump por refactor + fix (sin features nuevas).

### Fixes
  - fix(release): endurecer release local CI/CD-independent — recibos `git.push`/`git.tag` `Started` reconciliados contra el efecto remoto por SHA (los pre-efecto se reintentan, los post-efecto cierran sin duplicar), ciclo ligado a trunk/HEAD (exige trunk limpio y `HEAD` ancestro del commit del manifest), `--cycle` propagado por CLI/agente/skill/prompt, `git.inspect` añadido a la autorización efectiva, orden release → archive coherente y prohibición de comandos ejecutables PR/CI/CD del proveedor

### Other
  - refactor(release): desacoplar workflow SDDK de CI/CD — ruta de release local `validate → push main → verificar SHA remoto → tag anotado` idempotente; Forge integración opcional, nunca gate ni autoridad; precondiciones locales exigen trunk limpio y `HEAD` ancestro del commit del manifest

## [1.6.0] - 2026-08-10

Consolida la integridad UAT fail-closed (P0) y el vault persistente por identidad estable (P1), cierra el loop dashboard → control plane (wizard → ingest), normaliza las superficies a `sddk-*` con cero intrusión (ADR-0011) y elimina el segundo checkout `~/.sddk-shared/` a favor del modelo asdf-vm (CWD + bundle XDG). Minor bump por las dos features (`feat(uat)` + `feat(persistence)`).

### Features
  - feat(uat): integridad UAT fail-closed con gate de release (P0) — el gate `release-uat-approved` exige sesión humana con verdict y verifica build fingerprint (commit/branch/tag/dirty) antes de permitir el tag; `sddk uat gate release --tag X` emite `BLOCKED`/`ALLOWED` con recovery plan cuando hay mismatch
  - feat(persistence): vault por identidad estable con CLI knowledge (P1) — `sddk vault <id>` resuelve el vault XDG del proyecto por identidad (no por path), `sddk knowledge` añade listado/búsqueda/export del vault (markdown + JSON)
  - feat(uat): schema v2 — plan con `context.{user_story, preconditions, workspace, timing, help, failure_protocol, postconditions, test_data}`, session con `metadata.{tester, env_fingerprint, build, duration_ms}`, evidence tipada (`file | screenshot | command_output | assertion | metric | note`), risk + automation + provenance, manifest XDG-resident con sha256-pinned entries + `sddk uat verify-integrity` (exit 0=ok / 0=partial / 1=fail)
  - feat(uat): history aggregator — `sddk uat history --release X --plan P --sessions S1 [--sessions S2 ...]` con per-scenario `runs_total/passing/failing/blocked`, `success_rate`, `flakiness_score`, `first/last_run` (con commit + tester_id), `defect_ids[]`, `avg/p95_duration_ms`, `trend`
  - feat(uat): wizard v2 (browser) — pre-flight checklist, sticky context bar (window/est-ceiling/risk/help), typed steps (shell/api → `<pre>`, ui/file/manual → prose), typed evidence capture por `evidence.kinds[]`, failure protocol flow con checklist + auto-filled defect template + clipboard copy + `linked_defect`, teardown checklist, persistent tester id `T-XXXX`
  - feat(uat): wired dashboard → control plane — `sddk uat open` levanta HTTP server en `127.0.0.1:0` (OS-assigned), wizard POSTea `/ingest`, server cierra con Ctrl+C vía `AtomicBool` shutdown flag. Mismo origen (GET / sirve el wizard HTML) → sin CORS
  - feat(uat): suggester + apply — `sddk uat scenario-context --plan FILE [--apply]` reglas deterministas (timing desde `est_minutes`, preconditions desde `step.kind`, risk desde `priority`, evidence default Note, automation Manual, provenance desde plan metadata); `user_story` queda placeholder para humano/LLM

### Fixes
  - fix(uat): wizard script order — `storage.js` debe cargar antes de `plan.js`/`wizard.js` (window.storage undefined rompía init)
  - fix(uat): collapse nested if-let en `apply_suggestion` user_story branch (clippy collapsible_if)
  - fix(uat): `uat history` acepta `--sessions X Y` (positional, `num_args = 1..`) además de `--sessions X --sessions Y`
  - fix(docs): replace all `.sddk-shared/` paths con CWD + XDG bundle runtime — 12 referencias en 8 archivos (AGENTS.md, docs/, scripts/, knowledge vault)

### Other
  - refactor(namespace): normalizar superficies a `sddk-*` y cero intrusión (ADR-0011) — `orchestrator`/`sddk-*`/`prompts/sddk/` activos; aliases `sdd-*`/`sdd-kernel-*`/`gentle-orchestrator` eliminados; cero ficheros framework plantados en repos de proyectos
  - docs(agents): AGENTS.md — directorio layout (asdf-vm inspired) + regresiones detectadas + recovery procedures + pre-commit checklist + 3 roles (repo de desarrollo / bundle runtime / workspace de uso) + resolution order
  - docs(agents): add session handoff section (current state + next steps) — qué está implementado, qué queda pendiente, cómo reabrir la sesión
  - docs(generated): regenerar inventory/workflow y alinear SPEC/BACKLOG/ADR con cero intrusión — alineado con RS-2026-08 / CP-2026-08

## [1.5.3] - 2026-08-07

Cierra U5 del milestone UAT-2026-08: el gate `release-uat-approved` deja de ser inerte — ahora se evalúa contra la config del proyecto (XDG) por tipo de release.

### Features
  - feat(uat): `sddk uat config show|set` — config per-proyecto XDG-resident (`~/.local/share/sddk/projects/<id>/uat.toml`): política `release_gate` por tipo (major/minor/patch → required/skip/advisory), `human` (developer/architect availability), `activation` (umbrales min_features/min_diff_lines/critical_domains). Default: major+minor=required, patch=skip.
  - feat(uat): `sddk uat gate release --tag X [--previous-tag Y|--release-type major|minor|patch]` — evalúa `release-uat-approved` para el release type derivado (semver diff). Emite `BLOCKED` con plan de recovery cuando `required`, `ALLOWED` cuando `skip`/`advisory`. JSON para orquestadores.
  - feat(uat): `UatConfig` + `ReleaseGateAction` (required/skip/advisory) + `ReleaseType` (major/minor/patch) en `sddk-domain`. Funciones puras: `evaluate_release_gate()`, `release_type_from_diff()`.

### Fixes
  - fix(uat): gate `release-uat-approved` ya no es inerte — antes declarado sin requires en transiciones; ahora evaluado dinámicamente por el orchestrator antes de tagear.

## [1.5.2] - 2026-08-07

Consolida el milestone UAT-2026-08 (U1-U7) y las correcciones post-1.5.0: cierra el loop humano end-to-end (wizard canónico → ingest → failures → agente estudia).

### Features
  - feat(uat): `sddk uat plan/validate/dashboard/ingest/report/status` — data-driven YAML canónico (ADR-0012)
  - feat(uat): `sddk uat open` — render dashboard + abrir en navegador del sistema sin servidor (file://); SO-aware (xdg-open/open/cmd-start); `--browser` override
  - feat(uat): `sddk uat failures` — lista FAIL/BLOCKED con contexto completo (feature, priority, assignee, rationale, comment, evidence); JSON para que el agente estudie cada fallo
  - feat(uat): dashboard kit en bundle (`assets/uat-dashboard/`) — kit/templates/views (guided/matrix/traceability); templates HTML inlinean JS+CSS (100% autocontenido, ADR-0010)
  - feat(uat): workflow fase `uat` + status `UAT_WAITING` + gates `uat-activated/uat-verdict/release-uat-approved` (ADR-0012)
  - feat(uat): control plane `uat_results` (verdict, coverage, defects por tag_version) + panel "UAT readiness" en dashboard de telemetría
  - feat(uat): 4 agentes (`uat-planner/guide/runner/reporter`) + 4 skills (`uat-dashboard/traceability/guided-mode/evidence`)

### Fixes
  - fix(uat): views HTML inlinean storage.js/components.js (Chrome bloqueaba scripts file:// vía CORS — el HTML ahora es 100% autocontenido y abre vía file://)
  - fix(uat): wizard canónico — `Finalizar y exportar reporte` genera JSON con la forma exacta de `UatSession` (schema_version, executor, executed_by, started_at, finished_at, results con evidence por hash); compatible directo con `sddk uat ingest`
  - fix(uat): guard de integridad en `uat ingest` — `executor: human` exige `executed_by` + `finished_at` + (evidencia o non-PASS); rechaza sesiones humanas fabricadas
  - fix(agents): `uat-planner` craft rule 9 — quoting YAML-safe (textos con `:` rompen el plan; hallazgo del dogfooding)
  - fix(skills): contradicciones ADR-0011 v3.5 — `adopt apply` ya no planta `workflow/workflow.yaml`; política Local-Only v3.3→v3.5 (docs al knowledge vault)
  - fix(tests): workspace completo verde 202+ tests (AdoptionStoragePaths new fields en test domain + unused binary)

## [1.4.0] - 2026-08-07

### Features
  - feat(uat): milestone UAT-2026-08 U1-U7 — dashboard kit en bundle (assets/uat-dashboard), dominio uat.rs, CLI uat plan/validate/dashboard/ingest/report/status, workflow fase uat + status UAT_WAITING + gates uat-activated/uat-verdict/release-uat-approved, control plane uat_results + panel "UAT readiness" en dashboard telemetría, agentes uat-planner/guide/runner/reporter + 4 skills (ADR-0012/0013, RF-019/020, RNF-010)
  - feat(uat): U8 dogfooding parcial — uat-plan v1.5.0 (6 features, 13 escenarios), dashboard guiado generado y validado (determinismo, cero URLs externas); la sesión humana queda PENDIENTE de validación real (la sesión inicial fue fabricada por el agente y eliminada del control plane)

### Fixes
  - fix(agents): uat-planner craft rule 9 — quoting YAML-safe (colon-space rompe el plan; hallazgo del dogfooding)
  - fix(skills): contradicciones ADR-0011 — adopt no planta workflow.yaml (C1/C2), política Local-Only v3.3→v3.5 (C3/C4, docs al knowledge vault)
  - fix(tests): workspace completo verde — AdoptionStoragePaths new fields en test domain + unused binary (202 tests PASS)

## [1.4.0] - 2026-08-07

### Features
  - feat(telemetry): G5 research packet cross-proyecto — analytics research --all-projects desde control plane + resumen por proyecto (CP-2026-08)
  - feat(rs): RS-6 resolución de versión asdf — sddk version con .sddk-versions → current → path: (ADR-0011)
  - feat(rs): RS-5 bundle runtime multi-versión — dev use (asdf-style) + dev link/update resuelven framework activo (ADR-0011)
  - feat(rs): RS-4 generate docs/inventory → XDG por defecto con --in-repo explícito (ADR-0011)
  - feat(rs): RS-3 cycle artifacts en XDG — cycle artifacts-dir + prompts/skills a {cycle-artifacts-dir} (ADR-0011)
  - feat(rs): RS-1 multiplataforma dirs + RS-2 adopt/lint no intrusivos — cero ficheros framework en repos de proyectos (ADR-0011)
  - feat(telemetry): control plane local — telemetry ingest/aggregate/status/dashboard + metrics record upsert (CP-2026-08 G1-G4)
  - feat(distribution): ALL Linux builds standalone (musl static) — aarch64 included (#92)
  - feat(validation): E2E suite — install variants, render, multi-language validation (#91)

### Fixes
  - fix(rs): framework_agent_names fallback a agentes del bundle sin permissions.yaml (RS-7 migración)
  - fix(ci): update release PR branch when behind before auto-merge (#83)
  - fix(ci): tag-release reads version from origin/main, not dirty worktree (#86)

### Other
  - docs(control-plane): CP-2026-08 IMPLEMENTADO — README control plane, milestone cerrado, backlog 49/49 (ADR-0009/0010)
  - docs(roadmap): RS-2026-08 IMPLEMENTADO — milestone cerrado, backlog E12 completa (ADR-0011)
  - docs(roadmap): milestones CP-2026-08 (ADRs 0009/0010) + RS-2026-08 (ADR-0011, modelo asdf, multiplataforma) — specs, PRD, backlog
  - docs(control-plane): CP-2026-08 milestone — ADRs 0009/0010, spec, PRD RF-016/017, roadmap, backlog E11
  - docs(validation): N3 editor checklist PASS — E2E-2026-08 fully closed (8/8)
  - docs(roadmap): E2E-2026-08 milestone implemented — 7/7 suites PASS (#91)
  - test(cli): environment-robust doctor test; docs: local-first CI via act (#90)
  - docs(validation): E2E validation plan — install, deploy, multi-language, render (#89)

## [1.3.0] - 2026-08-06

### Features
  - feat(cli): completion install — installs shell completions (#84)

### Fixes
  - fix(ci): gh pr list --head does not glob; filter with startswith (#81)

## [1.2.0] - 2026-08-06

### Features
  - feat(ci): release robot — cron poller that removes all bot friction (#79)

### Fixes
  - fix(ci): dispatch Release workflow explicitly from tag-release (#77)

## [1.1.0] - 2026-08-06

### Features
  - feat(ci): fully automatic release pipeline (#71)
  - feat(distribution): hardened installer, completions, signed assets, brew tap (#66)
  - feat(install): interactive installer with framework release bundle (#64)

### Fixes
  - fix(ci): trigger via Auto-merge workflow_run + anti-loop (#74)
  - fix(ci): extract pending tag from explicit new-tag line (#73)
  - fix(ci): reindent release PR body block (invalid YAML) (#72)
  - fix(install): cosign identity regexp + dev update creates missing root (#70)
  - fix(release): pin cosign v3.8.1 — v4.1 sign-blob breaks on output-signature/certificate (#69)
  - fix(release): build darwin-x86_64 cross from arm64 runner (macos-13 retired) (#68)
  - fix(agents): normalize frontmatter models to provider-qualified names (#63)

### Other
  - docs(validation): v1.0.0 published (#62)

## [1.8.0] - 2026-08-11
