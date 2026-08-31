# Architecture Fitness Functions

Automate architectural intent instead of relying only on reviews.

## Proposed checks

### ARCH001 — Kernel purity
Kernel/domain cannot depend on adapters/storage/HTTP/IDE implementations.

### ARCH002 — No adapter construction in application core
Detect direct `Sqlite*`, OpenCode client, filesystem concrete construction in application modules.

### ARCH003 — No transport/domain mixing
CLI/HTTP handlers invoke application services; no workflow transition logic in handlers.

### ARCH004 — No new super-ports
Reject new use of legacy `Ledger` outside compatibility modules.

### ARCH005 — God-module ratchet
Track module LOC/public items/dependency fan-in/fan-out. Warn on thresholds, then ratchet.

### ARCH006 — No crate dependency cycles
Fail immediately.

### ARCH007 — Delegation authority
Agent manifests cannot grant unrestricted `spawn/delegate` unless role = supervisor/sub-supervisor and scope is declared.

### ARCH008 — Workflow core is SDD-agnostic
Kernel cannot match on `Explore/Specify/Design/...` or `CyclePath`.

### ARCH009 — Side effects require capability
Static/dynamic checks ensure privileged adapters are reachable only through governed application use cases.

### ARCH010 — Governed effect must produce receipt
Integration tests fail if a privileged capability reports success without verified receipt event.

### ARCH011 — Projections rebuild
Golden event fixture replay must reproduce canonical projection snapshots.

### ARCH012 — Event schema compatibility
Known historical event fixtures decode/migrate under current version.

## Ratchet strategy

```text
measure → warning → baseline exception → reduce exception count → error
```

Never introduce a rule that immediately forces a giant refactor unless the migration is intentionally scheduled.

### ARCH013 — Dynamic workflow mutations are evented

Runtime execution graph cannot be mutated outside approved orchestration application services that append graph-expansion/revision events.

### ARCH014 — No arbitrary generated orchestration code authority

Supervisor/model output must compile to typed WorkflowIR/ExpansionProposal. Kernel paths must not `eval`/execute generated JS/Python/shell as scheduler authority.

### ARCH015 — SDD invariant checks cannot depend on legacy phase presence

Adaptive SDD completion rules verify ChangeContract/evidence/governance invariants rather than requiring `Explore/Specify/Design/Tasks` enum states.
