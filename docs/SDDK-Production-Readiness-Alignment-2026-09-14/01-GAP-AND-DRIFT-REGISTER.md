# Gap and Drift Register

**Audit basis:** `2daa4f8fe4414377e313eb21ff50a42e63f770ba`  
**States:** `PASS`, `PASS_WITH_COMPAT`, `PARTIAL`, `NOT_STARTED`, `DEFERRED`, `REJECTED`.  
**Normative strength:** `MUST` blocks the named readiness profile; `SHOULD` requires explicit accepted-risk disposition if omitted.

## A. Baseline convergence and ownership

| ID | Finding | Audit state | Requirement / exit | Blocks |
|---|---|---:|---|---|
| `PR-GAP-001` | Canonical event authority | `PASS_WITH_COMPAT` | `events_v1` remains the sole production append authority; legacy ledger paths are read-only/removed; migrated-repo rebuild proves parity. | Base |
| `PR-GAP-002` | Universal Evidence convergence | `PARTIAL` | No production write may use legacy `PlanningEvidenceKind` as authority. Legacy taxonomy may survive only as decode compatibility with removal trigger. | Base |
| `PR-GAP-003` | Cycle vs Run runtime truth | `PASS_WITH_COMPAT` | Runtime-derived Cycle variants remain decode-only; approval/UAT/retry/recovery truth is Run/Authority/Evidence-owned and guarded by static tests. | Base |
| `PR-GAP-004` | Revision substrate vs Decision Memory | `PASS` | Classify every OID/ref/CAS/ancestry primitive as shared, decision-semantic, compatibility, or redundant. Semantically identical generic invariants have one implementation. Narrow differences require ADR. **Evidence (2026-09-14):** classification in `13-A0-REVISION-SUBSTRATE-CLASSIFICATION.md`; no semantically-identical duplication (generic `Revision<T>`/`Oid`/`RefStore` vs decision-semantic `MemoryId`/`Reflog`/commit DAG differ in canonicalization, identity shape and added semantics). Narrow differences governed by accepted `ADR-0097`. PR-UAT-004 evidence: `revision_substrate::tests::ref_store_cas_race_has_single_winner`. | Base |
| `PR-GAP-005` | SQLite schema ownership | `PASS` | Define one migration/schema owner. Stores consume an already-governed schema/connection contract rather than independently evolving shared tables. Add fresh DB, migrated DB, FK and crash/reopen fixtures. See ARCH-SPEC-020. **Evidence (2026-09-14):** `SqliteEventStore` no longer runs a competing migration subset behind a private `sddk_eventstore_version` pragma; it delegates shared schema to `crate::migrations::run_migrations` and owns only its auxiliary `event_snapshots_v1`. The duplicate `MIGRATION_5` `projects` stub and the dead `telemetry.rs::SCHEMA_V1` DDL were removed. Fixtures: `crates/sddk-storage/tests/schema_ownership.rs` (fresh, event-store-first FK bootstrap, reopen idempotency, version parity) plus existing `legacy_ledger_migration`/`migration_16`/`sqlite_storage` migrated-DB tests. | Base |
| `PR-GAP-006` | AuthorityEngine cutover | `PARTIAL` | Enumerate governed effects; route all through one AuthorityEngine decision path; complete the `LowMedium → All` rollout or replace that staging mechanism with an equally explicit cutover; prove zero bypass. **Investigation (2026-09-14):** `14-A0-PR-GAP-006-ENFORCEMENT-CUTOVER-INVESTIGATION.md`. Naive flip to `All` blocks every action on the six High-band surfaces and breaks 16 core CLI workflows (gate/release/phase); awaiting a product decision (options A/B/C). `ENFORCEMENT_STAGE` unchanged. | Base |
| `PR-GAP-007` | Command registry source of truth | `PARTIAL` | Eliminate authoritative handwritten duplication between the CLI command model and `CommandSpec`. Help, examples, agent surface and allowed-command sets derive from one typed substrate. | Base |
| `PR-GAP-008` | Public legacy compatibility surface | `PASS` | Active API stops glob-exporting legacy semantics. Required historical decode paths move behind an explicit compatibility namespace/allowlist with owner, fixture and removal trigger. **Evidence (2026-09-14):** removed `pub use legacy::*` from `sddk-domain/src/lib.rs`; consumers opt in via the explicit `sddk_domain::legacy` namespace (`result_cmd.rs`). `legacy.rs` module docs record decode-only scope, owner and removal trigger. Fixture: `legacy.rs::tests` (tolerant decode) → PR-UAT-007. | Base |
| `PR-GAP-009` | Dead/staged code and dependencies | `PARTIAL` | Run dead dependency/code scans; remove real dead dependencies. Every surviving `allow(dead_code)`, deprecated path or staged branch has owner + reason + exit trigger. | Base |
| `PR-GAP-010` | Documentation authority drift | `PARTIAL` | Reconcile 09/09 baseline, C0→C7 closeout and 10/09 roadmap into one documented succession. No implemented normative spec remains `proposed` without reason; no historical package claims competing authority. | Base |
| `PR-GAP-011` | C7 conformance receipt | `NOT_STARTED` | SPEC-001..018 all PASS/PASS_WITH_COMPAT, UAT-01..22 green, clean + migrated + recovery + projection-rebuild runs green, commit SHA recorded. | Base |

## B. Context-first Base capabilities

| ID | Finding | Audit state | Requirement / exit | Blocks |
|---|---|---:|---|---|
| `PR-GAP-012` | Physical bounded-context cut (R0) | `NOT_STARTED` | Move current code into `shared/planning/execution/decision/knowledge/alignment/verification/governance/agent_experience/extension` ownership without semantic change; keep temporary facades only where compatibility requires them; dependency fitness tests block new reverse edges. | Base |
| `PR-GAP-013` | Knowledge model / KnowledgeBasis / KMT | `NOT_STARTED` | Implement `KnowledgeAssertion`, `KnowledgeBasis`, freshness/invalidation, KMT v1 and SemanticGraph overlay. LLM statements are never promoted directly to observed/verified truth. | Base |
| `PR-GAP-014` | Agent advisory/instruction split | `NOT_STARTED` | Add explicit advisory context to `ContextCapsule`; prove Software Alignment output cannot implement/convert to `InstructionSource` and cannot alter the effective-instruction hash. | Base |
| `PR-GAP-015` | Software Alignment core | `NOT_STARTED` | Implement universal concerns, versioned lenses, project Architectural Intent, assessments and opportunities. `UNKNOWN` for missing evidence; no universal quality score; Alignment remains advisory. | Base |
| `PR-GAP-016` | Verify delta synchronization | `NOT_STARTED` | Implement delta-scoped knowledge synchronization, impact/staleness, EvidenceGap, DeepeningCandidate and `VerifyReceipt`; no default full scan. | Base |
| `PR-GAP-017` | DebVerify reconciliation | `NOT_STARTED` | Implement whole-baseline challenge, debt taxonomy, contradiction/stale-decision reconciliation and global receipt. It MUST remain distinct from `verify --full`. | Base |
| `PR-GAP-018` | Completion providers vs intelligence providers | `PARTIAL` | Current model/completion routing must not become the owner of static/runtime engineering intelligence merely because both are called providers. Give each role explicit types/names and keep Code/Runtime intelligence behind the Extension Platform ports. | Base |

## C. Enhanced intelligence profiles

| ID | Finding | Audit state | Requirement / exit | Blocks |
|---|---|---:|---|---|
| `PR-GAP-019` | Provider absence semantics | `NOT_STARTED` | OPTIONAL/PREFERRED absence maps to `NOT_EVALUATED`/EvidenceGap; REQUIRED absence fails the requested operation explicitly. Provider absence can never manufacture green Alignment/Verify evidence. | Static + Runtime enhanced |
| `PR-GAP-020` | CogniCode static intelligence integration (R7) | `NOT_STARTED` | Versioned capability/RPC contract, provider lifecycle, `CodeIntelligencePort` adapter, evidence normalization, basis/digest provenance, deterministic fixtures and end-to-end Verify/Alignment UAT. | Static enhanced |
| `PR-GAP-021` | Chronos runtime intelligence integration (R8) | `NOT_STARTED` | Versioned runtime/scenario contract, `RuntimeIntelligencePort` adapter, runtime evidence refs/fingerprints, cancellation/resource bounds, golden scenarios and end-to-end Verify/Alignment UAT. | Runtime enhanced |
| `PR-GAP-022` | Cross-provider reconciliation | `NOT_STARTED` | Static and runtime evidence may corroborate or contradict each other without either provider becoming canonical truth. Contradictions are preserved and surfaced. | Fully enhanced |
| `PR-GAP-023` | Provider reproducibility basis | `NOT_STARTED` | Every enhanced receipt records protocol version, provider build/version, capability snapshot, analyzer/instrumentation basis and stable result references/digests. | Static + Runtime enhanced |

## D. Accepted post-Base evolution and independent GA tracks

| ID | Finding | Audit state | Requirement / exit | Blocks |
|---|---|---:|---|---|
| `PR-GAP-024` | Workbooks/control tower (R9) | `NOT_STARTED` | Rebuildable projections only; edits emit semantic commands to owners; provenance on every rendered assessment/evidence item; no canonical writes from workbook code. | Program convergence, not Base |
| `PR-GAP-025` | Governance ratchets + enriched WHY (R10) | `PARTIAL` | Governance may consume explicit evidence/contracts but never Alignment opinions directly. WHY/WHY-NOT includes Knowledge/Alignment provenance. | Program convergence, not minimum Base if existing Authority/WHY gates are already green |
| `PR-GAP-026` | Crate split evaluation (R11) | `DEFERRED` | Split only with sustained dependency/change metrics. No aesthetic split. | None |
| `PR-GAP-027` | Agentic Workspace / JCode | `NOT_STARTED` | Execute J0→J9 from `09-AGENTIC-WORKSPACE-ROADMAP.md`: public SDDK Agentic API/SDK, separate `sddk-jcode` ACL, SessionBinding, event-driven materiality/debounce, KMT/Verify reactivity, ContextDelta, structured Contribution, advanced capability mediation and second-host portability validation. Public SDK/API boundaries only; no JCode internals in SDDK. | Separate Agentic/JCode GA declarations, not Base |

`PR-GAP-027` is scheduled, not a vague future placeholder: J0/J1 preparation is P1 once A2/A3 contracts are stable enough; after A5 the default product priority is J2→J6 `JCODE_CORE_GA`, in parallel with A6/A7 provider tracks. J7 MCP remains optional P3; J8/J9 are P2 post-Core-GA/portability work.

## Mandatory resolution policy

A `MUST` item can close only as:

- `PASS`: current production path satisfies the requirement and executable evidence is recorded; or
- `PASS_WITH_COMPAT`: production authority is already correct and a compatibility path remains with **read-only scope, fixture, owner and objective removal trigger**.

`PARTIAL`, `UNKNOWN`, undocumented `accepted_risk`, or a test that exercises only the new path while an old production bypass remains reachable cannot close a readiness gate.

## No silent scope loss

When implementation discovers that an accepted requirement should no longer be built, the change MUST update `06-PROPOSAL-DISPOSITION-REGISTER.md` with one of:

- `SUPERSEDED_BY(<id>)` — same goal is achieved by a better contract;
- `DEFERRED(reason, trigger)` — still accepted but intentionally later;
- `REJECTED(reason, decision-ref)` — intentionally discarded.

Deleting a roadmap line or leaving a specification perpetually `proposed` is not a valid disposition.
