# A5 — UAT Matrix

> Cycle: `p-63676b11dc0ef88f/a5-plan-base-production-ready`
> Status: **A5-PLAN deliverable (planning only)**

The A5 UAT matrix defines what must be *operationally exercised* before
`BASE_PRODUCTION_READY`. It complements the falsification matrix
(which targets failure modes).

## Machine-readable source principle (§10)

Each invariant has **one machine-readable source** → optional
CLI/doctor/release presentations. No invariant is duplicated as
prose + test + grep + release check independently.

## UAT-0 — A4 regression (every cycle)

| Scenario | Command | Gate |
|---|---|---|
| A4 certified contracts intact | `cargo test --workspace` + `a4_closeout_milestone_audit` | G1 |

## UAT-1 — Fresh-machine distribution (§16)

No implicit dependency on the repo checkout. No `cargo run` as a
substitute for the published binary.

```text
fresh isolated environment (container/VM, no repo, no cargo cache)
  → download public release assets
  → verify checksums (sddk.sha256 + CHECKSUMS)
  → install (scripts/install.sh --version <tag> --editor none)
  → sddk dev doctor → all_present: true
  → create/use a minimal project
  → exercise the canonical workflow (intake → verify → debverify → alignment → advisory)
  → restart
  → recover / rebuild (projection rebuild equivalence)
  → upgrade to next version
  → rollback to previous certified version
```

| Assertion | Evidence |
|---|---|
| checksum verifies | install log + `sha256sum` |
| binary runs outside the repo | workflow exit 0 |
| doctor coherent after install | `binary.bundle_coherence: present` |
| rebuild is equivalent | digest equality |
| upgrade succeeds | version reports new tag |
| rollback succeeds | version reports previous tag |
| no repo dependency | environment has no checkout |

**Gate:** G8, G9, G12, G15.

## UAT-2 — Operator diagnostics (§14)

A base operator must answer, with concrete commands:

| Question | Probe |
|---|---|
| what is running? | `sddk version` / `sddk dev doctor` |
| what failed? | cycle/run status surfaces |
| why did it fail? | `architecture why` / WHY surface |
| which revision / run / receipt / policy / evidence? | receipts + `doctor --format json` |
| can it be reproduced? | replay/reproduce command |
| can it be rebuilt? | projection rebuild probe |

**Reuses** existing telemetry/WHY/receipts. **No second observability
system.**

## UAT-3 — Operator onboarding / `--help` (§15, ASC-MA-1)

A new operator can, for each command: discover it, understand its
destructive effect, identify the required capability, run a dry/check mode
where applicable, diagnose a failure, find WHY, and recover.

**Source of truth:** `CommandRegistry`. No hand-written parallel help.

## UAT-4 — Authority/effects behaviour (§8)

| Scenario | Expected |
|---|---|
| denied action | zero side effect |
| `RequireApproval` action without approval | no effect |
| retry after success | idempotent (no double apply) |
| stale approval vs changed proposal | rejected (snapshot bound) |
| crash between authorization and effect | recoverable + auditable |
| actor/proposal/policy binding | enforced |

**Gate:** G5.

## UAT-5 — Concurrency (§7)

| Scenario | Expected |
|---|---|
| two writers, same expected ref | one wins, other gets typed conflict |
| stale writer | typed conflict, no silent overwrite |
| retry after CAS rejection | succeeds or typed failure |
| parallel independent namespaces | both succeed |
| parallel cycle/run operations | typed outcomes |
| duplicate approval/action proposal | single effect |
| concurrent projection rebuild | deterministic |

**Property:** `conflict → explicit typed outcome`; never last-writer-wins.
ADR-0097 remains the authority.

## UAT-6 — Certification (A5-C)

Produces the A5 Milestone Receipt with the exact-revision identity set
(see `A5-RELEASE-CERTIFICATION-PROTOCOL.md`) and the final disposition
`BASE_PRODUCTION_READY` (or not).

## Evidence requirements

- Every UAT run produces a durable receipt (command, revision, result).
- A UAT that only passes on the developer's machine is **not** evidence.
- Seeds / generators / scenario counts for randomized tests are durable.
