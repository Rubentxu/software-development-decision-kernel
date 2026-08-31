# DRAFT-ADR-B — Gate classification: security / process / mixed

> **Status**: DRAFT (not accepted). Awaiting cycle-52+ adoption.
> **Date**: 2026-08-31
> **Author**: deep-research-orchestrator (read-only research)
> **Cycle binding**: none yet; candidate cycle-52
> **Amends**: none (additive)
> **Supersedes**: none
> **Authority target**: `workflow/workflow.yaml` + `crates/sddk-cli/src/cycle.rs`

---

## Pre-decision: context, decisions, alternatives, compatibility, authority, migration

### Context

The framework's gate semantics today (`crates/sddk-cli/src/cycle.rs:305-315`,
`GateOutcomeArg::{Passed, Failed, Waived}`) treats all gates uniformly
with `Failed` as the default — i.e. the framework fails closed. But it
does not distinguish a `process` gate failure (test defect, missing
artifact) from a `security` gate failure (release-receipt forgery,
secret in tracked file). Both produce the same UX: a blocked cycle.

The Wave plan §Wave 1.5 enumerates 5 budget gates (`tests-pass`,
`policy-compliant`, `debt-severity-assigned`, `debt-priority-assigned`,
`bounded-execution`) without classification. The 4 `phase.*` gates
(review-approved, etc.) are also unclassified. Drift is structural.

### Decision (proposed)

Add a `class` field to each gate definition in `workflow/workflow.yaml`:

```yaml
- id: tests-pass
  class: process         # security | process | mixed
  recoverable: true      # whether a recovery action exists
  recovery_action: retry # retry | replan | waive | escalate
```

Default mapping (proposed, requires cycle-52 acceptance):

| Gate | Class | Recoverable | Recovery |
|---|---|---|---|
| `tests-pass` | process | yes | `retry` |
| `policy-compliant` | process | yes | `waive` (owner; 30d) |
| `debt-severity-assigned` | process | yes | `retry` |
| `debt-priority-assigned` | process | yes | `retry` |
| `bounded-execution` | mixed | partial | `retry` (process) / `escalate` (security) |
| `review-approved` | (orphan — see ADR-E) | n/a | n/a |
| `archive-manifest` integrity | security | no | (block; require re-archive) |
| `release-receipt` integrity | security | no | (block; require re-release) |
| `repair-receipt-hash` | process | yes | `retry` |
| `lock-lease-valid` | security | no | (block; require re-acquire) |

### Alternatives considered

| # | Alternative | Why rejected |
|---|---|---|
| B1 | Hard-code classification in the orchestrator | Violates ADR-0047 ("the priority considers... overrides require owner + justification + caducity") |
| B2 | Classify by file extension (yaml vs json) | Insufficient; semantics matter more than format |
| B3 | LLM-classifies at runtime | Shifting the Burden; no canonical source of truth |
| B4 | Adopt Severity taxonomy (`docs/debt/SEVERITY.md`) directly | Severity is intrinsic-impact; class here is recoverability — different axis |

### Compatibility with current ledger

- **Gate descriptor files are additive**. New `docs/gates/<gate>.yml` (one
  per gate) coexist with existing inline definitions.
- **`gate_receipt` event is unchanged** (carries `outcome` + `evidence`).
  A new `gate_descriptor` event (or registry file at
  `$XDG_DATA_HOME/sddk/gates/<gate>.json`) carries the class.
- **Backward compatibility**: gates without a `class` annotation default
  to `process`. This is the **safer default** — current behavior is
  permissive (any failure blocks); the new behavior keeps that until
  each gate is explicitly classified.
- **No ledger digest change** in cycle-52 implementation (events
  unchanged).

### Authority limits

- **Re-classifying a gate from `process` to `security`** requires an ADR
  amendment.
- **Re-classifying from `security` to `process` is FORBIDDEN** — security
  classification is **monotone-up**. This prevents accidental de-escalation.
- **`Mixed` class requires an explicit `recovery_action` per branch** (the
  gate definition enumerates which outcome triggers which branch).
- **Waiver authority is closed-set**: `Owner | Lead | Security`. Per
  ADR-0047, waiver requires owner + justification + caducity.

### Migration path

1. **Phase 1 (this research)**: ADR-B drafted; blueprint drafted.
2. **Phase 2 (cycle-52 candidate, A-min)**: implement gate descriptor
   registry (`docs/gates/*.yml`); orchestrator reads `class` BEFORE
   applying gate. RED test first.
3. **Phase 3 (cycle-53+)**: wire to recovery-action contract (ADR-G) —
   process gates emit `recover-forward <command>`; security gates emit
   `fail-closed <reason>`.
4. **Phase 4 (cycle-54+)**: integrate with `sddk run` facade verb (Wave
   4 completion).

### Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Default-class drift (LLM misclassifies) | medium | medium | Class is loaded from registry, not LLM |
| Recovery infinite-loop | medium | medium | Recovery has bounded retry count + escalation |
| Security class downgrade attempt | low | critical | ADR amendment required; monotone-up rule |
| Gate annotation forgotten (silent default) | medium | low | Lint rule: `lint.rs` requires `class` on every gate definition |

---

## Post-decision: formal sections (for adoption)

### Status

(pending acceptance)

### Date

(pending acceptance)

### Consequences

(pending)

### Implementation notes

- New crate: `sddk-gate-classifier` (or co-locate in
  `crates/sddk-domain/src/gate.rs`).
- New file: `docs/gates/<gate>.yml` per gate (10+ files).
- New lint rule in `crates/sddk-cli/src/lint.rs`.

### Compatibility / migration

See Phase 1–4 above.

### Revisit trigger

Revisit when:

- A new gate class emerges (e.g., `audit` for compliance audits).
- The Wave plan §Wave 4 facade introduces new gate categories.

### Implementation trace

- **cycle-52** (target): implements gate classification. Refer to
  `research/cycle-supersede-replan/blueprints/gate-classification.yml`
  and `research/cycle-supersede-replan/evidence-cards/ec-css-002-gate-classification.yml`.

---

## References

- `docs/debt/SEVERITY.md` (severity taxonomy)
- `docs/debt/PRIORITY.md` (priority taxonomy)
- `docs/adr/ADR-0047-durable-debt-remediation.md` (override discipline)
- `crates/sddk-cli/src/cycle.rs:305-315` (GateOutcomeArg)
- `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md` §Wave 1.5 (5 budget gates)
- `crates/sddk-vault/src/repair.rs:16` (VAULT003 allow-list — precedent for closed-set classification)
- `docs/research/sddk-a-full-lifecycle-review-phase-research-report.md` (orphan review)