# C4 — RELEASE PENDING: snapshot operacional pre-cut

**Cycle:** C4 (Release y certificación de producto, ROADMAP §C4)
**Status:** RELEASE_PENDING_OPERATOR — workspace admission-ready
**Date:** 2026-09-22T10:57:00Z
**Workspace:** 1.169.152
**HEAD:** `35146ff`
**Suggested tag:** `v1.170.0` (minor)

## Snapshot pre-cut (observed)

| Item | Value | Source |
|---|---|---|
| Workspace version | `1.169.152` | `Cargo.toml [workspace.package] version` |
| HEAD | `35146ff1f1611613dfebe48c0c231020a745c8c2` | `git rev-parse HEAD` |
| Branch | `main` | `git status -sb` |
| Tree | clean | `git status` (no output after `-sb`) |
| Commits ahead of origin/main | 6 | `git rev-list --count origin/main..HEAD` |
| Last public release tag | `v1.169.122` | `git tag --sort=-v:refname \| head -1` |
| Suggested next tag | **`v1.170.0` (minor)** | `bash scripts/release-bump.sh --dry-run` |
| Resolved framework version (running binary) | `1.169.122` | `sddk version` (resolved via `~/.local/share/sddk/framework/1.169.122/`) |
| Local release-profile binary | `1.169.152` | `/var/home/rubentxu/cargo-targets/release/sddk` |
| Pre-flight (release.sh --dry-run steps 0..8 + 8b) | PASSED | earlier this session, `c4-pre-flight/RECEIPT.md` |

## Drift observation (operational, not a bug)

The running CLI binary on PATH (`~/.local/bin/sddk`) resolves to framework
**1.169.122**, while the freshly built release binary (`cargo-targets/release/sddk`)
is **1.169.152** and matches the workspace version.

This drift is **expected** because the binary on PATH was installed by an earlier
`bash scripts/release.sh` (when v1.169.122 was the latest published release).
The release script (step 10-13) will overwrite this with the new version
1.170.0 once cut, restoring coherence.

**Operational implication:** When you run `sddk version` today, you see
`1.169.122`. When you run the freshly built binary directly, you see `1.169.152`.
When you cut the release and the install step runs, you will see `1.170.0`.

## Test coverage snapshot (workspace --lib)

| Metric | Value |
|---|---|
| Tests passed | **2931** |
| Tests failed | 0 |
| Tests ignored | 5 (legitimate: env-gated EXT tests, diagnostic harness, fixture regenerator) |
| Clippy | clean (`cargo clippy --workspace --all-targets -- -D warnings`) |
| cargo-audit | 0 vulnerabilities |
| cargo fmt | clean |
| MANIFEST.sha256 | verified (`sddk dev manifest --verify` exit 0) |
| sddk dev doctor | `all_present: true` |

## Cycle status (per ROADMAP)

| Hito | Status |
|---|---|
| C1 — Falsación de contratos y hardening acotado | CERRADO (H02/H05+H06/cycle-c) |
| C2 — Integraciones reales | NOT_EVALUATED systemic (C2a/b: PROVIDER_MISSING, C2c: ADAPTER_MISSING) |
| C3 — Resiliencia, seguridad y fiabilidad | CERRADO (C3a-h PASS_OBSERVED) |
| C4 — Release y certificación de producto | **RELEASE_PENDING_OPERATOR** |
| C5 — Evolución condicionada | DEFERRED por contrato |

## Outstanding operator decisions

| Decision | Effect | When |
|---|---|---|
| Cut v1.170.0 now | `bash scripts/release.sh` → publish + install + close C4 | When ready |
| Pick a feature from FC-1..FC-6 | New cycle with `feat():` commits → another MINOR or MAJOR | When product need identified |
| Provision provider artifacts (C2) | Enables real C2 UAT evaluation (currently NOT_EVALUATED) | When integration priority emerges |
| Trigger one of C5 deferred (X08/J7-J9/R11) | New cycle with explicit scope | When trigger criteria met |

## References

- `docs/roadmap/receipts/c4-pre-flight/RECEIPT.md` (steps 0..8 PASS)
- `docs/roadmap/ROADMAP.md` §C4 (release contract)
- `docs/architecture/CONTRIBUTING-SEMVER.md` §4.3 (SemVer mechanics)
- `docs/roadmap/FEATURE-CANDIDATES.md` (FC-1..FC-6 inventory)
- `scripts/release.sh` (canonical 14-step pipeline)
- `scripts/release-bump.sh` (SemVer calculation)
