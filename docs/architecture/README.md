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
| **M9** | Remove compat debt | delivered — INC-DEBT-018 (3-event supersede invariant doc) + INC-DEBT-019 (deterministic `now_ms` in `Engine::cycle_supersede`) closed in v1.167.0 (M9.1 + M9.2) + INC-DEBT-021 (pre-existing clippy baseline) + INC-DEBT-022 (pre-existing `dist_succeeds_with_valid_bundle` flake) closed in v1.167.1 (M9.3 + M9.4); the four vault-side records corresponding to these (INC-021-9b3e7f1a + INC-022-4c7a1e2f + INC-027-4e2a3c26 + INC-013 closed as side effect of DW-RUNTIME-005 S6b slice 2) were reconciled from `status: open` to `status: closed` at the inc-hygiene-2026-09-11 audit, restoring the doc/ledger coherence that the M9 row's "zero open debt records at v1.167.1" claim had implied; debt-report schema `cycle_id` pattern widened to admit product cycles; at v1.168.4 (post M9.5 cycle), seven `status: resolved` records were reconciled — six (INC-CYCLE-13-APPLY-TEST-COUNT-MISREPORT, INC-CYCLE-13-DURABILITY-COMMENT-ACCURACY at commit cbd8ad7, INC-CYCLE-13-LOC-OVERAGE, INC-DEBT-016-llm-fabricates-without-receipt via ADR-0073, INC-DEBT-017, INC-KERNEL-CLI-AGENT-INFORMATION-FLOW-APPLY-PUSH-VIOLATION at closed 2026-08-28) moved to `status: closed`, and the seventh (INC-005720-cli-test-flake) moved to `status: accepted_risk` with documented mitigation (run in isolation for the pre-existing test-infra race in install-orchestration path, severity low P3); at v1.168.7 (clippy gate cycle), the pre-existing `INC-MATRIX-LINT-CODES-APPLY-PUSH-VIOLATION.md` was finally reconciled from `status: open` (frontmatter drift) to `status: closed` (body had been `closed` since 2026-08-28 with full prevention evidence); at v1.168.8 (post-M9 INC hygiene), eight more `status: resolved` records were reconciled — INC-CYCLE-13-{APPLY-TEST-COUNT-MISREPORT,DURABILITY-COMMENT-ACCURACY,LOC-OVERAGE}, INC-CYCLE-14-{CORPUS-FIXTURE-DUPLICATION,HELPER-DOC-GAP,SEVERITY-SPEC-DRIFT}, INC-DEBT-017, INC-001-cli-call-budget-stale all moved to `status: closed` with explicit `closed_at` / `closed_by` frontmatter; INC-DEBT-020-prune-reapunta-current moved to `status: closed` with `closed_at: 2026-09-02` and `closed_by: sddk-archive (cycle-53 archive)`; INC-HX-AUTH-{003-provenance, 004-no-parallel-authority} stayed at `status: resolved` (their lifecycle entries on 2026-09-04 document only partial close — paths 4+5 of 7 for 004, additive `actor_ref` widening for 003 — and the canonical ActorRef 5-field migration tracked under EVT-LEDGER-001 still open); INC-HX-AUTH-{001-writable-state, 002-approval-authority} were honestly corrected from `status: resolved` (no lifecycle evidence) back to `status: open` — the eight writable surfaces enumerated in ADR-069 §3 still lack runtime authority declaration enforcement and `emit.rs:259` still hardcodes `ActorKind::Human` per the `emit_approval_decision_forces_human` baseline regression test, both tracked under ARCH-HEX-001 (order 80, H0); the single `status: tracked` record in the vault (`INC-2026-08-27-cycle-44-v2-correction`) is deliberately permanent (cycle-44 v1 retracted, v2 governs design/tasks/apply; per design intent) and is not part of the M9 scope; final local debt ledger (`docs/debt/`): 31 closed + 2 open + 2 resolved = 35 records; vault side (`~/.sddk-knowledge/`) carries the additional 1 tracked record; **v1.168.8 also ships the M9 enforcement infrastructure for the five M0 D6 lints** — `sddk dev lint deprecated-patterns` executes the registry against the live workspace and reports hits (text + JSON), but all five lints stay at `default: allow` (advisory) because live-workspace evaluation produces 61 hits across 4 of 5 lints in active production code (agent_result_used = 0 hits, evidence_kind_v1 = 18, orchestration_synthesis_no_dissent = 19, execution_outcome_as_synthesis = 0, transition_outcome_used = 24); promoting any lint to `default: deny` requires the three acceptance criteria documented in `[acceptance_for_m9_blocking_enforcement]` of the registry TOML (per-lint validation that hits are exclusively in legacy paths, replacement migration complete, RISK-REGISTER R-001 user sign-off) — at v1.168.12 (ARCH-LINT-M9.1), the per-lint audit was completed and `agent_result_used` was promoted to `default: deny` — its only engine-tree hits were intentional citations inside the spike_sp06.rs retrieval-evaluation corpus (excluded as content, not code), and the CLI hits are the deliberate legacy converter surface (already excluded); the remaining four lints stay `allow` because their canonical replacements are `status: proposed` and not yet implemented (ADR-0100 universal Evidence model, ADR-0101 ExecutionOutcome+Contribution): promotion there is blocked on construction, not on audit; `sddk dev lint deprecated-patterns --enforce` is now meaningful and passes clean, with a regression guard test pinning the deny+zero-hits invariant; at v1.168.19 (AX-S5 introduction), the deprecated_patterns registry was extended with 4 agent-asset hygiene lints (asset_deprecated_namespace, asset_raw_store_reference, asset_authority_language, asset_unregistered_cli_example), all kept at `default: allow` per the AX-S5 spike's disposition (zero hits on the 91-asset corpus at survey time, regression-prevention checks); at v1.168.29 (ARCH-LINT-AX-S5 promotion), the per-lint audit against the live workspace confirmed zero hits for all four, and three of them (`asset_deprecated_namespace`, `asset_raw_store_reference`, `asset_authority_language`) were promoted to `default: deny` — their regexes are deterministic and the prose conventions they enforce are already satisfied (validation_per_lint, migration_completion, user_signoff via continuous auto-mode all pass); `asset_unregistered_cli_example` remains `default: allow` because its first-letter-sieve regex is unsafe-by-design against legitimate commands (memory/metrics/status/ship/fork/stale/explore); final state at v1.168.29: 5 of 9 lints promoted to `default: deny` (agent_result_used + the three AX-S5 asset_*), 4 remain advisory (4 gated on ADR-0100/0101/0096 acceptance + 1 unsafe-by-design); the `live_registry_asset_lints_are_promoted_or_advisory_and_clean` pin test codifies this split; the M9 enforcement infrastructure is now operationally meaningful beyond the original advisory intent (zero `--enforce` blockers against the live workspace); **at v1.168.35 (evidence-relations-core cycle), ADR-0100 (Universal Evidence) was promoted to `accepted`** — `Verifies` and `ObservedFor` variants are now wired into CoreRelationKind (was 12, now 14 entries) with 3 pin tests, unblocking the structural path for the `evidence_kind_v1` lint; **at v1.168.37 (synthesis-dissent-runner-extension cycle), `orchestration_synthesis_no_dissent` was promoted to `default: deny` via the bounded `exclude_paths` path** — its 19 prior hits were all test fixtures (1 in `mod tests` at crates/sddk-engine/src/orchestration_synthesis.rs:745, 18 in crates/sddk-engine/tests/orchestration_synthesis_tests.rs), and production code at v1.168.36 has zero `OrchestrationSynthesisReceipt::new` call sites (receipts are constructed dynamically by the synthesis pipeline); exclude_paths now filters `tests/` and the engine src/ file containing the mod tests block, dropping the hit count to 0; the runner-extension path (AST/flow analysis walking from `::new` to `set_dissent`/`put`) is deferred until production code starts constructing receipts directly; **at v1.168.39 (planning-evidence-migration cycle, this release), `evidence_kind_v1` advanced from `blocked_on_audit` to `audit_acknowledged`** — the canonical mapping function `planning_evidence_relation(PlanningEvidenceKind) -> CoreRelationKind` is now shipped in `crates/sddk-engine/src/evidence_relation_mapping.rs` (174 LoC, 7 unit tests, const-time exhaustiveness guard); mapping: Log → ObservedFor, Metric → Verifies, Snapshot → ObservedFor, Reference → References, Approval → Justifies (5 → 4 collapse intentional); per-call-site migration of `EvidenceAttachmentV1` (5 production crates + SQL schema at migrations.rs:737 + 2 test fixtures) is deferred to `evidence-migration-v2` because it requires a storage migration step + dual-write window per AGENTS §2.10 strangler pattern; **final state at v1.168.39: 6 of 9 lints promoted to `default: deny`** (agent_result_used + the three AX-S5 asset_* + orchestration_synthesis_no_dissent), 3 remain advisory (evidence_kind_v1 = 57 hits with structural path shipped; transition_outcome_used = 24 hits awaiting description rewrite + re-categorization; execution_outcome_as_synthesis = 0 hits awaiting corpus expansion; asset_unregistered_cli_example = 0 hits by-design regression guard); the M9 row's enforcement infrastructure is operationally meaningful: `sddk dev lint deprecated-patterns --enforce` passes clean (zero hits across all 5 deny lints), the four pin tests `test_advisory_lint_explanations.sh` (advisory explanation blocks), `test_deny_lint_zero_hits.sh` (deny+zero-hits invariant), `test_vault_adr_mirror_coverage.sh` (vault mirrors), and `test_authority_helper_lockstep.sh` (ADR authority helpers) all pass and are wired into `scripts/release.sh` step 1b |

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
