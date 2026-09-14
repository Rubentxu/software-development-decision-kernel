# Agentic Workspace + JCode Reactive Integration

**Status:** adopted post-Base product track  
**First host:** JCode  
**Integration style:** independent Rust runtimes + public SDK/API boundaries + anti-corruption bridge  
**Base dependency:** none — Agentic Workspace is not required for `BASE_PRODUCTION_READY`

## 1. Intent

Agentic Workspace Integration connects SDDK's deterministic engineering knowledge, decisions, verification and governance to interactive coding-agent hosts without turning SDDK into an IDE/agent runtime and without forcing hosts to adopt SDDK internals.

The first implementation targets JCode. The architecture MUST remain capable of supporting a second host later without reducing JCode to a lowest-common-denominator interface.

Core rule:

> SDDK owns engineering semantics; the host owns agent/session/tool execution; the integration owns translation and reactive coordination.

## 2. Product boundary

```text
JCode runtime
    |
    | Harness API v1
    v
jcode-sdk (Rust)
    |
    v
sddk-jcode
Integration Plugin / ACL + Reactive Bridge
    |
    | SDDK Agentic Workspace public API/SDK
    v
SDDK
Knowledge + Decision + Alignment + Verification + Governance
```

The three components remain independently versioned and releasable.

### JCode owns

- sessions and transcripts;
- models/reasoning configuration;
- tools and tool execution;
- native permissions UI and host ergonomics;
- swarm/collaboration mechanics;
- conversation history, compaction and rewind;
- host-native event stream and workspace locality.

### SDDK owns

- Goals/WorkItems/Runs/Tasks and engineering intent;
- Decision Memory and KnowledgeAssertion;
- KMT and SemanticGraphProjection;
- ContextCompiler and bounded ContextCapsule semantics;
- Software Alignment assessments;
- Verify / DebVerify;
- AuthorityEngine and explicit engineering policy;
- Evidence, receipts, provenance and WHY/WHY-NOT.

### `sddk-jcode` owns

- mapping JCode SDK contracts into SDDK Agentic Workspace contracts;
- mapping SDDK semantic host actions to JCode SDK operations;
- reactive event normalization/materiality classification;
- SessionBinding maintenance;
- context bootstrap/delta delivery;
- typed structured-work result collection;
- capability negotiation and compatibility evidence.

It MUST NOT own architectural truth, debt semantics, verification truth, Alignment decisions or governance policy.

## 3. Agent Experience vs Agentic Workspace

These bounded contexts are related but distinct.

**Agent Experience** defines what the agent receives and returns:

```text
TaskContract
ContextCapsule
EffectiveInstructions
SkillSet
AgentCommandSurface
ReturnContract
AgentExecutionReceipt
```

**Agentic Workspace** defines where/how the agent is hosted:

```text
AgenticSessionDescriptor
SessionBinding
HostCapabilitySet
WorkspaceIdentity / WorkspaceLocation
ContextDelivery
HostEvent
HostAction
PermissionMediation
StructuredExecution
```

An Alignment advisory may appear in `ContextCapsule.advisory_context[]`; it MUST NOT become an instruction merely because a host can inject messages.

## 4. Public SDDK integration boundary

SDDK SHALL expose a stable Rust-facing integration API/SDK instead of requiring integration projects to import SDDK internal crates/modules.

Candidate packaging, subject to evidence against unnecessary crate proliferation:

```text
sddk-agentic-api   semantic public contracts / transport-neutral DTOs
sddk-agentic-sdk   client for SDDK integration endpoint
```

An on-demand local endpoint is preferred over a permanently required daemon. Valid implementation forms include local socket or stdio child, for example:

```text
sddk integration serve --stdio
```

The semantic API MUST remain transport-neutral.

## 5. Capability model

The generic contract SHALL negotiate semantic capabilities rather than infer host behavior from product version strings.

Initial portable capability vocabulary:

```text
PersistentSessions
ContextInjection
StructuredExecution
SafeInterruption
PermissionMediation
ModelSelection
ReasoningEffort
SessionCompaction
SessionRewind
GlobalEventStream
WorkspaceLocality
```

Hosts MAY expose namespaced native extensions. The generic API MUST NOT prevent JCode-specific power merely because another host lacks an equivalent.

Version metadata is reproducibility evidence; negotiated capability state is runtime authority.

## 6. SessionBinding

A host session is not an SDDK Run.

```text
JCode Session != SDDK Run
```

Conceptual model:

```rust
struct AgenticSessionRef {
    host: HostId,
    session_id: String,
    workspace: WorkspaceIdentity,
}

struct SessionBinding {
    session_ref: AgenticSessionRef,
    project: ProjectRef,
    goal: Option<GoalRef>,
    work_item: Option<WorkItemRef>,
    run: Option<RunRef>,
    task: Option<TaskRef>,
    agent_profile: Option<AgentProfileRef>,
    context_basis: ContextBasisRef,
    binding_mode: BindingMode,
}

enum BindingMode {
    Project,
    WorkItem,
    Run,
    Task,
    Ephemeral,
}
```

JCode retains transcript authority. SDDK persists only semantic references/bindings, receipts, Contributions, Evidence and resulting durable engineering knowledge.

## 7. Reactive event bridge

The integration is event-driven. Polling the host is not the target architecture.

```text
JCode global_events / turn_done
              |
              v
       EventNormalizer
              |
              v
     MaterialityClassifier
       /               \
 EPHEMERAL            MATERIAL
 discard/telemetry      |
                        v
               WorkspaceChangeSet
                        |
                        v
                       KMT
                        |
                        v
                VerifyPreflight
                        |
             optional deepening
              /               \
        CogniCode            Chronos
              \               /
               v             v
               Knowledge / Evidence
                        |
                        v
                    Alignment
                        |
                        v
                  ContextDelta
                        |
                        v
          JCode send_message(no_reply)
```

### Materiality policy

Routine reads/searches/listings SHALL NOT flood the Canonical Event Log.

Typical EPHEMERAL examples:

- read file;
- grep/search;
- list directory;
- routine model/tool housekeeping.

Typical MATERIAL examples:

- source/workspace mutation;
- build/test outcome relevant to current work;
- commit/change-set creation;
- permission denial that changes feasible execution;
- completed agent turn with semantic contribution;
- structured Contribution result;
- externally changed branch/worktree/session basis.

Materiality is semantic and testable, not a hardcoded list of every host event name.

## 8. Semantic debounce and KMT

Repeated edits MUST coalesce into a bounded change unit before expensive verification.

```text
foo.rs edit
foo.rs edit
foo.rs edit
    -> one WorkspaceChangeSet
    -> one incremental KMT update
    -> at most one useful ContextDelta for that semantic boundary
```

`turn_done` is the preferred natural boundary for normal interactive work. KMT relevance and freshness prevent irrelevant repository changes from invalidating every context/session.

Existing `ledger watch` and `diff-watch` remain useful operator/debugging surfaces; they MUST NOT become the host integration protocol or create a second semantic event authority.

## 9. Reactive attention levels

Host-facing updates use four semantic levels:

```text
SILENT       no user-facing interruption
CONTEXTUAL   inject/update context without demanding attention
ATTENTION    surface important advisory information
INTERRUPT    request a safe host interruption
```

Normal Software Alignment tension is normally `CONTEXTUAL` or `ATTENTION`, never automatically `INTERRUPT`.

`INTERRUPT` is reserved for explicitly material conditions such as:

- task/run cancellation;
- critical invariant contradiction affecting current execution;
- decision superseded while the agent is acting on the old basis;
- external branch/worktree replacement invalidating the active execution basis.

SDDK emits semantic action `InterruptSafely`; the JCode adapter decides how that maps to `soft_interrupt`.

## 10. Context delivery

On session activation/attachment:

```text
resolve project/workspace
 -> establish SessionBinding
 -> ContextCompiler
 -> compact SessionBootstrapContext
 -> host context injection without agent reply
```

After bootstrap, deliver only `ContextDelta` when the effective semantic basis changes materially.

A delta MUST identify its basis and why it is relevant. Re-sending a complete ContextCapsule after every edit is prohibited because it creates noise, context-cache churn and provenance ambiguity.

## 11. Structured work

Orchestrated work SHOULD prefer typed execution:

```text
AgentWorkRequest
    -> JCode run_structured(schema)
    -> schema-valid ContributionV2
    -> SDDK Synthesis / Evidence
```

Markdown scraping is not a contract.

Companion mode and orchestrated mode share the same adapter:

- **Companion:** human drives JCode; integration observes/contextualizes/verifies/advises.
- **Orchestrated:** SDDK requests typed work; adapter creates/attaches session and returns a typed Contribution.

## 12. Permissions and authority

Avoid double-approval UX.

For a JCode permission request:

- no relevant SDDK policy -> let normal JCode permission UX decide;
- explicit SDDK deny -> deny with attributable explanation;
- advisory SDDK concern -> decorate a single host approval flow;
- explicit governance enforcement -> only when configured as an authoritative policy path.

Initial modes:

```text
OBSERVE
ADVISE   # default
ENFORCE  # explicit
```

JCode remains authority for host/tool permission mechanics. SDDK AuthorityEngine remains authority for declared SDDK-governed engineering effects.

## 13. Workspace locality

The contract SHALL represent where workspace operations actually execute:

```text
Local
Ssh
Container
Remote
```

`WorkspaceIdentity` is logical; `WorkspaceLocation` is operational. JCode SSH/native remote support is consumed through the public SDK; SDDK MUST NOT guess local filesystem paths for a remote session.

## 14. Provider loop

Agentic integration does not require CogniCode or Chronos. Base flow works without either.

When enhanced providers are available:

```text
material host delta
 -> Base Verify/KMT
 -> CogniCode static deepening when risk/uncertainty justifies it
 -> Chronos runtime deepening when behavior evidence is required
 -> Knowledge reconciliation
 -> Alignment delta
 -> ContextDelta
```

Provider absence remains `NOT_EVALUATED` / EvidenceGap; it never blocks ordinary companion interaction unless the active Task/Policy explicitly requires that capability.

## 15. JCode integration basis as of 2026-09-14

The first external spike SHALL pin an exact upstream Git revision, not a moving branch/tag.

Current verified basis:

```text
JCode product version: 0.84.0
JCode master revision: 752df77d3c13fa7a648eda257dbfcb8d3ea5974d
jcode-sdk version:     0.1.0
jcode-sdk publish:     false
Harness API major:     1
```

Development dependency initially:

```toml
jcode-sdk = {
  git = "https://github.com/1jehuang/jcode.git",
  rev = "752df77d3c13fa7a648eda257dbfcb8d3ea5974d"
}
```

`Cargo.lock` SHALL be committed for executable integration projects.

Because the upstream SDK is currently unpublished, crates.io publication of a crate that normally depends on it cannot be treated as solved. The integration project may tag/release from Git immediately; crates.io publication waits for a publishable upstream dependency chain or another explicitly accepted packaging decision.

## 16. Packaging and naming

Preferred ownership:

```text
SDDK repository/product:
  sddk-agentic-api
  sddk-agentic-sdk

Separate integration project:
  sddk-jcode-adapter
  sddk-jcode-plugin
```

`sddk-jcode-runtime` is optional and MUST only exist if measured responsibility/change pressure justifies a separate crate.

Do not republish or rename upstream `jcode-sdk` as `sddk-jcode-sdk` merely for branding. A combined `sddk-jcode-sdk` is justified only if third parties eventually need a stable API to program against the combined integration itself.

## 17. Independent SemVer

Versions are independent:

```text
JCode             0.84.0
Harness API       1.x
jcode-sdk         0.1.0
sddk-agentic-api  own SemVer
sddk-agentic-sdk  own SemVer
sddk-jcode        own SemVer
```

Compatibility is recorded as a tested matrix plus capability negotiation, for example:

```text
sddk-jcode 0.3.x
SDDK integration API >=0.2,<0.3
jcode-sdk basis 0.1.0 @ pinned revision
Harness API major 1
Tested JCode: explicit revisions/releases
```

The matrix is evidence, not a substitute for runtime negotiation.

## 18. Anti-goals

The track explicitly rejects:

- importing JCode internals directly into SDDK;
- one combined SDDK+JCode binary as architecture target;
- making MCP the primary integration when a richer native SDK exists;
- using an LLM as an intermediary between host and SDDK;
- mirroring every host event into the Canonical Event Log;
- reimplementing JCode swarm/session/model orchestration inside SDDK;
- making Software Alignment a host permission authority;
- making provider availability a prerequisite for host integration;
- lowest-common-denominator host APIs that hide useful native capabilities.

## 19. Normative links

Repository-native specifications:

- `arch-spec-022-agentic-workspace-boundary.md`
- `arch-spec-023-host-capability-negotiation.md`
- `arch-spec-024-agentic-session-binding.md`
- `arch-spec-025-reactive-host-event-bridge.md`
- `arch-spec-026-context-delta-delivery.md`
- `arch-spec-027-structured-agent-work.md`
- `arch-spec-028-permission-interruption-bridge.md`
- `arch-spec-029-workspace-locality.md`
- `arch-spec-030-agentic-integration-api-sdk.md`
- `arch-spec-031-jcode-anti-corruption-layer.md`

Execution order: `09-AGENTIC-WORKSPACE-ROADMAP.md`.  
Acceptance evidence: `10-AGENTIC-WORKSPACE-UAT.md` and `07-UAT-EVIDENCE-MATRIX.md`.
