# A5-CLEAN-MACHINE-UAT.md — UAT-1 Traceability Matrix

> Cycle: `p-63676b11dc0ef88f/a5-5-clean-machine-sweep`
> Phase: build/post-tasks · Released baseline: `v1.169.86`
> Authority: `A5-UAT-MATRIX.md` §22-51 (UAT-1 contract)

This document maps every assertion in `tests/clean_machine_uat.sh` to its
gate, risk, evidence artifact, and disposition. It is the **single source of
truth** for which scenario proves which requirement.

## §1 UAT-1 Scenario Coverage

Each row corresponds to one scenario in `spec.md` §"ADDED Requirements".
Row count: **16** (10 workflow scenarios + CI + shim + receipt + disposition
× 2 + release traceability).

| # | Scenario | Gate | Risk | Assertion in script | Evidence artifact | Disposition |
|---|---|---|---|---|---|---|
| 1 | Podman container launches with no bind mount | G9, G12 | R8 | `podman run --name sddk-clean-machine-uat --rm --detach $IMG` (no `-v`) | `clean-machine-uat-receipt.json::scenario_1_launch` | MUST_CLOSE |
| 2 | Script downloads public release assets via `gh release view` + `curl` | G8 | R8 | `gh release view --jq '.assets[].url'` → `curl -o /tmp/<name>` → HTTP 200 check | receipt::scenario_2_download + HTTP 200s in container stdout | MUST_CLOSE |
| 3 | `sha256sum -c sddk.sha256` exits 0 | G8 | R8 | `podman exec ... sha256sum -c sddk.sha256` | receipt::scenario_3_checksums + `BINARY_SHA256` in receipt | MUST_CLOSE |
| 4 | `sha256sum -c CHECKSUMS` exits 0 for all 9 assets | G8 | R8 | `podman exec ... sha256sum -c CHECKSUMS` | receipt::scenario_3_checksums | MUST_CLOSE |
| 5 | `install.sh --version <TAG> --editor none` exits 0 (real CDN) | G8, G9 | R8 | `bash /tmp/install.sh --version ... --editor none` (no `SDDK_BASE_URL` override) | receipt::install_exit (field in receipt JSON) | MUST_CLOSE |
| 6 | `sddk dev doctor --prefix $HOME/.local` → `all_present: true` | G8, G12 | R8 | `jq '.all_present' == "true"` + `jq '.binary.bundle_coherence' == "present"` | receipt::doctor_exit + JSON snippet in log | MUST_CLOSE |
| 7 | Canonical workflow (intake/verify/debverify/alignment/advisory) all exit 0 | G9, G12 | R1 | `sddk <step> --format json` loop → exit code == 0 for each | receipt::workflow_exits (array in receipt) | MUST_CLOSE |
| 8 | Restart scenario: kill+relaunch → same workflow succeeds | G2, G3 | R1 | `pkill sleep infinity` + re-exercise intake+verify → exit 0 | receipt::restart_scenario_exit | MUST_CLOSE |
| 9 | Projection rebuild equivalence: delete + rebuild → digest equality | G3 | R1 | digest before == digest after (jq `projects[0].digest`) | receipt::rebuild_digest_eq | MUST_CLOSE |
| 10 | Upgrade-to-next-version N/A step: structured line + exit 0 | G8 | — | `gh release list --limit 1` → echo "N/A — no subsequent release at run time" | receipt::upgrade_na_justification | N/A |
| 11 | Rollback to prior certified version succeeds | G15 | R15 | `install.sh --version <prior>` → doctor coherent → `sddk --version` == prior tag | receipt::rollback_exit + `ROLLBACK_TO` + `rollback_version` in log | MUST_CLOSE |
| 12 | CI workflow runs `clean-machine-uat` job in container | G8 | R8 (process) | `.github/workflows/clean-machine-uat.yml` committed + `act` parseable | workflow YAML at canonical path | MUST_CLOSE |
| 13 | Shim invokes act with correct workflow path | G8 | R8 (process) | `bash tests-e2e/clean-machine/run.sh` → `act ... -W .github/workflows/clean-machine-uat.yml` | shim exit == act exit | MUST_CLOSE |
| 14 | Receipt JSON written under cycle artifacts | G7, G16 | R7 | `clean-machine-uat-receipt.json` exists + `jq .` valid + required keys | receipt file present (`.sddk/cycles/.../clean-machine-uat-receipt.json`) | MUST_CLOSE |
| 15 | Debt disposition §3.1 updated → CLOSED_A5 | G14 | R13, R18 | `A5-DEBT-DISPOSITION.md` §3.1 row for UAT-1 → **CLOSED_A5** + pointer to receipt | A5-DEBT-DISPOSITION.md §3.1 diff | CLOSED |
| 16 | Risk register R8/R15/R11 → CLOSED | G14 | R13, R18 | `A5-RISK-REGISTER.md` R8 → **CLOSED**, R15 → **CLOSED**, R11 → **CLOSED** | A5-RISK-REGISTER.md diff | CLOSED |

## §2 Gate Mapping

| Gate | Meaning | Rows |
|------|---------|------|
| G2 | Restart survival (projection durability) | #8 |
| G3 | Projection rebuild equivalence | #9 |
| G7 | Receipt persistence as durable evidence | #14 |
| G8 | Fresh-machine distribution (no repo dependency) | #2, #3, #4, #5, #6, #10, #12, #13 |
| G9 | Binary runs outside the repo | #1, #5 |
| G12 | Canonical workflow exercised | #1, #6, #7 |
| G14 | Disposition bookkeeping | #15, #16 |
| G15 | Rollback to prior certified version | #11 |
| G16 | Post-release state verification | #14 |

## §3 Risk Mapping

| Risk | Description | Rows |
|------|-------------|------|
| R1 | Workflow restart / projection rebuild | #7, #8, #9 |
| R7 | Evidence chain (receipt) | #14 |
| R8 | Fresh-machine distribution (process) | #1, #2, #3, #4, #5, #6, #12, #13 |
| R10 | Release pipeline completeness | (M7: handled by `scripts/release.sh` 14-step pipeline) |
| R13 | Disposition bookkeeping completeness | #15, #16 |
| R15 | Rollback correctness | #11 |
| R18 | Risk register accuracy | #15, #16 |

## §4 Receipt Schema

The receipt at
`.sddk/cycles/p-63676b11dc0ef88f/a5-5-clean-machine-sweep/clean-machine-uat-receipt.json`
has the following required fields:

| Field | Type | Description |
|-------|------|-------------|
| `tag` | string | Resolved tag tested (e.g. `v1.169.86`) |
| `binary_sha256` | string | SHA-256 of the `sddk` binary asset |
| `install_exit` | integer \| null | Exit code of `install.sh` |
| `doctor_exit` | integer \| null | Exit code of `sddk dev doctor` |
| `rollback_from` | string | Tag before rollback (same as `tag`) |
| `rollback_to` | string | Prior tag rolled back to (e.g. `v1.169.85`) |
| `rollback_exit` | integer \| null | Exit code of rollback install |
| `assertions_passed` | integer | Count of passing assertions |
| `assertions_total` | integer | Total assertions run |
| `scenario_timings` | object | Map of `scenario_N_name` → elapsed seconds |

## §5 Closed Items

After a green M5 run, the following are **CLOSED_A5** / **CLOSED**:

- UAT-1 (fresh-machine distribution) — certified by receipt
- R8 (fresh-machine mechanism-pending gap) — closed by #1-#6, #12, #13
- R15 (rollback mechanism) — closed by #11
- R11 (projection rebuild / restart) — closed by #8, #9

## §6 Out of Scope

| Item | Reason |
|------|--------|
| R12 async-Parallel | POST-BASE feature; tracked separately |
| R14 (secrets) | Tracked separately; not in A5-5 budget |
| R17 (operator diagnostics) | Partial mitigation only; harness produced, not new commands |
| UAT-2 / UAT-3 / UAT-4 | Not in A5-5 scope; UAT-1 only |
