---
id: INC-RELEASE-TAG-FIX
title: "release script step 9 anchors tag to stale origin/main, requiring manual repointing"
status: closed
severity: medium
priority: P2
fingerprint: "release-002-step9-tag-anchoring"
fingerprint_aliases:
  - INC-RELEASE-TAG-FIX
  - manual-tag-repointing-after-release
cluster_id: CL-RELEASE-PIPELINE-INTEGRITY
created: 2026-09-12
created_by: orchestrator
owner: release-pipeline
closed_at: 2026-09-12
closed_by: orchestrator (commit a86e85d "feat(release): sync HEAD to origin/main before publish (closes INC-RELEASE-TAG-FIX)" — released as v1.168.41)
resolution_note: |
  Closed at v1.168.41 by adding step 1c/14 to scripts/release.sh. The
  step fetches origin/main, then either fast-forwards origin (the
  pre-push hook enforces the bump predicate), or fails closed if
  origin/main has advanced concurrently. Step 1c lives outside the
  SKIP_TESTS guard — pushing is part of the release contract, not the
  test gate. New pin test tests/test_release_tag_anchoring.sh (174 LoC)
  verifies five invariants: (a) step 1c ordering between 1b and 2, (b)
  pushes the branch (no tag, no --force), (c) outside SKIP_TESTS guard,
  (d) does not duplicate the pre-push bump predicate, (e) fail-closes
  via merge-base ancestor check.

  Verified end-to-end at v1.168.41 release:
    - Step 1c pushed HEAD (6ff895b) to origin/main before step 9.
    - gh release create v1.168.41 --target main resolved to 6ff895b.
    - Tag v1.168.41 → 6ff895b (matches HEAD, no manual repointing
      required — the entire point of the fix).
    - Local binary 1.168.41, doctor all_present: true.

  Known caveat documented: the fix assumes the operator does not amend
  HEAD after step 1c pushes. In the current release flow, step 3 (cargo
  build) does not commit, so step 1c's push is still authoritative when
  step 9 runs. If the flow changes to commit during the build phase,
  step 1c would need a re-push or a pre-publish invariant.
last_updated: 2026-09-12
---

# INC-RELEASE-TAG-FIX — release script step 9 anchors tag to stale origin/main

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.
> Pattern reference: INC-RELEASE-001 (closed via commit adb39cd; same
> release-pipeline cluster, same fix-shape: shell test + step in release.sh).

## Context

Across cycles v1.168.33 → v1.168.40 the release flow has required manual
intervention after `bash scripts/release.sh` succeeds: the operator must
execute `git tag -f v<X.Y.Z> HEAD && git push origin v<X.Y.Z> --force` to
repoint the published tag to the actual release commit.

### Mechanism (root cause)

`scripts/release.sh` step 9 calls `gh release create --target main`. GitHub
resolves `main` to the commit at the tip of `origin/main` **at the time of
the API call**. The script **never invokes `git push origin main`** — it
assumes the local repository is already in sync with the remote. In
practice the assumption is wrong:

1. Local commits `feat(...)` + `chore(release): ceremonial handoff` +
   `chore(release): bump version to v<X.Y.Z>` are not yet pushed when the
   script starts.
2. `gh release create --target main` resolves `main` to the commit at
   `origin/main` (one or more commits behind the local HEAD).
3. The published tag `v<X.Y.Z>` therefore points to the **stale** commit,
   not the bump commit.
4. The operator must manually repoint the tag (force-push) after the
   script exits.

### Trigger evidence

- `archive-manifest.md` of cycle
  `p-63676b11dc0ef88f-synthesis-dissent-runner-extension` records the
  manual repointing as a known gotcha and proposes INC-RELEASE-TAG-FIX as
  the tracking identifier.
- Same manual repointing was required in cycles 8, 9, and 10 (visible in
  archive-manifests of `vault-mirror-accepted-adrs`,
  `planning-evidence-migration`, and `transition-outcome-m9-2-closeout`).
- Tag anchoring was changed at v1.89.7 (commit f4b… — not in current repo)
  to use `--target main` instead of an explicit SHA; that change removed
  the HTTP 422 error class but introduced the staleness problem because
  the script still has no push step.

## Rationale

- The release script's contract (per `docs/RELEASING.md` and AGENTS.md §8)
  is to be the **single source of truth** for end-to-end release
  publication. Anything that requires post-script operator action
  violates that contract.
- The pre-push hook (`githooks/pre-push`) already gates `git push origin
  main` on the bump-commit predicate, so adding a push step inside the
  script cannot bypass governance: the hook still rejects the push if
  the predicate fails. This makes the new step safe to add without
  audit changes.
- Cloud CI is disabled (AGENTS.md §2.5), so the local script is the only
  place where this gate can be enforced.

## Resolution (planned)

Insert a new step `1c/14 — git push origin main` in `scripts/release.sh`
between step 1b (shell contract tests) and step 2 (version read). The
step must:

1. Fetch `origin/main` to detect concurrent advances.
2. If `origin/main` is ahead of local HEAD, refuse to release and prompt
   the operator to merge — fail-closed (avoids silent non-fast-forward
   during the release flow).
3. If `origin/main` is behind, push local HEAD. The pre-push hook already
   enforces the bump-commit predicate; pass-through to that hook.

The new step is **only added to the script**. A new pin test
`tests/test_release_tag_anchoring.sh` verifies the script's ordering
(push step exists, ordered before publish, uses `git push origin main`
with the pre-push hook active).

Scope of change (estimated):
- `scripts/release.sh`: +30 LoC (new step 1c, between step 1b and 2)
- `tests/test_release_tag_anchoring.sh`: ~70 LoC (new file)
- `tests/test_push_prevention_hook.sh`: 0 LoC (pre-push hook unchanged)
- `docs/debt/README.md`: 1 row appended
- Archive-manifest of cycle
  `p-63676b11dc0ef88f/release-tag-anchoring`: evidence + closure

`--skip-tests` continues to skip step 1c only if the operator also opts
out of the release gate (the new step lives outside the existing
`SKIP_TESTS` guard by design — pushing is part of the release, not the
test gate).

## Evidence

- Archive-manifest of cycle
  `p-63676b11dc0ef88f-synthesis-dissent-runner-extension` (this INC
  originated as a gotcha note there).
- Tag `v1.168.40` currently points to commit `0279dde` (`bump version to
  v1.168.40`), but only **after** manual `git tag -f` + force-push — the
  release's `--target main` call would otherwise have left the tag at
  `7a3bebe` (the previous bump, v1.168.39).
- The release flow's README (`docs/RELEASING.md`) does not document the
  manual repointing requirement; absence of that documentation is itself
  evidence of the bug's stealth.

## Status

Open. Tracked by cycle `p-63676b11dc0ef88f/release-tag-anchoring`
(sequence 510, B-direct path).
