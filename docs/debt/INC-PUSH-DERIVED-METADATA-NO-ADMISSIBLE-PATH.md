---
id: INC-PUSH-DERIVED-METADATA-NO-ADMISSIBLE-PATH
title: "A derived-metadata commit (MANIFEST.sha256) has no admissible push path without a version bump"
status: closed
severity: medium
priority: P2
fingerprint: "cb5e231d85cd8989"
fingerprint_aliases: []
cluster_id: CL-APPLY-PUSH-DISCIPLINE
created: 2026-09-17
created_by: config-model-v1-cycle
owner: unassigned
---

# INC-PUSH-DERIVED-METADATA-NO-ADMISSIBLE-PATH — a derived-metadata commit has no admissible push path

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

`githooks/pre-push` admits a push to `refs/heads/main` iff:

- **(A)** the range contains a real `[workspace.package] version` change in
  `Cargo.toml`, **or**
- **(B)** the range is non-empty and **every** changed path lies under
  `docs/**` or `.sddk/followups/**`.

`MANIFEST.sha256` is committed, generated state: it hashes the bundle assets
(`prompts/`, `agents/`, `skills/`, `packs/`). Editing any bundle asset makes
the committed manifest stale, and step 1 of `scripts/release.sh`
(`cli_dev_install_accepts_committed_manifest`) verifies the **committed tree**
via `git archive HEAD`, while the manifest is only regenerated at step 4.

A commit that fixes **only** the manifest therefore satisfies neither (A) nor
(B): it is source-adjacent, not `docs/**`. Observed during the
`config-model-v1` cycle: commit `450e993` (MANIFEST.sha256 only) was rejected,
and the only reason-respecting way forward was to push it together with a new
version bump — which **burned `1.169.72`** (pushed, never released; the release
shipped `1.169.73`).

## Rationale

- **Severity medium.** No correctness or data loss; the gate fails closed and
  the noise is visible. But it forces a version number to be spent on a
  non-functional change, and it creates pressure to reach for `--no-verify`,
  which would silently disable the whole admission gate.
- **Priority P2.** Workaround exists (fold the manifest fix into the next bump);
  it costs one version number, not a release.
- **Cluster `CL-APPLY-PUSH-DISCIPLINE`.** Same gate, same failure family as
  `INC-M7-9-PRE-PUSH-HOOK-CEREMONIAL-COMMIT` and
  `INC-A5-PUSH-RELEASE-MARKER-FRICTION` (both closed by A5-1).

Not a duplicate of the A5-1 incidences: those were about a **subject** being
treated as authority (too permissive). This is the opposite direction — a
legitimate derived-metadata change has **no** admitted path (too strict).

## Options (not decided)

1. Extend the allowlist with a small **generated-only** set
   (`MANIFEST.sha256`, and `BUNDLE.toml` if it ever becomes tracked), accepted
   only when the range touches **no** source path. Preserves the "no silent
   source push" intent while removing the version burn.
2. Keep the gate as-is and make the workflow explicit: regenerate the manifest
   **before** creating the bump commit, so the fix always rides the bump.
   Already partly done — `AGENTS.md` §5 checklist now requires
   `sddk dev manifest` + `git add MANIFEST.sha256` before the gate.
3. Move manifest verification to step 0 and fail with an actionable message
   ("regenerate and commit MANIFEST.sha256"), so the failure cannot be confused
   with a test regression.

## Resolution (cycle inc-derived-metadata-push-path, v1.169.118)

Adopted **Option 1**: `githooks/pre-push` now admits a third route (C) — a
non-empty range whose changed paths are ALL `MANIFEST.sha256` (closed
generated-only set) optionally combined with the (B) documentation-only
allowlist. Any source path alongside the manifest still requires a real
version bump (A): generated metadata can never smuggle source.

Falsification: 4 new cases in `tests/test_push_prevention_hook.sh`
(manifest only → ACCEPT, manifest+docs → ACCEPT, manifest+crates → REJECT,
nested/MANIFEST.sha256 → REJECT). Matrix 39/39 PASS.

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-09-17 | config-model-v1 cycle | created | push rejection of `450e993`; release run 1 step-1 failure `prompts/sddk/orchestrator.md: hash mismatch` |
| 2026-09-20 | inc-derived-metadata-push-path | closed (Option 1) | `githooks/pre-push` rule (C) + 4 test cases; matrix 39/39 |

## References

- `githooks/pre-push` (rules A/B, cluster header)
- `scripts/release.sh` steps 1 and 4
- `crates/sddk-cli/tests/cli.rs` — `cli_dev_install_accepts_committed_manifest`
- `AGENTS.md` §5 (checklist line added)
- `docs/architecture/receipts/CONFIG-MODEL-V1-RECEIPT.md` §3 findings 1–3
