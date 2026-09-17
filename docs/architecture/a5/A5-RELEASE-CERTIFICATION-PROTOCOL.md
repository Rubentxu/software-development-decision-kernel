# A5 — Release & Certification Protocol

> Cycle: `p-63676b11dc0ef88f/a5-plan-base-production-ready`
> Status: **A5-PLAN deliverable (planning only)**

## §17 Exact-revision certification

A5 final must certify an **immutable** revision — never a floating
`origin/main`. The identity set:

| Field | Meaning |
|---|---|
| `released_baseline` | the tag the cycle started from |
| `development_head` | full SHA of `origin/main` at cycle open |
| `workspace_version` | `[workspace.package] version` at close |
| `actual_release_tag` | the published tag |
| `release_sha` | the SHA the tag resolves to |
| `binary_sha256` | the published binary digest |
| `bundle_digest` | the framework bundle digest |
| `sbom_digest` | the SBOM digest |
| `test_receipt_digest` | digest of the acceptance test receipt |

Invariants:

```text
certified SHA == tag SHA == binary provenance SHA   (when the build format allows)
workspace_version coherent across binary / bundle / framework
```

`INC-A4-RELEASE-VERSION-DRIFT` documents why these must be recorded
*separately* and never inferred from one another.

## Release governance

- `scripts/release.sh` is the certified mechanism (14 steps, gated).
- `PublicReleaseGate` (step 9b) is **baseline**; A5 hardens *around* it.
- The 9-asset contract is the current contract; re-verify it against the
  live contract each release (do not assume it stays 9).
- `--skip-install` must not be used to bypass step 9b or the CDN poll.

## A5-PLAN specific release policy (§25)

- A **planning-only** cycle does not need a semantic release. If
  governance (the pre-push hook) requires a version bump to integrate the
  planning docs into `main`, that bump is **not** evidence of A5
  implementation and must not be presented as such.
- The behaviour is recorded as evidence for the push-contract
  investigation (`A5-PUSH-CONTRACT-INVESTIGATION.md`).
- Do not publish a production release solely to close a planning cycle.
- If a release *is* published for other reasons, every release gate still
  applies unchanged.

## Certification artifact

`A5-C` produces `docs/architecture/receipts/A5-MILESTONE-RECEIPT.md`:

- certified baseline + development head
- all gate receipts
- exact-revision identity set
- risk register final status
- debt disposition final status
- UAT + falsification results
- final disposition: `BASE_PRODUCTION_READY` or not

The milestone receipt is a **PROJECTION** — evidence, not a runtime
authority. No `BaseProductionReadyReceipt` type is introduced.

## Disposition vocabulary

Only `BASE_PRODUCTION_READY` or not. No score, no percentage, no maturity
rating.

## Push-contract hardening (A5-1)

Desired invariant:

```text
release commit
  ≠
handoff documentation commit
  ≠
cycle close marker
```

A push must never require asserting a version bump that did not happen.
See `A5-PUSH-CONTRACT-INVESTIGATION.md` for the root-cause analysis and
the remediation options. Registered as
`INC-A5-PUSH-RELEASE-MARKER-FRICTION` (P2), independent of
`INC-A4-RELEASE-VERSION-DRIFT`.
