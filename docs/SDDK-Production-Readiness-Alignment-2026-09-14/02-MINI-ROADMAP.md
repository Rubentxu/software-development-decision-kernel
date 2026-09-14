# Mini-roadmap — From Current Main to Production Readiness

This roadmap does not replace C0→C7 or R0→R11. It compresses them into a finite execution sequence based on the 2026-09-14 code audit.

## Dependency line

```text
A0 Close current authority drifts
  -> A1 Baseline C7 receipt
  -> A2 Context cut + semantic ownership
  -> A3 Knowledge + Agent advisory foundation
  -> A4 Alignment + Verify + DebVerify
  -> A5 Base production certification
  -> A6 CogniCode Static Enhanced
  -> A7 Chronos Runtime Enhanced
  -> A8 Full Enhanced + program convergence
```

Provider protocol spikes may run in parallel after A2, but production provider integration must consume the stable SDDK ports delivered by A3/A4 rather than shaping the domain around provider APIs.

---

## A0 — Close the remaining 09/09 authority drifts

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

## A1 — Close C7 and freeze the baseline receipt

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

## A2 — R0 + R1: context boundary cut and semantic consolidation

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

Required fitness rules include:

- no domain → RPC/provider SDK;
- no Knowledge → provider SDK;
- no Alignment → AuthorityEngine implementation;
- no Alignment → InstructionCompiler;
- no workbook → canonical write;
- no new root-level context module after R0 without ADR.

**Exit:** behaviour parity from A1 remains green and dependency fitness tests enforce the new boundaries.

## A3 — R2 + R3: Knowledge substrate and Agent advisory boundary

Implement only the foundations required by Alignment/Verify:

- `KnowledgeAssertion` with evidence/status discipline;
- `KnowledgeBasis`;
- KMT v1 identity/freshness/invalidation;
- SemanticGraph cross-tree overlay;
- Software Unit cards/progressive disclosure;
- `ContextCapsule.advisory_context`;
- separate `context_capsule_hash` and `effective_instruction_set_hash`;
- explicit invariant that Alignment cannot become `InstructionSource`.

**Exit:** deterministic fixtures prove freshness/invalidation and an Alignment-like advisory payload changes context hash without changing instruction-set hash.

## A4 — R4 + R5 + R6: Alignment, Verify and DebVerify

Implement the intelligence loop in Base mode before external providers:

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

## A5 — Certify `BASE_PRODUCTION_READY`

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

At this point SDDK may be used in production without enhanced providers.

## A6 — R7: CogniCode `STATIC_ENHANCED`

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

## A7 — R8: Chronos `RUNTIME_ENHANCED`

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

## A8 — R9/R10 + Full Enhanced convergence

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

## R11 — deliberately outside the critical path

Evaluate crate splits only after A5/A8 measurements exist. Splitting `knowledge`, `alignment` or `verification` is not a production-readiness requirement. A split needs evidence from dependency pressure, compile cost, release cadence or change coupling and MUST preserve temporary facade paths during migration.

## What may be parallelized

After A2:

- CogniCode protocol spike and Chronos protocol spike may proceed independently;
- provider fixture servers/clients may be built against copied contract schemas;
- neither spike may introduce provider-specific types into SDDK domain to save time.

A3/A4 remain the semantic authority: provider spikes adapt to them, not the reverse.
