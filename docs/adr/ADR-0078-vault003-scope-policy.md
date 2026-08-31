# ADR-0078 - VAULT003 per-cycle scope policy

**Status:** accepted (retroactive)
**Type:** decision
**Created:** 2026-08-31
**Created in cycle:** none; documentary adoption that codifies v1.65.6
**Supersedes:** none
**Suppressed:** none
**Renamed from:** none
**Amends:** none

---

## Context

`crates/sddk-cli/src/vault_cmd.rs:172` cites the VAULT003 policy by ADR number,
but the referenced ADR was absent. The decision itself shipped in commit
`87c5a97` (v1.65.6): `crates/sddk-vault/src/repair.rs` contains the closed
allow-list `ALLOW_LIST = ["VAULT003"]` and the RepairReceipt queue.

This ADR records that existing decision. It neither changes the queue nor
invalidates existing receipts.

---

## Decision

VAULT003 (broken-link diagnostics) is eligible for scoped down-classification
through a RepairReceipt. The allow-list is closed and currently contains only
`VAULT003`.

Adding a diagnostic code requires an ADR amendment that documents why the
class is repairable rather than merely reclassified. The amendment MUST retain
the bounded RepairReceipt validity rule: `valid_to <= created_at + 90 days`.

The allow-list is monotone-add: reduction is forbidden because historical
receipts remain valid. Each allowed code may define its own waiver policy in
the RepairReceipt fields.

---

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| Delete the citation from `vault_cmd.rs` | It loses the reason VAULT003 is exceptional. |
| Make the allow-list dynamic | It bypasses the project's closed-set ADR authority pattern. |
| Add this ADR and invalidate queued receipts | Existing receipts are sound and bounded. |

---

## Consequences

- The citation in `vault_cmd.rs:172` has a durable authority.
- VAULT003 and the RepairReceipt queue behave exactly as shipped in v1.65.6.
- Future allow-list expansions require an explicit reviewed decision.

---

## Compatibility and migration

- No ledger event, schema, API, or code behavior changes.
- Existing RepairReceipts remain valid until their existing expiration.
- No data migration is required.

---

## Revisit trigger

Revisit this ADR when a new diagnostic code is proposed for the allow-list or
when the 90-day RepairReceipt validity window needs reconsideration.

---

## References

- `crates/sddk-cli/src/vault_cmd.rs:172`
- `crates/sddk-vault/src/repair.rs:16`
- Commit `87c5a97` (v1.65.6) - VAULT003 scope policy and RepairReceipt queue
- Commit `d19c305` (v1.65.6) - receipt validation and error emissions
- Commit `c5a9ad4` (v1.65.6) - scope-key fixes
- [Research evidence](../../research/cycle-supersede-replan/evidence-cards/ec-css-008-dangling-adr-0078.yml)
