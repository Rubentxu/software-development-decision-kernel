# A5 — Falsification Matrix

> Cycle: `p-63676b11dc0ef88f/a5-plan-base-production-ready`
> Status: **A5-PLAN deliverable (planning only)**

A4 demonstrated that the model is correct. A5 must try to **destroy it
operationally**: crash at the worst point, two simultaneous writers, a
corrupt asset, a clean install, a rollback, a stale ref, a side effect
after an authority decision that is no longer valid, secrets in logs, a
projection deleted and rebuilt.

Each row is a falsification probe. "Expected" is the property that must
hold; a violation is a finding (severity per the contract §20).

## §6 Durability / recovery (A5-2, G2/G3)

| Probe | Expected |
|---|---|
| crash between canonical append and projection rebuild | canonical fact survives; projection rebuilds |
| crash during CAS/ref update | no partial state becomes authority; typed recovery |
| partial write | detectable; not silently accepted |
| corrupt CAS object | detected (hash mismatch → typed error) |
| missing CAS object | typed error; never silent default |
| stale ref | typed conflict; no overwrite |
| projection deletion + rebuild | identical projection |
| repeated rebuild | byte-identical |
| replay after interruption | converges to same state |
| duplicate event delivery | idempotent |
| out-of-order external observation ingestion | canonical handling; no silent reorder of facts |
| empty store bootstrap | valid empty state |
| restore from durable artifacts | state restored; projections rebuild |

**Pin:** canonical facts survive · projections rebuild · no projection
becomes authority. **No new store** to ease recovery.

## §7 Concurrency / races (A5-3, G4)

| Probe | Expected |
|---|---|
| two writers / same expected ref | one succeeds; other typed conflict |
| stale writer | typed conflict |
| retry after CAS rejection | succeeds or typed failure |
| parallel independent namespaces | both succeed |
| parallel cycle/run operations | typed outcomes |
| authority check vs effect execution race | no side effect after invalidation |
| duplicate approval/action proposal | single effect |
| concurrent projection rebuild | deterministic |

**Property:** `conflict → explicit typed outcome`; never last-writer-wins.
ADR-0097 is the common revision substrate authority.

## §8 Authority / effects (A5-3, G5)

| Probe | Expected |
|---|---|
| denied action | zero side effect |
| `RequireApproval` bypass attempt | rejected |
| retry does not double-apply | idempotent |
| stale approval vs changed proposal | rejected (snapshot bound) |
| actor/proposal/policy snapshot binding | enforced |
| crash between authorization and effect | recoverable + auditable |
| TOCTOU between decision and effect | no window, or the boundary is named and safe |

**Blocker rule:** if no safe decision→effect atomicity boundary exists,
that is a `BASE_PRODUCTION_READY` blocker.

## §13 Security / secrets (A5-5, G11)

Production surfaces inventoried: credentials, environment variables,
agent/provider secrets, logs, receipts, telemetry, CLI output, CAS
payloads, handoffs, error messages.

| Probe | Expected |
|---|---|
| secret appears in log | not present |
| secret appears in receipt | not present |
| secret appears in error | not present |
| secret becomes semantic identity | not present |
| secret enters telemetry | not present |

**No new security framework** — only gates and blockers.

## §12 Property / stress testing

Persist the approach only where it earns value. Candidates (decide, don't
adopt blindly):

| Candidate | Value |
|---|---|
| namespace identity | high (collision safety) |
| content-addressed identities | high |
| CAS concurrency | high |
| receipt ordering independence | high |
| projection rebuild equivalence | high |
| event replay | medium |
| authority idempotency | high |

Requirements when adopted: durable **seed + generator + scenario count +
reproduction command**. No automatic proptest/quickcheck adoption.

## §11 Test-reliability falsification (A5-5, G13)

| Probe | Expected |
|---|---|
| `uat_stale_tests::stale_detects_geometry_change` | `REPRODUCED_AND_FIXED` \| `PROVEN_INFRA_FLAKE_WITH_MITIGATION` \| `OBSOLETE_TEST_REMOVED` |
| each ignored test | individually dispositioned |
| flaky detection | not closed by "couldn't reproduce" |

Permitted outcomes exclude `couldn't reproduce → closed`.

## §5 Release / distribution falsification (A5-1, G7/G8)

| Probe | Expected |
|---|---|
| tag → certified SHA | exact match |
| workspace/binary/bundle version coherence | coherent |
| 9-asset contract | satisfied |
| CHECKSUMS / SBOM | present and verify |
| public URLs | HTTP 200 |
| install from public release | works |
| corrupt asset detection | detected |
| partial asset publication | detected, release fails closed |
| interrupted release recovery | recoverable, no partial published state |
| rollback / previous-version reinstall | works |

`PublicReleaseGate` is baseline; harden **around** it, don't redesign
without evidence.
