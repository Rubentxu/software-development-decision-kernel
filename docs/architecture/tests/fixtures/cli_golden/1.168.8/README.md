# CLI golden fixtures — v1.168.8 snapshot

> **Snapshot:** 2026-09-11 (`v1.168.8`).
> Captures deterministic `--help` output for 10 representative commands
> exercising the critical paths of the SDDK facade, including the three
> surfaces added since v1.151.3 (graph/cockpit M8, lint M9 enforcement
> infra, ledger watch M9.5).

## Captured fixtures

| File | Source command | Lines |
|---|---|---|
| `sddk-help.txt` | `sddk --help` | 54 |
| `sddk-cycle-help.txt` | `sddk cycle --help` | 24 |
| `sddk-plan-help.txt` | `sddk plan --help` | 52 |
| `sddk-run-help.txt` | `sddk run --help` | 19 |
| `sddk-dev-help.txt` | `sddk dev --help` | 29 |
| `sddk-dev-graph-help.txt` | `sddk dev graph --help` | 13 |
| `sddk-dev-cockpit-help.txt` | `sddk dev cockpit --help` | 14 |
| `sddk-dev-lint-help.txt` | `sddk dev lint --help` | 10 |
| `sddk-ledger-help.txt` | `sddk ledger --help` | 17 |
| `sddk-ledger-watch-help.txt` | `sddk ledger watch --help` | 62 |

## What changed since 1.151.3 (evolution trail)

Top-level additions (visible in `sddk --help`):

- `memory` (SPEC-004 decision memory) — first-class subcommand tree.
- `agent-help` (M7.1, SPEC-015 agent-facing cheat sheet).
- `plan change`, `plan verify`, `plan audit`, `plan config`,
  `plan introspect`, `plan target` (M6.1/M6.2 facade + DAG registry).

New `dev` surfaces:

- `dev graph` (M8.0–M8.2 Active Graph + WHY engine + Cockpit Views).
- `dev cockpit` (M8.3–M8.10 observability, diff, digest, diff-watch).
- `dev lint` (v1.168.8 M9 enforcement infrastructure — deprecated patterns).

`plan --help` grew from 25 to 52 lines (M6.x facade commands + flags).

## Automated regression test

`crates/sddk-cli/tests/cli_golden.rs` compares the just-compiled binary's
`--help` output against these fixtures (trailing-whitespace normalized).
A failure means the CLI surface drifted from the last consciously-blessed
snapshot — either regenerate the fixture (intentional change) or revert
(unintentional regression).

## Regeneration

```bash
DEST=docs/architecture/tests/fixtures/cli_golden/1.168.8
sddk --help > $DEST/sddk-help.txt 2>&1
sddk cycle --help > $DEST/sddk-cycle-help.txt 2>&1
sddk plan --help > $DEST/sddk-plan-help.txt 2>&1
sddk run --help > $DEST/sddk-run-help.txt 2>&1
sddk dev --help > $DEST/sddk-dev-help.txt 2>&1
sddk dev graph --help > $DEST/sddk-dev-graph-help.txt 2>&1
sddk dev cockpit --help > $DEST/sddk-dev-cockpit-help.txt 2>&1
sddk dev lint --help > $DEST/sddk-dev-lint-help.txt 2>&1
sddk ledger --help > $DEST/sddk-ledger-help.txt 2>&1
sddk ledger watch --help > $DEST/sddk-ledger-watch-help.txt 2>&1

# Verify diffs:
git diff --stat docs/architecture/tests/fixtures/cli_golden/
```

## Acceptable diff triggers

A diff that is **not** a regression:

- New command or subcommand added.
- New option or flag on an existing command.
- Reordering of options alphabetically.

A diff that **is** a regression (requires review):

- Removed command or option.
- Renamed command or option without crosswalk.
- Spelling or formatting changes without intent.

## Forward compatibility

Future cycles that change the CLI add a sibling `1.X.Y/` directory rather
than overwriting this one, preserving the evolution trail, and point
`FIXTURE_DIR` in `crates/sddk-cli/tests/cli_golden.rs` at the new snapshot.
