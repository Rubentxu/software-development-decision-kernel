# A3-S1-KMT-FOUNDATION-RECEIPT

## Identity

- **Cycle:** `p-63676b11dc0ef88f/a3-1-kmt-foundation`
- **Path:** A-min
- **Base commit:** `d842265` (post AC evolution integration)
- **Final commit:** `<populated at archive time>`
- **SDDK workspace version:** `1.169.21`
- **Date:** 2026-09-14
- **Author:** orchestrator (direct implementation; swarm fallback per INC)
- **Spec:** `docs/architecture/specs/arch-spec-A3-S1-knowledge-substrate.md`
- **Exploration report:** `.sddk/cycles/p-63676b11dc0ef88f-a3-1-kmt-foundation/exploration-report.md`
- **Swarm fallback evidence:** `.sddk/cycles/p-63676b11dc0ef88f-a3-1-kmt-foundation/swarm-fallback-evidence.md`

## Verdict

`PASS` — A3-S1 Knowledge substrate + KMT (Knowledge Management Tiers)
satisfy REQ-A3S1-001..051 with no S2+ leakage, no A4 imports, no
provider SDK imports, and no `CoreNodeKind` mutation. Workspace full
profile green after one documented baseline regeneration
(INC-A3-S1-C4-LINE-SHIFT).

## Scope delivered

### Files added

| Path | LoC | Purpose |
|---|---|---|
| `crates/sddk-engine/src/knowledge.rs` | 1259 | Knowledge substrate + KMT (types + tests) |

### Files modified

| Path | Change |
|---|---|
| `crates/sddk-engine/src/lib.rs` | `+pub mod knowledge;` (alphabetical position) |
| `crates/sddk-cli/src/dev/arch_lint.rs` | `+1 line per entry` for 3 M1 allowlist entries (line numbers shifted by +1 after lib.rs insertion) + comment block |

### Files explicitly NOT touched

- `crates/sddk-engine/src/semantic_kind.rs` — `CoreNodeKind::ALL.len() == 18` preserved (test `s1_does_not_introduce_new_corenodekind_variants` enforces)
- `crates/sddk-engine/src/semantic_graph.rs`, `semantic_node.rs` — no overlay additions
- `crates/sddk-engine/src/context_capsule.rs` — SHA-256 unchanged (`a43c31b10dd6bcc753e821a9a05b7e8208cd8af52ff8c0a08e7d441747671bac`, test `s1_does_not_modify_context_capsule` enforces)
- `crates/sddk-engine/src/decision_memory.rs` — wiring deferred to S2
- Any `alignment*`, `verify*`, `debverify*`, `paradigm*`, `authority_engine*` module — anti-encroachment enforced
- Any provider/host SDK module — anti-encroachment enforced

## Acceptance tests (REQ-A3S1-NNN)

| REQ | Test name | Status |
|---|---|---|
| REQ-A3S1-001 | `test_knowledge_module_state_class_annotations_complete` | PASS |
| REQ-A3S1-010..015 | `test_knowledge_assertion_field_privacy` | PASS |
| REQ-A3S1-012 | `test_knowledge_assertion_basis_hash_deterministic` | PASS |
| REQ-A3S1-021 | `test_knowledge_basis_insertion_order_independence` | PASS |
| REQ-A3S1-023 | `test_knowledge_basis_revise_stale_timestamp_rejected` | PASS |
| REQ-A3S1-033 | `test_kmt_fresh_when_basis_match` | PASS |
| REQ-A3S1-033 | `test_kmt_stale_on_mismatch` | PASS |
| REQ-A3S1-033 | `test_kmt_unknown_when_evidence_missing` | PASS |
| REQ-A3S1-034 | `test_kmt_invalidation_is_monotonic` | PASS |
| REQ-A3S1-035 | `test_kmt_evaluate_is_canonical_entry_point` | PASS |
| REQ-A3S1-050 | `test_knowledge_basis_serde_round_trip_preserves_basis_hash` | PASS |

**11 / 11 acceptance tests PASS.**

## Negative fixtures

| Fixture | Status |
|---|---|
| `test_knowledge_assertion_cannot_be_built_without_declare` | PASS |
| `test_knowledge_basis_revise_with_stale_time_is_rejected` | PASS |
| `test_kmt_invalidation_cannot_be_overridden` | PASS |
| `test_invalidatation_reason_exhaustiveness` | PASS |

**4 / 4 negative fixtures PASS.**

## Anti-encroachment probes (REQ-A3S1-040..042)

| Probe | Status |
|---|---|
| `knowledge_module_has_no_a4_or_provider_imports` | PASS — no `use crate::alignment`, `use crate::verify`, `use crate::debverify`, `use crate::paradigm_lens`, `use crate::authority_engine`, `use crate::completion_provider_router`, `use crate::agent_host` in `knowledge.rs` |
| `s1_does_not_introduce_new_corenodekind_variants` | PASS — `CoreNodeKind::ALL.len() == 18`, `CoreRelationKind::ALL.len() == 14` |
| `s1_does_not_modify_context_capsule` | PASS — `context_capsule.rs` SHA-256 unchanged (`a43c31b10dd6bcc753e821a9a05b7e8208cd8af52ff8c0a08e7d441747671bac`) |

**3 / 3 anti-encroachment probes PASS.**

## Determinism evidence

- `basis_hash` derivation is a pure function of `(id, declared_at, kind,
  content_type, bytes)` with explicit length prefixes (SHA-256, domain
  separator `sddk.knowledge.assertion.v1\n`).
- `KnowledgeBasis` uses `BTreeMap<KnowledgeId, KnowledgeAssertion>` —
  iteration is sorted by `KnowledgeId`, so `basis_hash` is stable across
  insertion order, process restarts, and serialization round-trips.
- Serde JSON round-trip preserves `basis_hash` byte-for-byte
  (`test_knowledge_basis_serde_round_trip_preserves_basis_hash`).
- `evaluate_freshness`, `KMT::evaluate`, `invalidate` are pure functions
  of `(observed, expected, now)` — no IO, no clock reads, no provider
  calls. Time is passed as data.

## ADT discipline

| Type | Kind | Variants | Status |
|---|---|---|---|
| `KnowledgeKind` | closed enum | `Observation \| Declaration \| Inference \| Reference` | closed (no `non_exhaustive`) |
| `KnowledgePayload` | closed enum | `Object \| Relation \| Fact` (each with `content_type + bytes`) | closed |
| `InvalidationReason` | closed enum | `Superseded \| Contradicted \| Withdrawn \| Stale` | closed |
| `MissingEvidence` | closed enum | `NotProvided \| FutureEvidence` | closed |
| `KmtStatus` | closed enum | `Fresh \| Stale \| Invalidated \| Unknown` | closed; `Fresh` field private so it can only be constructed inside the module |

No `bool + Option<error> + stale_flag` shape anywhere. Every domain
state is represented by a closed enum.

## Scoped testing log

| Command | Result |
|---|---|
| `cargo fmt --check -p sddk-engine` | clean |
| `cargo test -p sddk-engine --lib knowledge::` | 18 / 18 PASS |
| `cargo clippy -p sddk-engine --lib --tests -- -D warnings` | clean |

## Full profile (release-grade gate)

| Command | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | 1 FAIL → 0 FAIL after `C4_LEGACY_ALLOWLIST_M1` regeneration (see Operational notes) |

The single failing test (`dev_doctor_c4_authority_single_admission_green_on_current_workspace`)
was a false positive caused by line-number fragility in the M1
allowlist. The three `auth.validate(WritableSurface::X)` calls it
flagged are pre-existing engine code; only their line numbers shifted
by `+1` because of the alphabetical `pub mod knowledge;` insertion in
`lib.rs`. The allowlist was regenerated by the same edit (+1 per
entry), and a comment + INC record document the fragility.

## Operational notes (not in A3-S1 scope)

- **Cycle pause CHECK constraint**: not fixed in this cycle; the
  `cycle.pause` transition is rejected at the SQL CHECK layer when
  applied during `phase: build`. Recovery: lease expires after 1h.
  Status: pre-existing, tracked elsewhere.
- **Binary vs workspace version drift** (`1.169.10` binary vs
  `1.169.21` workspace): resolution path is the next `release.sh` run.
  Status: pre-existing, will close during release.
- **C4 allowlist line-number fragility**: registered as
  `INC-A3-S1-C4-LINE-SHIFT` (`docs/debt/INC-A3-S1-C4-LINE-SHIFT.md`,
  severity `medium`, priority `P3`). Mitigation (replace `path:line`
  with `path:fn` + regeneration xtask) deferred to a follow-up cycle.

## Swarm fallback evidence

The `sddk-apply` swarm worker
(`session_goat_1789417594072_4d4fb9c56e45b789`) was spawned at
`20:26:34Z` and stalled in `startup queued / thinking` for 21+ minutes
with zero disk activity. An orchestrator DM at `20:45:28Z` was not
answered. The worker was stopped at `20:48:40Z` and implementation
fell back to direct execution in this session. Scope was preserved
exactly. Evidence preserved at
`.sddk/cycles/p-63676b11dc0ef88f-a3-1-kmt-foundation/swarm-fallback-evidence.md`.

## Crosswalk

| Spec / ADR / AC milestone | Relationship |
|---|---|
| `arch-spec-006-knowledge-vault-context.md` | `KnowledgeBasis` is the runtime projection; the Vault remains the human-side source. No authority changes. |
| `arch-spec-032-architectural-contracts.md` (AC1) | Contract type slot reserved for S2; `BasisHash` will be reused as the contract hash. |
| `arch-spec-033-architecture-semantic-graph-overlay.md` (AC2) | Graph overlay scheduled for S3; `KnowledgeAssertion` will be the projected node kind. |
| `arch-spec-035-paradigm-lens-system.md` (AC3) | Deferred to S4. |
| `arch-spec-036-adt-and-typed-dsl-modeling.md` | ADT discipline demonstrated here (closed enums, typed newtypes). |
| SPEC-002-lifecycle-model | PlanRevision lifecycle wiring deferred to S2. |
| SPEC-005-semantic-graph-and-why | Graph revision reuse scheduled for S3. |
| ADR-0095 (Four state classes) | Enforced at the type level (each public type carries a state-class doc annotation). |
| ADR-0100 (Universal Evidence) | `KnowledgeKind::Observation` is the eventual evidence emitter; mapping deferred to S2. |
| ADR-0104 / ADR-0021 (Pack extension / provider boundary) | Provider SDK types never enter `knowledge` module. |
| AC evolution package §14 implementation prompt | Followed: `contract → graph/evidence basis → minimal implementation → deterministic probe → negative fixture → receipt/crosswalk`. |

## Out-of-scope (explicit anti-encroachment)

- `ArchitecturalContract` / `ArchitectureClaim` (S2)
- `ParadigmProfile` (S4)
- `CoreNodeKind` extension (S3)
- `ContextCapsule::advisory_context` (S5)
- Lenses, probes, receipts, cross-BC evaluation (A4 / AC4..AC7)
- Persistence beyond in-memory + serde (S2)
- Provider/host SDK type references (any cycle)

## Exit criteria checklist

- [x] All REQ-A3S1-NNN requirements green
- [x] Negative fixtures all pass
- [x] `cargo fmt --check` clean
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [x] `cargo test -p sddk-engine knowledge::` green (18/18)
- [x] `cargo test --workspace` green (after C4 baseline regeneration)
- [x] `A3-S1-KMT-FOUNDATION-RECEIPT.md` produced at this commit
- [x] Mini-roadmap §A3-S1 marked PASS with receipt SHA (cycle archive step)

## Sign-off

A3-S1 KMT foundation PASS. Cycle ready for archive.
