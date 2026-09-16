---
id: INC-A4-RELEASE-VERSION-DRIFT
title: "Release-discipline drift: preflight target version vs workspace development version vs actual release tag"
status: open
severity: low
priority: P2
fingerprint: "a4_release_version_drift_v1"
fingerprint_aliases: ["a4_release_version_drift_v1", "release_version_drift_v1"]
cluster_id: CL-RELEASE-DISCIPLINE
created: 2026-09-16
created_by: orchestrator (cycle p-63676b11dc0ef88f/a4-4br-subject-general-evidence)
owner: orchestrator
cycle_source: p-63676b11dc0ef88f/a4-4br-subject-general-evidence
finding_ref: A4-4bR-preflight
---

# INC-A4-RELEASE-VERSION-DRIFT — recurring confusion between three release version sources

> Durable cross-cycle record. Registered from observation during the
> A4-4bR preflight (cycle
> `p-63676b11dc0ef88f/a4-4br-subject-general-evidence`). See ADR-0047
> §3.2.

## Context

A4-4b's `chore(release): bump version 1.169.56 -> 1.169.57 (A4-4b
release)` commit message said it would publish **v1.169.56**. The
release script, correctly per the cycle-46 install-coherence contract
(which requires bumping the workspace BEFORE the release tag so the
binary built from HEAD has a version greater than the published tag),
published **v1.169.57**. The cycle spec's "Expected semantic release:
v1.169.56" was off-by-one vs. the bump step.

Three version sources drifted:

1. **Cycle spec preflight target**: `v1.169.56`
2. **Workspace development version at release**: `1.169.57`
3. **Actual release tag**: `v1.169.57`

The `0cb981d` (docs) commit's H1 row still said "release SHA pending
v1.169.56"; that drift was reconciled in the A4-4bR preflight to
`closed — v1.169.57 (release SHA 7fc820d...)`.

## Why this matters

A reader of the cycle spec, the handoff, the README, and the GH Release
must not have to reconcile three different version numbers by hand.
Each should agree. Today they don't, unless every drift site is
manually corrected.

## Scope (registered, NOT fixed in A4-4bR's budget)

A future cycle may tackle any of:

1. Pre-bump the workspace in a separate `chore(preflight): bump
   workspace 1.169.56 -> 1.169.57` commit BEFORE the cycle's
   feat/test/docs commits land; that way the spec can already read
   `1.169.57` from the start and the `chore(release)` commit only
   becomes the formal release marker.
2. Update `scripts/release.sh`'s step 0 to print all three sources
   side-by-side and fail-fast if the cycle spec's "expected semantic
   release" disagrees with the workspace version by one patch (with
   `--force-allow-off-by-one` for the cycle-46 contract).
3. Add a CLI lint `sddk dev lint release-version-coherence` that walks
   recent handoffs and the README and asserts all release-version
   references agree.

A4-4bR does NOT take any of these on. This is a follow-up so the next
cycle can address it cleanly.

## Acceptance (when picked up)

- One of (1), (2), or (3) above lands in a dedicated cycle.
- A4-4b-style drift can no longer occur; the release script either
  refuses to publish with a clear error, or the spec/workspace/tag
  always agree.
- The `FU-A4-3-CONSTRAINT-BINDING` (P1) and this follow-up (P2) are
  the only A4-era FU items open.

## Related

- AGENTS.md §8 — release pipeline steps.
- Cycle-46 install-coherence contract.
- A4-4b close-checkpoint (`docs/architecture/README.md`).
- A4-4b handoff (`HANDOFF-2026-09-16-a4-4b-alignment-lens-kernel-v1.169.57.md`).
