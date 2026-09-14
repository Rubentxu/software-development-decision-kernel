# Proposal Disposition Register

Purpose: prevent accepted ideas from disappearing during consolidation and prevent superseded/rejected ideas from returning accidentally.

This is a **goal-level register**, not a replacement for ADRs. Implementation details may evolve while the accepted goal/invariant remains stable.

## A. Adopted — required to converge the core program

| Proposal / invariant | Disposition | Completion evidence |
|---|---|---|
| One Canonical Event Log append authority | `ADOPTED` | C1 + C7 receipt |
| Universal Evidence relation model | `ADOPTED` | C1 + compatibility guard |
| Cycle is not Run | `ADOPTED` | C2 static/write guards |
| Shared revision/OID/ref/CAS substrate where semantics are identical | `ADOPTED` | C3 classification + tests |
| Decision Memory remains typed durable curated decision history | `ADOPTED` | Decision tests + common-substrate convergence |
| One AuthorityEngine for governed effects | `ADOPTED` | C4 zero-bypass evidence |
| Target → Task DAG + typed command surface | `ADOPTED` | Command registry parity + UAT |
| Single Command Registry source for help/docs/agent surface | `ADOPTED` | C5 mutation/drift test |
| Context-first bounded-context ownership | `ADOPTED` | R0 dependency fitness |
| State classes Fact/Object/Projection/Ephemeral | `ADOPTED` | R1 architecture guard |
| KnowledgeAssertion + KnowledgeBasis + KMT | `ADOPTED` | R2 fixtures |
| SemanticGraph is rebuildable projection, not authority | `ADOPTED` | rebuild UAT |
| ContextCapsule advisory/instruction separation | `ADOPTED` | R3 hash/fitness tests |
| Software Alignment is advisory, paradigm-neutral | `ADOPTED` | R4 fixtures + forbidden-edge guards |
| Alignment lenses + Architectural Intent | `ADOPTED` | R4 lens/intent UAT |
| No universal architecture quality score | `ADOPTED` invariant | static/API review + UAT |
| Verify synchronizes knowledge with a delta | `ADOPTED` | R5 no-full-scan UAT |
| DebVerify questions the global baseline | `ADOPTED` | R6 global-debt fixture |
| Provider SPI + capability negotiation | `ADOPTED` | R7/R8 contract tests |
| CogniCode as Static Enhanced provider | `ADOPTED` | Static Enhanced receipt |
| Chronos as Runtime Enhanced provider | `ADOPTED` | Runtime Enhanced receipt |
| Providers optional to Base; absence never becomes PASS | `ADOPTED` invariant | Base/no-provider + absence UAT |
| Workbooks are rebuildable projections | `ADOPTED` | R9 no-canonical-write guard |
| Governance may consume explicit evidence/contracts, not Alignment opinion | `ADOPTED` | R10 dependency/policy tests |
| WHY/WHY-NOT preserves provenance | `ADOPTED` | R10 query UAT |

## B. Adopted but explicitly later / non-blocking for Base

| Proposal | Disposition | Trigger |
|---|---|---|
| Agentic Workspace Integration | `DEFERRED` | Start after Base semantic contracts are stable enough to expose through a host integration API/SDK. |
| JCode first host integration via separate `sddk-jcode` Rust component | `DEFERRED` | Agentic Workspace API/SDK proof + pinned external `jcode-sdk` spike. |
| OpenCode/Claude Code host adapters | `DEFERRED` | Validate generic host API after JCode, not before. |
| MCP for host integrations | `DEFERRED` / secondary | Add only if empirical pull-use cases remain after native host SDK integration. |
| R11 bounded-context crate splits | `DEFERRED` | Sustained metrics prove compile/release/dependency/change-coupling benefit. |
| Advanced provider visualization/control tower polish | `DEFERRED_AFTER_BASE` | R9 after Alignment/Verify semantics are stable. |

These rows are still accepted work. They simply do not block `BASE_PRODUCTION_READY`.

## C. Superseded implementation approaches

| Previous approach | Disposition | Replacement |
|---|---|---|
| Parallel writable legacy ledger + canonical event store | `SUPERSEDED` | one canonical append authority + migration/read compatibility only |
| Runtime approval/UAT/recovery truth encoded as mutable Cycle truth | `SUPERSEDED` | Run/Authority/Evidence truth + derived Cycle summary |
| `PlanningEvidenceKind` as production evidence authority | `SUPERSEDED` | universal Evidence relations |
| Hand-maintained command examples as authority | `SUPERSEDED` | typed Command Registry generation/introspection |
| Provider integration through a generic LLM intermediary | `SUPERSEDED` | direct typed Code/Runtime Intelligence ports + adapters |
| Treat current completion/model `provider_router` as all provider integration | `SUPERSEDED` | explicit completion-provider responsibility vs engineering-intelligence Extension Platform |
| Copy provider-heavy full graphs/traces into SDDK as normal integration | `SUPERSEDED` | provider-side heavy data + stable refs/digests + selected SDDK semantic projection |
| Full SDDK architecture ZIP copied into each provider as working contract | `SUPERSEDED` | narrow versioned provider handoff bundles with source manifest |

## D. Rejected invariants / anti-goals

| Idea | Disposition | Reason |
|---|---|---|
| Alignment directly blocks releases or approves code | `REJECTED` | violates advisory boundary; Governance owns explicit policy/gates |
| One universal “architecture quality” score | `REJECTED` | hides paradigm/project intent/tradeoffs and turns unknown evidence into false precision |
| Missing provider interpreted as green/pass | `REJECTED` | epistemically unsafe; must be EvidenceGap/NOT_EVALUATED or explicit REQUIRED failure |
| Provider protobuf/SDK types in SDDK domain | `REJECTED` | reverses ownership and couples semantic core to transport/provider implementation |
| Provider writes SDDK canonical state directly | `REJECTED` | bypasses Evidence/Knowledge/Authority ownership |
| `verify --full` as implementation of DebVerify | `REJECTED` | conflates change synchronization with global baseline challenge |
| New crate per bounded context by diagram alone | `REJECTED` | split must emerge from measured pressure |
| JCode internals imported directly into SDDK | `REJECTED` | products remain independent; public SDK/API + ACL bridge only |
| One combined SDDK+JCode binary as architectural target | `REJECTED` | independent parcels/runtimes are intentional |

## E. Disposition rule for future reconciliation

Any accepted proposal found in historical packages must end in exactly one of:

```text
IMPLEMENTED -> evidence/receipt
ADOPTED     -> roadmap item + owner + exit
DEFERRED    -> reason + objective trigger
SUPERSEDED  -> replacement identifier
REJECTED    -> rationale + decision reference
```

`forgotten`, `implicitly obsolete`, `not mentioned in the latest roadmap`, and `implemented differently` are not valid dispositions by themselves.

Before final production certification, perform one final sweep of historical/current packages and append any important proposal not represented here. The goal is not to preserve every experiment; it is to prove that no important, non-discarded architectural requirement vanished silently.
