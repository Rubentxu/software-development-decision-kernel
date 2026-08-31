# Migration Strategy — v1.9.1 to SDDK 2.0

## Principle: strangler, not rewrite

The current CLI and behavior remain usable while internals move behind ports/packs. New architecture should grow around verified seams and progressively absorb old paths.

## Step 1 — Freeze behavior

- Capture CLI snapshot/contract tests for current commands.
- Record UAT schema v3/v4 fixtures and release acceptance fixtures.
- Record current XDG paths and zero-intrusion contract.
- Snapshot current dependency graph and entropy metrics.

## Step 2 — Introduce inward ports

Create ports adjacent to application/domain code and implement them with existing storage/gateway components. Initially, the implementation may still call the old code internally.

## Step 3 — Move composition outward

CLI creates adapters and application services; it stops owning SQL/storage rules. Keep command names unchanged.

## Step 4 — Add ledger in parallel

For one slice, dual-write temporarily:

```text
legacy mutable state + new event ledger
```

Validate that the ledger projection reconstructs equivalent state. Once proven, flip reads to projection and remove legacy authority.

Dual-write MUST be time-bounded and covered by equivalence tests.

## Step 5 — Extract universal evidence

Introduce adapters that convert current UAT evidence into the universal Evidence contract. No user-visible format change is required initially.

## Step 6 — UAT pack facade

Existing `sddk uat ...` commands call the new UAT application API. Move domain code in slices:

1. plan parsing/validation;
2. execution state;
3. evidence;
4. review/assessment;
5. sign-off/staleness;
6. web/guided runner assets.

Delete old code only after parity tests pass.

## Step 7 — Graph projection

Start read-only. Rebuild graph from ledger and expose query/inspection commands. Only after stable reconstruction should reactive behaviors be enabled.

## Step 8 — Reactive proposals

Enable deterministic pattern behaviors that only emit proposals/observations. Capability execution remains unchanged and authoritative.

## Step 9 — Forks/replay

Add fork metadata after event semantics are stable. Avoid building forks against mutable legacy stores.

## Step 10 — Explorer

Explorer consumes stable read APIs/projections. It must not become a new source of mutation authority.

## Compatibility policy

- Existing public CLI flags should have a deprecation period.
- Event/schema compatibility should use upcasters/fixtures.
- Pack manifest changes are versioned.
- Old knowledge/history is migrated or archived, never silently discarded.
