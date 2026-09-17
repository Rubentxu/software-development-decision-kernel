# A5 — Release/Push Contract Investigation

> Cycle: `p-63676b11dc0ef88f-a5-plan-base-production-ready`
> Status: **A5-PLAN deliverable (investigation only — nothing changed)**

## The observed friction

Across many recent cycle closes, after a release was published correctly,
a subsequent **documentation/handoff-only** commit could not be pushed to
`main` until an **empty** commit was added:

```text
chore(release): bump version (cycle close marker)
```

Example cycle closes: A4-5a, A4-S15R, A4-5b, A4-5C, A4-CLOSEOUT.

## Root-cause analysis

### The hook

`githooks/pre-push` rejects a push to `refs/heads/main` unless the pushed
commit range contains **either**:

1. a commit whose subject matches `^chore\(release\): bump version`, **or**
2. a commit that changes `[workspace.package] version` in `Cargo.toml`.

It checks the range `remote_sha..local_sha`.

### The sequencing

`scripts/release.sh` (line ~232) runs `git push origin main` itself, which
pushes the release commit `chore(release): bump version` **before** the
handoff documentation is written. When the handoff docs are pushed
afterwards, the new range contains **only** the docs commits — no bump,
no marker. The hook therefore rejects the push.

### Why the marker is "empty"

Because the real version bump was already pushed, the only way to satisfy
the hook in that range is a commit that *looks* like a release marker. The
standard workaround is an empty commit with the release subject.

## Classification

| Option | Verdict |
|---|---|
| **A. Intentionally contractual** | **Partially.** The hook deliberately *permits* a marker (INC-M7-9 option 3). `docs/handoff/HANDOFF-2026-09-11-inc-m7-9-hook-fix.md` explicitly says post-release doc handoffs "still require the marker (preserves the safety guarantee)". |
| **B. A consequence of `INC-A4-RELEASE-VERSION-DRIFT`** | **No.** That INC is about *pre-bump vs release-tag confusion* (workspace version ahead of the tag). Different root cause. |
| **C. An independent operational defect** | **Yes.** The requirement to emit a fake release marker for a docs-only post-release push is a defect in the release/push protocol. |

**Classification: C — independent defect.**

The desired invariant the protocol violates:

```text
release commit
  ≠
handoff documentation commit
  ≠
cycle close marker
```

unless the contract documents them as the *same fact*. Today they are
three distinct facts, but the hook forces the third to masquerade as the
first. **A push must never require asserting a version bump that did not
happen.**

## Is it a lie?

The subject `chore(release): bump version (cycle close marker)` is a
marker, not a claim of a real bump — but the commit is empty, contributes
nothing, and exists solely to satisfy the hook. As the user put it: the
Git history should express real facts; a tool aiming at production
readiness should not need ceremonial commits.

## Proposed remediation options (NOT implemented — A5 workstream item)

1. **Path-scoped allowance.** Let a range pass when it contains *only*
   documentation/planning paths (`docs/**`, `.sddk/**`, `**/*.md`) and no
   runtime/build files. Preserves the safety guarantee (no code lands
   unannounced) while removing the ceremonial marker.
2. **Single-push release.** Stop `release.sh` from pushing `main`; let the
   operator push once so the release commit and the handoff docs land in
   the same range (the bump satisfies the hook).
3. **Explicit honest marker.** Introduce a documented, honest
   `docs(cycle-close)` allowance rather than reusing the release subject.

Option 1 or 2 are preferred (they remove the ceremony without weakening
the safety property). This is a **release-tooling** change, not a
semantic one.

## Disposition

- Registered as an A5 incidence: `INC-A5-PUSH-RELEASE-MARKER-FRICTION`
  (see `docs/debt/`), **P2** (production-hardening friction with a
  workaround), owner: A5-1 (release/distribution workstream).
- `INC-A4-RELEASE-VERSION-DRIFT` and this INC share the *release-area*
  but **not** the root cause; they must be resolved independently.
- This cycle (A5-PLAN) records the behaviour as evidence and does **not**
  change `release.sh` or the hook (§24).

## Empirical evidence collected this cycle

A5-PLAN is docs-only and publishes **no** release. Instead of the
ceremonial marker, its two commits (`docs(a5): …` + an honest
`chore: advance workspace version to 1.169.69 (A5-PLAN docs integration)`)
were pushed in **one** push:

```text
$ git push origin main
   4086347..a020b0b  main -> main        # accepted, no marker commit
```

The hook accepted the push via its **semantic** `[workspace.package]`
version-bump check — exactly the desired behaviour. This confirms:

- The friction is **not** the hook's regex per se; it is the
  **sequencing** where `release.sh` pushes the bump commit before the
  handoff docs exist, splitting one logical close into two ranges.
- A single push whose range contains the version bump needs no marker.

This is direct support for remediation option **2** (single-push release):
the ceremony disappears when the release commit and its documentation
land together.

### Reproduction of the rejection (OBSERVED)

A **subsequent** docs-only push (a one-line correction to this very
document) was then attempted and **rejected** by the hook:

```text
$ git commit -m "docs(a5): record push-contract empirical evidence"
$ git push origin main
ERROR: Push to main rejected — no release marker or Cargo.toml version bump found in range.
ERROR: (apply/release split | INC-MATRIX-LINT-CODES-APPLY-PUSH-VIOLATION)
ERROR: A push to main requires EITHER:
ERROR:   (1) a commit with subject matching:  ^chore\(release\): bump version
ERROR:   (2) a commit that bumps [workspace.package] version in Cargo.toml
ERROR: (apply/release split | INC-M7-9-PRE-PUSH-HOOK-CEREMONIAL-COMMIT) ...
```

So the friction is **reproduced**: once the bump commit has been pushed,
*any* later documentation correction to `main` needs a new
version bump (or a ceremonial marker). The fix (option 1 or 2) is
required so that honest documentation does not have to masquerade as a
release.
