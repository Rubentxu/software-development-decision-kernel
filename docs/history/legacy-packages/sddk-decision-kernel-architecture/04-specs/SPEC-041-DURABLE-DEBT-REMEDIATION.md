# SPEC-041 — Durable technical-debt remediation

**Status:** Proposed

## Goal

Make technical-debt verification, lifecycle and future-work selection
deterministic, replayable and enforceable through the Rust application layer and
CLI while keeping SDD semantics outside the generic kernel.

## Authority and projections

1. The CAS copy of `DebtReportV2` is immutable evidence.
2. Canonical `debt.*` events in the Event Ledger are lifecycle authority.
3. Debt queue, Active Graph nodes and `INC-NNN` Markdown are rebuildable
   projections.
4. A projection failure never rolls back or duplicates an accepted event.

## DebtReportV2

Required bindings:

- schema and SDD-pack versions;
- workflow/cycle identity;
- base/head source revisions and normalized diff digest;
- verification artifact identity, digest and verdict;
- incidence-projection revision/digest;
- analyzer IDs, versions, coverage status and evidence;
- policy/evaluator versions;
- canonically ordered findings and tagged proposals.

The validator MUST reject unknown breaking versions, non-canonical ordering,
duplicate operation IDs, malformed fingerprints, stale subject revisions,
missing required analyzer coverage and artifact digest mismatch.

## Verdict derivation

The Rust evaluator MUST derive:

```text
PASS | PASS_WITH_WARNINGS | FAIL | INCONCLUSIVE
```

The caller MUST NOT provide the outcome. Required analyzer failure, timeout,
malformed output or missing coverage yields INCONCLUSIVE. Blocking findings yield
FAIL. Non-blocking follow-up yields PASS_WITH_WARNINGS. The generic gate receipt
maps the verdict to passed/failed while signing structured evidence that retains
the exact verdict.

`debt-approved` does not permit a waiver. FAIL and INCONCLUSIVE block
integration. INCONCLUSIVE may retry inside the same WorkflowRun up to the
declared convergence budget; exhaustion waits for human review or aborts.

## Incidence lifecycle

States:

```text
open | accepted_risk | resolved
```

Agent-derived tagged operations:

```text
Create | Observe | Reopen | Reprioritize | Resolve | AliasFingerprint
```

Governed human operations:

```text
Defer | AcceptRisk | ExpireRisk | ResolveAcceptedRisk | EmergencyPlanOverride
```

All operations MUST be idempotent by operation ID and causally linked to a report
artifact or human-decision receipt. `Defer` requires a human reason, available
policy budget and a debt-plan artifact that records the next count. Expired accepted risk is effective-open.
Resolution requires positive evidence on the same subject revision. Absence from
a report MUST NOT resolve debt.

## Priority policy

Severity is intrinsic harm. Priority is scheduling policy.

- P0: zero deferrals; blocks unrelated start without expiring emergency approval.
- P1: one published-run deferral by default.
- P2: three published-run deferrals by default; scope/due-date selected.
- P3: visible without a hard default limit.

Priority MUST be a stable function of policy version, severity, confidence,
criticality, recurrence, age, due date, accepted-risk expiry and deferral count.
The output MUST include ordered `priority_reasons` and `policy_version`. Project
policy may be stricter than the defaults.

## Debt plan

`sddk debt plan` stores a CAS artifact containing:

```yaml
schema_version: 1
project_id: string
projection_revision: integer
projection_digest: sha256:...
scope: [path]
selected_debt: [INC-NNN]
deferred_debt:
  - incidence_id: INC-NNN
    reason: string
    next_deferral_count: integer
required_now: [INC-NNN]
policy_version: string
```

Workflow start MUST reject a stale projection baseline, required debt omitted
from selected work, exhausted deferral budgets and an unapproved P0 override.
Selected debt MUST become a ChangeContract invariant and MUST be resolved before
`debt-approved` passes.

## Event payloads

The initial event set is:

```text
debt.report.accepted
debt.incidence.created
debt.incidence.observed
debt.incidence.reprioritized
debt.incidence.deferred
debt.risk.accepted
debt.risk.expired
debt.incidence.resolved
debt.incidence.reopened
debt.fingerprint.aliased
debt.plan.created
debt.plan.overridden
```

Payload schemas MUST version independently and include operation ID, project ID,
workflow ID, subject revision, actor, occurred-at timestamp and evidence refs.

## Focused application ports

New use cases may depend only on focused generic ports:

```text
ArtifactReader / ArtifactWriter
EventAppender / EventReader
ProjectionReader
PolicyEvaluator
ApprovalPort
ReceiptStore
Clock
```

No use case receives the aggregate `Ledger`. Projection writers subscribe to
events and are not called as a second mutation from the CLI.

## CLI requirements

Commands and machine-readable outputs are specified in `../07-reference/CLI.md`.
Every mutating command supports an idempotency key, emits canonical events and
returns a receipt or artifact reference. `--at` or `Clock` supplies time; domain
rules do not read the system clock directly.

## Retention

`sddk artifact inventory` is read-only and reports size, age, class and durable
references. Preservation is the default. No delete/compact command is included
in this specification.

## Acceptance scenarios

1. A three-run fixture creates, defers, selects and resolves one incidence.
2. A resolved fingerprint reappears at higher severity and reopens the same ID.
3. A fingerprint rule migration aliases old/new values without duplicate nodes.
4. Expired accepted risk returns to the queue without automatic extension.
5. A P0 incidence blocks workflow start; an expiring governed approval permits
   one declared plan and leaves an auditable receipt.
6. An interrupted projector rebuilds from its checkpoint without duplicate
   incidence observations.
7. A stale debt-plan projection digest is rejected at workflow start.
8. A selected unresolved incidence forces `debt-approved` to fail.
9. Missing analyzer coverage yields INCONCLUSIVE and never accepted debt.
10. Replaying the same ledger produces byte-equivalent queue JSON after canonical
    serialization.
