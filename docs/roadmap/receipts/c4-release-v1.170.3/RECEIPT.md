# C4 v1.170.3 — Certificación RECEIPT (release cut, override SemVer)

```yaml
schema_version: 1
profile: BASE
status: PASS_OBSERVED
repository: Rubentxu/software-development-decision-kernel
source_sha: "7bedfe7e657214686412f15edde8ecbb5f482fb9"
tag: "v1.170.3"
binary_sha256: "sha256:924f7de683dff4ff26aa510aa0fe79f37ce0d77b7bb9a42c018408a871c2d1cd"
bundle_manifest_sha256: "sha256:608c6d9ced950456e9d453d8e54529b6c3dc06e45302189c3c15c01c738fd39f"
schema_version_storage: "2"
policy_digest: "<per SDDK policy bundle>"
command_registry_digest: "<per SDDK command registry>"
spec_manifest_digest: "<per SDDK spec manifest>"
environment:
  os: "linux x86_64 (host)"
  rust: "1.91.0"
  host: null
  provider_static: null
  provider_runtime: null
```

## What was executed

Full `SDDK_RELEASE_ADMISSION_MODE=v2 bash scripts/release.sh --force-version 1.170.3`
on `main@7bedfe7`. All 14 pipeline steps PASS.

| # | Step | Status | Evidence |
|---|------|--------|----------|
| 0 | Preflight | PASS | branch main, tree clean, gh auth OK, jq in PATH |
| 1 | Workspace green | PASS | cargo fmt --check, cargo clippy -D warnings, cargo test --workspace (5037/0) |
| 1b | Shell contract tests | PASS | 9/9 shell tests OK (test_adr_promotion_format, test_vault_adr_mirror_coverage, test_release_tag_anchoring, test_release_admission, test_release_receipt_authority, test_authority_helper_lockstep, test_advisory_lint_explanations, test_deny_lint_zero_hits, test_vault_mirror_auto, test_push_prevention_hook) |
| 1c | Sync HEAD to origin/main | PASS | `git push origin main` accepted by pre-push hook (range contains workspace bump 1.170.2→1.170.3 per rule A) |
| 1d | EXT auto-activation | PASS | no EXT env vars set; skipping |
| 2 | Read version | PASS | `1.170.3` from `[workspace.package] version` |
| 2.5 | SemVer-correct tag | PASS | `release-bump.sh --dry-run --force-version 1.170.3` → `new tag: v1.170.3`. Warning emitted: `operator forced version override: 1.170.3 (SemVer algorithm bypassed)`. Tag matches workspace tag. |
| 3 | Build binary | PASS | `cargo build --release --offline --bin sddk` → `sddk 1.170.3` |
| 4 | Manifest | PASS | `sddk dev manifest --root . --verify` → manifest OK, 377 files hashed |
| 5 | Bundle tarball | PASS | `software-development-decision-kernel.tar.gz` (669238 bytes) |
| 6 | BUNDLE.toml (v2) | PASS | manifest_sha256 `608c6d9c...`, bundle.{version,binary_min_version,binary_max_version}=`1.170.3` |
| 7 | Unified tarball | PASS | `sddk-v1.170.3-sddk-linux-x86_64-musl.tar.gz` (12173239 bytes), exec bit + BUNDLE.toml OK |
| 8 | sha256 + CHECKSUMS + sbom | PASS | binary sha256 `924f7de683dff4ff26aa510aa0fe79f37ce0d77b7bb9a42c018408a871c2d1cd`, CycloneDX 1.5 sbom.json |
| 8b | Vault ADR mirror sync | PASS | 0 created, 47 skipped (all present) |
| 9 | gh release create | PASS | `v1.170.3` published on GH Releases as Latest (2026-09-22T14:16:07Z) |
| 9b | Public-release gate | PASS | 9/9 scenarios PASS (tag SHA anchored via `git ls-remote origin $TAG`, isDraft=false, isPrerelease=false, 9-asset contract, all `https://github.com/$REPO/releases/download/$TAG/<asset>` HTTP 200) |
| 10 | Install from real URL | PASS | `bash scripts/install.sh --version v1.170.3 --editor none` → exit 0, bin/sddk extracted with exec bit |
| 11 | sddk dev doctor | PASS | `binary.bundle_coherence: present`, `all_present: true` |
| 12 | sddk dev update --prune-only --keep 1 | PASS | "removed 1, kept 1.170.3" (1.169.155 pruned) |
| 13 | Re-install + final state | PASS | distrib round-trip OK; final state: binary=1.170.3, bundle=1.170.3, current=1.170.3, framework=1.170.3 |

## Release tag override rationale (audit-trail)

The operator requested via message 2026-09-22T13:15Z "saltamos antes al 1.170.x"
after observing that the v1.170.0 release had been published the same day.
The SemVer-correct tag from `release-bump.sh` was `v1.171.0` (minor) due to
3 `feat():` commits since v1.170.0:

- `1b2795b` feat(uat): sddk uat validate --format json (FC-8)
- `13c9690` feat(uat): sddk uat status --format json (FC-7)
- `21fcfff` feat(cli): sddk dev doctor --format json (FC-2)

Override applied: `--force-version 1.170.3` (workspace and tag both bumped
through 1.170.1 → 1.170.2 → 1.170.3 to satisfy the admission monotonicity
check and pre-push hook rule A for the cumulative push range).

Trade-off: the release tag no longer reflects the conventional-commit
algorithm. This is documented in:
- AGENTS.md §2.3 (workspace version = puntero ceremonial del release tag)
- The commit bodies of `de7b77d`, `fb9d712`, `7bedfe7`
- This RECEIPT (audit-trail above)

## Fixes bundled in this release

- **`fix(release)`** (799d387): `--force-version` flag actually passes through
  to `release-bump.sh`. Previously the comment promised passthrough but the
  code invoked `--dry-run` without forwarding operator args. Bug unmasked by
  the override decision in this cycle.
- **`fix(docs)`** (8411b8f): frontmatter missing on ADR-0142 (regression vs
  ADR-0001 §3.4, introduced in session-11 commit 8730252 without passing
  through `tests/test_adr_promotion_format.sh`).
- **`test(release)`** (b5d794f): exclude `--force-version` from the
  tag-anchoring grep (regex was naive: `grep -q -- --force` matched
  `--force-version` substring). Anchored to `(^|[sep])(--force|--force-with-lease)([sep]|$)`.

## Features delivered in this release (since v1.170.0)

- **FC-2** `sddk dev doctor --format json` — `DoctorOutput` derives
  `serde::Serialize`; JSON path emits `binary.bundle_coherence`, `all_present`,
  `missing[]`. (commit 21fcfff)
- **FC-7** `sddk uat status --format json` — `UatStatusOutput { release, plan,
  report }` derives Serialize; text rendering through named `uat_status_text`.
  (commits 13c9690, a06083c, 95f2250)
- **FC-8** `sddk uat validate --format json` — `UatValidateOutput
  { schema_version, plan_features, scenarios_total, form_dsl_errors }`
  derives Serialize. New test `validate_plan_json_output_has_counts`.
  (commit 1b2795b)

## Accepted risks

- **Override SemVer**: tag v1.170.3 contains features that SemVer would call
  minor (1.171.0). Revisit trigger: when the next release contains only
  `fix:/test:/docs:/chore:` commits, the algorithm can run normally without
  override. Until then, the override is recorded in every step-2.5 log.
  Approver: operator (2026-09-22T13:15Z message). Expires: at next
  SemVer-clean release cycle.

## Limitations

- C2 (CogniCode/Chronos/JCode integrations) remains `NOT_EVALUATED` — the
  provider binaries and SDKs required for these tests are not present in
  this environment. Receipts `docs/roadmap/receipts/c2a/`, `c2b/`, `c2c/`
  preserve the NOT_EVALUATED state honestly.
- C5 (X08, J7-J9, R11) remains `DEFERRED` — no trigger criteria met.

## Operator approval ref

Operator message 2026-09-22T13:15Z (recorded in session journal) plus
global `AGENTS.md` AUTO-mode authorization (any gate or decision approved).

## Verified at UTC

2026-09-22T14:16:30Z

## Sources

- [v1.170.3 GitHub Release](https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.170.3)
- Session journal entry 2026-09-22T14:16:30Z
- AGENTS.md §2.3 (workspace version contract)
- ADR-0142 (release script SemVer-correctness)
