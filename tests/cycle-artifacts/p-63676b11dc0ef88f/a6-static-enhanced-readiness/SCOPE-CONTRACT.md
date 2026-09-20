# SCOPE-CONTRACT — A6 — Static Enhanced Readiness (macro-cycle)

> **Cycle id:** `p-63676b11dc0ef88f/a6-static-enhanced-readiness`
> **Document role:** SCOPE-CONTRACT of the macro-cycle (the operator authorised this macro-cycle with bounded autonomy; the SCOPE defines slices, dependencies, UAT, STOP conditions, and exit criteria).
> **Status:** reconciling slices against `origin/main = e026511` (v1.169.94) and the canonical roadmap live at `docs/SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md` §"A6 — CogniCode STATIC_ENHANCED".
> **Mode:** auto-run between slices; STOP only on the conditions named by the operator in the macro-cycle authorisation.

## §1 Reconcile against canonical roadmap

Mini-roadmap live definition (excerpt verbatim):

> **A6 — CogniCode STATIC_ENHANCED** [P1 parallel]
> └─ AC10 static evidence
>
> Follow `04-COGNICODE-HANDOFF.md` and arch-spec-021.
>
> AC10 consumes CogniCode call/dependency/impact/graph observations as Evidence through `CodeIntelligencePort`; provider types never enter Knowledge/Alignment/Verification domain. Static evidence can verify or contradict architecture claims but CogniCode never owns status.
>
> **Exit:** Base stays green with provider absent; **pinned enhanced UAT yields `STATIC_ENHANCED` receipt**.

UAT matrix source: `docs/SDDK-Production-Readiness-Alignment-2026-09-14/07-UAT-EVIDENCE-MATRIX.md` §E (PR-UAT-C01..C10).

### 1.1 What is already CLOSED (recognised, not re-done)

| Receipt | Cycle | Status | Evidence |
|---|---|---|---|
| `A6-COGNICODE-CC-S0-RECEIPT.md` | `a6-cognicode-protocol-spike` | CLOSED 53e03d0 | Trait `CodeIntelligencePort`, 4 operations, lifecycle, capability profile enum, 6 falsification tests. Pinned by `cargo test -p sddk-engine --test a6_cognicode_protocol_spike` (6/6 PASS). |
| `tests/cycle-artifacts/.../a6-cc-s1-static-graph-completeness/RECEIPT.md` | `a6-cc-s1-static-graph-completeness` | CLOSED e026511 (v1.169.94) | `CoverageContract`, `CoverageBasis`, `CoverageEvaluation`, `CoverageVerdict` ADTs. `arch-acceptance-coverage-001`. ADR-0139. 12 PASS / 1 ignored / 0 FAIL. |

CC-S0 + CC-S1 deliver:
- The transport (CC-S0).
- The evaluation contract (CC-S1).
- Falsifications F-α (file omitted), F-β (strategy lacks capability), F-γ (operational partial).
- Architectural guard: zero CogniCode types cross into `sddk-domain` (`no_knowledge_to_provider_sdk`).

### 1.2 What is NOT yet done (the macro-ciclo work)

| # | Capability | UAT coverage | Status |
|---|---|---|---|
| 1 | **AC10**: static evidence consumed by Verify / Knowledge / Alignment via `CodeIntelligencePort` | `PR-UAT-C04..C07, C09` | 🔲 not implemented |
| 2 | **CoverageContract for a real consumer** (e.g. `verify-kernel::static_evidence`) | acceptance contract `arch-acceptance-coverage-001` AR-2, AR-3 | 🔲 not implemented |
| 3 | **UAT-C09 contradiction preservation** | `PR-UAT-C09` | 🔲 not implemented |
| 4 | **Real CogniCode run** with `COGNICODE_MCP_BIN`; produce pinned `Satisfied` receipt | macro-ciclo exit criterion | 🔲 not executed (`t_ar_6_ext_real_cognicode_run` ignored) |
| 5 | **Durability of static-evidence artefacts** (`ObservationSet`, `CoverageEvaluation`) | `PR-UAT-024` (crash/reopen boundary) | 🔲 not demonstrated for static surface |
| 6 | **Fake relocation** under `dev-dependencies` with `#[cfg(any(test, feature = "test-support"))]` and `test-support` feature gate | CC-S0 receipt §3 finding 1; CC-S1 SCOPE M9 | ⚠ deferred to CC-S2 (named in CC-S1 RECEIPT §4.1) |

## §2 Slice plan

Each slice follows `SCOPE → caracterización/RED → implementación/GREEN → UAT → RECEIPT`, in the spirit of the macro-ciclo execution model. Each slice has **STOP conditions** that map to the operator's macro-ciclo STOP list (architectural change, authority, security, public contract, destructive migration, new source of truth, contradictory requirements, demonstrated impossibility).

### Slice S1 — `a6-static-enhanced-readiness-c1-uat-coverage-fake`

**Goal:** Cover `PR-UAT-C01, C02, C03, C04, C06, C08, C10` (the deterministic, fake-runnable subset) end-to-end through `CoverageContract` evaluation. Convert the "spike" character of CC-S0/CC-S1 into a UAT-anchored coverage matrix using the fake provider.

**Scope:**
- Pin the `CoverageContract` for `consumer = "verify-kernel::static_evidence"`, `scope = <workspace>`, `required_capabilities = [SymbolKind::fn, struct, trait, RelationClass::find_usages, RelationClass::analyze_impact]`.
- Add UAT-style integration tests in `crates/sddk-engine/tests/` mapping each `PR-UAT-C01..C10` row to a concrete test (deterministic subset) with `Evidence`-shaped output.
- Demonstrate: `Satisfied` for `verify-kernel::static_evidence` against the workspace inventory; `Incomplete` with explicit gaps for F-α/F-β/F-γ scenarios.
- Receipt: `tests/cycle-artifacts/.../a6-static-enhanced-readiness/S1-RECEIPT.md` with UAT evidence matrix per `§I` of `07-UAT-EVIDENCE-MATRIX.md`.

**STOP conditions:**
- Discovery that AC10 (Verify consumes static evidence) requires architectural change to Verify semantics — STOP and report (authority/architectural).
- Discovery that the fake provider's `CapabilitySnapshot` cannot faithfully represent a real consumer's claim — STOP and report (contradictory requirement).

**Depends on:** CC-S0, CC-S1.
**UAT matrix:** `PR-UAT-C01, C02, C03, C04, C06, C08, C10` (deterministic subset; C05, C07, C09 deferred to S2 or external).
**Estimated commits:** 1 SCOPE + 1 ADR (if needed) + 1 feat + 1 RECEIPT.
**Estimated size:** small (~150–300 LOC + tests).

### Slice S2 — `a6-static-enhanced-readiness-c2-uat-c05-c07-c09`

**Goal:** Cover `PR-UAT-C05` (impact, no DTO leakage), `C07` (restart/reconnect, stable refs), `C09` (contradiction preservation).

**Scope:**
- **C05**: prove through the adapter that a CogniCode-shaped `analyze_impact` response maps to `Evidence` with **zero** provider DTO leakage. Add a `no_provider_dto_leakage` test that inspects the SDDK surface after a fake-driven `analyze_impact` and fails if any field types `cognicode::*` appear.
- **C07**: simulate a restart mid-request at the adapter layer (already covered in CC-S0 t5); extend with stable-ref retention across reconnects.
- **C09**: when the provider's evidence **contradicts** an existing `KnowledgeAssertion`, the contradiction is preserved; SDDK produces an explicit `EvidenceGap { reason: contradiction }` and a reconciliation note — never overwrites.

**STOP conditions:**
- C09 reveals that current Knowledge/Alignment domain lacks a contradiction-preserving slot — STOP and report (architectural/authority; **this would be a §G5-class blocker for `STATIC_ENHANCED` declaration**).
- C05 reveals that `analyze_impact` cannot be implemented without leaking provider types into `Evidence` — STOP and report (architectural; would require IPB re-design).

**Depends on:** S1.
**UAT matrix:** `PR-UAT-C05, C07, C09`.
**Estimated size:** medium (~300–500 LOC + tests).

### Slice S3 — `a6-static-enhanced-readiness-c3-ac10-verify-integration`

**Goal:** Close AC10 — static evidence is consumed by Verify.

**Scope:**
- New module or extension in `sddk-engine/src/verify` that, given a `CoverageEvaluation` from `CodeIntelligencePort`, can produce a `VerifyReceipt` entry that distinguishes `OBSERVED_STATIC` from `INFERRED` evidence classes.
- The Verify path must be **delta-scoped** (`PR-UAT-019` invariant): a localized source delta + `analyze_delta` → `VerifyReceipt`, no unconditional full repo scan.
- Provider types never enter the Verify domain (boundary contract preserved).

**STOP conditions:**
- Implementing AC10 requires changing `Evidence` or `EvidencePosture` (A4 §2 freeze) — STOP and report (architectural/authority).
- Discover that the current `Verify` semantics cannot consume a `CoverageEvaluation` without violating IPB-002 (provider-as-evidence-source) — STOP and report.

**Depends on:** S1, S2.
**UAT matrix:** `PR-UAT-019` (Verify delta-scoped), `PR-UAT-C04` (already covered), `PR-UAT-C05` (already covered), AC10.
**Estimated size:** medium-large (~500–800 LOC + tests, careful architectural boundary).

### Slice S4 — `a6-static-enhanced-readiness-c4-durability`

**Goal:** Demonstrate that the static-evidence artefacts (`ObservationSet`, `CoverageEvaluation`, `VerifyReceipt` entries from S3) survive crash/reopen without loss or duplication. Maps to `PR-UAT-024`.

**Scope:**
- Persistence layer for `ObservationSet` and `CoverageEvaluation` in `sddk-storage` (or extension of an existing store). Pin via a `Ledger`-anchored event class.
- Falsification matrix:
  - Crash after observation appended but before `CoverageEvaluation` returned → on reopen, observation is durable; `CoverageEvaluation` is **not** present (correct partial state).
  - Crash mid-`CoverageEvaluation` write → reopen reconstructs from event log; no duplicate evaluation.
  - Reopen with empty store → no false "all satisfied" claim.

**STOP conditions:**
- Durability requires a schema migration incompatible with the live `SqliteLedger` — STOP and report (architectural/migration).
- Discover that the existing `Storage::cycle_exists` / event-log primitives cannot host static-evidence events without semantic loss — STOP and report.

**Depends on:** S3.
**UAT matrix:** `PR-UAT-024`, `PR-UAT-001`, `PR-UAT-023`.
**Estimated size:** medium (~400–600 LOC + tests).

### Slice S5 — `a6-static-enhanced-readiness-c5-ext-real-cognicode`

**Goal:** Execute `t_ar_6_ext_real_cognicode_run` with `COGNICODE_MCP_BIN` available; produce the pinned `Satisfied` `STATIC_ENHANCED` receipt required by the mini-roadmap exit criterion.

**Scope:**
- ENV-gated test (already drafted in CC-S1) is exercised against `cognicode-mcp v0.97.1` on the pinned Git revision.
- Adapter capability surface is queried for `available_strategies` and `semantic_classes`; if absent, the evaluation reports `Unknown` (per M6) and the slice is partial (NOT_EVALUATED, not closed).
- Produce the cycle's `S5-RECEIPT.md` with the UAT evidence YAML schema (`uat_id`, `sddk_commit`, `profile`, `fixture`, `command_or_test`, `result`, `result_ref`, `provider_basis`).

**STOP conditions:**
- `COGNICODE_MCP_BIN` not available in this environment — slice cannot close; remain `NOT_EVALUATED` (per macro-cycle rule: "Si una prueba externa indispensable queda ignorada o no ejecutada, mantener ese requisito como NOT_EVALUATED").
- Discover that the live CogniCode wire surface has drifted from the AIW-S1 handshake (`tests/cycle-artifacts/.../aiw-s1-cognicode-real/DISCOVERY.md`) — STOP and report; reopen adapter cycle.
- Discover that the live `cognicode-mcp` does not advertise `STATIC_ENHANCED` capabilities consistent with the contract — STOP and report; do not declare `STATIC_ENHANCED=true`.

**Depends on:** S1, S2, S3, S4 (the contract evaluation must be trustworthy before the EXT closes).
**UAT matrix:** AC10, `PR-UAT-C01..C10` against the real provider.
**Estimated size:** depends on CogniCode availability (likely small if available; otherwise remains open).

### Slice S6 — `a6-static-enhanced-readiness-c6-fake-relocation`

**Goal:** Close the CC-S0 RECEIPT §3 finding 1: move `code_intelligence_port_fake` under `dev-dependencies` + `#[cfg(any(test, feature = "test-support"))]` with a `test-support` Cargo feature. Audit downstream consumers first.

**Scope:**
- Catalogue every reference to `FakeCodeIntelligenceProvider` / `NullCodeIntelligenceProvider` outside `crates/sddk-engine/tests/`.
- Move the module under the feature gate.
- Verify: 6 CC-S0 tests still green; 12 CC-S1 tests still green; workspace tests still green; `cargo build --release` for the production binary does not compile the fake.

**STOP conditions:**
- Audit reveals a downstream crate legitimately depends on the fake for production-side integration testing — STOP and report (architectural/contract); propose alternative.
- Moving the fake breaks the 6 CC-S0 tests or the 12 CC-S1 tests and the breakage cannot be fixed without exposing the fake to a wider surface — STOP and report.

**Depends on:** independent; can run in parallel with S1..S5 if no file contention.
**UAT matrix:** internal regression (CC-S0, CC-S1); release-profile check.
**Estimated size:** small (~50–150 LOC + tests).

### Slice S7 — `a6-static-enhanced-readiness-c7-closeout`

**Goal:** Reconcile the roadmap at macro-ciclo close. Deliver the integrated report named by the operator: slices executed, commits, receipts, real UAT results (including ignored/failed), pending limitations, A6 status contrasted with the canonical roadmap.

**Scope:**
- Read-only synthesis across S1..S6 receipts.
- Honest reporting of which `PR-UAT-C0X` rows passed against fake, which against real, which remained `NOT_EVALUATED`, which revealed STOP conditions.
- Update `docs/architecture/a6/A6-CC-*-CLOSE-RECEIPT.md` with the macro-ciclo result.
- DO NOT declare A6 completed unless S5 produced a real-CogniCode `Satisfied` receipt AND all the named STOP conditions have been resolved.

**Depends on:** S1..S6 (or as many as reachable).
**Estimated size:** docs only.

## §3 Slice dependency graph

```text
S1 (UAT coverage — fake) ─┬─ S2 (UAT C05/C07/C09) ─┬─ S3 (AC10 Verify) ─┬─ S4 (durability) ─┬─ S5 (EXT real)
                           │                          │                       │                       │
                           │                          │                       └───────────────────────┘
                           └──────────────────────────┴─ S6 (fake relocation, parallel)
                                                                                       │
                                                                       S7 (closeout, after S1..S6)
```

S6 is intentionally parallel and non-blocking; can be picked up whenever no engine source is touched.

## §4 STOP / pause matrix (operator-named, applied)

| Operator condition | Where it could trigger in this plan | Reporting |
|---|---|---|
| Cambio material de arquitectura | S3 (Verify ↔ static-evidence seam), S2/C09 (contradiction slot), S4 (event class addition) | STOP; report with proposed ADR scope |
| Autoridad | S7 (declaring `STATIC_ENHANCED=true`) | STOP; operator decides whether the macro-ciclo exit criterion is met |
| Seguridad | Any slice touching `sddk-storage` or redaction paths | STOP; report |
| Contratos públicos | Changes to `arch-spec-021` IPB / `arch-acceptance-coverage-001` AR-N | STOP; require spec/acceptance-contract amendment cycle |
| Migraciones destructivas | S4 (schema migration), S6 (feature gate breaks downstream) | STOP; report |
| Nuevas fuentes de verdad | Adding static evidence as canonical (must remain projection per IPB-002) | STOP; report |
| Requisitos contradictorios | A UAT row that cannot be satisfied without breaking another | STOP; report both UATs |
| Imposibilidad demostrada | S5 with `COGNICODE_MCP_BIN` unavailable | Mark `NOT_EVALUATED`; do not fabricate; continue S7 reporting |

## §5 Macro-ciclo exit criterion (per mini-roadmap)

The macro-ciclo is **CLOSED** when:

1. S5 has produced a real-CogniCode `Satisfied` `CoverageContract` evaluation for the pinned contract (`static-enhanced-workspace-v1` for `verify-kernel::static_evidence`).
2. AC10 is implemented and demonstrated through `PR-UAT-019` with static evidence consumed by Verify.
3. All of `PR-UAT-C01..C10` have a documented evidence row (`PASS` / `NOT_EVALUATED` / `FAIL`); no row is silently skipped.
4. The cycle's `S7-CLOSEOUT-RECEIPT.md` reflects the actual state honestly.

If S5 cannot run (no `COGNICODE_MCP_BIN`), the macro-ciclo closes with `STATIC_ENHANCED` declared as `NOT_EVALUATED` for that dimension; the A6 profile is **not** declared closed.

## §6 Out-of-scope (operator-named, applied)

- Independent CLI proposals (`sddk-*` commands not in CC-S1 scope).
- Chronos / RUNTIME_ENHANCED expansions (AIW-S5 already closed in v1.169.93; not part of A6-static).
- Dynamic workflow expansion.

## §7 Reporting

The orchestrator emits a Human Feed at each slice close (per the global prompt overlay). The macro-ciclo closeout feed (S7) includes:

- Slices executed (with commit SHAs and receipts).
- Real UAT results (PASS / NOT_EVALUATED / FAIL), explicit per row.
- Pending limitations (slice that could not run; slice that revealed a STOP condition).
- A6 status vs the canonical roadmap: which exit-criterion dimensions are satisfied, which remain open.
- New incidents opened, if any.

## §8 References

- `docs/SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md` (live roadmap)
- `docs/SDDK-Production-Readiness-Alignment-2026-09-14/04-COGNICODE-HANDOFF.md`
- `docs/SDDK-Production-Readiness-Alignment-2026-09-14/07-UAT-EVIDENCE-MATRIX.md` (§E, §I)
- `docs/architecture/a6/A6-COGNICODE-CC-S0-RECEIPT.md` (CLOSED)
- `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-cc-s1-static-graph-completeness/RECEIPT.md` (CLOSED)
- `docs/architecture/adrs/ADR-0137-CODE-INTELLIGENCE-PORT-SEAM.md`
- `docs/architecture/adrs/ADR-0139-STATIC-ENHANCED-COVERAGE-CONTRACT.md`
- `docs/architecture/specs/arch-spec-021-intelligence-provider-boundary.md`
- `docs/architecture/specs/arch-acceptance-coverage-001.md`
- `githooks/pre-push` (push admission contract — conditions A and B)
