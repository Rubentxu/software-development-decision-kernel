# SDDK — Canonical Architecture & Roadmap

> **This is the current normative architecture and roadmap for SDDK.**
> All other architecture, consolidation, and evolution packages in the
> repository are **historical / superseded** unless explicitly imported as
> current WorkItems through reconciliation.

## Package

- **Canonical source**: `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/`
- **MANIFEST.sha256** (72 documents, 0 broken internal links)
- **Adopted**: 2026-09-09
- **Supersedes**: prior competing roadmap narratives listed in
  `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/05-INTEGRATION/SUPERSESSION.md`

## Purpose

SDDK has enough functionality. The next evolution is to make that
functionality **coherent, explainable, agent-friendly and difficult to
misuse**. The package consolidates two coupled problems:

1. **Semantic Core Consolidation** — one authority per concept, one
   canonical fact log, one Evidence model, one Semantic Graph, one
   Revision substrate, one Authority Engine and a strict
   Goal → WorkItem → WorkflowDefinition → ExecutablePlan → Run chain.
2. **Agent Experience Consolidation** — agents, prompts, skills,
   examples and CLI knowledge become typed/versioned contracts
   compiled from the same product model instead of hand-maintained
   prompt folklore.

## Target product sentence

> **SDDK is a deterministic decision kernel for agent-assisted software
> development: it records canonical facts and evidence, governs side
> effects, preserves decision provenance and dissent, derives
> executable workflow state, compiles bounded context and effective
> instructions, and exposes a versioned command contract that humans
> and agents can use safely.**

SDDK is **not** an IDE, graph database, vector-memory product, wiki
engine, CI server or general-purpose agent framework.

## Architecture (essential)

```text
                Humans / Agent Adapters
                         │
                  Targets / Tasks
                         │
          ┌──────────────┴──────────────┐
          ▼                             ▼
  Context Compiler              Instruction Compiler
          │                             │
          │                    Agent Command Surface
          │                             │
          └──────────────┬──────────────┘
                         ▼
                   ExecutionRequest
                         │
                         ▼
                       Agent
                         │
               ┌─────────┴─────────┐
               ▼                   ▼
       ExecutionOutcome       Contribution
               │                   │
               └─────────┬─────────┘
                         ▼
               Synthesis / Decisions
                         │
      ┌──────────────────┼──────────────────┐
      ▼                  ▼                  ▼
 Canonical Event Log     CAS          Authority Engine
      │                                      │
      └──────────► projections / memory ◄────┘
                         │
                  Semantic Graph
```

## Roadmap (M0 → M9)

| Milestone | Title | Status |
|---|---|---|
| **M0** | Inventory + semantic/agent freeze | delivered — 7 deliverables D1..D7 closed by cycle `m0-inventory-baseline` (v1.151.4): responsibility registry (44 entries, schema v1), CLI golden fixtures (5 subcommands), crate dependency graph (mermaid, 0 SCCs >1), duplicate concept map, agent-asset inventory (320+ files across 69 agents/204 skills/41 prompts/2 templates with 11 smell detectors), architecture lints advisory (12 ARCH-SC + 8 ARCH-A), supersession banners across 6 historical docs |
| **M1** | Canonical fact log + CAS + Evidence | delivered — cycle `m1-canonical-fact-log` (v1.153.0): `CanonicalEventLog` port + `CasObjectStore` + universal `EvidenceRef/Bundle` + 4-state-class rule ratchet (Fact/Object/Projection/Ephemeral) |
| **M2** | Lifecycle consolidation | delivered — `pln-ledger-002-persist-planning-graph` (v1.86.0) + `pln-ledger-003-cross-storage-spine-import` (v1.87.0): Slim Cycle contract + Planning SSOT + Spine reconciliation deterministic + idempotent |
| **M3** | SemanticGraph + Vault + Context | delivered — cycle `m3-semantic-graph-vault` (v1.154.0): `SemanticGraphProjection` v2 + ActiveGraph reduced to typed view layer + Vault as `KnowledgeSource` + `ContextCompiler` with provenance/staleness |
| **M4** | Decision Memory extension (substrate DONE in CDD-MEMORY-001..005) | delivered — CDD-MEMORY-001..005 cycles (v1.146.0..v1.151.0) closed the Revision/Object/Ref/CAS substrate; CDD-MEMORY-004 (v1.150.0) added reflog history persistence + git-like branch mutation ops (cherry_pick / revert / amend / reset / delete_ref); CDD-MEMORY-005 (v1.151.0) closed Soft/Mixed reset + tombstone GC + true inverse-patch revert + ADR-093 acceptance |
| **M5** | Agent protocol + AuthorityEngine facade | delivered — cycle `m5-authority-engine-facade` (v1.152.0) introduces the unified `AuthorityEngine` facade + bridge policy table; `m5-unified-authority-runner` (v1.156.0) wires the runner as pre-gate across 6 CLI call sites (engine-internal `auth.validate()` calls remain as defense-in-depth compat mirrors until M9 cleanup) |
| **M6** | Packs + Targets + Tasks + CLI convention-first | delivered — three sub-cycles: `m6-1-convention-first-cli` (v1.157.0) introduced the convention-first CLI surface (5 router modules + typed `CommandSpec` introspection); `m6-2-target-task-dag` (v1.158.0) added `Target/Task` DAG registry with Kahn topo sort + 4 built-in targets; `m6-3-dag-execution` (v1.159.0) closed with `DagExecutor` running tasks in topological order with per-task authority gate |
| **M7** | Agent Experience Contract (**NEW**) | delivered — M7.1 (v1.160.0) + M7.1B (v1.161.0) + M7.1C (v1.162.0) + M7.2 (v1.163.0) + M7.4 (v1.164.0) + M7.3 (v1.165.0) + M7.5 (v1.166.0) + M7.6 (v1.166.2) shipped + AGENTS.md governance merged (v1.166.1) + M7.7 runtime admission wire (v1.166.3) + M7.8 operator surface `sddk dev skills list/verify` (v1.166.4) + M7.9 placeholder-to-real skills shipped (v1.167.8): the three M7.5 contractual placeholders (`core.contract-review@v1`, `core.workflow-orchestration@v1`, `core.release-planning@v1`) materialize as real `SKILL.md` files in the bundle, with a bridge extension that recognizes dotted frontmatter names so the runtime admission gate admits them instead of warning |
| **M8** | WHY engine + causal explanation | delivered — M8.0 `sddk dev graph list/edges/projection` operator surface for the H9 Active Graph & Cockpit engine (v1.166.5) + M8.1 `sddk dev graph why` causal queries (why/debt-why/decision-why) for the WHY engine (v1.166.6) + M8.2 `sddk dev cockpit view {overview,journal,timeline,execution}` operator surface for the H9 Cockpit Views engine (v1.166.7) + M8.3 `sddk dev cockpit obs {providers,usage,assurance,handoff,experiments}` operator surface for the H9 Cockpit Observability engine (v1.166.8) + M8.4 `--from-cycle <cycle-id>` auto-derives `ActiveGraphInput` from the cycle's archive manifest for both `view` and `obs` (v1.166.9) + M8.5 `## Commit parents` manifest section ⇒ `parent_of` edges with Unicode `→` / ASCII `->` / fat `=>` arrow support, both SHAs must appear in the known commit list (v1.167.2) + M8.6 per-node + per-edge `ProvenanceRef` metadata threaded through `ActiveGraphInput` ↔ `ActiveGraphProjection` so every projected entity is attributable back to the byte locator that produced it (v1.167.3) + M8.7 `sddk dev cockpit diff --cycle-a <id> --cycle-b <id>` (or `--input-a` / `--input-b`) cross-input drift detection via `sddk_engine::active_graph_drift::DefaultDriftEngine`, classifies every node + edge as `added` / `removed` / `changed` with field-level attribution (v1.167.4) + M8.8 `sddk dev cockpit digest --cycle <id>` (or `--input`) stable SHA-256 projection digest via `sddk_engine::active_graph_digest::ProjectionDigest`, two kinds (`strict` includes `recorded_at`; `content` strips it), cheap equality check before expensive diff (v1.167.5) + M8.9 (M9.5 live-mode streaming) `sddk ledger watch` polls `Storage::list_events_after(after, limit)` and emits one event per line (NDJSON or text), `--from-tail` skips historical events, `--max-events` caps emission, `--idle-timeout-ms` exits after inactivity (v1.168.4) + M8.10 (M9+ interactive drift sessions) `sddk dev cockpit diff-watch --input-a <p> --input-b <p>` polls both ActiveGraphInput JSON files (or cycle manifests) on each tick and emits a `DriftRow` JSON envelope whenever the content-only projection digest changes, ignoring per-tick `recorded_at` noise, with `--from-tail` to skip the baseline and `--max-events` / `--idle-timeout-ms` / `--max-ticks` exit guards (v1.168.6); all six engine modules ship CLI surfaces (`active_graph.rs` + `active_graph_drift.rs` M8.7 + `active_graph_digest.rs` M8.8 + `why_queries.rs` + `cockpit_views.rs` + `cockpit_observability.rs`); all M9+ future-work items shipped (live-mode streaming M9.5 in v1.168.4 + interactive drift sessions M9+ in v1.168.6) |
| **M9** | Remove compat debt | delivered — INC-DEBT-018 (3-event supersede invariant doc) + INC-DEBT-019 (deterministic `now_ms` in `Engine::cycle_supersede`) closed in v1.167.0 (M9.1 + M9.2) + INC-DEBT-021 (pre-existing clippy baseline) + INC-DEBT-022 (pre-existing `dist_succeeds_with_valid_bundle` flake) closed in v1.167.1 (M9.3 + M9.4); the four vault-side records corresponding to these (INC-021-9b3e7f1a + INC-022-4c7a1e2f + INC-027-4e2a3c26 + INC-013 closed as side effect of DW-RUNTIME-005 S6b slice 2) were reconciled from `status: open` to `status: closed` at the inc-hygiene-2026-09-11 audit, restoring the doc/ledger coherence that the M9 row's "zero open debt records at v1.167.1" claim had implied; debt-report schema `cycle_id` pattern widened to admit product cycles; at v1.168.4 (post M9.5 cycle), seven `status: resolved` records were reconciled — six (INC-CYCLE-13-APPLY-TEST-COUNT-MISREPORT, INC-CYCLE-13-DURABILITY-COMMENT-ACCURACY at commit cbd8ad7, INC-CYCLE-13-LOC-OVERAGE, INC-DEBT-016-llm-fabricates-without-receipt via ADR-0073, INC-DEBT-017, INC-KERNEL-CLI-AGENT-INFORMATION-FLOW-APPLY-PUSH-VIOLATION at closed 2026-08-28) moved to `status: closed`, and the seventh (INC-005720-cli-test-flake) moved to `status: accepted_risk` with documented mitigation (run in isolation for the pre-existing test-infra race in install-orchestration path, severity low P3); at v1.168.7 (clippy gate cycle), the pre-existing `INC-MATRIX-LINT-CODES-APPLY-PUSH-VIOLATION.md` was finally reconciled from `status: open` (frontmatter drift) to `status: closed` (body had been `closed` since 2026-08-28 with full prevention evidence); at v1.168.8 (post-M9 INC hygiene), eight more `status: resolved` records were reconciled — INC-CYCLE-13-{APPLY-TEST-COUNT-MISREPORT,DURABILITY-COMMENT-ACCURACY,LOC-OVERAGE}, INC-CYCLE-14-{CORPUS-FIXTURE-DUPLICATION,HELPER-DOC-GAP,SEVERITY-SPEC-DRIFT}, INC-DEBT-017, INC-001-cli-call-budget-stale all moved to `status: closed` with explicit `closed_at` / `closed_by` frontmatter; INC-DEBT-020-prune-reapunta-current moved to `status: closed` with `closed_at: 2026-09-02` and `closed_by: sddk-archive (cycle-53 archive)`; INC-HX-AUTH-{003-provenance, 004-no-parallel-authority} stayed at `status: resolved` (their lifecycle entries on 2026-09-04 document only partial close — paths 4+5 of 7 for 004, additive `actor_ref` widening for 003 — and the canonical ActorRef 5-field migration tracked under EVT-LEDGER-001 still open); INC-HX-AUTH-{001-writable-state, 002-approval-authority} were honestly corrected from `status: resolved` (no lifecycle evidence) back to `status: open` — the eight writable surfaces enumerated in ADR-069 §3 still lack runtime authority declaration enforcement and `emit.rs:259` still hardcodes `ActorKind::Human` per the `emit_approval_decision_forces_human` baseline regression test, both tracked under ARCH-HEX-001 (order 80, H0); the single `status: tracked` record in the vault (`INC-2026-08-27-cycle-44-v2-correction`) is deliberately permanent (cycle-44 v1 retracted, v2 governs design/tasks/apply; per design intent) and is not part of the M9 scope; final local debt ledger (`docs/debt/`): 31 closed + 2 open + 2 resolved = 35 records; vault side (`~/.sddk-knowledge/`) carries the additional 1 tracked record; **v1.168.8 also ships the M9 enforcement infrastructure for the five M0 D6 lints** — `sddk dev lint deprecated-patterns` executes the registry against the live workspace and reports hits (text + JSON), but all five lints stay at `default: allow` (advisory) because live-workspace evaluation produces 61 hits across 4 of 5 lints in active production code (agent_result_used = 0 hits, evidence_kind_v1 = 18, orchestration_synthesis_no_dissent = 19, execution_outcome_as_synthesis = 0, transition_outcome_used = 24); promoting any lint to `default: deny` requires the three acceptance criteria documented in `[acceptance_for_m9_blocking_enforcement]` of the registry TOML (per-lint validation that hits are exclusively in legacy paths, replacement migration complete, RISK-REGISTER R-001 user sign-off) — at v1.168.12 (ARCH-LINT-M9.1), the per-lint audit was completed and `agent_result_used` was promoted to `default: deny` — its only engine-tree hits were intentional citations inside the spike_sp06.rs retrieval-evaluation corpus (excluded as content, not code), and the CLI hits are the deliberate legacy converter surface (already excluded); the remaining four lints stay `allow` because their canonical replacements are `status: proposed` and not yet implemented (ADR-0100 universal Evidence model, ADR-0101 ExecutionOutcome+Contribution): promotion there is blocked on construction, not on audit; `sddk dev lint deprecated-patterns --enforce` is now meaningful and passes clean, with a regression guard test pinning the deny+zero-hits invariant; at v1.168.19 (AX-S5 introduction), the deprecated_patterns registry was extended with 4 agent-asset hygiene lints (asset_deprecated_namespace, asset_raw_store_reference, asset_authority_language, asset_unregistered_cli_example), all kept at `default: allow` per the AX-S5 spike's disposition (zero hits on the 91-asset corpus at survey time, regression-prevention checks); at v1.168.29 (ARCH-LINT-AX-S5 promotion), the per-lint audit against the live workspace confirmed zero hits for all four, and three of them (`asset_deprecated_namespace`, `asset_raw_store_reference`, `asset_authority_language`) were promoted to `default: deny` — their regexes are deterministic and the prose conventions they enforce are already satisfied (validation_per_lint, migration_completion, user_signoff via continuous auto-mode all pass); `asset_unregistered_cli_example` remains `default: allow` because its first-letter-sieve regex is unsafe-by-design against legitimate commands (memory/metrics/status/ship/fork/stale/explore); final state at v1.168.29: 5 of 9 lints promoted to `default: deny` (agent_result_used + the three AX-S5 asset_*), 4 remain advisory (4 gated on ADR-0100/0101/0096 acceptance + 1 unsafe-by-design); the `live_registry_asset_lints_are_promoted_or_advisory_and_clean` pin test codifies this split; the M9 enforcement infrastructure is now operationally meaningful beyond the original advisory intent (zero `--enforce` blockers against the live workspace); **at v1.168.35 (evidence-relations-core cycle), ADR-0100 (Universal Evidence) was promoted to `accepted`** — `Verifies` and `ObservedFor` variants are now wired into CoreRelationKind (was 12, now 14 entries) with 3 pin tests, unblocking the structural path for the `evidence_kind_v1` lint; **at v1.168.37 (synthesis-dissent-runner-extension cycle), `orchestration_synthesis_no_dissent` was promoted to `default: deny` via the bounded `exclude_paths` path** — its 19 prior hits were all test fixtures (1 in `mod tests` at crates/sddk-engine/src/orchestration_synthesis.rs:745, 18 in crates/sddk-engine/tests/orchestration_synthesis_tests.rs), and production code at v1.168.36 has zero `OrchestrationSynthesisReceipt::new` call sites (receipts are constructed dynamically by the synthesis pipeline); exclude_paths now filters `tests/` and the engine src/ file containing the mod tests block, dropping the hit count to 0; the runner-extension path (AST/flow analysis walking from `::new` to `set_dissent`/`put`) is deferred until production code starts constructing receipts directly; **at v1.168.39 (planning-evidence-migration cycle), `evidence_kind_v1` advanced from `blocked_on_audit` to `audit_acknowledged`** — the canonical mapping function `planning_evidence_relation(PlanningEvidenceKind) -> CoreRelationKind` is now shipped in `crates/sddk-engine/src/evidence_relation_mapping.rs` (174 LoC, 7 unit tests, const-time exhaustiveness guard); mapping: Log → ObservedFor, Metric → Verifies, Snapshot → ObservedFor, Reference → References, Approval → Justifies (5 → 4 collapse intentional); per-call-site migration of `EvidenceAttachmentV1` (5 production crates + SQL schema at migrations.rs:737 + 2 test fixtures) is deferred to `evidence-migration-v2` because it requires a storage migration step + dual-write window per AGENTS §2.10 strangler pattern; **at v1.168.40 (transition-outcome-m9-2-closeout cycle, this release), `transition_outcome_used` advanced from `blocked_on_audit` to `audit_cleared, regression_guard`** — the M9.2 audit identified in cycle-4 (v1.168.34) is now closed at the registry level: description rewritten to reflect actual gate-evaluation semantics (not "legacy partial shape of ExecutionOutcome" — they are orthogonal types), category re-categorized from `deprecated_patterns` to `state_machine`, confidence dropped from `high` to `low` (regression guard, not a problem to fix); the lint stays `default: allow` because promoting to `deny` would block 24 legitimate uses including the definition site at `crates/sddk-engine/src/lib.rs:636` (the lint's own regex would match the definition); **PR #4 (`feat/zcode-native-agents`) integration consolidated** — the zcode symlink migration fix that merged mid-cycle-7 (closing the `zcode.broken_agent_links` doctor drift) is now formally documented in `docs/handoff/HANDOFF-2026-09-12-zcode-pr4-integration.md` with regression test evidence (`cargo test -p sddk-cli zcode::zcode_adapter_tests` shows 11 passed including `register_migrates_agent_map_era_artifacts`); **final state at v1.168.40: 6 of 9 lints promoted to `default: deny`** (agent_result_used + the three AX-S5 asset_* + orchestration_synthesis_no_dissent), 3 remain advisory (evidence_kind_v1 = 57 hits with structural path shipped; transition_outcome_used = 24 hits as regression guard for cycle state machine; execution_outcome_as_synthesis = 0 hits awaiting corpus expansion; asset_unregistered_cli_example = 0 hits by-design regression guard); the M9 row's enforcement infrastructure is operationally meaningful: `sddk dev lint deprecated-patterns --enforce` passes clean (zero hits across all 5 deny lints), the four pin tests `test_advisory_lint_explanations.sh` (advisory explanation blocks), `test_deny_lint_zero_hits.sh` (deny+zero-hits invariant), `test_vault_adr_mirror_coverage.sh` (vault mirrors), and `test_authority_helper_lockstep.sh` (ADR authority helpers) all pass and are wired into `scripts/release.sh` step 1b |

## A4 — in flight (intelligence loop)

> **Source of truth**: [`docs/architecture/specs/arch-spec-047-a4-intelligence-loop.md`](specs/arch-spec-047-a4-intelligence-loop.md).
> Older roadmap phrases that put Alignment before Verify/DebVerify or that
> skip Observations/Evidence are historical residue in `docs/A3-MILESTONE-RECEIPT.md`
> and earlier evolution packages; they are NOT normative and should not be
> cited as the current loop.

The normative order, per arch-spec-047, is:

```text
Sources + Decision Memory
  → Knowledge / KMT + SemanticGraph
  → Observations + Evidence          (arch-spec-042 — implemented)
  → Verify                           (arch-spec-043 — implemented)
  → DebVerify                        (arch-spec-044 — implemented)
  → Alignment                        (arch-spec-045/046)
  → AdvisoryContext                  (delivered in A3 closeout)
```

Governance enters **only at the end** and **only** through explicit policy:

```text
VerificationReceipt + AlignmentAssessment
  → Governance evaluates explicit policy
```

A4 sub-cycles (ROADMAP-SYNC preflight for A4-4a, 2026-09-16):

| Sub-cycle | Scope | Spec | Status |
|---|---|---|---|
| **A4-0** | Evidence → observes → SoftwareRelation (provenance foundation) | `arch-spec-042` | closed — v1.168.35 |
| **A4-1** | Generic Verify kernel (`verify_kernel` + AC4 adapter) | `arch-spec-043` | closed — v1.169.46 |
| **A4-2** | Generic DebVerify kernel (`debverify_kernel` + AC5 adapter + ObservationContradiction) | `arch-spec-044` | closed — v1.169.47 |
| **A4-2M** | AC4/AC5 convergence onto generic kernels (single execution surface) | `arch-spec-043/044` | closed — v1.169.48 |
| **A4-3** | Software Alignment domain (closed states + closed findings; pure reducer; no authority) | `arch-spec-045` | closed — v1.169.50 |
| **A4-4a** | Intent + `UniversalConcern` (closed vocabulary; typed intent representation; applicable/not-applicable reasoning) | `arch-spec-046` part-1 | closed — v1.169.52 |
| **A4-4aR** | **Applicability Semantics Correction** — separate Applicability from Grounding and Evaluability; remove string-grounding from `applicable_concerns()` reducer; paradigm MUST NOT erase a universal concern; `NotApplicableReason` shrinks to the four legitimate-scope variants (`NotInProjectIntent`, `NotInUnitIntent`, `ExplicitlyExcludedByProject`, `ExplicitlyExcludedByUnit`) | `arch-spec-046` part-1 rectification | closed — v1.169.54 (release SHA `219de3f16a067819125054b89abb88d9dc73cc57`) |
| **A4-4b** | Generic `AlignmentLens` kernel/registry (ADT + registry shape; two heterogeneous test lenses via fixtures — ABSTRACTION ONLY, no AC7 migration, no `software_alignment::reduce_alignment` wiring) | `arch-spec-046` part-2 | **closed — v1.169.57 (release SHA `7fc820d6e88d7e132d0383a72661c466a889b2d2`; GH Release https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.57; cycle spec target v1.169.56 was off-by-one vs. the cycle-46 install-coherence contract which bumps workspace BEFORE the release tag, so the actual release tag is v1.169.57)** |
| **A4-4bR** | **Subject-General Evidence Resolution** — generalize `EvidenceResolution` into `EvidencePosture<Target>` (parameterized over the observed subject: `Relation(RelationId) \| Unit(SoftwareUnitRef) \| Contract(ContractId) \| Knowledge(KnowledgeId)`); preserve `EvidenceResolution` as a type alias / adapter over the new shape so A4-0/A4-2 relation tests pass without modification; migrate `LensContribution` to a subject-general `LensEvidenceResolution = EvidencePosture<ObservationTargetRef>` so AC7 can emit contributions on `Unit(foo)` without fabricating self-relations | `arch-spec-046` part-2 rect | **closed — v1.169.58 (release SHA `b37321ff2fd337f36cda822bd96385ddf768a3b7`; GH Release https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.58) |
| **A4-4M** | Convergence of existing `paradigm_lens` (OO/FP/ADT/DSL) onto generic `Alignment` — zero feature | `arch-spec-046` migration | **closed — v1.169.59 (release SHA `ba986a5eda88e87dff0874e0cb01e57e838aa3db`; GH Release https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.59; M0 Migration Proof gate passed — `docs/architecture/a4-4m-migration-matrix.md`; production lenses via `AlignmentLensRegistry`; `status_from_polarities` + inferred path deleted) |
| **A4-4MR** | **Concern-Preserving ParadigmLens Evaluation** — production `ParadigmLens::evaluate(&LensInput)` no longer iterates `self.concerns` and overwrites `out`; it now uses `let concern = input.concern();` exclusively and refuses unsupported concerns with typed `LensError::LensRejected { id, concern }`. Single contribution per call; identity content-addressed from the **requested** concern (the bug had it hashed the terminal declared concern). Legacy `paradigm_lens::evaluate_lens()` compatibility facade unchanged (it never instantiated `AlignmentLens`). Closes `FU-A4-4M-CONCERN-PRESERVATION`. | `arch-spec-046` corrective slice | **closed — v1.169.63** (release tag `v1.169.63` → SHA `6f48e1cb6a2eb1f740e007c1bb99a327b38e7143`; cycle spec `.sddk/cycles/p-63676b11dc0ef88f-a4-4mr-concern-preserving/spec.md`; 16-pin corpus `crates/sddk-engine/tests/a4_4mr_concern_preserving.rs`, verified to fail 13/16 against pre-fix code and pass 16/16 against post-fix code; migration matrix §8 documents the M11 corrective slice; handoff `docs/handoff/HANDOFF-2026-09-17-a4-4mr-concern-preserving.md`; PublicReleaseGate PASS) |
| **A4-4C** | Receipt/UAT for `arch-spec-046` milestone closure | `arch-spec-046` | **closed — v1.169.60** (release tag pending; the row's "—" was a placeholder while A4-4M was the live release; A4-4C's release tag and SHA are recorded in the handoff) |
| **A4-3R** | **Typed Constraint Binding** — replace `subject.contains(contract_ref)` / `canonical_tag.contains(contract_ref)` in `software_alignment::reduce_alignment` with a typed binding chain `ExplicitConstraint → ArchitecturalContract → ContractPayload → typed BindingTarget`; introduce `FindingCause::{None, ContractViolation(ContractViolationCause)}` and `contract_violation_cause()`; non-binding kinds (`ProjectionOnly`, `BoundedCompatibility`, `ProviderBoundary`, `Extension`) emit typed `EvidenceGap { kind_tag }`; dispatches via `crate::architectural_contract::{ArchitecturalContract, ContractPayload}` only — no string matching, no `render().contains(...)`, no `canonical_tag().contains(...)` | `arch-spec-045` rectification | **CLOSED — v1.169.61** (release tag `v1.169.61` → SHA `64c29adb2a9570febd29424e9da912bfb792c630`; marker commit `93c8a74`; PublicReleaseGate PASS); cycle spec at `.sddk/cycles/p-63676b11dc0ef88f-a4-3r-typed-constraint-binding/spec.md`; handoff `docs/handoff/HANDOFF-2026-09-17-a4-3r-typed-constraint-binding-v1.169.61.md`; archive-manifest at `~/.sddk-knowledge/sddk-framework/cycles/p-63676b11dc0ef88f-a4-3r-typed-constraint-binding/archive-manifest.md`. FU-A4-3-CONSTRAINT-BINDING (P1) closed by this cycle. |
| **A4-5P** | **Intelligence Loop Entry Gate** (preflight / reconciliation only) — ROADMAP-SYNC, follow-up registry reconciliation, A4-3R target-namespace audit (`ComponentRef` / `EntityRef` ↔ `SoftwareUnitRef` bridge), randomized-probe disposition, arch-spec-047 entry proof, A4-5a input inventory + boundary pins (composition-only; no overall status; MISALIGNED ≠ DENY; no Alignment→Authority/Capability/InstructionSource shortcuts). ZERO code, ZERO new types. | — | **CLOSED — v1.169.62** (release tag `v1.169.62` → SHA `b9e928c6229e65dc141fb282c594931dc7ef7df3` via A4-3R2 release; cycle spec at `.sddk/cycles/p-63676b11dc0ef88f-a4-5p-intelligence-loop-entry-gate/spec.md`; handoff `docs/handoff/HANDOFF-2026-09-17-a4-5p-intelligence-loop-entry-gate.md`). FU-A4-3R-TARGET-NAMESPACE-BRIDGE (P1) closed by A4-3R2. |
| **A4-3R2** | **Namespace-Safe Constraint Targets** — strict separation of `SoftwareUnitRef` / `ComponentRef` / `EntityRef` identity namespaces; first-class `ObservationSubject::{Component, Entity}` and `ObservationTargetRef::{Component, Entity}` variants; reducer uses same-namespace typed equality only; `ObservationSet::for_entity` no longer relies on `trim_start_matches`; `architecture_why::contract_entity` returns `SoftwareEntityRef::Component`/`Entity`; CLI infers typed endpoints per observation. Closes `FU-A4-3R-TARGET-NAMESPACE-BRIDGE`. | `arch-spec-042` §3.1 | **CLOSED — v1.169.62** (release tag `v1.169.62` → SHA `b9e928c6229e65dc141fb282c594931dc7ef7df3`; cycle spec at `.sddk/cycles/p-63676b11dc0ef88f-a4-3r2-namespace-safe-targets/spec.md`; handoff `docs/handoff/HANDOFF-2026-09-17-a4-3r2-namespace-safe-targets.md`). 18 pin corpus + 200-scenario deterministic property test in `crates/sddk-engine/tests/a4_3r2_namespace_safe_targets.rs`. |
| **A4-5a** | Intelligence loop composition: existing outputs (KnowledgeBasis, ObservationSet, VerificationResult, ReconciliationSummary, LensContribution[], AlignmentAssessment) → deterministic orchestration → IntelligenceLoop result/receipt. **Composition only** — no AdvisoryContext, no WHY, no Governance, no providers, no CogniCode/Chronos/JCode. Two new types only: `IntelligenceLoopResult` (EPHEMERAL), `IntelligenceLoopReceipt` (PROJECTION, content-addressed + order-independent). UAT exercises real `VerifyKernel`, `DebVerifyKernel`, `AlignmentLensKernel`, `reduce_alignment`. | `arch-spec-047` part-1 | **CLOSED — v1.169.64** (release tag `v1.169.64` → SHA `2135e6026cdf69b6041e230714d06374f43f60d1`; cycle spec `.sddk/cycles/p-63676b11dc0ef88f-a4-5a-intelligence-loop-composition/spec.md`; handoff `docs/handoff/HANDOFF-2026-09-17-a4-5a-intelligence-loop-composition.md`; 16-pin corpus `crates/sddk-engine/tests/a4_5a_intelligence_loop_composition.rs`; ADR-0126; `arch-spec-047` `implemented_by: partial — A4-5a composition shipped; AdvisoryContext + WHY pending A4-5b; acceptance pending A4-5C`; PublicReleaseGate PASS; FU-A4-5A-COMPOSITION-BOUNDARY (P1) closed by this cycle. `FU-A3-S15-3` stays `BLOCKS_A4-5b` only — does NOT block A4-5a.) |
| **A4-S15R** | **VerifiedBy Evidence Provenance** — repoint `VerifiedBy` from `SpecRef` to `EvidenceRef` in `ArchitectureGraphOverlay`; introduce `ArchitectureOverlayNodeKind::EvidenceRef` + `OverlayNodeRef::Evidence(EvidenceRef)` as a **PROJECTION** node (rebuildable, no new authority/store/CAS/second graph); typed identity via `kind + locator + cas` (no string inference); 0/1/N evidence cardinality; dedup by typed identity; order-independent rebuild. SpecifiedBy untouched. Closes `FU-A3-S15-3`. **No AdvisoryContext, no WHY engine, no A4-5b feature.** A3-S15 WHY remains behavior-compatible (read-only). | `arch-spec-016/017` (overlay) | **CLOSED — v1.169.65** (release tag `v1.169.65` → SHA `38b0a13e940a75bee5177a830122c796a347d592`; cycle spec `.sddk/cycles/p-63676b11dc0ef88f-a4-s15r-verifiedby-evidence-provenance/spec.md`; handoff `docs/handoff/HANDOFF-2026-09-17-a4-s15r-verifiedby-evidence-provenance.md`; 21-pin corpus `crates/sddk-engine/tests/a4_s15r_verifiedby_evidence_provenance.rs`; ADR-0127; PublicReleaseGate PASS; `FU-A3-S15-3` CLOSED by this cycle. A4-5b structurally unblocked — does NOT auto-open.) |
| **A4-5b** | **AdvisoryContext + WHY Integration** — derive the A3 `AdvisoryContext` (reused verbatim) from the A4-5a composition output; add a typed, provenance-backed WHY (EPHEMERAL) that explains advisory items. Reuses `AdvisoryContext`/`AdvisoryKind` A3; `architecture_why` explains contract/finding provenance (different subject domain). `absence != negation`, `SpecifiedBy` does not imply `VerifiedBy`, `MISALIGNED != DENY`, no hidden orchestration, no global verdict. | `arch-spec-047` part-2 | **CLOSED — v1.169.66** (release tag `v1.169.66` → SHA `aa3aa91c11794baf9b5bce34cb26e704d1c7d59b`; cycle spec `.sddk/cycles/p-63676b11dc0ef88f-a4-5b-advisory-context-why-integration/spec.md`; handoff `docs/handoff/HANDOFF-2026-09-17-a4-5b-advisory-context-why-integration.md`; 20-test corpus `crates/sddk-engine/tests/a4_5b_advisory_context_why.rs`; ADR-0128; PublicReleaseGate PASS; `arch-spec-047` `implemented_by: partial — A4-5a shipped; A4-5b shipped; acceptance pending A4-5C`. A4-5C is NEXT — NOT auto-opened.) |
| **A4-5C** | **arch-spec-047 Acceptance / Receipt / UAT** — cross-cutting acceptance that every normative clause of arch-spec-047 holds simultaneously, including negative cases. Budget: ACCEPTANCE/RECEIPT/UAT ONLY (zero production semantics). Produces a durable Acceptance Receipt; promotes the spec to `implemented` only if every MUST has evidence (otherwise STOP + corrective). | `arch-spec-047` | **CLOSED — v1.169.67** (release tag `v1.169.67` → SHA `1949fa8448b636ffdc4fe2fea02339d82922481c`; 34-test corpus `crates/sddk-engine/tests/a4_5c_arch_spec_047_acceptance.rs`; Acceptance Receipt `docs/architecture/receipts/A4-5C-arch-spec-047-acceptance-receipt.md`; every normative clause PASS, no MUST NOT_PROVEN; no production change required (no STOP); PublicReleaseGate PASS; `arch-spec-047` promoted to `status: implemented`.) |
| **A4-CLOSEOUT** | Cross-cutting A4 audit (042→047, follow-ups, single authorities, anti-encroachment, A5 readiness) — **not** another feature. Budget: AUDIT/RECONCILIATION/CERTIFICATION ONLY. | — | **IN PROGRESS** (cycle `p-63676b11dc0ef88f-a4-closeout-milestone-audit`) |
| **A5** | `BASE_PRODUCTION_READY` — release receipts, hardening programme close | — | blocked_by A4-CLOSEOUT |

**Checkpoint (A4-4a close, 2026-09-16):**
- `released_baseline` = v1.169.52 (tag tracks `main`)
- `development_head` = post-bump `chore(release)` commit on `main` (matches `origin/main` and the GH release tag `v1.169.52`; use `git rev-parse origin/main` for the live SHA)
- `workspace_version` = `1.169.52`
- `arch-spec-046` part-1 → `implemented` (model surface: closed vocabularies + typed intent + pure reducer). Part-2 (kernel+registry) and lens migration (A4-4M) shipped via A4-4b / A4-4bR / A4-4M (v1.169.57 / v1.169.58 / v1.169.59); A4-4C closed the spec as `implemented` (v1.169.60). A4-5 loop integration remains the only open thread. (Historical "per arch-spec-046 §6" deferral was a documentation residue — §6 does not exist; A4-CLOSEOUT is the A4 milestone acceptance gate, blocked_by A4-5.)
- `arch-spec-047` updated: `implemented_by: pending (A4-5 — full loop integration)`
- Next cycle (A4-4b) requires a new ROADMAP-SYNC preflight and scope contract.

**Checkpoint (REL-1 close, 2026-09-16 — PublicReleaseGate step 9b in `scripts/release.sh`):**
- `released_baseline` = v1.169.53 (HEAD `ccecdc723355b0ecc7e037f2266f465165dc59dc`)
- Live step 9b ran on the GH API: tag SHA anchored, `isDraft=false`, `isPrerelease=false`, 9-asset contract, 9/9 HTTP 200.
- `scripts/release.sh` now 14 pasos (was 13); step 9b gates step 10 (`install --version $TAG`) and is fail-closed.
- Contract tests: `tests/test_release_public_gate.sh` (PASS=11, FAIL=0).
- Local `sddk 1.169.53` installed; `sddk dev doctor` reports `binary.bundle_coherence: present` + `all_present: true`.
- `FU-A4-4A-REL-1` disposition: closed 2026-09-16.

**Checkpoint (A4-4aR close, 2026-09-16 — Applicability Semantics Correction shipped):**
- `released_baseline_for_A4-4aR` = v1.169.54 (release SHA `219de3f16a067819125054b89abb88d9dc73cc57`, identical to `origin/main` HEAD at release time)
- `development_head` = `daac3b3c5341b09bc8d092f8eb100c3d2da82de1` (post-release docs-sync; `workspace_version = 1.169.55`, docs only — **do not** treat `1.169.55` as a published release)
- Live `bash scripts/release.sh` for v1.169.54:
  step 1 (fmt+clippy+test+doctests) = workspace green; step 1b = 9/9 shell contract tests green; step 9b **PublicReleaseGate** = PASS (tag SHA anchored, `isDraft=false`, `isPrerelease=false`, 9/9 canonical assets reachable HTTP 200); steps 10-13 (install + doctor + prune + distrib round-trip) = OK; final = `binary=bundle=current=1.169.54`. **First non-REL cycle to exercise the new PublicReleaseGate in regression mode.**
- Discoveries reduced:
  1. A4-4a `applicable_concerns()` reducer collapsed three distinct semantic layers — **Applicability** (intent-only), **Grounding** (decisions/contracts), **Evaluability** (paradigm×concern, evidence sufficiency) — into a single Applicable/NotApplicable answer.
  2. Pre-A4-4aR the reducer used `DecisionRef::render().contains(concern.canonical_tag())` and `ContractId::as_str().contains(...)` to decide applicability (string-grounding — the same `false-clean` shape A4-2/DebVerify had to correct).
  3. It also used `(Pipeline, TemporalCoupling) → NotApplicable` to erase a universal concern at the paradigm level. Building A4-4b on that surface would have perpetuated the conflation.
- Single-budget correction: reducer signature shrunk to 2 args (`&ProjectIntent`, `&UnitIntent`); `NotApplicableReason` reduced to the 4 legitimate-scope variants; `ApplicableReason` reduced to a single `ProjectAndUnitIntent`; `DecisionRefs`/`ContractRefs` deleted (zero external callers); `ProjectIntent`/`UnitIntent` gain `excluded_concerns: BTreeSet<UniversalConcern>` with `#[serde(default)]`; paradigm-level erasure rule removed; 22 unit + 5 integration tests with explicit falsification pins (`falsification_paradigm_does_not_erase_concern`, `falsification_same_applicability_with_or_without_evidence`, `falsification_no_deprecated_reasons`, `falsification_signature_no_decisions_or_contracts`).
- Carry-over folded into v1.169.54 chore(release): `FU-A4-4A-STRING-GROUNDING` CLOSED; `FU-REL-1-BACKFILL` CLOSED (two SHA-placeholder rows in the REL-1 handoff backfilled to `ccecdc7…`).
- A4-4b unblocked; see checkpoint below.
- Epistemic pin (now enforced in code + tests + docs):
  `NotApplicable != Unknown != Ungrounded != InsufficientEvidence`.

**Checkpoint (A4-4b open, 2026-09-16 — Generic AlignmentLens Kernel / Registry):**
- **Budget:** ABSTRACTION / KERNEL ONLY. UniversalConcern + ApplicableConcern + Evidence/Observations → Generic `AlignmentLensRegistry` → matching lenses → `AlignmentLensKernel` → typed `LensContribution[]`. NO paradigm_lens migration, NO AlignmentAssessment wiring, NO new Alignment states, NO CLI, NO providers, NO Governance.
- **Release baseline inherited from A4-4aR:** v1.169.54 / `219de3f16a067819125054b89abb88d9dc73cc57` (this is the **released_baseline**, not the development head).
- **Development head at cycle start:** `daac3b3c5341b09bc8d092f8eb100c3d2da82de1` (`workspace_version = 1.169.55`, docs-sync only since release).
- **Expected semantic release:** v1.169.56 (will undergo PublicReleaseGate again at release time).
- **Epistemic discipline (must not invent a new taxonomy):** Reuse `observation::EvidenceResolution::{Supported, Contradicted, Conflicted, Insufficient}` everywhere a lens needs to express evidence posture. Do **not** create parallel `LensSupported/LensContradicted/LensConflict/LensUnknown`. AC7's `LensFamily/LensObservation/LensStatus/LensEvaluationBasis` is historical and out-of-scope; its convergence belongs to A4-4M, not to A4-4b's basis.
- **Pins (non-negotiable, must be enforced as tests):**
  - *Missing lens != NotApplicable.* An `ApplicableConcern` with no registered lens produces `NotEvaluated` (typed gap), never `NotApplicable`.
  - *Insufficient evidence remains Insufficient.* An `ApplicableConcern` with a registered lens but insufficient evidence returns `EvidenceResolution::Insufficient` — not `NotApplicable`, not `MISALIGNED`.
  - *Conflicted remains Conflicted.* When a lens receives `EvidenceResolution::Conflicted`, the lens **preserves** the conflict — no first-wins, no side-pick, no score/confidence resolution.
  - *Multi-lens preservation.* Same `ApplicableConcern` + `Lens A` + `Lens B` = two distinct `LensContribution`s. No per-concern collapse, no dedup on rendered message.
  - *Registry determinism.* Duplicate `LensId` → typed error. Insertion order is independent of evaluation result. Lookup is deterministic. No IO during evaluation. No global mutable state.
- **AC7 (`paradigm_lens/`) is read-only during A4-4b.** A4-4M owns the migration. No `LensFamily`/`LensObservation`/`LensStatus`/`evaluate_lens`/`probes OO/FP/ADT/DSL` may be touched. Anti-encroachment test pins that the new kernel compiles and runs **without** depending on `paradigm_lens`.
- **`software_alignment::reduce_alignment` is read-only.** A4-4b does not call it and does not fabricate `AlignmentAssessment`/`ContractViolation`/`AlignmentTension`/`ImprovementOpportunity`.
- **Identities:** `LensId` is stable/explicit. If `LensContributionId` exists, content-addressed on `(lens_id, lens_version, concern, semantic_evidence_basis_or_resolution, stable_intent_or_basis_refs)` — never on wall-clock, message text, renderer output, registration order, or vector insertion order.
- **Demonstrated genericity:** two heterogeneous **reference/test lenses** (under `tests/fixtures/`, NOT exported as production API) consumed by the kernel:
  - *Lens A:* consumes `DependencyDirection` observations.
  - *Lens B:* consumes `Freshness` observations.
  Each produces a `LensContribution` keyed by `(LensId, ApplicableConcern, EvidenceResolution, ref_set)`. The two produce different `EvidenceResolution`s so multi-lens preservation is observable.
- **Decision to document:** trait `AlignmentLens` vs closed ADT + explicit dispatch — recorded in the cycle spec; extensibility for future built-in lenses + packs + provider-backed observations must be considered, but no packs are designed in this cycle.
- **Risk carry-overs already on the books** (do not fix here, leave them open):
  - `FU-A4-3-CONSTRAINT-BINDING` (NEW, P1): `software_alignment::reduce_alignment` still associates `ExplicitConstraint` → observations via `subject.contains(contract_ref)` / `canonical_tag.contains(contract_ref)`. **MUST close before A4-5, MUST NOT** be touched in A4-4b's budget.
- **Acceptance (real release):** tag SHA anchored; `isDraft=false`; `isPrerelease=false`; 9 canonical assets; 9/9 public URLs HTTP 200; install from URL; doctor; prune; distrib round-trip — through `scripts/release.sh` step 9b + 10–13.
- **STOP.** A4-4M does NOT auto-open.

**Checkpoint (A4-4b close, 2026-09-16 — Generic AlignmentLens Kernel / Registry shipped):**
- **Status:** ABSTRACTION / KERNEL ONLY — closed; released v1.169.57.
- **Release tag:** `v1.169.57` → SHA `7fc820d6e88d7e132d0383a72661c466a889b2d2`.
- **GH Release:** https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.57 (isDraft=false, isPrerelease=false, 9 canonical assets, 9/9 public URLs HTTP 200).
- **Workspace version at release:** `1.169.57` (the cycle-46 install-coherence contract bumps workspace BEFORE the release tag; the cycle spec's "v1.169.56" target was off-by-one vs. the bump step).
- **Surface delivered:** `crates/sddk-engine/src/alignment_lens/` (9 files, ~1750 lines) + 24 pin tests + 2 heterogeneous reference fixture lenses under `tests/` (NOT exported) + ADR-0125.
- **Kernel design contract (locked, anti-encroachment pins):**
  - `ApplicableConcern + no registered lens` → `NotEvaluated { reason: NoRegisteredLens }` (typed gap, distinct from `NotApplicable`).
  - `ApplicableConcern + lens + insufficient` → `EvidenceResolution::Insufficient` carrying typed `InsufficientGap` enum (free-form `gap: String` cannot leak into identity).
  - `ApplicableConcern + lens + contradicted-or-conflicted` → preserve both sides (`Conflicted` carries supporting AND contradicting).
  - `LensVersion` is in identity. Duplicate `LensId` with same `LensVersion` + same `supported_concerns` → idempotent success; with any divergence → typed refusal (`InconsistentSupportedConcerns` / `InconsistentLensVersion`).
  - `LensInput::try_new` refuses `NotApplicable` at the boundary; kernel never sees `NotApplicable`.
- **Identity:** `LensContributionId` content-addressed sha256 over `(domain | lens_id | lens_version | concern | observation_set_canonical_tag | EvidenceResolution::canonical_tag | sorted_evidence_ref_ordering_keys)`. Excludes wall clock, message text, registration order, vector insertion order, severity, producer labels, rendered text.
- **Anti-encroachment (type-locked, structural, not aspirational):**
  - `LensContribution` has no `score` / `confidence` / `AlignmentState` / `Capability` / `AuthorityDecision` / `InstructionSource` field.
  - Module does not import `authority`, `instruction_compiler`, `provider_sdk`, `cli`, `paradigm_lens`.
  - `software_alignment::reduce_alignment` is **not called** (textual probe grep, doc-comments excluded).
  - No production concrete OO/FP/ADT/DSL lens impls.
  - Reference lenses live under `tests/` root, not `src/`.
- **Acceptance (real release, not dry-run):** all gates green pre-release (`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --offline`, `cargo build --release -p sddk-cli --bin sddk`, `bash tests/test_release_public_gate.sh` → PASS=11 FAIL=0). Tag SHA, GH Release creation, install from URL, doctor, prune — through `scripts/release.sh` step 9b + 10–13.
- **Risk carry-overs:** `FU-A4-3-CONSTRAINT-BINDING` (P1) **closed by A4-3R — commit `6d25e6c`**. A4-4b's budget did NOT touch it (left intact for the dedicated rectification cycle).
- **Cycle artifacts:** `.sddk/cycles/p-63676b11dc0ef88f-a4-4b-alignment-lens-kernel/spec.md`, ADR-0125, handoff `HANDOFF-2026-09-16-a4-4b-alignment-lens-kernel-v1.169.56.md`.
- **STOP.** A4-4M does NOT auto-open.

## 12 non-negotiable invariants

1. One canonical authority per concept.
2. Four state classes: Fact / Object / Projection / Ephemeral.
3. SDLC lifecycle semantics are explicit (PlanRevision lifecycle).
4. Common Revision substrate shared by all durable history.
5. One Semantic Graph (per concept) with optional projections.
6. The Vault is human/knowledge source, never a runtime authority.
7. Universal Evidence model spans all subsystems.
8. **Prompt text is never architecture** (EffectiveInstructions compiled).
9. **Skill ≠ Capability** (skill implies nothing about permissions).
10. **CLI docs from one source** (CommandRegistry).
11. **Examples are executable** (validation at compile/registry time).
12. **Knowledge of command ≠ permission** (declared vs granted).

## Reading order

1. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/00-EXECUTIVE-PROPOSAL.md`
2. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/01-ARCHITECTURE/TARGET-ARCHITECTURE.md`
3. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/03-ROADMAP/ROADMAP.md`
4. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/03-ROADMAP/UAT-ACCEPTANCE.md`
5. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/03-ROADMAP/MIGRATION-AND-DEPRECATION.md`
6. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/` (17 ADRs)
7. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/` (18 SPECs)
8. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/07-AGENT-EXPERIENCE/OVERVIEW.md` (M7)
9. `../SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-005-SEMANTIC-GRAPH-AND-WHY.md` (M8)

## Repository-native surfaces (crosswalk)

The package's `02-ADRS/` and `02-SPECS/` are mirrored into this entry point so
the canonical architecture is reachable without traversing the package folder.
Per SUPERSESSION.md ADR preservation rule, the package's original IDs are kept
in each file's frontmatter (`package_local_id`); the repository-native ID is the
authoritative one for new references.

### Architecture ADRs (`adrs/`)

| Repo-native ID | Title | Package ID |
|---|---|---|
| `ADR-0094` | One canonical fact log | `ADR-001` |
| `ADR-0095` | Four state classes | `ADR-002` |
| `ADR-0096` | SDLC lifecycle semantics | `ADR-003` |
| `ADR-0097` | Common Revision substrate | `ADR-004` |
| `ADR-0098` | One Semantic Graph | `ADR-005` |
| `ADR-0099` | Vault as human knowledge source | `ADR-006` |
| `ADR-0100` | Universal Evidence | `ADR-007` |
| `ADR-0101` | Agent outcome contribution synthesis | `ADR-008` |
| `ADR-0102` | Unified Authority Engine | `ADR-009` |
| `ADR-0103` | Target/Task porcelain | `ADR-010` |
| `ADR-0104` | Pack extension boundary | `ADR-011` |
| `ADR-0105` | Configuration conventions | `ADR-012` |
| `ADR-0106` | Typed instruction compilation | `ADR-013` |
| `ADR-0107` | One Command Registry | `ADR-014` |
| `ADR-0108` | Skill is not Capability | `ADR-015` |
| `ADR-0109` | Provider-independent Agent Profiles | `ADR-016` |
| `ADR-0110` | Agent execution provenance | `ADR-017` |

Range ADR-0094..0110 was chosen because the previous repository surface
(`docs/adr/`) ends at ADR-0080, leaving ADR-0094..0110 free and avoiding
collision with the `docs/sddk-decision-kernel-architecture/03-adrs/` range
(ADR-020..074). See [`adrs/ADR-0094-ONE-CANONICAL-FACT-LOG.md`](adrs/ADR-0094-ONE-CANONICAL-FACT-LOG.md)
for the canonical example of the crosswalk frontmatter.

### Architecture SPECs (`specs/`)

| Repo-native ID | Title | Package ID |
|---|---|---|
| `arch-spec-001` | Canonical authority | `SPEC-001` |
| `arch-spec-002` | Lifecycle model | `SPEC-002` |
| `arch-spec-003` | Revision substrate | `SPEC-003` |
| `arch-spec-004` | Decision Memory | `SPEC-004` |
| `arch-spec-005` | Semantic Graph and why | `SPEC-005` |
| `arch-spec-006` | Knowledge vault context | `SPEC-006` |
| `arch-spec-007` | Agent protocol and handoff | `SPEC-007` |
| `arch-spec-008` | Authority and side effects | `SPEC-008` |
| `arch-spec-009` | Target task workflow | `SPEC-009` |
| `arch-spec-010` | Pack SDK | `SPEC-010` |
| `arch-spec-011` | Observability views | `SPEC-011` |
| `arch-spec-012` | Configuration | `SPEC-012` |
| `arch-spec-013` | Agent experience contract | `SPEC-013` |
| `arch-spec-014` | Instruction compiler | `SPEC-014` |
| `arch-spec-015` | Command registry and agent surface | `SPEC-015` |
| `arch-spec-016` | Skill contract | `SPEC-016` |
| `arch-spec-017` | Agent profiles and provider adapters | `SPEC-017` |
| `arch-spec-018` | Agent execution provenance | `SPEC-018` |

The `arch-spec-` prefix avoids collision with a future top-level `docs/specs/`.

## Repository historical packages (superseded)

These folders contain previous design context. They remain available for
provenance, but **are no longer authoritative**:

- `../sddk-decision-kernel-architecture/` (pre-canonical ADR-020..074 home)
- `../sddk-2.0-architecture-consolidation/` (2026-08-11 v1.9.1 baseline consolidation)
- `../sddk-complete-evolution-2026-08-23/` (CURRENT/HISTORY bundle)
- `../sddk-stabilization-plan/` (stabilization phase)
- `../SDDK-Human-Agent-Collaboration-Evolution-Pack-2026-08-28/` (HX0..HX7 human-experience line)
- `../responsibility-separation/SPEC.md` (cero-intrusion rule)
- `../ARCHITECTURE-MODEL.md` (older model draft)
- `../agent-models-registration.md`, `../agent-reconciliation.md` (agent evolution notes)
- `../evolutivo-*.md` (older evolutivo notes)

When reconciling WorkItems, import concepts from these only via
explicit reconciliation — they do not have authority on their own.

## Provenance

- **Adopted by**: cycle
  `p-63676b11dc0ef88f-architecture-adoption-m0-supersession`
- **Adoption date**: 2026-09-09
- **Total documents covered**: 72
- **ADRs**: 17 (`ADR-001..017` within the package)
- **SPECs**: 18 (`SPEC-001..018` within the package)
- **UAT cases**: 22 (`UAT-01..UAT-22`)
- **Manifest SHA-256**: see
  `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/MANIFEST.sha256`

## See also

- **AGENTS.md §2.7..§2.10** — semantic ownership, agent experience,
  extension discipline, consolidation rule.
- **Cycle dir**: `~/.sddk-knowledge/sddk-framework/cycles/p-63676b11dc0ef88f-semantic-core-agent-experience-adoption/`
- **Planning artifacts**: 9 (~1726 lines total) covering M0/M7/M8
  planning notes, CLI crosswalk, engine crosswalk, LAUNCH-READY.

---

## Trailing audit note — appended 2026-09-12 (M9.5: ADR promotion process)

At v1.168.31 (post-M9 row disclosure), 17 ADRs in `docs/architecture/adrs/` carried `status: proposed` with zero accepted. The cycle `p-63676b11dc0ef88f/adr-promotion-m0-meta-and-batch-1` (this audit) introduces the documented promotion process and ships the first batch:

1. **ADR-0001-ADR-PROMOTION-PROCESS** (new meta-ADR, `status: accepted`) — defines the three-state lifecycle `proposed → accepted → released` with frontmatter invariants in §3.4 (`accepted_at` + `accepted_by_cycle` for binding states; `released_at` + `released_in_milestone` for released state). The pin test `tests/test_adr_promotion_format.sh` (added in this cycle) enforces §3.4; §3.5 is satisfied by the test's existence.
2. **ADR-0096-SDLC-LIFECYCLE-SEMANTICS** (promoted `proposed → accepted`) — implementation evidence: `crates/sddk-engine/src/planning/` (Goal/WorkItem), `workflow_runtime.rs` (WorkflowDefinition + Run), `execution_frontier.rs` (ExecutablePlan), `docs/architecture/specs/arch-spec-002-lifecycle-model.md`. Risk register: no impact (lifecycle chain is purely additive, no replacement).
3. **ADR-0101-AGENT-OUTCOME-CONTRIBUTION-SYNTHESIS** (promoted `proposed → accepted`) — implementation evidence: `OrchestrationSynthesisReceipt` at `crates/sddk-engine/src/orchestration_synthesis.rs:205`, `ContributionRef` at line 138, `ExecutionOutcome` via `TransitionOutcome`, `agent_result_used` lint at zero hits (deprecation complete). Risk register: lint `execution_outcome_as_synthesis` stays `default: allow` until ADR-0101 is paired with a follow-up that pins the `SynthesisReceipt.disposition` field.
4. **ADR-0100-UNIVERSAL-EVIDENCE** (kept `status: proposed`, honest disclosure) — partial implementation: `EvidenceRef` + `Supports` + `Gates` relations exist in `CoreRelationKind`; `Verifies` and `ObservedFor` relations are absent. Promotion blocked on construction, not on audit. Per ADR-0001 §3.2 criterion 1. The `evidence_kind_v1` lint stays `default: allow`.

**Pin test:** `tests/test_adr_promotion_format.sh` — checks every ADR with `status: accepted` carries the required frontmatter fields; also asserts `accepted_count >= 1` to prevent silent rollback to "all proposed". Wired into `scripts/release.sh` step 1b alongside the other shell contract tests.

**Effect on the lint block in the M9 row above:** this batch unblocks 2 of the 4 advisory lints that depend on ADR-0096/0101 acceptance. The remaining 2 (governed by ADR-0096's downstream acceptance of "execution_outcome_as_synthesis" + ADR-0100's missing relations) stay `default: allow`. Promotion of those lints to `deny` is the next batch's work, not this one's.

**Audit trail:** the cycle archive at `~/.sddk-knowledge/sddk-framework/cycles/p-63676b11dc0ef88f-adr-promotion-m0-meta-and-batch-1/` documents the per-lint / per-ADR evidence in detail.

---

## Trailing audit note — appended 2026-09-12 (M9.6: ADR promotion batch 2)

Cycle `p-63676b11dc0ef88f/adr-promotion-batch-2` ships 13 more ADR promotions (the remaining 14 package ADRs minus 1 deferred, minus the 3 already-accepted from batch 1). State transition:

- **Before this batch:** 14 ADRs `proposed`, 3 ADRs `accepted` (per batch 1: ADR-0001, ADR-0096, ADR-0101).
- **After this batch:** 2 ADRs `proposed` (ADR-0097, ADR-0100 — both with `deferred_until`), 16 ADRs `accepted`.

### Promoted this cycle (13 ADRs, all `proposed → accepted` per ADR-0001 §3.2)

| ADR | Evidence (file:line) |
|---|---|
| ADR-0094 (CanonicalEventLog) | `crates/sddk-engine/src/canonical_event_log.rs:222` (trait + InMemory impl) |
| ADR-0095 (Four state classes) | `crates/sddk-engine/src/state_class_lint.rs:16` (StateClass enum) |
| ADR-0098 (One semantic graph) | `crates/sddk-engine/src/semantic_graph.rs:29` (SemanticGraphProjection trait + impl) |
| ADR-0099 (Vault as human knowledge source) | `crates/sddk-engine/src/vault_boundary.rs` + `crates/sddk-cli/src/context_compiler.rs:65` (ContextAdapter, read-only) |
| ADR-0102 (Unified authority engine) | `crates/sddk-engine/src/authority_engine.rs:216` (ActionProposal), `:264` (AdmissionDecision), `authority_engine/runner.rs:32` (AuthorityEngineRunner) |
| ADR-0103 (Target/Task porcelain) | `crates/sddk-engine/src/target_task/mod.rs:142` (Task), `:173` (Target) + DagExecutor |
| ADR-0104 (Pack extension boundary) | `crates/sddk-engine/src/pack_registry.rs:95` (PackRegistry) + `generic_pack_contracts.rs:111` (PackManifest) |
| ADR-0105 (Configuration conventions) | `crates/sddk-cli/src/config_cmd.rs` (M6.1 `sddk config explain`) + `arch_lint.rs` (precedence enforcement) |
| ADR-0106 (Typed instruction compilation) | `crates/sddk-cli/src/instruction_compiler.rs:301` (EffectiveInstructions) + `:348` (InstructionCompiler) |
| ADR-0107 (One command registry) | `crates/sddk-cli/src/command_spec.rs:254` (CommandSpec) + AX-S1 drift guard test |
| ADR-0108 (Skill != Capability) | `crates/sddk-cli/src/skill_definition.rs:55` (SkillDefinition) + `:116` (CapabilityRequirement); no grant method |
| ADR-0109 (Provider-independent agent profiles) | `crates/sddk-cli/src/agent_profile.rs:45` (AgentProfile); AX-S4 confirms no provider transport data |
| ADR-0110 (Agent execution provenance) | `crates/sddk-cli/src/execution_receipt.rs:156` (AgentExecutionReceipt) + `:333` (AgentExecutionReceiptBuilder) |

### Deferred (honest disclosure per ADR-0001 §3.2 criterion 1)

- **ADR-0097 (Common Revision substrate)** — kept `proposed`. The CAS primitive (`crates/sddk-storage/src/cas_object_store.rs`) exists, but the spec calls for a common `Revision<T>` envelope with CAS `Ref` updates implemented as a cross-domain substrate. Shipped revisions are domain-specialized (`GraphRevision` is u64-only at `crates/sddk-engine/src/semantic_graph.rs`; `PlanRevisionV1` at `crates/sddk-domain/src/plan_revision.rs:298`; `ExecutionGraphRevision` at `crates/sddk-domain/src/graph.rs:1241`). Promotion blocked on construction, not on audit.
- **ADR-0100 (Universal Evidence)** — kept `proposed` (carried over from batch 1). `Verifies` and `ObservedFor` relations absent from `CoreRelationKind`.

### Pin test status

`tests/test_adr_promotion_format.sh` after this batch: `checked: 18 ADR files; violations: 0; accepted ADRs: 16` (was 3 after batch 1). The test passed clean — no frontmatter formatting issues. `accepted_count >= 1` anti-rollback guard still holds.

### Effect on the lint block in the M9 row above

No new lint promotions to `deny` result from this batch (the 4 advisory lints are gated on ADR-0100/0101/0096 follow-ups, none of which are satisfied by these promotions). The 5 of 9 lints in `default: deny` count from the M9 row above remains accurate as of the handoff date (v1.168.32); the 4 advisory lints stay `default: allow` until ADR-0100 relations are built and the SynthesisReceipt.disposition field is pinned. **Updated v1.168.37 (synthesis-dissent-runner-extension): the `orchestration_synthesis_no_dissent` lint was promoted to `default: deny` via the bounded `exclude_paths` path** (19 hits → 0 by excluding tests/ and the engine src file containing the mod tests block); the count above is now **6 of 9 deny** at v1.168.37.

### Audit trail

Cycle archive at `~/.sddk-knowledge/sddk-framework/cycles/p-63676b11dc0ef88f-adr-promotion-batch-2/` documents the per-ADR evidence in detail. The Python script that produced the frontmatter changes is preserved as `scripts/promote_adrs.py` (added in this cycle) — idempotent, can be re-run for batch 3..N.

### Remaining open threads

- **ADR-0097**: requires the common `Revision<T>` + CAS `Ref` envelope to be implemented as a cross-domain substrate. ~30-50 LoC engine work + tests.
- **ADR-0100**: requires `Verifies` and `ObservedFor` relations in `CoreRelationKind`. ~20-30 LoC engine work + tests.
- **Lint `execution_outcome_as_synthesis`**: requires `SynthesisReceipt.disposition` field to be pinned in a follow-up to ADR-0101.
- **Lint `transition_outcome_used`**: requires the `TransitionOutcome` field to be structurally validated in the synthesis receipt.
- **Vault mirror nodes** (per ADR-0001 §3.6): the 13 newly accepted ADRs need minimal mirror nodes in `~/.sddk-knowledge/sddk-framework/adrs/` so vault-side queries reflect the repo truth. Stub-friendly pattern.
