# UAT and Evidence Matrix

This matrix complements the existing 09/09 UAT-01..22. It does not renumber or replace them; it verifies the convergence work, accepted context-first/enhanced-provider capabilities, and links the separate Agentic Workspace/JCode GA evidence track.

Every row must end with a reproducible test/fixture/receipt reference, not prose-only sign-off.

## A. Baseline closure UAT

| ID | Scenario | Expected invariant | Gaps | Profile |
|---|---|---|---|---|
| `PR-UAT-001` | Fresh DB + first canonical append | exactly one append authority; complete required project/schema state; no FK/bootstrap ambiguity | 001,005 | Base |
| `PR-UAT-002` | Migrate supported legacy DB containing historical ledger/evidence data | migration is idempotent; new writes go only to canonical authority; projections rebuild equivalently | 001,002,011 | Base |
| `PR-UAT-003` | Attempt a runtime-derived Cycle write after cutover | production path rejects/does not emit it; compatibility decode still renders old data | 003 | Base |
| `PR-UAT-004` | Revision/CAS race from two writers | one generic CAS invariant decides outcome; Decision Memory adds only decision semantics | 004 | Base |
| `PR-UAT-005` | Approval-required governed side effect through every enumerated effect class | all routes hit AuthorityEngine; no direct bypass succeeds | 006 | Base |
| `PR-UAT-006` | Mutate command syntax in one source | generated/introspected help/docs/agent surface remain coherent or drift test fails build | 007 | Base |
| `PR-UAT-007` | Load historical legacy agent/evidence payload | decode succeeds through compatibility namespace; no active API/write path regains legacy authority | 002,008 | Base |
| `PR-UAT-008` | Workspace dead-code/dependency scan | no unowned production dead dependency; staged/deprecated exceptions have owner+trigger | 009 | Base |
| `PR-UAT-009` | Documentation/spec status scan | one succession path 09/09 → C7 → 10/09; no competing current authority | 010 | Base |
| `PR-UAT-010` | Full clean + migrated C7 run | SPEC-001..018 PASS/PASS_WITH_COMPAT and existing UAT-01..22 PASS on named commit | 011 | Base |

## B. Context/Knowledge/Alignment UAT

| ID | Scenario | Expected invariant | Gaps | Profile |
|---|---|---|---|---|
| `PR-UAT-011` | Compile dependency graph after R0 cut | forbidden bounded-context edges fail fitness test; behavior unchanged from C7 | 012 | Base |
| `PR-UAT-012` | Change implementation body in one leaf unit | KMT invalidates required dimensions/ancestors deterministically without treating KMT as dependency graph | 013 | Base |
| `PR-UAT-013` | Comment/documentation-only change | only relevant fingerprints/staleness change; unrelated dependency/alignment state is not globally invalidated | 013,016 | Base |
| `PR-UAT-014` | LLM/analyzer produces unverified claim | claim remains `INFERRED`; cannot appear as deterministic `OBSERVED/VERIFIED` without evidence | 013 | Base |
| `PR-UAT-015` | Add Alignment advisory to ContextCapsule | capsule hash changes; EffectiveInstructionSet hash does not | 014 | Base |
| `PR-UAT-016` | Alignment lens lacks required evidence | assessment is `UNKNOWN`/`NOT_APPLICABLE`, never false `ALIGNED/MISALIGNED` | 015 | Base |
| `PR-UAT-017` | Same evidence under two declared architectural intents/lenses | assessments may differ with preserved evidence/intent provenance; no universal score averages them | 015 | Base |
| `PR-UAT-018` | Alignment emits serious tension | it reaches advisory context/workbook but cannot itself deny Authority or gate release | 014,015 | Base |

## C. Verify / DebVerify UAT

| ID | Scenario | Expected invariant | Gaps | Profile |
|---|---|---|---|---|
| `PR-UAT-019` | Localized source delta | Verify analyzes affected scope/impact without unconditional full repository scan; emits VerifyReceipt | 016 | Base |
| `PR-UAT-020` | Evidence needed but optional deep provider absent | EvidenceGap/NOT_EVALUATED is explicit; receipt cannot claim unsupported PASS | 016,019 | Base |
| `PR-UAT-021` | Global fixture contains debt outside current Git delta | DebVerify finds/reconciles it; ordinary Verify remains delta-scoped | 017 | Base |
| `PR-UAT-022` | Knowledge contradiction + stale DecisionRef | DebVerify preserves contradiction, identifies revisit need and creates reconciliation receipt without rewriting history | 017 | Base |

## D. Operational Base readiness UAT

| ID | Scenario | Expected invariant | Profile |
|---|---|---|---|
| `PR-UAT-023` | Delete all rebuildable projections | projections reconstruct from canonical/object inputs with semantic parity | Base |
| `PR-UAT-024` | Crash/reopen around event append/CAS boundary | no acknowledged fact is lost and no already committed fact is duplicated by recovery | Base |
| `PR-UAT-025` | Unsupported future schema/config/protocol | explicit incompatible/unsupported result; no best-effort state corruption | Base |
| `PR-UAT-026` | Installed release binary executes documented workflows | examples and command surface match registry; no `cargo run`-only success | Base |
| `PR-UAT-027` | Base mode with CogniCode and Chronos missing | all Base UAT remains green; enhanced evidence is absent/NOT_EVALUATED rather than fabricated | Base |
| `PR-UAT-028` | Redaction/security fixture contains credentials/sensitive value | receipts/evidence/logging follow redaction contract and do not leak it | Base |

## E. CogniCode Static Enhanced UAT

| ID | Scenario | Expected invariant | Profile |
|---|---|---|---|
| `PR-UAT-C01` | Provider stopped, requirement OPTIONAL/PREFERRED | SDDK remains Base-capable; records EvidenceGap/NOT_EVALUATED | Static |
| `PR-UAT-C02` | Provider stopped, requirement REQUIRED | requested operation fails explicitly; Base state remains valid | Static |
| `PR-UAT-C03` | Compatible provider startup/handshake | runtime protocol/capabilities negotiated and recorded | Static |
| `PR-UAT-C04` | Localized delta analysis | typed result + basis/analyzer provenance; no forced full graph transfer | Static |
| `PR-UAT-C05` | Impact analysis crosses dependency boundary | provider result maps to SDDK Evidence/Knowledge without provider DTO leakage | Static |
| `PR-UAT-C06` | Cancellation/deadline | incomplete/cancelled result; no green VerifyReceipt from missing requested evidence | Static |
| `PR-UAT-C07` | Provider restart/reconnect | SDDK resumes with explicit lifecycle transition; prior stable refs retain declared semantics | Static |
| `PR-UAT-C08` | Protocol major incompatible | provider state is INCOMPATIBLE; no capability inference from product version | Static |
| `PR-UAT-C09` | Provider evidence contradicts existing KnowledgeAssertion | contradiction is preserved and reconciled explicitly, not overwritten | Static |
| `PR-UAT-C10` | Repeat deterministic analysis on same basis/analyzer set | promised deterministic semantic digest/result identity is stable | Static |

## F. Chronos Runtime Enhanced UAT

| ID | Scenario | Expected invariant | Profile |
|---|---|---|---|
| `PR-UAT-H01` | Provider stopped OPTIONAL/PREFERRED | Base remains green; runtime evidence is NOT_EVALUATED | Runtime |
| `PR-UAT-H02` | Provider stopped REQUIRED | requested runtime operation fails explicitly without corrupting Base | Runtime |
| `PR-UAT-H03` | Compatible scenario handshake/run/observe | protocol/capabilities + ScenarioBasis recorded | Runtime |
| `PR-UAT-H04` | Golden normal behavior scenario | stable evidence ref + behavior fingerprint within declared determinism/tolerance | Runtime |
| `PR-UAT-H05` | Error/exception behavior scenario | runtime observation maps to Evidence/Knowledge with scenario/instrumentation provenance | Runtime |
| `PR-UAT-H06` | Concurrent scenario | correlation and causal/temporal limitations remain explicit; no false total ordering | Runtime |
| `PR-UAT-H07` | Timeout/cancellation | scenario cleanup occurs; partial evidence marked incomplete; no false PASS | Runtime |
| `PR-UAT-H08` | Trace/resource budget exceeded | provider stops/bounds collection according to policy; SDDK remains healthy | Runtime |
| `PR-UAT-H09` | Provider restart/reconnect | lifecycle recovers and stable refs follow declared retention semantics | Runtime |
| `PR-UAT-H10` | Protocol incompatible | explicit INCOMPATIBLE state | Runtime |

## G. Fully Enhanced UAT

| ID | Scenario | Expected invariant | Profile |
|---|---|---|---|
| `PR-UAT-F01` | Static + runtime evidence corroborate | both provenances remain visible; evidence diversity increases without source collapse | Full |
| `PR-UAT-F02` | Static says A, runtime observes not-A | contradiction survives Knowledge/Alignment; no average/universal score hides it | Full |
| `PR-UAT-F03` | CogniCode dies mid-cycle while Chronos remains | capability profile degrades accurately; unaffected Runtime/Base work continues | Full |
| `PR-UAT-F04` | Chronos dies mid-cycle while CogniCode remains | capability profile degrades accurately; unaffected Static/Base work continues | Full |
| `PR-UAT-F05` | Both providers unavailable | system returns to truthful Base behavior; no stale FULLY_ENHANCED advertisement | Full |

## H. Agentic Workspace / JCode GA UAT

The Agentic track is intentionally separate from Base/Static/Runtime/Full readiness. Its complete matrix is `10-AGENTIC-WORKSPACE-UAT.md` using `AW-UAT-*` identifiers.

The primary cross-profile invariants are:

| Gate | Required evidence | Relationship to production profiles |
|---|---|---|
| `AGENTIC_API_EXPERIMENTAL` | J1 generic API/SDK + fake-host/no-type-leak UAT | independent of Base deployment profile, but consumes A3 semantic contracts |
| `JCODE_CORE_GA` | J0-J6 + required AW-UAT external SDK, binding, context, reactive Verify and structured execution rows | requires a stable Base semantic implementation; CogniCode/Chronos not required |
| `JCODE_ADVANCED` | selected J8 permission/interruption/model/remote UAT | additive after Core GA |
| `MULTI_HOST_VALIDATED` | J9 + AW-UAT-090..092 | required before generic Agentic API 1.0/stable claim |
| `AGENTIC_API_STABLE` | multi-host evidence + no unresolved host-specific generic contract leak | does not imply Static/Runtime/Full provider profiles |

Agentic cross-checks that MUST remain green:

- no JCode SDK/protocol types enter generic SDDK semantic contracts;
- `Session != Run`;
- host transcripts remain host-owned;
- ephemeral host events do not flood the Canonical Event Log;
- `turn_done`/semantic debounce produces bounded delta-scoped Verify;
- `ContextDelta` preserves advisory/instruction separation;
- JCode Core GA passes with CogniCode and Chronos absent;
- existing `ledger watch`/`diff-watch` are not required as host transport;
- capability negotiation, not product version inference, controls optional host behavior;
- invalid structured output cannot become a successful Contribution.

## I. Evidence required to mark a row PASS

Each readiness row should record:

```yaml
uat_id: PR-UAT-...
sddk_commit: "..."
profile: "..."
fixture: "..."
command_or_test: "..."
result: PASS
result_ref: "..."
provider_basis: null | {...}
notes: "..."
```

Agentic rows use the richer record defined in `10-AGENTIC-WORKSPACE-UAT.md`, including SDDK Agentic API version, `sddk-jcode` commit, JCode revision/Harness API and negotiated host capabilities.

A row is not PASS merely because a unit test with a similar name exists. The evidence must exercise the authority/boundary described by the scenario, including the negative/bypass path where applicable.
