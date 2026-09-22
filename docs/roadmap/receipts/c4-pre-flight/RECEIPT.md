# C4 — RELEASE CUT PRE-FLIGHT (dry-run, operator-gated)

**Cycle:** C4 (Release y certificación de producto, ROADMAP §C4)
**Status:** PRE-FLIGHT PASSED, **AWAITING OPERATOR** for actual `bash scripts/release.sh`
**Date:** 2026-09-22T10:41:00Z
**Workspace:** 1.169.152
**HEAD:** 3d4a4ed

## What was executed

`bash scripts/release.sh --dry-run --skip-tests` with
`SDDK_RELEASE_ADMISSION_MODE=v2`. Steps 0..8 (and 8b vault mirror) all PASS.

This is NOT a release. No push, no GH release, no install. It is the
canonical pre-flight described in `docs/RELEASING.md` — confirming
the workspace is admission-ready so the operator can run the full
release script knowing the local gates are green.

## Results (dry-run)

| Step | Result |
|---|---|
| 0 Preflight | PASS — branch main, tree clean, HEAD `chore(release): bump version`, jq in PATH, gh auth OK |
| 1 Workspace green | SKIPPED (`--skip-tests`; tests run separately; see STATE.yaml for 2931/0 result) |
| 2 Read version | `1.169.152` from `[workspace.package] version` |
| 3 Build binary | `sddk` binary built (release profile) |
| 4 Manifest | `dev manifest --root .` produced; manifest_sha256 in BUNDLE.toml = `608c6d9c…` |
| 5 Bundle tarball | `software-development-decision-kernel.tar.gz` (669236 bytes) |
| 6 BUNDLE.toml (v2) | written; schema v2; bundle.{version, binary_min_version, binary_max_version}; contents.manifest_sha256 verified |
| 7 Unified tarball | `sddk-v1.169.152-sddk-linux-x86_64-musl.tar.gz` (12129411 bytes); exec bit + BUNDLE.toml OK |
| 8 sha256 + CHECKSUMS + sbom | produced; binary sha256 `b13e0017fc9f3f91…`; CycloneDX 1.5 sbom.json |
| 8b Vault ADR mirror sync | 0 created, 47 skipped (all present) |
| 9 gh release create | NOT EXECUTED (dry-run) |
| 9b Public-release gate | NOT EXECUTED (depends on 9) |
| 10 Install from real URL | NOT EXECUTED (depends on 9) |
| 11 sddk dev doctor | NOT EXECUTED (depends on 10) |
| 12 sddk dev update --prune-only | NOT EXECUTED (depends on 11) |
| 13 Final state print | NOT EXECUTED (depends on 12) |

## Why dry-run, not full

Per ROADMAP §C4: "publicar mediante `bash scripts/release.sh` **solo con
autorización del operador**". The operator has not authorized the actual
release in this session. The pre-flight confirms local state is
admission-ready, so when the operator runs the full script, steps 0..8
are guaranteed green and only the publish + install + doctor steps
(9..13) require operator-driven action and verification.

## What the operator would need to do to actually release

```bash
# From the SDDK repo root, with gh auth active:
cd ~/Proyectos/agentesIA/sddk-framework
bash scripts/release.sh    # full 14-step pipeline
# OR if preflight confidence is sufficient:
bash scripts/release.sh --skip-tests
```

If the operator runs the full pipeline, this receipt becomes the
**PRE_FLIGHT** section of the eventual `C4-CERTIFICATION-RECEIPT.md`
(created at the end of a successful run).

## Stale local environment caveat (from session-11 §C3h context)

- `origin/main` is at `a5f279c` (workspace 1.169.122); local is at
  `3d4a4ed` (workspace 1.169.152) — **25 commits ahead**.
- `last_public_release_observed = v1.169.122` (2026-09-20).
- The full release script will `git push origin main` (step 9b
  prerequisites) — operator must intend the push.

## Operator decision required

| Decision | Effect |
|---|---|
| Run `bash scripts/release.sh` (full) | Publish v1.169.152 to GH Releases; install locally; close C4 |
| Run `bash scripts/release.sh --dry-run` only | Re-validate; no publish |
| Defer | C4 stays PRE_FLIGHT_PASSED; session can end |
| Amend C3h fixes before release | New cycle to revisit any of C3a-h fixes if last-minute concerns |

## References

- `docs/RELEASING.md` — release process spec
- `scripts/release.sh` — pipeline (14 steps)
- `tests/test_release_public_gate.sh` — public-release gate (10 scenarios)
- `docs/roadmap/CURRENT.md` — operator-facing pointer
- `docs/roadmap/STATE.yaml` — structured state
