# Deprecated Patterns Lint Registry

> ADR-0001 §3.2 (`acceptance_for_m9_blocking_enforcement`): a lint
> promoted to `default: deny` MUST report **zero hits** against the live
> workspace. Promoting a lint with hits is a contract violation.

## Files

| File | Purpose |
|---|---|
| `deprecated_patterns.toml` | The registry: 9 lints, each with `default: deny\|allow`, `severity`, `description`, `explanation`, etc. |

## Promotion lifecycle (per ADR-0001)

1. **Introduce** with `default: allow` (advisory).
2. **Audit**: every advisory lint carries a populated `explanation` block
   with the four required sub-keywords (Hits, Verdict, Reason, Unblock).
3. **Promote** to `default: deny` when:
   - **validation_per_lint** ✅ — zero hits against the live workspace
     (excluding paths explicitly allowed via `exclude_paths`)
   - **migration_complete** ✅ — the canonical replacement is implemented
     and live (no remaining `proposed` ADRs gating the replacement)
   - **user_signoff** ✅ — user acknowledgement (continuous auto-mode
     counts as sign-off in this repo per AGENTS §4)
4. **Regression-guard** by `tests/test_deny_lint_zero_hits.sh`
   (wired into `scripts/release.sh` step 1b).

## Current state (v1.168.37)

| Lint ID | Default | Hits | Blocker |
|---|---|---|---|
| `agent_result_used` | deny | 0 | — |
| `asset_deprecated_namespace` | deny | 0 | — |
| `asset_raw_store_reference` | deny | 0 | — |
| `asset_authority_language` | deny | 0 | — |
| `orchestration_synthesis_no_dissent` | deny | 0 | — (v1.168.37) |
| `evidence_kind_v1` | allow | 22 | cycle-9 per-call-site migration (ADR-0100 construction closed v1.168.35) |
| `transition_outcome_used` | allow | 24 | description correction + re-categorization |
| `execution_outcome_as_synthesis` | allow | 0 | corpus expansion (organic, no dedicated cycle) |
| `asset_unregistered_cli_example` | allow | 0 | regex unsafe-by-design (first-letter-sieve allows legitimate commands) |

## Pin tests

| Test | Purpose | Wired in |
|---|---|---|
| `tests/test_advisory_lint_explanations.sh` | Every `default: allow` lint has a populated `explanation` block with the four sub-keywords | release.sh step 1b (shellcheck + dynamic) |
| `tests/test_deny_lint_zero_hits.sh` | Every `default: deny` lint reports zero hits against the live workspace | release.sh step 1b (shellcheck + dynamic) |
