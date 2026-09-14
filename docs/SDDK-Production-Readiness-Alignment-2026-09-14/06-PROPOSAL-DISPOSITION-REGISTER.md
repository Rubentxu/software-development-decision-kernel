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

## B. Adopted product tracks — non-blocking for Base but explicitly scheduled

These items are **not deferred ideas anymore**. They are accepted work with a named roadmap, priority and exit evidence. They do not block `BASE_PRODUCTION_READY`.

| Proposal | Disposition | Priority / trigger | Completion evidence |
|---|---|---|---|
| Agentic Workspace Integration | `ADOPTED_TRACK` | J0/J1 preparation after A2/A3 contracts stabilize; J2+ after A5 by default | `09-AGENTIC-WORKSPACE-ROADMAP.md` + AW-UAT |
| JCode first host integration via separate `sddk-jcode` Rust component | `ADOPTED_TRACK` | P1 post-Base product priority; external SDK spike J0 may start earlier | `JCODE_CORE_GA` receipt after J0-J6 |
| SDDK public Agentic Workspace API/SDK | `ADOPTED_TRACK` | J1 after A3 contract basis + J0 capability evidence | API contract/fake-host UAT + no-host-type-leak guard |
| Event-driven host reactivity (`global_events`/`turn_done` -> KMT -> Verify -> ContextDelta) | `ADOPTED_TRACK` | J5 after SessionBinding/ContextBridge | AW-UAT-040..053 |
| Typed structured host execution | `ADOPTED_TRACK` | J6 after reactive/session bridge | AW-UAT-060..063 |
| OpenCode/Claude Code second-host adapter validation | `ADOPTED_TRACK` | J9 P2 before Agentic API 1.0 stability claim | `MULTI_HOST_PORTABILITY_RECEIPT` |
| Advanced JCode permissions/interruption/model/remote capabilities | `ADOPTED_TRACK` | J8 P2 after JCode Core GA | advanced AW-UAT rows |
| MCP for host integrations | `OPTIONAL_TRACK` | J7 P3; only if native SDK push leaves demonstrated pull use-cases | per-operation empirical acceptance |

### B.1 Agentic invariants preserved by the track

- JCode Session is not SDDK Run.
- JCode owns transcripts/session/model/tool runtime; SDDK owns engineering semantics.
- `sddk-jcode` is an anti-corruption/reactive bridge, not a domain authority.
- SDDK generic Agentic API/SDK contains no JCode-specific types.
- host events are materially classified; ephemeral events do not flood the Canonical Event Log.
- `turn_done` is a preferred semantic debounce boundary, not a new lifecycle authority.
- after bootstrap, host context uses relevant `ContextDelta`, not repeated full-capsule injection.
- Alignment remains advisory and cannot become instruction/permission authority through host integration.
- safe interruption is a semantic host action; adapters map it only when capability is negotiated.
- CogniCode/Chronos enhance J5 when available; neither is required for JCode Core GA.
- existing `ledger watch`/`diff-watch` stay operator/debugging surfaces, not the host transport protocol.

## C. Adopted but explicitly later / evidence-driven

| Proposal | Disposition | Trigger |
|---|---|---|
| R11 bounded-context crate splits | `DEFERRED` | Sustained metrics prove compile/release/dependency/change-coupling benefit. |
| Advanced provider visualization/control tower polish | `DEFERRED_AFTER_BASE` | R9 after Alignment/Verify semantics are stable. |
| crates.io publication of `sddk-jcode` with normal upstream dependency graph | `DEFERRED_BY_UPSTREAM` | `jcode-sdk` dependency chain becomes registry-publishable or an explicit alternate packaging ADR is accepted. |

These rows remain accepted/known scope but are not on the default immediate path.

## D. Superseded implementation approaches

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
| Poll JCode/session state as the primary reactive integration | `SUPERSEDED` | native host event stream + materiality + semantic debounce |
| Re-send full agent context after every workspace event | `SUPERSEDED` | bootstrap once + KMT-relevant `ContextDelta` |
| Parse agent markdown as the structured orchestration contract | `SUPERSEDED` | `run_structured`/schema -> typed Contribution |

## E. Rejected invariants / anti-goals

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
| Rename/republish upstream `jcode-sdk` as `sddk-jcode-sdk` merely to bypass publication constraints | `REJECTED_BY_DEFAULT` | creates avoidable fork/sync/security/API maintenance burden; requires explicit ADR if ever needed |
| JCode host events copied one-for-one into Canonical Event Log | `REJECTED` | event volume is not semantic authority; violates materiality/state ownership |
| JCode permission UX replaced wholesale by SDDK approvals | `REJECTED` | creates double authority/poor UX; mediation must be explicit and scoped |
| Software Alignment tension automatically triggers host interruption | `REJECTED` | advisory analysis is not execution authority |
| MCP as mandatory/primary host integration despite native SDK | `REJECTED` | loses event/native capabilities and drives lowest-common-denominator integration |

## F. Disposition rule for future reconciliation

Any accepted proposal found in historical packages must end in exactly one of:

```text
IMPLEMENTED    -> evidence/receipt
ADOPTED        -> roadmap item + owner + exit
ADOPTED_TRACK  -> named track + priority + UAT/GA receipt
DEFERRED       -> reason + objective trigger
SUPERSEDED     -> replacement identifier
REJECTED       -> rationale + decision reference
```

`forgotten`, `implicitly obsolete`, `not mentioned in the latest roadmap`, and `implemented differently` are not valid dispositions by themselves.

Before final production certification, perform one final sweep of historical/current packages and append any important proposal not represented here. The goal is not to preserve every experiment; it is to prove that no important, non-discarded architectural requirement vanished silently.
