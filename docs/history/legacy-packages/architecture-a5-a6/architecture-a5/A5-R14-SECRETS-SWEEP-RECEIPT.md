# R14 Secrets Sweep Receipt — log / receipt / error / telemetry / CAS

> Cycle: `r14-secrets-sweep-2026-09-20` · Release: v1.169.119
> Risk: **R14** (`docs/architecture/a5/A5-RISK-REGISTER.md` §2) — "Secret
> leaks into log / receipt / error / telemetry / CAS", P0 severity,
> OPEN_NON_BLOCKER, gate G11 "NOT VERIFIED with R14 accepted".

## 0. Verdict

**R14: VERIFIED — no leak path found; one latent gap closed
(hardening), one tooling gap closed (push-time secret screen already
existed; repo-config scan documented).**

G11 remains scoped: this is a single-machine source audit, not a
dedicated adversarial security review. R14 moves from
`OPEN_NON_BLOCKER` to `SWEPT (source-level)`; the residual is
re-swept at each release via the existing mechanisms listed in §5.

## 1. Surfaces audited (the R14 matrix)

| # | Surface | Entry point | Finding | Evidence |
|---|---------|-------------|---------|----------|
| 1 | Env → subprocess | `sddk-gateway` env allowlists (`computer_use_env`, `semantic_env`, `browser_env`, `git_capability_env`, `current_env_as_allowlist`) | CLOSED allowlists only; `GH_TOKEN`/`GITHUB_TOKEN` explicitly excluded from git capability env (`LOCAL_GIT_ENV_KEYS`, pinned by `LOCAL_GIT_ENV_KEYS_must_not_contain_*` tests); `is_secret_like` drops suffix-matching keys | `crates/sddk-gateway/src/git.rs:11-38`, `test_runner/mod.rs:59-83` |
| 2 | Receipt persistence (request/result) | `CapabilityGateway` → `Storage::{begin,finalize}_capability_receipt` | All 4 persistence call sites route through `redact()` (key-level) + `redact_text()` (string-level) | `crates/sddk-gateway/src/gateway.rs:247,268,350,387` |
| 3 | String-level scan coverage | `STRING_LEVEL_KEY_PATTERN` | **LATENT GAP CLOSED**: only `stdout`/`stderr` were scanned; a credential embedded in `error`/`message`/`reason` passed verbatim into the persisted receipt. Extended to all 5 containers + 3 regression tests | `crates/sddk-gateway/src/lib.rs:224` (this cycle) |
| 4 | Log / tracing | workspace-wide grep | **NO logger in the dependency graph** (no `tracing`, `env_logger`, `log` crates in non-dev deps); no `println!`/`eprintln!` interpolating env/request values in engine or gateway | grep receipts in cycle log |
| 5 | Telemetry | `sddk telemetry ingest` → central plane | `MetricsRecord` is a typed scalar schema (ids, durations, counts, verdicts) — no free-form text field exists to smuggle a credential | `crates/sddk-domain/src/metrics.rs:8-47` |
| 6 | CAS | `insert_evidence_attachment` → `cas_put` | CAS stores caller-supplied bodies by content hash; no env/secret read occurs on the CAS path. Content responsibility stays with the caller (same posture as A5 cycle audit); no new mechanism warranted | `crates/sddk-storage/src/lib.rs` |
| 7 | Repo config files | `.yml/.yaml/.json/.toml/.sh` scan | Only hits are the documented synthetic canaries in `tests/test_push_prevention_hook.sh` (test fixtures for the secret screen itself) | §4 below |

## 2. Change made (the only code change of this cycle)

`STRING_LEVEL_KEY_PATTERN` extended from `["stdout", "stderr"]` to
`["stdout", "stderr", "error", "message", "reason"]`. Rationale: these
are the free-form string containers that cross the receipt boundary; a
credential embedded in an error message or a proposal reason is the
same leak class as one in stdout. Non-matching text passes verbatim
(diagnostic value preserved).

## 3. Falsification

- `redaction_masks_secrets_embedded_in_error_message` (canary
  `ghp_…` inside `error`) — PASS
- `redaction_masks_secrets_embedded_in_reason` (canary inside
  `reason`) — PASS
- `redaction_leaves_plain_errors_verbatim` (no false positive on
  `message`) — PASS
- Pre-existing battery re-run: gateway 179 pass / 0 fail (includes
  `sec1_redactor_unit`, `sec1_capability_receipt_redaction`,
  `runner_receipt_e2e`, `properties`).

## 4. Repo config scan

```sh
grep -rInE '(ghp_[A-Za-z0-9]{20,}|github_pat_…|AKIA…|xox[baprs]-…|sk-…)' \
  --include='*.yml' --include='*.yaml' --include='*.json' \
  --include='*.toml' --include='*.sh' .
```

7 hits, all inside `tests/test_push_prevention_hook.sh` — the
falsification fixtures for the pre-push secret screen (canaries by
construction, never real credentials). No hits elsewhere.

## 5. Residual / re-sweep mechanics

- Push-time screen (`githooks/pre-push` → `file_contains_secret_pattern`)
  re-filters every docs-only and cycle-artifacts push.
- `release.sh` step 1 (`cli_dev_install_accepts_committed_manifest`)
  and the manifest gate re-validate the published tree at every release.
- The remaining honest residual: this was a static, single-audit pass.
  A dedicated adversarial cycle (fuzzed receipts, hostile workflow
  definitions) remains POST-BASE scope if ever requested.

## 6. Disposition

R14 status in `A5-RISK-REGISTER.md`: `OPEN_NON_BLOCKER` →
`SWEPT_SOURCE_LEVEL (v1.169.119)`. G11 note in
`A5-CURRENT-ROADMAP.md` unchanged in substance (still "NOT VERIFIED"
for adversarial purposes), now referencing this receipt.
