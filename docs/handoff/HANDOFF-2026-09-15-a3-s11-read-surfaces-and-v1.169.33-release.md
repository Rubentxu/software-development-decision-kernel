# HANDOFF — A3-S11 architecture read surfaces + v1.169.33 release

- **Date:** 2026-09-15
- **Cycle:** `p-63676b11dc0ef88f/a3-11-architecture-read-surfaces` (A-lite)
- **Status:** **CLOSED** (sequence 14)
- **Release:** `v1.169.33` — https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.33
- **HEAD:** `3ec0e11` (== `origin/main`)
- **Binary / bundle / framework:** all `1.169.33`

## What shipped

Five of the seven `12-CLI-AGENT-UX.md` inspection commands, complementing the
A3-S10 receipt verb:

```text
sddk architecture contracts      every declared contract (id/kind/subject/rev)
sddk architecture authorities    SingleAuthority grouped by component
sddk architecture ownership      UniqueOwner grouped by entity
sddk architecture compatibility  BoundedCompatibility + window_status
sddk architecture graph          units + relations + digest, --scope filter
```

All read-only, all `--format text|json`, all fail closed (exit 2), none emits a
verdict or a score.

## Declaration v2: `relations[]`

```yaml
relations:
  - from: comp:graph
    to: comp:canonical-event-log
    kind: reads
```

Nine declarable kinds; the five system-produced semantic kinds are rejected.
Both endpoints must name declared units (fail-closed).

### Why this was load-bearing

`dependency_boundary` was permanently `not_reproduced` on CLI runs: AC5's
`AuthorityBypass` detector needs a live edge and the declaration could not state
one. Now it can, verified by `acceptance_declared_bypass_is_detected` (engine)
and `architecture_declared_bypass_reaches_the_receipt` (CLI).

## Two real issues found and fixed during apply

1. **Unreproducible dependency boundary** (above).
2. **A dishonest clock.** The commands pinned `now = 0`, so every elapsed
   compatibility window reported `open`. They now default to the wall clock and
   accept `--now-ms` to pin it for reproducible runs.

Also cleaned `Debug`-wrapper leakage (`Revision("rev:1")`,
`SoftwareUnit(SoftwareUnitRef("..."))`) out of the rendered surfaces.

## Four surface gates fired

| Gate | Outcome |
|---|---|
| `agent_surface_golden` | regenerated: `total` 51 → 56, exactly the five new entries |
| `cli_compatibility` (help snapshot) | **unchanged** — nested subcommands do not alter the root surface |
| `cli_golden` (blessed help) | **unchanged** |
| `command_spec_tests` (spec ↔ clap sync) | fired: dotted names were wrong; renamed to space-separated `"architecture contracts"` etc. |

The two unchanged gates confirm the A3-S10 golden work is stable for nested
additions.

## One transient failure, investigated

`cli_dev_install_default_layout_is_executable_and_verify_passes` failed once with
`ExecutableFileBusy: "Text file busy"` — an environment race, unrelated to this
change. Re-run in isolation: passes. Final workspace run is green.

## Gates (all green)

explore · specify · design · build · verify (4) · release (2) · archive (2) = 12.

## Evidence

```
cargo test --workspace                                  -> 201 blocks, 4199 passed, 0 failed
cargo test -p sddk-engine --lib architecture_declaration:: -> 17 passed (was 11)
cargo test -p sddk-cli --test architecture_read_cli_e2e      -> 10 passed
cargo test -p sddk-cli --test architecture_cli_e2e           -> 9 passed
cargo test -p sddk-cli --test command_spec_tests             -> 13 passed
cargo fmt --check / clippy (engine + cli, -D warnings)        -> clean
sddk dev doctor -> c4.authority_single_admission: present
bash scripts/release.sh --skip-tests                     -> exit 0 (14 steps, 197s)
```

Installed-binary smoke: all five surfaces render; `graph --scope crates/`
filters units.

## Carry-over debt

None.

## Next

1. **`sddk architecture --changed`** — git-diff → affected units, giving the
   change-scoped delta a real basis (still always empty).
2. **`architecture paradigms`** — needs AC3 paradigm declarations in the schema.
3. **`architecture behavior-map`** — needs generation semantics (the doc says it
   SHOULD be generated, not handwritten).
4. **AC6/AC7 data as declaration inputs** so `missing_negative_evidence`
   reproduces on CLI runs.
