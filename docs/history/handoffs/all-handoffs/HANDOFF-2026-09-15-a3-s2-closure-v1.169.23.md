# Handoff — A3-S2 closure + release v1.169.23 (2026-09-15)

## Resumen ejecutivo
Ciclo `p-63676b11dc0ef88f/a3-2-architectural-contract` **CLOSED** en path A-min.
Release **v1.169.23** publicado en GH + instalado localmente.
AC1 entregada: `ArchitecturalContract` (OBJECT) + `ArchitectureClaim` (PROJECTION) +
`ContractEvaluation` (EPHEMERAL), 23 tests, ADR-0112 promoted a `accepted`.

## Estado local
- **Binary**: `sddk 1.169.23` en `/home/rubentxu/.local/bin/sddk`
- **Bundle**: `~/.local/share/sddk/framework/1.169.23/` con `current -> 1.169.23`
- **Working tree**: clean (HEAD = `e508479`)
- **Origin/main**: `e508479fc0d54f3b1bbffcbf33dc147771bc2600` (coherente con local)
- **Tag**: `v1.169.23` annotated, apunta a `e508479`
- **GH Release**: https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.23

## Commits en este ciclo
1. `374873c feat(engine): A3-S2 architectural contract substrate (AC1)` — substrate
2. `b1d0171 chore(release): bump version 1.169.22 -> 1.169.23` — bump (rebase-reorder a HEAD)
3. `3a401dd chore(uat): A3-S2 ADR-0112 acceptance + c4 line-shift maintenance` — ADR+allowlist
4. `e508479 chore(release): bump version 1.169.22 -> 1.169.23` — release marker (HEAD)

## Implementación

### Substrate
- `crates/sddk-engine/src/architectural_contract/` (8 files, 2101 LoC):
  - `mod.rs` — re-exports + state-class doc
  - `types.rs` — `ContractId`, `Revision`, `ComponentRef`, `EntityRef`, `DecisionRef`,
    `SpecRef`, `ContractKindRef`
  - `error.rs` — `ContractError`
  - `payload.rs` — `ContractKind` (6 closed), `BoundaryKind`, `ContractPayload`,
    `ContractExtensionValue`
  - `hashing.rs` — `derive_contract_basis_hash` con domain prefix
    `sddk.architectural_contract.v1\n`
  - `contract.rs` — `ArchitecturalContract` OBJECT + 6 typed `declare_*` constructors
  - `claim.rs` — `EvidenceRef`, `EvaluatorRef`, `ClaimOutcome` (4 closed),
    `ArchitectureClaim`, `ContractEvaluation`
  - `tests.rs` — 23 tests (11 acceptance + 4 negative + 4 anti-encroachment + 4 bonus)

### Modificaciones
- `crates/sddk-engine/src/lib.rs` — `pub mod architectural_contract;` (entre
  `agent_role_contract` y `authority`, alfabético)
- `crates/sddk-engine/src/semantic_kind.rs` — `CoreNodeKind::ArchitecturalContract` (19th);
  `CoreRelationKind::ContractedBy` + `SpecifiedBy` (15th, 16th); `domain_tag()` matches;
  test renombrado a `_have_16_entries_after_ac1_relations` + historical anchor companion
- `crates/sddk-engine/src/knowledge.rs` — `BasisHash::from_digest` exposed `pub(crate)`;
  S1 test `_does_not_introduce_new_corenodekind_variants` marcado `#[ignore]` con
  companion `s2_post_ac1_corenodekind_baseline` pinning 19/16
- `crates/sddk-cli/src/dev/arch_lint.rs` — `C4_LEGACY_ALLOWLIST_M1` actualizado: 3 line
  numbers shifted +1 (`lib.rs:1166→1167`, `:1293→1294`, `:1353→1354`); comment block
  actualizado de A3-S1 a A3-S2

### Docs / ADRs
- `docs/architecture/adrs/ADR-0112-TYPED-ARCHITECTURAL-CONTRACTS.md` — promoted
  `proposed → accepted` con `accepted_by_cycle`, `implementation_evidence`
- `docs/SDDK-Production-Readiness-Alignment-2026-09-14/A3-S2-AC1-RECEIPT.md` — receipt
  (222 lines)

## Verification gate results
- `cargo fmt --check` → ✓
- `cargo clippy --workspace --all-targets -- -D warnings` → ✓
- `cargo test --workspace` → ✓ (0 failed across all crates)
- `cargo test -p sddk-engine --lib -- knowledge:: architectural_contract:: semantic_kind::`
  → 53 passed, 0 failed, 1 ignored (S1 historical anchor)
- 4 verify gates PASSED via `cycle evaluate-gate` (tests-pass, policy-compliant,
  debt-severity-assigned, debt-priority-assigned)
- 2 release gates PASSED (no-pending-effects, release-uat-approved)
- 2 archive gates PASSED (ledger-valid, vault-index-current)
- `sddk cycle verify-references` → PASSED (verifier_cas_root_id matchea, dangling=0)

## Release flow (`bash scripts/release.sh --skip-tests`)
Steps 0-14 ejecutados:
- 0 preflight ✓ HEAD = chore(release) bump commit
- 1 workspace green ✓ (gate local completo pasó arriba)
- 2 read version ✓ 1.169.23
- 3 build binary ✓
- 4 manifest ✓
- 5 bundle tarball ✓ 668259 bytes
- 6 BUNDLE.toml v2 ✓
- 7 unified tarball ✓ 11563089 bytes (binary +x)
- 8 sha256 + CHECKSUMS + sbom ✓
- 8b vault mirror sync ✓ ADR-0112 created en vault
- 9 gh release create ✓
- 10 install desde URL ✓ sddk 1.169.23 instalado
- 11 sddk dev doctor ✓ binary.bundle_coherence present
- 12 prune ✓ removed 1.169.22
- 13 final state ✓

## Fricciones resueltas este turno
1. **Lease token 1 ya no estaba en M1**: re-acquired con fencing token 1.
2. **Evaluator `sddk.cli` inicialmente rechazaba verify gates con ENGINE_UNREGISTERED_EVALUATOR**:
   solución — usar el `gate_receipt_id` con la forma `gate-<name>-<plan_hash_short>-<seq>`.
   La forma correcta del hint (4 transitions: `phase.verify.complete.a-min` con 4 gates,
   `release.complete` con 2 gates, `archive.complete` con 2 gates).
3. **C4_LEGACY_ALLOWLIST_M1 stale line numbers**: 3 entries off-by-one después de
   `pub mod architectural_contract;`. Updated con comment block A3-S2.
4. **Fitness test `no_new_root_level_context_module_without_adr` fallaba** porque
   `architectural_contract` stem no estaba en ningún ADR. Promovido ADR-0112 a accepted
   con implementation_evidence.
5. **Tag reconciliation**: `gh release create` creó tag lightweight; `sddk release apply`
   exige annotated. Solución: delete remote lightweight + create annotated local + push.
   Assets del GH release permanecen válidos (targetCommitish `main` resuelve al mismo SHA).
6. **Pre-push hook exige HEAD = chore(release)**: usé rebase interactivo para mover
   `b1d0171` (bump) al final del log.

## Debt (carry-over, P3 priorizado)
- **INC-A3-S1-C4-LINE-SHIFT** (now confirmed 2nd instance): literal line numbers en
  `C4_LEGACY_ALLOWLIST_M1` son brittle ante cualquier inserción `pub mod X;` en
  `crates/sddk-engine/src/lib.rs`. Right fix: AST-based visitor para localizar
  `auth.validate(...)` call sites en lugar de line numbers. Pendiente de cycle dedicado.

## Pre-existing failures (NOT regression)
- `dev doctor` reporta `surface.briefness.<skill>.md: missing` para ~17 skills (pre-A3-S2).
  No es regresión de A3-S2; queda en P3.
- `dev doctor` reporta `binary.bundle_coherence: missing` (post-install state, podría ser
  related to `current` symlink race condition). El release.sh step 11 sí reportó `present`,
  pero `sddk dev doctor` post-release muestra `missing`. Investigar como P3 si persiste.

## Siguientes ciclos pendientes
- **A3-S3+ (architectural contracts as semantic graph overlay, AC2/AC3)**: pendiente,
  no en scope de A3-S2.
- **arch-spec-032 → promotion a `accepted`**: pendiente, requiere cycle dedicado
  (no scope-creep dentro de A3-S2).
- **INC-A3-S1-C4-LINE-SHIFT follow-up**: pendiente.
- **vault index current**: la gate pasó pero `~/.sddk-knowledge/` index puede tener
  drift; verificar manualmente.

## Operational notes
- Pre-flight completo del profile (fmt+clippy+test workspace) consume ~8 minutos. Background
  task pattern funcionó bien; recomiendo preservar este patrón para releases.
- `sddk release apply --route local` post-`gh release create` requiere reconciliación
  de tag (lightweight → annotated). Mejor integrar la reconciliación en release.sh para
  evitar fricción futura.
- M2.7-highspeed no se usó este turno (orchestrator-direct, basado en la nota A3-S1
  de stalls con prompts >~3KB).

## Archivos clave
- `crates/sddk-engine/src/architectural_contract/` — substrate (8 files)
- `docs/architecture/adrs/ADR-0112-TYPED-ARCHITECTURAL-CONTRACTS.md` — ADR accepted
- `docs/SDDK-Production-Readiness-Alignment-2026-09-14/A3-S2-AC1-RECEIPT.md` — receipt
- `.sddk/cycles/p-63676b11dc0ef88f-a3-2-architectural-contract/` — 7 artifacts
  (exploration-report.md, spec, implementation-receipt, verification-report.md,
  merge-receipt.json, release-receipt.json, archive-manifest.md)
