> **ROADMAP HISTÓRICO / CONTRATO DE TRAZABILIDAD (anotación 2026-09-21).** La sección «current checkpoint» refleja el 14/09 y NO el HEAD actual. El plan ejecutable de continuación es [docs/roadmap/ROADMAP.md](../roadmap/ROADMAP.md), con [certificaciones](../roadmap/CERTIFICATIONS.md). Sus gates y especificaciones siguen siendo fuentes de requisitos, no tareas pendientes automáticas. Texto original conservado debajo.

---

# Mini-roadmap — Production Readiness, Architecture Conformance and Agentic GA

This roadmap reconciles C0→C7, R0→R11, provider integration, Agentic Workspace/JCode and the Architecture Conformance (AC) evolution.

## Current checkpoint

- **A0:** CLOSED — 7/7 production-readiness gaps closed.
- **A1:** CLOSED — C7 `PASS`, certified at `0c2ca56` / SDDK `1.169.19`.
- **A2:** current next structural milestone; behavior-preserving context cut.

## Priority model

- **P0:** blocker/release gate.
- **P1:** high product/semantic value.
- **P2:** important enhancement after preceding GA profile.
- **P3:** optional/evidence-driven.

## Global dependency map

```text
BASE semantic path

A0 authority/drift closeout             [CLOSED]
  → A1 C7 baseline receipt              [CLOSED]
  → A2 context cut + ownership          [P0]
       └─ AC0 ownership/fitness inventory
  → A3 Knowledge + Agent advisory       [P1]
       ├─ AC1 typed ArchitecturalContract
       ├─ AC2 architecture graph overlay
       └─ AC3 paradigm intent/profiles
  → A4 Alignment + Verify + DebVerify   [P1]
       ├─ AC4 architecture Verify
       ├─ AC5 architecture DebVerify
       ├─ AC6 critical mutation probes
       └─ AC7 OO/FP/ADT/DSL lenses
  → A5 BASE_PRODUCTION_READY            [P0 release gate]
       └─ AC8 SDDK self-audit receipt

Preparation after A2/A3 contracts stabilize
  ├─ J0 external JCode SDK proof
  ├─ J1 SDDK Agentic API/SDK
  ├─ CogniCode protocol spike
  └─ Chronos protocol spike

After A5
  ├─ J2→J6 JCODE_CORE_GA                [P1 default product priority]
  │    └─ AC9 reactive conformance at J5
  ├─ A6 CogniCode STATIC_ENHANCED       [P1 parallel]
  │    └─ AC10 static evidence
  └─ A7 Chronos RUNTIME_ENHANCED        [P1 parallel]
       └─ AC11 runtime evidence

Then
  ├─ A8 FULLY_ENHANCED                  [P2]
  │    ├─ AC12 conformance workbooks/time travel
  │    ├─ AC13 counterfactual architecture
  │    └─ AC14 proof-carrying changes/immune ratchets
  ├─ J8 advanced host capabilities      [P2]
  ├─ J9 second-host validation          [P2]
  ├─ J7 MCP pull surface                [P3]
  └─ R11 crate-split evaluation         [P3]
```

## A2 — R0 + R1 Context boundary cut — P0

Perform behavior-preserving MOVE→VERIFY→CONSOLIDATE→DELETE across:

`shared / planning / execution / decision / knowledge / alignment / verification / governance / agent_experience / extension`.

AC0 contributes only ownership/state/dependency inventories and fitness rules. Do **not** implement ArchitecturalContract, new Knowledge, paradigm lenses or Verify semantics early.

Required fitness includes no inward provider/host SDK types, no Alignment→Authority/InstructionCompiler dependency, no workbook canonical write and no new root context without ADR.

Exit: A1 behavior parity remains green; dependencies/ownership are machine-protected.

## A3 — R2 + R3 Knowledge + advisory foundation — P1

Implement:

- KnowledgeAssertion / KnowledgeBasis;
- KMT identity/freshness/invalidation;
- SemanticGraph cross-tree overlay;
- Software Unit cards/progressive disclosure;
- ContextCapsule `advisory_context` and separate instruction hash;
- **AC1:** typed ArchitecturalContract/ArchitectureClaim;
- **AC2:** architecture nodes/relations inside the one SemanticGraphProjection;
- **AC3:** scoped ParadigmProfile metadata for OO, Functional/Pure, ADT, DSL and extensible lenses.

Exit: deterministic graph rebuild/freshness fixtures; Alignment-like advisory payload never changes EffectiveInstructions.

## A4 — R4 + R5 + R6 Alignment, Verify, DebVerify — P1

Base intelligence loop:

```text
Sources + Decision Memory
  → Knowledge/KMT + SemanticGraph
  → Alignment
  → Verify / DebVerify
  → Evidence + receipts
```

Architecture Conformance becomes operational here:

- **AC4:** changed units → affected contracts → minimal probes → ConformanceDelta;
- **AC5:** DebVerify searches for duplicate/shadow authority, ownership gaps, bypasses, stale compatibility and contradictions;
- **AC6:** critical mutation probes prove fitness guards detect injected violations;
- **AC7:** paradigm-aware lenses evaluate OO, pure/functional, ADTs and typed DSL design relative to declared intent.

Rules:

- Alignment remains advisory;
- missing evidence → UNKNOWN/NOT_EVALUATED;
- no universal quality score;
- Verify is delta-scoped; DebVerify is global challenge, not `verify --full`;
- Base requires neither LLM, CogniCode nor Chronos.

## A5 — BASE_PRODUCTION_READY — P0 gate

Hardening includes migration/recovery/rebuild, concurrency/CAS/authority races, installed-binary UAT, blocking architecture/deprecation lints, secrets/telemetry review and release hygiene.

**AC8 becomes part of the gate:** SDDK runs its Architecture Conformance capability against itself and emits `ARCHITECTURE-CONFORMANCE-RECEIPT` at an exact revision. The self-audit must reproduce representative A0/A1 defect classes in Base mode.

Artifacts:

- `PRODUCTION-READINESS-RECEIPT.md` profile BASE;
- `ARCHITECTURE-CONFORMANCE-RECEIPT.md`.

At this point Base is production-ready without external providers/agentic hosts.

# Agentic Workspace / JCode

Detailed docs remain `08-AGENTIC-WORKSPACE-JCODE-ARCHITECTURE.md`, `09-AGENTIC-WORKSPACE-ROADMAP.md`, `10-AGENTIC-WORKSPACE-UAT.md`.

## J0/J1 — preparation — P1

J0 proves the external `jcode-sdk` boundary at a pinned revision. J1 defines host-neutral `sddk-agentic-api/sdk` contracts. J0 may begin after A2 boundaries stabilize; J1 finalizes after A3 semantic types stabilize.

## J2→J4 — ACL, SessionBinding, ContextBridge — P1 after Base

`jcode-sdk → sddk-jcode → sddk-agentic-sdk`; Session != Run; transcript ownership stays in JCode; context uses bootstrap then relevant deltas.

## J5 — Reactive Verify + AC9 — P1

```text
JCode material events / turn_done
 → coalesced WorkspaceChangeSet
 → KMT
 → affected ArchitecturalContracts
 → Verify
 → optional provider deepening
 → Knowledge/Alignment
 → ArchitectureConformanceDelta
 → useful ContextDelta
```

Routine reads/grep/list stay ephemeral. Normal architecture tensions are contextual/advisory, not automatic interrupts.

## J6 — Structured work / JCODE_CORE_GA — P1

`AgentWorkRequest → run_structured(schema) → ContributionV2 + receipts`.

JCode Core GA does not require enhanced providers, MCP or advanced host controls.

## J7/J8/J9

J7 MCP remains P3 evidence-driven. J8 adds safe interruption/permissions/model/rewind/remote capabilities. J9 validates a second host before Agentic API 1.0.

## A6 — CogniCode STATIC_ENHANCED — P1 parallel

Follow `04-COGNICODE-HANDOFF.md` and arch-spec-021.

AC10 consumes CogniCode call/dependency/impact/graph observations as Evidence through `CodeIntelligencePort`; provider types never enter Knowledge/Alignment/Verification domain. Static evidence can verify or contradict architecture claims but CogniCode never owns status.

Exit: Base stays green with provider absent; pinned enhanced UAT yields `STATIC_ENHANCED` receipt.

## A7 — Chronos RUNTIME_ENHANCED — P1 parallel

Follow `05-CHRONOS-HANDOFF.md`.

AC11 uses runtime paths, effects, traces and behavior summaries to corroborate/contradict static assumptions. Traces stay provider-side by default; SDDK stores stable refs/fingerprints/evidence.

Exit: Base green without Chronos; pinned runtime UAT yields `RUNTIME_ENHANCED` receipt.

## A8 — Full Enhanced + Architecture Intelligence — P2

Converge Workbooks/control tower, Knowledge Health, contradiction-preserving static/runtime reconciliation, Governance ratchets and WHY provenance.

AC additions:

- **AC12** authority/ownership/compatibility/paradigm topology workbooks and semantic architecture time travel;
- **AC13** counterfactual refactor planning over ephemeral candidate graph deltas;
- **AC14** proof-carrying changes and immune-system ratchets converting solved defects into durable fitness protection.

Counterfactual/proof-carrying novelty must not delay AC1..AC8 or Base readiness.

## Programming paradigm policy

The platform is poly-paradigm. OO, Functional/Pure Functional, ADTs, Typed DSL, reactive/event-driven and custom profiles are **lenses selected by project/unit intent**, never universal policy. DSL authoring syntax compiles to typed semantic AST/IR and still crosses AuthorityEngine for effects.

## ActiveGraph design transfer

SDDK adopts inspiration from event-sourced graph state, small shared vocabulary, behavior-local context, layered/optional integrations and generated behavior maps. It does **not** replace Goal/WorkItem/Workflow/Run orchestration with a no-orchestrator model.

Reference: `yoheinakajima/activegraph-packs@6639a5385518ad49f74813373c85cf96eff9adc0`.

## R11 — outside critical path — P3

Evaluate further crate splits only from measured dependency/compile/release coupling. Public Agentic API crates are a separate external release-boundary justification.

## Parallelism rule

After A2/A3: J0/J1 prep and provider protocol spikes may run in parallel. After A5: J2→J6, A6 and A7 are independent P1 tracks. A3/A4 remain semantic authority: external hosts/providers adapt to SDDK, not the reverse.
