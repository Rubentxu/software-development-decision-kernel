# CLI golden fixtures — M0 D2

> **Snapshot:** 2026-09-09 (`v1.151.3` baseline).
> Captures deterministic `--help` output for the 5 representative top-level
> subcommands that exercise the critical paths of the SDDK facade.

## Captured fixtures

| File | Source command | Lines |
|---|---|---|
| `sddk-help.txt` | `sddk --help` | 46 |
| `sddk-cycle-help.txt` | `sddk cycle --help` | 24 |
| `sddk-plan-help.txt` | `sddk plan --help` | 25 |
| `sddk-run-help.txt` | `sddk run --help` | 8 |
| `sddk-dev-help.txt` | `sddk dev --help` | 25 |

## Why only 5 (not the full 40 from CLI-CROSSWALK.md)

Per `propose-manifest.md` T-2, the full suite targets 45 fixtures (40 top-level
commands + 5 representative subcommands). M0 D2 captures the **subset that
exercises the critical CLI surface** without requiring an automated test
runner. The remaining 40 fixtures are a follow-up cycle that wires
`crates/sddk-cli/tests/cli_golden.rs` and the runner framework.

## Regeneration

```bash
# Manual regeneration (CI run):
for cmd in "" " cycle" " plan" " run" " dev"; do
  name="sddk$(echo "$cmd" | tr ' ' '-')-help.txt"
  sddk ${cmd} --help > "docs/architecture/tests/fixtures/cli_golden/1.151.3/${name}" 2>&1
done

# Verify diffs:
git diff --stat docs/architecture/tests/fixtures/cli_golden/
```

## Whitelisted dynamic fields

The following fields are excluded from baseline-diff comparison:

- Timestamps (none in `--help` output).
- Build metadata (none in `--help` output).
- Resolved framework version path (only present in `sddk version`).

These do not affect the fixtures in this directory.

## Acceptable diff triggers

A diff that is **not** a regression:

- New command added to a subcommand (e.g., a new `--format` flag on `sddk plan`).
- New option to an existing command.
- Reordering of options alphabetically.

A diff that **is** a regression (requires review):

- Removed command or option.
- Renamed command or option without crosswalk.
- Spelling or formatting changes.

## Forward compatibility

This directory uses `1.151.3` as the snapshot directory name. Future cycles
that change the CLI add a sibling `1.152.0/` etc. directory rather than
overwriting, preserving the evolution trail.

## Out of scope (deferred)

- The 35 remaining top-level subcommands (per CLI-CROSSWALK.md).
- The `cli_golden` Rust test runner (`crates/sddk-cli/tests/cli_golden.rs`).
- Subcommand-level `--help` snapshots (e.g., `sddk cycle start --help`).

These belong to a follow-up M0+ cycle and are not on the M0 critical path.