# C0-WORKSPACE-TESTS — Snapshot del workspace tests antes de release

> **Slice id:** `p-63676b11dc0ef88f/c0-reconciliation-baseline` (snapshot annex)
> **Date (UTC):** 2026-09-21T12:50:00Z
> **Status:** OBSERVED — cargo test --workspace run live, output captured to `/tmp/full_test.log`.
> **Purpose:** Capture the workspace test state at HEAD `62494ae` so the operator's release can verify the baseline before publishing v1.169.128.

## §1 Command and result

```
$ cargo test --workspace --no-fail-fast > /tmp/full_test.log 2>&1
$ echo $?
0
```

## §2 Aggregate counts

```
$ grep "^test result:" /tmp/full_test.log | awk '{passed+=$4; failed+=$6; ignored+=$8} END {print "TOTAL passed:", passed; print "TOTAL failed:", failed; print "TOTAL ignored:", ignored}'
TOTAL passed: 4966
TOTAL failed: 0
TOTAL ignored: 15
```

**Summary:** 4966 passed, 0 failed, 15 ignored, exit 0.

## §3 Per-crate breakdown (top contributors)

| Suite | Passed | Ignored | Notes |
|---|---|---|---|
| `sddk-cli` (lib + bins) | 780 | 1 | largest suite; one doc-test ignored |
| `sddk-engine` (lib) | 184 | 0 | includes 5 SAW tests |
| `sddk-storage` (lib) | 184+ | 0 | high I/O test suite, 92.57s |
| `sddk-gateway` (lib) | 184 | 0 | test_runner module |
| (rest) | ~3634 | 14 | smaller crates |

Full per-suite counts available in `/tmp/full_test.log` (255 result lines, all `0 failed`).

## §4 What this means

- The workspace is in a healthy state: 100% of runnable tests pass.
- The release candidate (HEAD `62494ae`, v1.169.128) has a clean test baseline.
- `scripts/release.sh` step 1 ("Workspace green") will see `cargo test --workspace` exit 0 and pass.
- The 15 ignored tests are documented in their respective test files (typically marked `#[ignore]` with reason, e.g. `SqliteLedgerFactory` doctest at line 2331 of `sddk-storage/src/lib.rs`).

## §5 Why this annex exists

The orchestrator had previously executed per-crate tests (AIW-S3, S4, S7a/b/c, S8-X02/X04/X06/X07). This annex provides the aggregate, single-shot test result for the operator's convenience. If `scripts/release.sh` step 1 fails, the operator can compare against this baseline to identify regressions.

## §6 Reproducibility

Anyone running the same command from `62494ae` should see the same aggregate numbers. If numbers differ materially, that's evidence of a regression introduced between the snapshot and the verification time — investigate before publishing.

## §7 Out-of-scope reaffirmation

- This annex does NOT certify any individual feature as production-ready. That requires UAT and clean-machine testing (see [C0-RECEIPT.md §2 G12](../../C0-RECEIPT.md)).
- This annex does NOT replace `sddk dev doctor` (release.sh step 11) which validates the bundle coherence.
- This annex is workspace-test-only; it does not cover EXT binaries (cognicode-mcp, chronos-mcp) which are `acceptance_blocked` per [C0-RECEIPT §5](../../C0-RECEIPT.md).
