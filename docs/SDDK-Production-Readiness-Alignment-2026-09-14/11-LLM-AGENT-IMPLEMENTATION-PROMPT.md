# LLM Agent Implementation Prompt

**Execution status:** `READY_FOR_IMPLEMENTATION`  
**Scope:** SDDK convergence A0→A8 + Agentic Workspace/JCode J0→J9  
**Primary rule:** implement the accepted roadmap in dependency order; do not invent a competing architecture.

Use the prompt below as the operating instruction for an implementation agent.

---

## Master prompt

You are implementing the accepted SDDK architecture and roadmap in `Rubentxu/software-development-decision-kernel`.

Your job is not to redesign SDDK from scratch. Your job is to inspect the current code, determine the first unfinished requirement in the accepted execution order, implement it with the smallest coherent change, prove it with executable evidence, update the relevant receipts/statuses, and then continue to the next eligible work item.

### 1. Read these sources before changing code

Read and obey, in this order:

1. repository `AGENTS.md`;
2. `docs/architecture/README.md`;
3. `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/`;
4. `docs/SDDK-Context-First-Semantic-Core-Agent-Experience-Software-Alignment-2026-09-10/08-BASELINE-CONFORMANCE-09-09/`;
5. `docs/SDDK-Production-Readiness-Alignment-2026-09-14/README.md`;
6. `01-GAP-AND-DRIFT-REGISTER.md`;
7. `02-MINI-ROADMAP.md`;
8. `03-PRODUCTION-READY-GATE.md`;
9. `06-PROPOSAL-DISPOSITION-REGISTER.md`;
10. `07-UAT-EVIDENCE-MATRIX.md`;
11. for Agentic/JCode work also read `08-AGENTIC-WORKSPACE-JCODE-ARCHITECTURE.md`, `09-AGENTIC-WORKSPACE-ROADMAP.md`, `10-AGENTIC-WORKSPACE-UAT.md`;
12. relevant repository-native specs in `docs/architecture/specs/arch-spec-019..031`.

Do not treat document presence or roadmap prose as proof that code is implemented. Verify code, tests, migrations and active call paths.

### 2. Execution priority

The default dependency order is:

```text
A0 -> A1 -> A2 -> A3 -> A4 -> A5 BASE_PRODUCTION_READY
```

Only after dependencies permit:

```text
J0/J1 preparation after A2/A3 contracts stabilize

After A5:
  J2 -> J6  JCODE_CORE_GA              P1 default product priority
  A6        CogniCode STATIC_ENHANCED   P1 parallel
  A7        Chronos RUNTIME_ENHANCED    P1 parallel

Then:
  J8/J9 and A8                         P2
  J7 MCP and R11 crate splits          P3/evidence-driven
```

Never skip a P0 blocker to implement a more novel P1/P2 item.

### 3. Start from code reality, not roadmap assumptions

At the start of every milestone:

1. inspect the current `main`/working revision;
2. trace the requirement from spec -> implementation -> tests -> UAT/receipt;
3. classify each relevant item as `PASS`, `PASS_WITH_COMPAT`, `PARTIAL`, `NOT_STARTED`, `SUPERSEDED` or `REJECTED`;
4. identify the smallest unresolved MUST requirement;
5. implement only what is necessary to close that requirement without adding a parallel abstraction.

If code already satisfies a requirement, do not rewrite it merely to match a diagram. Add missing evidence/fitness tests or update the crosswalk instead.

### 4. A0 is the first implementation target

Close remaining baseline convergence before R0 physical movement.

A0 includes:

- reconcile `Revision<T>` / OID / Ref / CAS primitives with Decision Memory;
- establish one SQLite schema/migration owner;
- complete AuthorityEngine cutover and prove zero governed-effect bypass;
- make Command Registry the single authoritative typed command source;
- move necessary historical decoding behind explicit compatibility surfaces;
- remove actual dead/deprecated/duplicated code and unused dependencies;
- reconcile architecture/spec status/documentation succession.

Important: semantic convergence first; bounded-context file moves later in A2. Do not mix large physical moves with semantic fixes.

### 5. Canonical-authority invariants

Preserve these invariants throughout all work:

- exactly one canonical append authority for ordered facts;
- exactly one production Evidence model;
- Cycle is not Run;
- one generic revision/OID/ref/CAS implementation wherever semantics are identical;
- one AuthorityEngine decision path for governed effects;
- SemanticGraph is a rebuildable projection, never a second source of truth;
- Vault/KnowledgeSource is source material, not runtime authority;
- Workbooks/projections cannot write canonical facts directly;
- no legacy compatibility path may silently regain production write authority.

Any surviving compatibility path must have owner, scope, fixture and objective removal trigger.

### 6. Context-first and hexagonal rules

When A2 begins, preserve behavior while moving ownership into:

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

Do not create a crate per bounded context automatically. Split crates only where an external/public boundary or measured pressure justifies it.

Enforce at least:

- no domain -> RPC/provider/host SDK;
- no Knowledge -> provider SDK;
- no Alignment -> AuthorityEngine implementation;
- no Alignment -> InstructionCompiler;
- no workbook -> canonical write;
- no host-specific JCode type in generic Agentic Workspace contracts;
- no new root-level context module after R0 without explicit ADR.

### 7. Knowledge / Alignment / Verify semantics

A3/A4 must preserve epistemic status and provenance.

`KnowledgeAssertion` must distinguish at least deterministic observation, declaration, inference, decision, verification, contradiction, staleness/supersession and unknown states according to the accepted specs.

Never promote LLM output directly to `OBSERVED` or `VERIFIED`.

Software Alignment:

> observes, compares, explains and suggests; it never orders.

Requirements:

- paradigm-neutral universal concerns + versioned lenses + project Architectural Intent;
- missing evidence => `UNKNOWN`/`NOT_APPLICABLE`, never invented green/red;
- no universal architecture-quality score;
- accepted tradeoffs/debt preserve DecisionRef/revisit semantics;
- Alignment output lives in advisory context, never EffectiveInstructions;
- changing advisory context may change ContextCapsule hash but must not change EffectiveInstructionSet hash.

Verify:

- delta/change scoped by default;
- explicit impact/staleness/EvidenceGap/deepening;
- no unconditional full repository scan.

DebVerify:

- questions the accumulated global baseline;
- may find debt/contradictions outside current Git delta;
- is not `verify --full`.

### 8. Provider integration rules

CogniCode and Chronos are optional intelligence providers, not domain authorities.

SDDK owns:

```text
CodeIntelligencePort
RuntimeIntelligencePort
provider requirement semantics
Knowledge/Alignment/Verification interpretation
```

Provider DTOs/protobuf/SDK types stop at gateway/adapter boundaries.

CogniCode owns static-analysis internals/heavy graphs/indices.
Chronos owns runtime capture/heavy traces/scenario internals.

Persist stable refs/digests and semantic evidence in SDDK rather than copying provider-heavy datasets by default.

Missing OPTIONAL/PREFERRED provider => `NOT_EVALUATED` / `EvidenceGap`, never PASS.
Missing REQUIRED provider => explicit operation failure without corrupting Base state.

Use:

- `04-COGNICODE-HANDOFF.md` for A6;
- `05-CHRONOS-HANDOFF.md` for A7.

Do not require either provider for `BASE_PRODUCTION_READY`.

### 9. Agentic Workspace / JCode rules

Agentic Workspace is a separate integration context from Agent Experience.

Core boundary:

```text
JCode runtime
  -> public jcode-sdk / Harness API
  -> separate Rust sddk-jcode ACL/reactive bridge
  -> public SDDK Agentic API/SDK
  -> SDDK semantic core
```

Never import JCode internals into SDDK.
Never build one combined SDDK+JCode binary as the architecture target.

JCode Session != SDDK Run.
JCode owns transcripts/session/model/tool runtime.
SDDK owns engineering semantics, Decisions, Knowledge, Alignment, Verify, Authority and receipts.

Current J0 reproducibility basis:

```text
JCode product: 0.84.0
JCode revision: 752df77d3c13fa7a648eda257dbfcb8d3ea5974d
jcode-sdk: 0.1.0
Harness API major: 1
jcode-sdk publish: false
```

J0 must prove public SDK sufficiency from an external Rust workspace using exact Git revision.

### 10. Reactive integration rules

For J5 use host-native events, not polling as the primary transport.

```text
host global_events / turn_done
 -> EventNormalizer
 -> MaterialityClassifier
 -> coalesced WorkspaceChangeSet
 -> KMT
 -> VerifyPreflight / Verify
 -> optional CogniCode/Chronos deepening
 -> Knowledge / Alignment
 -> useful ContextDelta
 -> host no-reply context delivery
```

Routine read/search/list/tool-start noise is EPHEMERAL/TELEMETRY and must not flood Canonical Event Log.

Use `turn_done` as a normal coalescing boundary, not as a new lifecycle authority.

`ledger watch` and `diff-watch` remain operator/debug surfaces; they are not the host protocol.

Context behavior:

- bootstrap once for a new binding/basis;
- then relevant `ContextDelta` only;
- irrelevant KMT changes cause no host-message churn;
- duplicate/reordered delivery is idempotent or deterministically rejected;
- normal Alignment tension does not automatically interrupt the agent.

### 11. Structured work

Prefer typed structured execution:

```text
AgentWorkRequest
 -> run_structured(return_schema)
 -> schema-valid ContributionV2
 -> Synthesis / Evidence / AgentExecutionReceipt
```

Do not make free-form Markdown parsing the contract.
Failed schema validation must stay visible as incomplete/rejected; never fabricate a Contribution.

### 12. Permission/interruption rules

Avoid double approval.

Default mediation:

- no relevant SDDK policy -> native host UX;
- advisory concern -> decorate/explain existing flow;
- explicit governed deny -> deny with attributable explanation;
- ENFORCE behavior requires explicit configured policy.

Software Alignment never participates directly in permission decisions.

`InterruptSafely` is an SDDK semantic host action. Adapter maps it to JCode `soft_interrupt` only when negotiated. Never emulate safe interruption by killing a session.

### 13. Testing protocol

Follow repository `AGENTS.md` and `prompts/sddk/change-scoped-testing.md`.

During apply:

- run the smallest justified verification batch from actual SUT impact;
- if impact cannot be justified, fail closed and document the missing relation rather than hiding uncertainty with arbitrary full-suite execution.

At milestone/verify/release gates run the required full profile and UAT/fitness tests.

A PASS requires executable evidence. A test with a similar name is not enough.

For every closed requirement record:

```yaml
requirement: "..."
commit: "..."
fixture: "..."
command_or_test: "..."
result: PASS | PASS_WITH_COMPAT
result_ref: "..."
compatibility: null | {...}
notes: "..."
```

Use the matrices in `07-UAT-EVIDENCE-MATRIX.md` and `10-AGENTIC-WORKSPACE-UAT.md`.

### 14. Quality and cleanup policy

Every milestone includes a bounded quality sweep for the touched responsibility:

- dead code;
- unused dependencies;
- deprecated APIs;
- stale compatibility shims;
- semantic duplication;
- giant modules/change-locality problems caused by the current work;
- comments/docs contradicting current authority.

Do not perform unrelated cleanup merely because you noticed it. Register it if outside the current concern.

Do not keep `#[allow(dead_code)]`, deprecated paths or staged branches without an owner, reason and exit trigger.

### 15. Proposal disposition

Never silently drop accepted scope.

If an accepted proposal should not be implemented, update `06-PROPOSAL-DISPOSITION-REGISTER.md` using exactly one of:

```text
IMPLEMENTED
ADOPTED
ADOPTED_TRACK
DEFERRED(reason, trigger)
SUPERSEDED_BY(id)
REJECTED(reason, decision-ref)
```

“not mentioned anymore” is invalid.

### 16. Commit and change discipline

Obey `AGENTS.md` repository workflow and commit conventions.

Keep work in small reviewable slices. One concern per commit/cycle. Prefer strangler migrations over flag-day rewrites when replacing an active authority.

For each new core abstraction explicitly state what it replaces, consolidates or makes unnecessary. Purely additive architecture during consolidation requires an ADR/explicit rationale.

Do not rewrite stable code only to make it resemble documentation if its behavior and ownership already satisfy the contract.

### 17. Autonomous milestone loop

For each milestone execute this loop:

```text
READ
  -> AUDIT CURRENT REALITY
  -> SELECT FIRST UNRESOLVED MUST
  -> DESIGN MINIMAL CHANGE
  -> IMPLEMENT
  -> RUN SCOPED TESTS
  -> RUN REQUIRED UAT/FITNESS
  -> CHECK LEGACY/BYPASS PATHS
  -> UPDATE CROSSWALK / GAP / RECEIPT
  -> COMMIT
  -> RE-EVALUATE DEPENDENCIES
  -> NEXT ELIGIBLE ITEM
```

Do not ask for confirmation between ordinary roadmap steps. Stop/escalate only when:

- two accepted specs materially contradict each other;
- the required change is destructive/irreversible beyond documented migration policy;
- an external dependency/API no longer supports the documented contract;
- a security/authority issue requires priority reordering;
- the next step requires a product decision not represented by an ADR/spec/disposition.

When escalating, provide the conflicting evidence and 2–3 bounded options rather than silently choosing a new architecture.

### 18. Completion claims

Never claim a milestone, profile or integration is complete because “the code exists” or “tests are green”.

Use receipts:

- A1 -> `09-09-CONFORMANCE-RECEIPT.md`;
- A5 -> `PRODUCTION-READINESS-RECEIPT.md` profile BASE;
- A6 -> STATIC_ENHANCED receipt;
- A7 -> RUNTIME_ENHANCED receipt;
- A8 -> FULLY_ENHANCED receipt;
- J0 -> `JCODE-SDK-SPIKE-RECEIPT.md`;
- J6 -> `JCODE-CORE-GA-RECEIPT.md`;
- J9 -> `MULTI-HOST-PORTABILITY-RECEIPT.md`.

Only declare a receipt when all mandatory rows are PASS/PASS_WITH_COMPAT according to the applicable gate.

### 19. Immediate first action

Start at **A0**.

Before editing, produce an implementation inventory for PR-GAP-004 through PR-GAP-010 with:

```text
requirement
current canonical implementation
legacy/parallel implementation(s)
production consumers
existing tests
missing acceptance evidence
proposed minimal slice
risk
```

Then implement the first unresolved MUST in dependency/risk order. Do not start R0 moves, CogniCode, Chronos or JCode production integration before their documented prerequisites are satisfied.

Continue until the current milestone exit is evidence-green, then advance according to `02-MINI-ROADMAP.md`.

---

## Expected agent reporting format

At the end of each implementation slice report:

```text
Milestone / requirement:
What changed:
Canonical authority after change:
Legacy/compatibility remaining:
Tests/UAT executed:
Evidence/receipt updated:
Architecture fitness result:
Known risks:
Next eligible requirement:
```

Do not report percentage complete as a substitute for evidence.
