# Mini-roadmap — From Current Main to Production Readiness + Agentic GA

This roadmap does not replace C0→C7 or R0→R11. It compresses them into a finite execution sequence based on the 2026-09-14 code audit and integrates the accepted Agentic Workspace/JCode evolution as a parallel product track.

## Priority model

- **P0 — blocker/release gate:** close before consuming the affected contract or declaring the corresponding readiness profile.
- **P1 — high:** next product-value work; may run in parallel when dependencies permit.
- **P2 — medium:** important enhancement but not a blocker for the preceding GA profile.
- **P3 — optional/evidence-driven:** only after a demonstrated need or portability/maintenance trigger.

## Global dependency map

```text
BASE / semantic critical path

A0 Close current authority drifts                 [P0]
  -> A1 Baseline C7 receipt                       [P0]
  -> A2 Context cut + semantic ownership          [P0]
  -> A3 Knowledge + Agent advisory foundation     [P1]
  -> A4 Alignment + Verify + DebVerify            [P1]
  -> A5 Base production certification             [P0 release gate]

Preparation allowed after A2/A3 contracts stabilize

                 +-> J0 external JCode SDK proof              [P1]
                 +-> J1 SDDK Agentic public API/SDK            [P1]

Post-Base product tracks

A5 BASE READY ---+-> J2 -> J6 JCode Core GA                    [P1]
                 +-> A6 CogniCode Static Enhanced              [P1 parallel]
                 +-> A7 Chronos Runtime Enhanced               [P1 parallel]

Then

J8 Advanced host capabilities                                  [P2]
J9 Second-host validation before Agentic API 1.0               [P2]
A8 Full Enhanced + program convergence                         [P2]
J7 MCP pull surface                                             [P3 optional]
R11 bounded-context crate split evaluation                     [P3 evidence-driven]
```

**Priority decision:** after `BASE_PRODUCTION_READY`, the default next product priority is **JCode Core GA (J2→J6)** because it exposes the already-built SDDK semantics directly in an interactive development workflow. CogniCode and Chronos remain P1 and may proceed in parallel. Neither provider is allowed to become a prerequisite for JCode Base-mode integration.

Provider/JCode protocol spikes may run in parallel after A2/A3, but production integrations must consume stable SDDK-owned semantic ports rather than shape the domain around external APIs.

---

## A0 — Close the remaining 09/09 authority drifts — P0

**Scope:** finish C3→C6 with the current code still physically where it is.

Work:

1. reconcile `Revision<T>` and Decision Memory generic revision primitives (`PR-GAP-004`);
2. establish one SQLite schema/migration owner (`PR-GAP-005`);
3. complete AuthorityEngine cutover and zero-bypass guard (`PR-GAP-006`);
4. make Command Registry single-source (`PR-GAP-007`);
5. internalize/time-bound legacy compatibility (`PR-GAP-008`);
6. run dead-dependency/staged-code cleanup (`PR-GAP-009`);
7. reconcile architecture/spec statuses and succession (`PR-GAP-010`).

**Do not do:** R0 file moves mixed with semantic fixes. Semantic fixes first; physical movement afterwards.

**Exit:** C3, C4, C5 and C6 have executable evidence and no unresolved MUST drift.

## A1 — Close C7 and freeze the baseline receipt — P0

Run the full closeout exactly once against a named commit after A0:

- SPEC-001..018 crosswalk;
- UAT-01..22;
- clean repository workflow;
- migrated repository workflow;
- projection delete/rebuild;
- crash/reopen/fresh-process recovery;
- installed CLI fixtures/examples;
- dependency and deprecated-path guards.

**Artifact:** `09-09-CONFORMANCE-RECEIPT.md` with commit SHA and test evidence.

**Exit:** only `PASS` / `PASS_WITH_COMPAT`; zero unresolved MUST.

## A2 — R0 + R1: context boundary cut and semantic consolidation — P0

Perform the behaviour-preserving file/module cut first, then consolidate semantic ownership in the new locations.

Target ownership:

```text
shared
planning
execution
decision
knowledge
alignment
verification
governance
agent_experience
extension
```

Agentic Workspace is a logical supporting integration context, but host-specific adapter code SHALL NOT be placed in these core contexts. JCode-specific code belongs to the separate `sddk-jcode` parcel.

Required fitness rules include:

- no domain → RPC/provider/host SDK;
- no Knowledge → provider SDK;
- no Alignment → AuthorityEngine implementation;
- no Alignment → InstructionCompiler;
- no workbook → canonical write;
- no host-specific JCode type in generic Agentic Workspace contracts;
- no new root-level context module after R0 without ADR.

**Exit:** behaviour parity from A1 remains green and dependency fitness tests enforce the new boundaries.

## A3 — R2 + R3: Knowledge substrate and Agent advisory boundary — P1

Implement only the foundations required by Alignment/Verify and later Agentic Workspace exposure:

- `KnowledgeAssertion` with evidence/status discipline;
- `KnowledgeBasis`;
- KMT v1 identity/freshness/invalidation;
- SemanticGraph cross-tree overlay;
- Software Unit cards/progressive disclosure;
- `ContextCapsule.advisory_context`;
- separate `context_capsule_hash` and `effective_instruction_set_hash`;
- explicit invariant that Alignment cannot become `InstructionSource`.

**Exit:** deterministic fixtures prove freshness/invalidation and an Alignment-like advisory payload changes context hash without changing instruction-set hash.

A3 is the minimum semantic basis for finalizing J1; J0 may execute earlier as an external SDK spike.

## A4 — R4 + R5 + R6: Alignment, Verify and DebVerify — P1

Implement the intelligence loop in Base mode before requiring external providers:

```text
Sources + Decision Memory
        -> Knowledge/KMT
        -> Alignment
        -> Verify / DebVerify
        -> Evidence + receipts
```

Key acceptance:

- Alignment is advisory and paradigm-neutral;
- missing evidence => `UNKNOWN`, never `MISALIGNED` or synthetic green;
- Verify is delta-scoped by default;
- DebVerify challenges the global accumulated baseline and is not `verify --full`;
- deterministic analyzers + optional LLM evaluator have provenance and status discipline;
- Base mode requires neither CogniCode nor Chronos.

**Exit:** R4/R5/R6 UAT fixtures green with both providers absent.

This stage supplies the reactive semantic loop later consumed by J5; the host adapter SHALL NOT reimplement Verify/Alignment semantics.

## A5 — Certify `BASE_PRODUCTION_READY` — P0 release gate

Hardening:

- migration/backward compatibility matrix;
- restart/recovery and projection rebuild;
- concurrency/CAS/authority races;
- corrupt/incompatible input behaviour;
- CLI installed-binary UAT;
- dependency/architecture lints blocking;
- no secrets or uncontrolled telemetry in receipts;
- docs and release process point to one normative architecture state.

**Artifact:** `PRODUCTION-READINESS-RECEIPT.md` with profile `BASE`.

At this point SDDK may be used in production without enhanced providers or agentic hosts.

---

# Agentic Workspace / JCode track

Detailed architecture: `08-AGENTIC-WORKSPACE-JCODE-ARCHITECTURE.md`  
Detailed milestones: `09-AGENTIC-WORKSPACE-ROADMAP.md`  
UAT: `10-AGENTIC-WORKSPACE-UAT.md`

## J0 — External JCode SDK boundary proof — P1

Pin current JCode integration basis by exact Git revision and prove from an external Rust workspace:

- shared runtime connect;
- list/attach/create session;
- `global_events` / `turn_done`;
- no-reply context delivery;
- `run_structured`;
- capability discovery;
- publication/package constraints.

Current verified basis on 2026-09-14:

```text
JCode 0.84.0
jcode-sdk 0.1.0
Harness API major 1
rev 752df77d3c13fa7a648eda257dbfcb8d3ea5974d
```

**Exit:** external spike receipt; no JCode internal imports.

## J1 — SDDK Agentic Workspace public API/SDK — P1

Define host-neutral SDDK contracts for SessionBinding, capabilities, workspace identity/locality, context bootstrap/deltas, normalized host events/actions, structured work and receipts.

Preferred boundary:

```text
sddk-agentic-api
sddk-agentic-sdk
```

Crate split is justified here by a public external integration/release boundary, not by bounded-context diagram aesthetics.

**Exit:** fake-host/client/server tests + no-JCode-type-leak fitness.

## J2 — `sddk-jcode` anti-corruption adapter — P1

Separate Rust integration parcel:

```text
jcode-sdk -> sddk-jcode -> sddk-agentic-sdk
```

No JCode internals and no SDDK internals.

## J3 — Activation + SessionBinding — P1

Attach shared host sessions to PROJECT / WORK_ITEM / RUN / TASK / EPHEMERAL bindings without assuming Session=Run. Preserve host transcript ownership and workspace locality.

## J4 — ContextBridge + KMT relevance — P1

One bootstrap context per basis, then `ContextDelta` only when relevant. Alignment remains advisory; irrelevant KMT updates create no host-message churn.

## J5 — Reactive Verify loop — P1

```text
JCode global_events/turn_done
 -> normalize/materiality
 -> coalesced WorkspaceChangeSet
 -> KMT
 -> VerifyPreflight/Verify
 -> optional CogniCode/Chronos deepening
 -> Knowledge/Alignment
 -> useful ContextDelta
```

Routine read/grep/list events remain ephemeral. `ledger watch` and `diff-watch` are observability/operator tools, not host transport.

## J6 — Structured work + `JCODE_CORE_GA` — P1

`AgentWorkRequest -> run_structured(schema) -> ContributionV2 -> Synthesis/Evidence/receipt`.

Core GA does **not** require MCP, providers, soft interruption, model routing, rewind or SSH. Unsupported optional host capabilities must be reported truthfully.

## J7 — MCP pull surface — P3 optional

Only after empirical J4-J6 usage demonstrates agent-initiated pull needs that native SDK push/context cannot satisfy. Keep initial surface to 3–5 semantic operations.

## J8 — Advanced JCode capabilities — P2

Safe interruption, permission mediation, model/reasoning selection, compaction/rewind, SSH/remote and richer native collaboration. SDDK MUST NOT create a second swarm orchestrator.

## J9 — Second-host validation — P2 before Agentic API 1.0

Use fake host + thin OpenCode/Claude Code spike to prove the generic API models SDDK semantics rather than JCode implementation details.

**Exit:** `MULTI_HOST_PORTABILITY_RECEIPT`; only then consider `AGENTIC_API_STABLE`/1.0.

---

## A6 — R7: CogniCode `STATIC_ENHANCED` — P1 parallel after A5

Follow `04-COGNICODE-HANDOFF.md`.

SDDK work:

- freeze `CodeIntelligencePort` ADTs;
- implement provider registry/capability negotiation/lifecycle;
- add CogniCode adapter in gateway/extension infrastructure;
- map provider observations to Evidence/Knowledge without provider types crossing inward;
- use the provider selectively for D1/D2/D3 deepening and impact/static evidence.

CogniCode work:

- versioned protocol + service surface;
- analysis basis/capability snapshot;
- stable result refs/digests;
- delta/scope/impact/summary/graph-slice operations required by the agreed contract;
- deterministic integration fixtures.

**Exit:** Base stays green with CogniCode absent; enhanced UAT is green with a pinned compatible CogniCode build.

**Artifact:** profile receipt `STATIC_ENHANCED`.

J5 MAY consume this evidence when available, but JCode Core GA MUST pass without it.

## A7 — R8: Chronos `RUNTIME_ENHANCED` — P1 parallel after A5

Follow `05-CHRONOS-HANDOFF.md`.

SDDK work:

- freeze `RuntimeIntelligencePort` ADTs;
- implement Chronos adapter;
- normalize runtime observations to Evidence refs/fingerprints;
- integrate selected scenario evidence into Knowledge/Alignment/Verify without making runtime evidence universally mandatory.

Chronos work:

- versioned scenario/runtime protocol;
- scenario basis and capability snapshot;
- observe/run/analyze/compare/summary contract;
- cancellation, timeout and resource limits;
- stable trace/evidence refs and golden scenarios.

**Exit:** Base remains green with Chronos absent; runtime-enhanced UAT passes against a pinned compatible Chronos build.

**Artifact:** profile receipt `RUNTIME_ENHANCED`.

J5 MAY consume runtime evidence when relevant/available; host integration is still Base-capable without it.

## A8 — R9/R10 + Full Enhanced convergence — P2

Finish operator/product-level convergence:

- Workbooks/control-tower projections with provenance;
- Knowledge Health and static/runtime views;
- contradiction-preserving cross-provider reconciliation;
- Governance ratchets consume explicit contracts/evidence, not Alignment opinion;
- WHY/WHY-NOT exposes decision/knowledge/alignment provenance;
- waivers have owner + expiry/revisit trigger.

Then run CogniCode + Chronos together and prove that:

- evidence can corroborate or contradict without authority inversion;
- provider degradation cannot corrupt Base state;
- both providers may be restarted/upgraded independently;
- all claimed capabilities are reflected by runtime negotiation, not guessed from version strings.

**Artifact:** profile receipt `FULLY_ENHANCED` plus final proposal-disposition reconciliation.

## R11 — deliberately outside the critical path — P3

Evaluate bounded-context crate splits only after A5/A8 measurements exist. Splitting `knowledge`, `alignment` or `verification` is not a production-readiness requirement. A split needs evidence from dependency pressure, compile cost, release cadence or change coupling and MUST preserve temporary facade paths during migration.

The public Agentic API/SDK is a different kind of boundary: a separate externally consumed release interface may justify dedicated package/crate boundaries even when internal BC crate splitting remains deferred.

## What may be parallelized

After A2/A3:

- J0 external JCode spike;
- early J1 contract/fake-host work once A3 types are stable enough;
- CogniCode protocol spike;
- Chronos protocol spike;
- provider fixture servers/clients against copied contract schemas.

After A5:

- J2→J6, A6 and A7 SHOULD proceed as three independent P1 tracks;
- J5 consumes provider evidence only through stable provider ports and only when negotiated/required;
- no track may introduce external SDK types into SDDK semantic domains to save time.

A3/A4 remain semantic authority: external integrations adapt to them, not the reverse.
