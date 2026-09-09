# Migration and deprecation plan

## Policy

Use a strangler pattern. A legacy type/command is first classified, then wrapped by a compatibility adapter, then new writes stop, then reads migrate/rebuild, then the symbol is removed.

Deprecation phases:

1. **DISCOVERED** — overlap documented.
2. **FROZEN** — no new features on legacy model.
3. **ADAPTER** — replacement available with parity tests.
4. **READ-ONLY** — no new canonical writes.
5. **REMOVED** — compatibility window closed.

## Proposed disposition matrix

| Current concept | Disposition | Replacement/role |
|---|---|---|
| `GraphProjection/GraphState` | EVOLVE | `SemanticGraphProjection` |
| `ActiveGraphProjection` | DEPRECATE as authority | typed SemanticGraph view/compat adapter |
| Vault `petgraph` graph | KEEP internal | Vault search/wiki projection only |
| `EvidenceBundle` | KEEP/STABILIZE | universal evidence model |
| `PlanningEvidenceKind` | DEPRECATE | evidence metadata/relation |
| `EvidenceAttachmentV1` | MIGRATE | EvidenceRef + WorkItem relation/fact |
| `AgentResult` | DEPRECATE | ExecutionOutcome + Contribution |
| `AgentContributionEnvelope` | EVOLVE | Contribution contract |
| `OrchestrationSynthesisReceipt` | KEEP/STABILIZE | SynthesisReceipt with dispositions/dissent |
| `FrontierProjection` | RENAME/EVOLVE | ExecutionFrontier |
| `ContinuationCandidate` | RENAME | ContinuationOption |
| `ResumeView` | KEEP as view | composes execution/advisory frontier + memory |
| `CycleStatus::UatWaiting` | DEPRECATE | run/gate facts + derived cycle summary |
| `CycleStatus::ApprovalPending` | DEPRECATE | run/authority facts + derived summary |
| `CycleStatus::Recovering` | DEPRECATE runtime meaning | Run recovery state |
| `CycleStatus::Remediating` | REVIEW | derived delivery/run summary or WorkItem |
| Spine “single source of truth” claim | REMOVE | desired planning manifest |
| Planning Ledger WorkItem | KEEP | operational planning authority |
| `PlanRevisionLineage` | ADAPT | common Revision substrate semantics |
| `ForkRecord` | KEEP domain semantics | reuse common object/ref primitives where useful |
| Decision Memory Git-like custom primitives | DO NOT duplicate | common Revision substrate |
| Permission/Gate/Approval/Risk engines | INTERNALIZE | one AuthorityEngine application facade |
| `metrics`/`analytics`/`telemetry` CLI | CONSOLIDATE UX | `observe`/doctor/status; plumbing retained |
| `graph why` / `stale` / future why variants | CONSOLIDATE UX | `sddk why`, `impact` profiles |
| raw `git`, `capability`, `ledger`, `artifact` | KEEP PLUMBING | low-level automation/debug surface |

## Data migration principles

- prefer rebuilding projections from canonical facts over row-by-row migrations;
- never rewrite historical evidence bytes merely to match a new model;
- maintain typed legacy decoders at read boundaries for the compatibility window;
- migrations are idempotent and dry-runnable;
- every destructive removal has an export/recovery test.

## SemVer strategy

During 1.x: additive replacement + deprecation warnings where feasible.  
At the chosen breaking release: remove expired APIs/commands together, with a generated migration report.


## Agent Experience migration additions

| Current pattern | Target | Migration policy |
|---|---|---|
| monolithic provider prompt | AgentProfile + Skill/Task/Instruction sources | SPLIT/MIGRATE |
| handwritten CLI cheat sheet | generated AgentCommandSurface | DEPRECATE then REMOVE |
| examples copied into prompts/docs | CommandRegistry examples | ABSORB |
| agent invokes `--help` to discover normal syntax | injected contextual command contract | retain help only as recovery |
| skill implies tool/write permission | Skill + separate Capability admission | BREAK semantic coupling |
| prompt encodes workflow state transition | Target/Task + Runtime | ABSORB into typed runtime |
| prompt encodes approval/risk rule | Policy/Authority | ABSORB into Authority |
| provider-specific role definition | AgentProfile + ProviderAdapter | MIGRATE |
| raw transcript as durable context | Decision Memory + ContextCapsule | DEPRECATE as authority |

Agent-facing compatibility must be time-bounded. A compatibility renderer may reproduce a legacy prompt shape from new contracts during migration, but new code must not add semantics to the legacy format.
