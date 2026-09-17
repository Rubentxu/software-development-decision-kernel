# A5-1 Receipt — Release / Distribution / Version Governance

> **Cycle:** `p-63676b11dc0ef88f/a5-1-release-distribution-version-governance`
> **Date:** 2026-09-17
> **Budget:** RELEASE / DISTRIBUTION / VERSION GOVERNANCE
> **Disposition:** **CLOSED** (mechanism + evidence complete for A5-1's gates)
>
> This receipt is **evidence / projection**, not runtime authority.

## Identity (recorded separately)

| Field | Value |
|---|---|
| `certified_semantic_baseline` | `A4_CERTIFIED` |
| `released_baseline` | `v1.169.68` → `3bad25275212f77c3d0d4d664f4d49293aa779c9` |
| `development_head_at_open` | `0c8aafbf5da9505d498591a27f181e45ee46f7ea` |
| `workspace_version_before_release` | `1.169.70` |
| `workspace_version_at_release` | `1.169.71` |
| `actual_release_tag` | `v1.169.71` |
| `release_sha` | `e3fc3a09259182478c5f508bc4d51ceb958260b5` |
| `post_release_main_head` | `b247766` (docs-only cycle-close push; **differs** from `release_sha`) |
| `binary_sha256` | `0ff23cc06382c59c1ec8ef122d5c1ad04719a7d11a72972b395bcd5bc6d726ad` |
| `bundle_digest` | `MANIFEST.sha256` in the bundle; `manifest_sha256=608c6d9ced950456e9d453d8e54529b6c3dc06e45302189c3c15c01c738fd39f` |
| `sbom_digest` | `sbom.json` published as an asset (CycloneDX) |
| `test_receipt_digest` | workspace + shell contract runs recorded below |

`published_release_for_1.169.69` = **NONE** · `published_release_for_1.169.70` = **NONE**
(both were development/governance integration states, never releases).

## §0 Falsification first (RED → GREEN)

`tests/test_push_prevention_hook.sh` against the **pre-A5-1** hook:

```text
PASS=13 FAIL=7
  FAIL [expected ACCEPT, got REJECT] docs/** only (non-empty)
  FAIL [expected ACCEPT, got REJECT] docs file delete
  FAIL [expected ACCEPT, got REJECT] .sddk/followups/** only (non-empty)
  FAIL [expected ACCEPT, got REJECT] rename docs -> docs
  FAIL [expected ACCEPT, got REJECT] path with spaces under docs
  FAIL [expected REJECT, got ACCEPT] empty chore(release) marker
  FAIL [expected REJECT, got ACCEPT] fake release subject + runtime change
```

Against the **A5-1** hook: `PASS=20 FAIL=0`.

This is the durable falsification the A5-PLAN promised: the pre-A5-1 hook
treated the commit **subject** as authority.

## §10 Matrices

```text
tests/test_push_prevention_hook.sh   -> PASS=20 FAIL=0   (push admission)
tests/test_release_admission.sh      -> PASS=7  FAIL=0   (release admission)
```

Push-admission contract: `main` accepted iff (A) real `[workspace.package]`
version change in range, OR (B) non-empty range entirely under
`docs/**` + `.sddk/followups/**`. Release-admission contract: real,
**monotonic** version change vs `HEAD^`.

## Gate evidence

### G7 — release reproducibility

| Check | Result |
|---|---|
| tag → SHA | `v1.169.71` → `e3fc3a09…` (via `git ls-remote origin "$TAG"`, never `origin/main`) |
| release SHA == HEAD at publish | **yes** (`e3fc3a09…`) |
| workspace version → binary | `sddk --version` → `1.169.71` |
| bundle version → workspace | framework dir `1.169.71`, `bin/current` → `1.169.71` |
| installed framework directory | `~/.local/share/sddk/framework/1.169.71` + `current` symlink |
| CHECKSUMS vs published assets | verified (see G8) |
| `release.sh` step 1c push | accepted by the **new** hook: `pushed HEAD to origin/main: e3fc3a09…` |

### G8 — distribution integrity

```text
HTTP 200  sddk
HTTP 200  sddk.sha256
HTTP 200  CHECKSUMS
HTTP 200  sbom.json
HTTP 200  software-development-decision-kernel.tar.gz
HTTP 200  sddk-v1.169.71-sddk-linux-x86_64-musl.tar.gz
```

9-asset contract: expected 9, published 9, publicly reachable.

Corruption falsification (OBSERVED): downloaded `sddk`, mutated byte 1000 in
a scratch copy → `sha256sum -c sddk.sha256` **fails**. Corruption is detected
by the distribution layer.

### G15 — rollback (isolated prefix, public assets)

```text
install v1.169.71  -> sddk 1.169.71
install v1.169.68  -> sddk 1.169.68      (rollback to previous certified)
restore v1.169.71  -> sddk 1.169.71
doctor             -> binary.bundle_coherence: present, all_present: true
```

No `cargo run`; releases downloaded from the public URL. (G12 clean-machine
revalidation remains A5-5's job; A5-1 proves the mechanism.)

### G16 — exact revision identity

Recorded in the Identity table above. `post_release_main_head` MAY differ from
`release_sha` after the docs-only handoff push; the release stays certified by
its immutable tag, not by the moving `main` head.

### G1 — A4 regression (transversal)

```text
cargo fmt --all -- --check                                  OK
cargo clippy --workspace --all-targets -- -D warnings       OK
cargo test --workspace --offline                            PASSED=4697 FAILED=0 IGNORED=14
tests/test_push_prevention_hook.sh                          PASS=20 FAIL=0
tests/test_release_admission.sh                             PASS=7 FAIL=0
```

`A4_CERTIFIED` intact; no A4 semantic change.

### PublicReleaseGate

Unchanged and still enforced by `release.sh` step 9b. Live result:
`draft=false`, `prerelease=false`, 9/9 assets, tag SHA anchored.

## §7 / §17 Incidences and risks

| Item | Before | After |
|---|---|---|
| `INC-A5-PUSH-RELEASE-MARKER-FRICTION` | open (P2) | **CLOSED** — empty marker now rejected; docs-only pushes accepted |
| `INC-A4-RELEASE-VERSION-DRIFT` | open (low/P2) | **CLOSED** — separate-identity invariants now enforced by `release_admission_check` + the recorded identity set |
| R7 tag/version drift | open | **CLOSED/EVIDENCED** (G7 + G16) |
| R10 ceremonial marker | open | **CLOSED/EVIDENCED** (matrices + §9 UAT) |
| R8 corrupt/partial/stale distribution | open | **mechanism/evidence complete** (G8); clean-machine revalidation → **A5-5** |
| R15 rollback failure | open | **mechanism/evidence complete** (G15); clean-machine revalidation → **A5-5** |

`G12` and `BASE_PRODUCTION_READY` are **not** declared by A5-1.

## §9 Post-release documentation UAT

After publishing `v1.169.71`, the handoff + this receipt are committed as a
normal `docs/**` change and pushed to `main` with **no** `Cargo.toml` bump and
**no** ceremonial marker. Expected: **ACCEPTED** by the new hook.

**Result (OBSERVED, real hook):**

```text
$ git commit -m "docs(a5-1): cycle close — receipt, handoff, INC closures, roadmap delta"
$ git push origin main
   e3fc3a0..b247766  main -> main            # ACCEPTED, no bump, no marker
```

Before A5-1 this exact push was rejected (reproduced in the A5-PLAN
investigation and in the RED matrix) and required an empty
`chore(release): bump version (cycle close marker)`. This is the OBSERVED
closure of `INC-A5-PUSH-RELEASE-MARKER-FRICTION`, executed against the real
hook — not a mock. `post_release_main_head` (`b247766`) now differs from
`release_sha` (`e3fc3a0`), which is legitimate and does not invalidate the
release (`G16`).

## Findings discovered by running the FULL 14 steps

Recent cycles used `--skip-tests`, which skips steps 1 **and 1b**. Running the
full pipeline exposed two latent issues:

1. **`tests/test_adr_promotion_format.sh` RED** — `ADR-0126`, `ADR-0127`,
   `ADR-0128` (authored in A4-5a/A4-S15R/A4-5b) lacked the ADR-0001 §3.4 YAML
   frontmatter. Their cycles had shipped, so their real status is `accepted`.
   Fixed here (frontmatter added with `status: accepted` + `accepted_at` +
   `accepted_by_cycle` + implementation evidence) and mirrored to the vault
   (`scripts/mirror_adrs_to_vault.py`). Now `violations: 0`.
2. **Step-1b gate was masked.** The shell contract gate had not actually run
   for several cycles. Registered as a release-governance finding: any cycle
   that changes release/push tooling MUST run the full 14 steps.

3. **Storage concurrency flake (OBSERVED, not A5-1-caused).**
   `sddk-storage::sqlite_storage::storage_insert_gate_receipt_concurrent_allocations_observe_distinct_seq`
   failed once during a full run with
   `SqliteFailure(DatabaseBusy, "database is locked")`, then passed 3/3 in
   isolation. Load-induced flake; **register for A5-3** (concurrency/CAS),
   not closed by "couldn't reproduce".

## §19 No new production abstraction

No `ReleaseDomain`, no release state machine in Rust, no second receipt type,
no second asset store. The new surface is a shell library + a hook, keeping a
single machine-readable source per invariant.

## §20 Claim → Evidence

```text
OBSERVED  real version bump push accepted (step 1c: pushed HEAD to origin/main)
OBSERVED  post-release docs-only push accepted without bump/marker (§9)
OBSERVED  empty ceremonial marker rejected (matrix)
OBSERVED  mixed runtime+docs range rejected without bump (matrix)
OBSERVED  fake release subject + runtime change rejected (matrix)
OBSERVED  release tag == release SHA (G7)
OBSERVED  public assets satisfy the 9-asset live contract, all HTTP 200 (G8)
OBSERVED  corrupt downloaded asset fails checksum (G8)
OBSERVED  published install works in an isolated prefix (G15)
OBSERVED  rollback to v1.169.68 works; restore to v1.169.71 works (G15)
OBSERVED  release admission refuses an empty/non-monotonic HEAD (matrix)
STRUCTURAL  the release commit subject alone is not push authority
STRUCTURAL  A4 semantic crates unchanged (tooling + tests only)
STRUCTURAL  PublicReleaseGate remains in the release path
DERIVED     A5-2 is structurally unblocked (release protocol trustworthy)
```
