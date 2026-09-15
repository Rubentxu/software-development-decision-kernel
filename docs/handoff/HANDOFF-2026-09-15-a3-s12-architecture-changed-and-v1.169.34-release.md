# HANDOFF — A3-S12 `architecture --changed` + v1.169.34 release

- **Date:** 2026-09-15
- **Cycle:** `p-63676b11dc0ef88f/a3-12-architecture-changed` (A-lite)
- **Status:** **CLOSED** (sequence 15)
- **Release:** `v1.169.34` — https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.34
- **HEAD:** `e55cd95` (== `origin/main`)
- **Binary / bundle / framework:** all `1.169.34`

## What shipped

One verb and one flag: `sddk architecture receipt --changed [--base <rev>]`.

```text
change_basis:      base=<sha> changed_units=1        # scoped
change_basis:      (global run; no --changed)        # global
change_basis:      base=<sha> changed_units=0        # scoped, nothing declared touched
```

The receipt gains `ChangeBasis { base, changed_units }`, absent on a global run.
The three states above are deliberately distinguishable: a global run and a
scoped run with no declared impact are different facts, and the renderer says so
rather than showing an ambiguous `0`.

## Base resolution is fail-closed

`--base <rev>`, then `origin/main`, then `HEAD~1`. If none resolves, or the named
revision is unknown, or the directory is not a git repo: **exit 2, no receipt**.
It never degrades to an empty basis. Silently reporting "nothing changed"
because git failed is the one failure mode that would make the eventual
`verify architecture --changed` a no-op that reports success.

Exit codes: `0` pass, `1` blocked verdict (receipt still emitted — an
evidence-free contract is truthfully `Unknown`), `2` usage or resolution error.

## Linkage claims come from AC1's evaluator

AC4 discovers contracts through `ArchitectureClaimedBy` relations, so scoping
needs a claim. The CLI attaches a genuine AC1 claim —
`ContractEvaluation::evaluate(contract, Vec::new(), now, evaluator, None)` — for
every (contract, subject-unit) pair over `SingleAuthority` (component) and
`UniqueOwner` (entity) subjects. Empty evidence yields `Unknown`, which is the
truthful state: a declaration asserts a contract, it does not evidence it. No
`ArchitectureClaim` is hand-built.

One subtlety: `find_contracts_for_unit` returns a *synthesized* contract-anchor
`NodeId`, and AC4 resolves it through the anchor node's `contract_id` prop,
written only by `add_contract_metadata`. The linkage claim alone is not enough;
both are required, or the delta fails with
`contract anchor ... has no contract_id prop`.

## Verification found two silent false-cleans

The acceptance suite was green at the build gate, so verify asked the question
it cannot ask: *for which inputs does `--changed` report a false clean?* Both
defects were in the **seam between `git diff` and `path_overlaps`**, which no
acceptance test reached (the fixture used `src/a.rs` / `src/b.rs`).

**1. Quoted paths (REQ-A3S12-010).** `git diff --name-only` quotes any path with
a byte outside ASCII, so a changed `café/y.rs` arrives as
`"caf\303\251/y.rs"`, matches no locator, and the basis came back **empty** for a
file that did change. Fixed by reading the diff NUL-separated (`-z`). Also
removed a latent bug: `.lines()` + `.trim()` was trimming whitespace off real
path names.

**2. Rename detection (REQ-A3S12-011).** Git detects renames by default, so a
move reports only the destination and the unit that lost its entire source was
never scoped — despite its `SingleAuthority` contract now being vacuous and its
`UniqueOwner` likely violated. Fixed with `--no-renames`: a move splits into a
delete plus an add and scopes both sides.

Both fixes are the fail-closed direction (more units scoped means more contracts
required to carry evidence) and both are promoted to spec requirements. Four
tests cover the seam; `detection_would_miss_the_renamed_away_side` asserts git's
own behaviour, so a future change to git's rename default fails loudly rather
than silently un-fixing the code.

## Nothing else moved

`--changed` and `--base` are args on an existing subcommand, so the top-level
help snapshot, the blessed `cli_golden` fixture and the agent-surface JSON are
byte-identical. The AC CLI surface stopped growing where A3-S11 predicted.

| Gate | Outcome |
|---|---|
| `cli_golden` | unchanged |
| `cli_compatibility` (help snapshot) | unchanged |
| `agent_surface_golden` | unchanged |
| `fmt --check` / `clippy -D warnings` | clean |
| `cargo test --workspace` | 202 blocks / **4215 passed** / 0 failed |

Baseline was 201 / 4199. Delta: +1 block (new e2e file), +16 tests (1 engine,
3 CLI unit, 12 e2e). Remediation round 1 added 4 of those tests.

## Cycle artifacts

`.sddk/cycles/p-63676b11dc0ef88f-a3-12-architecture-changed/` and the XDG mirror
under `~/.local/share/sddk/projects/p-63676b11dc0ef88f/cycle-artifacts/...`:

| Artifact | |
|---|---|
| `exploration-report.md` | |
| `design-note.md` | |
| `implementation-receipt.md` | incl. the full remediation-round-1 section |
| `verification-report.md` | the adversarial probes and both defects |
| `merge-receipt.json` | 4 commits, base `013ad8e` → head `e55cd95` |
| `release-receipt.json` | pipeline, hashes, smoke results |
| `archive-manifest.json` | closure |

## Zero carry-over debt

No new debt entries. Both verify findings were fixed in-cycle rather than
recorded: each was a silent false-clean in a gate whose whole purpose is to
prevent silent false-cleans, so deferring would have shipped the very failure
the AC track exists to detect.

## One observation promoted to the next session (not debt)

`sddk dev doctor --prefix <framework-root>` loads `<prefix>/sddk-install.json`.
On a split-prefix install (binary in `~/.local/bin`, bundle in the data dir) the
data-dir root still holds a **stale pre-v2 receipt** (v1.145.1 as of today), so
that invocation reports `binary.bundle_coherence: missing` and
`all_present: false` on a perfectly healthy install. With the real install
prefix it reports `present` and `all_present: true`. Pre-existing, out of
A3-S12's scope, deliberately not carried as debt — recorded here so the next
session does not misread it as a release regression, as this session nearly did.

## Next

1. **`verify architecture [--changed]`** — the consumer of this basis, and the
   reason A3-S12 exists. The change-scoped delta (`claim_results`, `unknowns`)
   is already in the receipt; the verb needs to gate on it and emit its own
   receipt.
2. `architecture paradigms` (needs AC3 declaration input).
3. `architecture behavior-map` (needs generation semantics).
