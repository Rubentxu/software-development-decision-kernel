# ADR-040 — Durable and prioritized technical-debt remediation

**Status:** Proposed

## Relationship to ADR-0047

ADR-0047 in the current-runtime ADR registry describes the legacy cycle/vault
bridge. This ADR governs new Decision Kernel code. Where the proposals differ,
ADR-040 prevails: the Event Ledger is authority, debt semantics remain
SDD-pack-owned, and Markdown is a projection. ADR-0047 remains useful only as
migration context until Workflow Runtime v2 and the projection path replace that
bridge.

## Context

The SDD debt-verification capability can produce a report for one change, but a
report alone does not guarantee that non-blocking debt survives across workflow
runs or that future planning considers it. Prompt-only policy can describe a
gate while leaving outcome calculation, priority and lifecycle interpretation to
an LLM.

The target architecture already establishes constraints that this decision must
respect:

- the Event Ledger is operational authority (ADR-021);
- the Active Graph is a rebuildable projection, never a second authority
  (ADR-022);
- governed effects use policy, approval, verification and receipts (ADR-031);
- new application services use focused ports, not the aggregate `Ledger`
  (ADR-032);
- SDD-specific semantics belong to the SDD pack, not to the generic kernel
  (ADR-034);
- verification converges through bounded workflow loops (ADR-039).

Severity and priority are different concepts. Severity measures intrinsic harm;
priority controls when remediation must displace other work.

## Decision

### 1. Keep the debt model in the SDD pack

The SDD pack owns `DebtReportV2`, `DebtVerdict`, `DebtFinding`,
`DebtIncidenceProposal`, `DebtPriorityPolicy` and their versioned schemas. The
generic kernel owns only reusable concepts: artifacts, events, workflow inputs,
policy evaluations, approvals and receipts.

The deterministic Rust implementation is exposed as built-in pack capabilities:

```text
sdd.debt.validate
sdd.debt.evaluate
sdd.debt.project
sdd.debt.prioritize
sdd.debt.plan
```

Agents may produce findings and evidence. They do not decide the final verdict,
mutate incidence state or calculate scheduling priority.

### 2. Use one authority and two projections

`debt-report.json` is immutable evidence stored in CAS. Canonical debt lifecycle
changes are append-only `debt.*` events in the Event Ledger. Two rebuildable
projections consume those events:

- an Active Graph projection for queue, dependency, scope and `sddk why` queries;
- an optional generated `INC-NNN-{slug}.md` view for human navigation.

The Markdown view is not a writable repository and cannot override events. This
avoids a ledger-vault dual-write saga. Projection checkpoints and event IDs make
retries idempotent; rebuilding projections never replays effects.

An incidence has a human ID allocated atomically in the ledger and one canonical
fingerprint. Fingerprints correlate equivalent observations; aliases preserve
identity when normalisation rules change. They never replace the human ID.

### 3. Derive the gate outcome in Rust

`DebtReportV2` binds:

- workflow/cycle ID and pack/schema version;
- base/head source revisions and normalized diff digest;
- verification artifact ID, digest and verdict;
- incidence-projection revision/digest used as baseline;
- analyzer coverage, provenance and policy version;
- normalized findings and a tagged union of incidence proposals.

The caller submits evidence, not an outcome. The evaluator validates canonical
ordering and derives one `DebtVerdict`:

```text
PASS | PASS_WITH_WARNINGS | FAIL | INCONCLUSIVE
```

The generic gate remains `passed | failed | waived` for compatibility.
`PASS`/`PASS_WITH_WARNINGS` map to `passed`; `FAIL`/`INCONCLUSIVE` map to
`failed`. The signed structured evidence preserves the exact debt verdict,
report digest, subject revision, baseline digest, policy version and evaluator
version. `waived` is not valid for the `debt-approved` release gate.

### 4. Remediate blocking debt in the same workflow run

| Verdict | Runtime behavior |
|---|---|
| `FAIL` | Return to BUILD/VERIFY through the existing bounded convergence loop. |
| `INCONCLUSIVE` | Block integration; retry the verifier within the convergence budget, then wait for human review or abort. |
| `PASS_WITH_WARNINGS` | Permit integration and append only non-blocking lifecycle proposals. |
| `PASS` | Permit integration and resolve selected debt when positive evidence proves remediation. |

No new hard-coded legacy `Phase` transition is added. Workflow Runtime v2 uses a
bounded Loop/Convergence node. The legacy A-min/A-lite/A-full bridge reuses its
existing verify remediation path until compiled to Workflow IR. A structural
replan is an ExecutionGraphRevision/Supervisor decision in the same WorkflowRun,
not a fabricated `CycleStatus` transition.

The convergence budget is explicit in the WorkflowRun policy and defaults to
three remediation attempts. Exhaustion cannot convert FAIL or INCONCLUSIVE into
accepted debt.

### 5. Model lifecycle commands as a tagged union

`DebtIncidenceProposal` is a closed tagged union so each operation carries only
valid fields:

```text
Create | Observe | Reopen | Reprioritize | Resolve | AliasFingerprint
```

Governed human commands produce separate operations:

```text
Defer | AcceptRisk | ExpireRisk | ResolveAcceptedRisk | EmergencyPlanOverride
```

Every operation includes an operation ID, workflow ID, subject revision, report
digest or human-decision receipt, actor and timestamp. Specific invariants are:

- `Resolve` requires selected debt, passing verification and a PASS report bound
  to the same subject revision;
- `Reopen` records previous/current severity and report provenance;
- `AliasFingerprint` records rule version, old/new fingerprint and reason;
- `Defer` requires a human reason, an available policy budget and the immutable
  debt plan that records the next deferral count;
- `AcceptRisk` requires owner, reason and expiry;
- expiry returns the incidence to effective-open state and cannot auto-extend;
- accepted risk may resolve before expiry through `ResolveAcceptedRisk`;
- absence from a scoped report never resolves an incidence.

### 6. Prioritize before workflow start

The Rust policy evaluator projects a stable queue from active incidences:

| Priority | Default policy |
|---|---|
| `P0` | Critical security, data-loss or invariant risk. Zero deferrals; blocks unrelated workflow start without an expiring emergency approval. |
| `P1` | High/recurrent risk. One published-workflow deferral, then required. |
| `P2` | Planned debt. Three deferrals; selected on scope overlap or due date. |
| `P3` | Minor debt. Remains visible and is reconsidered on recurrence, severity drift or related scope. |

Priority starts from severity, confidence and declared criticality. Recurrence,
age, due date, risk expiry and deferrals may elevate it. Scope overlap selects
work; it never lowers severity. Every result includes `priority_reasons` and
`policy_version`.

`sddk debt plan` stores an immutable `debt-plan` artifact containing selected,
deferred and required debt plus the projection revision/digest. Workflow start
binds that artifact as a governed input and rejects a stale baseline or exhausted
deferral budget. The legacy bridge may expose
`cycle start --debt-plan-artifact`; the target interface is generic:

```text
sddk workflow start sdd.full --input debt-plan=<artifact-id>
```

Selected debt becomes a `ChangeContract` invariant and acceptance obligation.
An unresolved selected incidence prevents `debt-approved` from passing. A
published workflow increments explicit deferrals; an abandoned run does not.
B-direct skips the debt-verification capability but still executes the P0
pre-start policy and never consumes a deferral implicitly.

### 7. Preserve artifacts until measured evidence justifies compaction

Phase 14 adds read-only inventory before deletion:

```text
sddk artifact inventory --project <id> --format json
```

Inventory reports bytes, age, class and durable references. Artifacts are
classified as durable, audit, working or disposable. Evidence reachable from
incidences, decisions, requirements, workflow runs, receipts or release
manifests is protected.

No garbage collection ships in this decision. A future compaction ADR requires
measured growth, dry-run, human approval and restore proof.

### 8. CLI hosts application services; it does not own policy

The proposed host surface is:

```text
sddk debt validate --report <file> --format json
sddk debt evaluate --run <id> --report-artifact <id> --at <RFC3339>
sddk debt queue --project <id> --scope <path> --at <RFC3339> --format json
sddk debt plan --project <id> --scope <path> [--select <INC-NNN>] [--defer <INC-NNN>=<reason>]
sddk debt accept-risk --incidence <INC-NNN> --owner <id> --reason <text> --expires-at <RFC3339> --approve
sddk debt why <INC-NNN>
sddk artifact inventory --project <id> --format json
```

The CLI opens concrete adapters and invokes application services. It does not
parse debt policy, coordinate projection writes or invent outcomes.

## Consequences

### Positive

- Technical debt survives workflow boundaries without creating a second source
  of truth.
- Gate outcomes, lifecycle and scheduling are deterministic and replayable.
- The generic kernel remains independent from SDD-specific concepts.
- Active Graph and `sddk why` explain recurrence, priority and affected scope.
- Current CLI/cycle behavior has a bounded migration bridge.

### Trade-offs and risks

- Delivery depends on Event Ledger, Workflow Runtime v2 and Active Graph seams.
- Pack schemas and event payloads require explicit versioning/migration.
- Strict P0/P1 policy may delay product work; overrides must remain visible and
  temporary.
- CAS grows until the retention inventory supports a separate compaction
  decision.

## Rejected alternatives

- **Prompt-only enforcement:** descriptive, not authoritative.
- **Mutable Markdown as authority:** duplicates the Event Ledger and makes retries
  a dual-write problem.
- **Debt types in the generic kernel:** violates pack isolation and creates an SDD
  special case.
- **Caller-provided gate outcome:** allows PASS without validating evidence.
- **Resolve by absence:** a scoped analyzer cannot prove global correction.
- **Automatic GC by age:** age does not prove an artifact is unreachable.

## Delivery sequence

1. Typed report validation, CAS binding, Rust verdict and evidence-bound receipt.
2. Canonical debt events and rebuildable incidence/graph/Markdown projections.
3. Deterministic P0-P3 queue, plan artifact and workflow-start policy.
4. Bounded same-run convergence and selected-debt acceptance enforcement.
5. Read-only artifact inventory; compaction remains a separate decision.

The Phase 8 slice prototypes typed evaluation and workflow-input policy. Phase 14
ratchets start enforcement, signed overrides and retention observability after
the preceding projections and compatibility evidence are stable.

The executable contract is SPEC-041. Roadmap dependencies and implementation
slices are tracked in `../02-roadmap/` and `../09-implementation/`.

## Revisit trigger

Revisit when any condition holds:

- more than 100 completed workflows or 5 GiB of artifacts per project;
- debt queue projection exceeds 500 ms at p95;
- incidences must be shared across project IDs;
- regulation requires fixed retention or erasure windows.
