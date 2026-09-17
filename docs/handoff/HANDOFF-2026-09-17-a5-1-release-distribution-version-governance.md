# HANDOFF — A5-1 Release / Distribution / Version Governance (v1.169.71)

## Certified state

| | |
|---|---|
| **Cycle** | `p-63676b11dc0ef88f/a5-1-release-distribution-version-governance` (CLOSED) |
| **Budget** | RELEASE / DISTRIBUTION / VERSION GOVERNANCE |
| **Certified semantic baseline** | `A4_CERTIFIED` (unchanged) |
| **Released baseline** | `v1.169.68` → `3bad25275212f77c3d0d4d664f4d49293aa779c9` |
| **Development head at open** | `0c8aafbf5da9505d498591a27f181e45ee46f7ea` |
| **Workspace version** | `1.169.70` → `1.169.71` |
| **Actual release tag** | `v1.169.71` |
| **Release SHA** | `e3fc3a09259182478c5f508bc4d51ceb958260b5` |
| **Binary sha256** | `0ff23cc06382c59c1ec8ef122d5c1ad04719a7d11a72972b395bcd5bc6d726ad` |
| **PublicReleaseGate** | **PASS** (draft=false, prerelease=false, 9/9 assets, doctor `all_present: true`) |
| **Receipt** | `docs/architecture/a5/A5-1-RECEIPT.md` |
| **Next** | `A5-2` (not auto-opened) |

## What shipped

The push/release contract is now **semantic**:

```text
push to main is ACCEPTED iff
  (A) the range contains a REAL [workspace.package] version change, OR
  (B) the range is NON-EMPTY and every changed path is under
      docs/** or .sddk/followups/**
```

The commit **subject is no longer authority**. An empty
`chore(release): bump version (cycle close marker)` is **rejected**.
`release.sh` additionally requires a real **monotonic** version change vs
`HEAD^` (an empty `chore(release)` HEAD is refused).

## Falsification first (RED → GREEN)

```text
pre-A5-1 hook : PASS=13 FAIL=7   (docs-only rejected; empty marker ACCEPTED;
                                  fake release subject + runtime ACCEPTED)
A5-1 hook     : PASS=20 FAIL=0
```

Plus `tests/test_release_admission.sh`: 7/7.

## §9 Post-release docs-only push (OBSERVED closure of R10)

This handoff (and the A5-1 receipt) is committed as a normal `docs/**`
change and pushed to `main` **with no `Cargo.toml` bump and no ceremonial
marker**. The push was **ACCEPTED** by the new hook — recorded in the
Roadmap Delta / archive manifest. That is the real-hook evidence, not a mock.

## Gate evidence (see the receipt for the full tables)

```text
G7  tag v1.169.71 -> e3fc3a09… == HEAD at publish; binary/bundle/framework coherent
G8  9/9 assets published and publicly reachable (HTTP 200); checksum verifies;
    corrupted byte -> checksum FAILS
G15 rollback: install v1.169.71 -> v1.169.68 -> v1.169.71 (isolated prefix,
    public assets, no cargo run); doctor coherent
G16 identity set recorded separately (release_sha != post_release_main_head allowed)
G1  A4 regression intact: cargo test --workspace PASSED=4697 FAILED=0 IGNORED=14
```

## Findings (honest)

1. **Step-1b gate had been masked.** Recent cycles used `--skip-tests`, which
   skips steps 1 **and 1b**. Running the full 14 steps exposed
   `tests/test_adr_promotion_format.sh` RED: `ADR-0126/0127/0128` (authored in
   A4-5a/A4-S15R/A4-5b) lacked the ADR-0001 §3.4 frontmatter. Fixed here
   (doc-only: frontmatter added with their real `status: accepted` + vault
   mirror). **Rule going forward: any cycle touching release/push tooling runs
   the full 14 steps.**
2. **Storage concurrency flake (not A5-1-caused).**
   `sqlite_storage::storage_insert_gate_receipt_concurrent_allocations_observe_distinct_seq`
   failed once with `SqliteFailure(DatabaseBusy, "database is locked")` under
   load, then passed 3/3 in isolation. Registered for **A5-3**
   (concurrency/CAS), not closed by "couldn't reproduce".
3. Pre-existing shellcheck warnings in `scripts/apply_banner.sh`,
   `tests/test_release_public_gate.sh`, `tests/test_release_tag_anchoring.sh`,
   `tests/test_vault_*` are untouched by A5-1 (verified: 0 changes in those
   files) and sit outside the step-1b gate scope.

## Incidences closed

- `INC-A5-PUSH-RELEASE-MARKER-FRICTION` → **CLOSED**
- `INC-A4-RELEASE-VERSION-DRIFT` → **CLOSED**

Risks: R7 CLOSED/EVIDENCED · R10 CLOSED/EVIDENCED · R8/R15 mechanism+evidence
complete (clean-machine revalidation → A5-5).

## Roadmap Delta

```text
A4       CLOSED / CERTIFIED
A5-PLAN  CLOSED
A5-1     CLOSED v1.169.71
A5-2     NEXT (not auto-opened)
A5-3     blocked_by A5-2
A5-4     blocked_by A5-1
A5-5     blocked_by A5-1
A5-C     blocked_by A5-1..A5-5
```

## STOP

`A5-2` is **not** auto-opened.
