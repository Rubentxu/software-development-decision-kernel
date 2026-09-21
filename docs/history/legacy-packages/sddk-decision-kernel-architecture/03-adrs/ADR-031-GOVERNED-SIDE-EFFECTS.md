# ADR-031-GOVERNED-SIDE-EFFECTS — Require proposal-policy-capability-verification-receipt for side effects

**Status:** Accepted


## Decision
Agents do not directly own unrestricted filesystem, git, network or deployment authority. They request scoped capabilities; policy/approval decides; adapters execute; postconditions verify; receipts record.

## Consequences
Supports least privilege, auditability, approval gates, replay safety and supply-chain provenance.
