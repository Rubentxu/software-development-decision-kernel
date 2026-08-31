# DRAFT-ADR-0078 — VAULT003 per-cycle scope policy (retroactive housekeeping)

> **Status**: DRAFT (not accepted). Awaiting cycle-50+ adoption.
> **Date**: 2026-08-31
> **Author**: deep-research-orchestrator (read-only research)
> **Cycle binding**: none yet; candidate cycle-50 (housekeeping)
> **Amends**: none (this is a retroactive ADR — codifies an existing
> decision that was made in commit `87c5a97`, v1.65.6)
> **Supersedes**: none
> **Authority target**: `crates/sddk-vault/src/repair.rs`

---

## Pre-decision: context, decisions, alternatives, compatibility, authority, migration

### Context

`crates/sddk-cli/src/vault_cmd.rs:172` cites "verbatim `{VAULT003}` per
ADR-0078", but ADR-0078 **does not exist** in `docs/adr/` or
`docs/sddk-decision-kernel-architecture/03-adrs/`. The substantive
decision (allow-list = 1 entry, code = VAULT003) was made in commit
`87c5a97` (v1.65.6, `feat(vault): add VAULT003 per-cycle scope policy and
RepairReceipt queue`).

This is a **dangling reference** — the code cites an authority that does
not exist. The substance stands (`crates/sddk-vault/src/repair.rs:16`,
`pub const ALLOW_LIST: &[&str] = &["VAULT003"];`), but the authority is
unwritten.

### Decision (proposed)

Adopt this ADR retroactively, codifying the decision made in commit
`87c5a97`. This is **housekeeping**: the decision already exists; the
ADR is the documentation.

### Content of the retroactive ADR

**Decision**: VAULT003 diagnostics (broken-link) are eligible for
scoped down-classification via RepairReceipt; the allow-list is closed
and currently contains exactly `VAULT003`.

**Allow-list extension requires an ADR amendment.** Adding a new code
(e.g., VAULT004) requires:

1. New ADR or amendment.
2. Justification (why this diagnostic class can be repaired, not just
   re-classified).
3. Bounded validity (RepairReceipt.valid_to ≤ created_at + 90 days).

### Alternatives considered

| # | Alternative | Why rejected |
|---|---|---|
| 0078.A1 | Delete the reference in `vault_cmd.rs:172` | Loses the intent of citing authority; future readers will not know why VAULT003 is special |
| 0078.A2 | Make the allow-list dynamic (no ADR needed for new codes) | Reduces authority; ADR-0073-style closed-set is the project's pattern |
| 0078.A3 | Add the ADR but invalidate the existing receipt queue | No reason to invalidate; the queue is sound |

### Compatibility with current ledger

- **No ledger event change** (the allow-list is a constant, not a ledger
  event).
- **Existing RepairReceipts remain valid** (their `valid_to` is bounded;
  they expire on schedule).
- **VAULT003 / RepairReceipt (v1.65.6) are unchanged**: this ADR
  codifies, it does not modify.

### Authority limits

- **Allow-list extension requires an ADR amendment** (monotone-add).
- **Allow-list reduction is FORBIDDEN** (monotone-up — historical
  receipts remain valid).
- **Per-code authority**: each code in the allow-list may carry its own
  waiver policy (per RepairReceipt fields).

### Migration path

1. **Phase 1 (this research)**: ADR-0078 drafted.
2. **Phase 2 (cycle-50 candidate, A-min, ≤ 1/2 día)**: move this draft
   to `docs/adr/ADR-0078-vault003-scope-policy.md` with status
   `accepted` and a `retroactive` header noting it codifies commit
   `87c5a97`.

### Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Future reader thinks VAULT003 was retroactively invented | low | low | ADR header explicitly states "retroactive; codifies commit 87c5a97" |
| Allow-list extension bypasses ADR review | low | medium | Monotone-add rule + ADR amendment requirement |

---

## Post-decision: formal sections (for adoption)

### Status

(pending acceptance — **retroactive**)

### Date

(pending acceptance)

### Consequences

- The dangling reference in `vault_cmd.rs:172` becomes a valid
  citation.
- Future allow-list extensions have a documented procedure.

### Implementation notes

- One file move: `research/cycle-supersede-replan/adr-drafts/DRAFT-ADR-0078-vault003-scope-policy.md`
  → `docs/adr/ADR-0078-vault003-scope-policy.md`.
- One status update: `Status: Accepted (retroactive)`.

### Compatibility / migration

See Phase 1–2 above.

### Revisit trigger

Revisit when:

- A new code is proposed for the allow-list (extension via ADR).
- The 90-day validity window is challenged.

### Implementation trace

- **cycle-50** (target, housekeeping): publishes ADR-0078.

---

## References

- `crates/sddk-cli/src/vault_cmd.rs:172` (dangling citation)
- `crates/sddk-vault/src/repair.rs:16` (`ALLOW_LIST = ["VAULT003"]`)
- Commit `87c5a97` (v1.65.6) — `feat(vault): add VAULT003 per-cycle scope policy and RepairReceipt queue`
- Commit `d19c305` (v1.65.6) — RFC3339 serde, scope validation, error_kind
  emissions, comprehensive tests
- Commit `c5a9ad4` (v1.65.6) — attach_scope cycle_id and
  apply_scope_downgrade key format bugs
- `research/cycle-supersede-replan/evidence-cards/ec-css-008-dangling-adr-0078.yml`