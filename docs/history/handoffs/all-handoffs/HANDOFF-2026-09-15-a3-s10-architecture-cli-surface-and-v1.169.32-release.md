# HANDOFF — A3-S10 / architecture CLI surface + v1.169.32 release

- **Date:** 2026-09-15
- **Cycle:** `p-63676b11dc0ef88f/a3-10-ac-cli-surface` (A-lite)
- **Status:** **CLOSED** (sequence 14)
- **Release:** `v1.169.32` — https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.32
- **HEAD:** `8ab20cc` (== `origin/main`)
- **Binary / bundle / framework:** all `1.169.32`

## What shipped

The CLI/agent surface deferred across every AC4–AC8 cycle (`12-CLI-AGENT-UX.md`),
plus the operator-visible artefact `10-UAT.md` requires:

```text
sddk architecture receipt [--root <p>] [--contracts <rel>] [--format text|json]
```

| Layer | Deliverable |
|---|---|
| engine | `architecture_declaration/` (~660 LoC): `DeclarationFile` + fail-closed `validate` → AC1 contracts + AC2 units |
| cli | `architecture_cmd.rs` + top-level `architecture` command + CommandRegistry entry |
| tests | 11 unit + 9 e2e |

### Behaviour (verified on the installed binary)

```
$ sddk architecture receipt --root /tmp/acdemo        # clean declaration
verdict: pass        audited_contracts: 2      exit 0

$ sddk architecture receipt --root /tmp/acdemo2       # duplicate authority
verdict: blocked
historical_class_coverage: duplicate_authority  reproduced
unresolved_must_findings:  shadow_authority subjects=["comp:auth"] waivers=[]
exit 1

$ sddk architecture receipt --root /tmp/empty         # no declaration
stderr: cannot read declaration ...             exit 2
```

Declaration format: `.sddk/architecture/contracts.yaml` — `revision`, optional
`knowledge_basis`/`waivers`, `units[]`, `contracts[]` with the five declarable
kinds (`single_authority`, `unique_owner`, `forbidden_dependency`,
`projection_only`, `bounded_compatibility`).

## ADR

`ADR-0120-DECLARATIVE-ARCHITECTURE-CONTRACTS` (accepted, mirrored; 27 in vault):
declaration files are **input, not authority**; the engine stays format-agnostic
(CLI owns YAML); validation is fail-closed; the verdict is closed and score-free.

## Three golden gates fired — all satisfied by regeneration

Adding a top-level command changes the CLI surface, and this repo guards that
surface three independent ways:

| Gate | Fixture | Diff |
|---|---|---|
| `agent_surface_golden` | `crates/sddk-cli/tests/fixtures/agent-surface.golden.json` | `total` 50→51 + 1 entry |
| `cli_compatibility` | `crates/sddk-cli/tests/fixtures/cli/help-top-level.txt` | +1 line |
| `cli_golden` | `docs/architecture/tests/fixtures/cli_golden/1.168.8/sddk-help.txt` | +1 line |

Every diff was inspected and is exactly the new command. The first two use
`UPDATE_SNAPSHOTS=1`; the third uses the documented `sddk --help 2> <fixture>`.
`cli_golden/1.151.3/` is historical and untested.

## Also fixed during apply (found by the smoke test)

`claim_results` comes from AC4's **change-scoped** delta, so a global run
reported `contracts: 0` on a repository that declared several. The receipt now
carries `audited_contracts` (from AC5's global pass) and the CLI prints
`audited_contracts` plus `affected_contracts` labelled as change-scoped.

## Gates (all green)

explore · specify · design · build · verify (4) · release (2) · archive (2) = 12.

## Evidence

```
cargo test --workspace                                 -> 200 blocks, 4183 passed, 0 failed
cargo test -p sddk-engine --lib architecture_declaration:: -> 11 passed
cargo test -p sddk-cli --test architecture_cli_e2e       -> 9 passed
cargo test -p sddk-cli --test context_fitness            -> 7 passed
cargo fmt --check / clippy (engine + cli, -D warnings)    -> clean
sddk dev doctor -> c4.authority_single_admission: present
bash scripts/release.sh --skip-tests                     -> exit 0 (14 steps, 213s)
```

## Commits

| SHA | Subject |
|---|---|
| spec | docs(spec): cycle-bounded spec for A3-S10 (architecture CLI surface) |
| impl | feat(cli): sddk architecture receipt + declarative contracts + ADR-0120 |
| golden1 | chore(cli): regenerate agent-surface golden for the architecture command |
| golden2 | chore(cli): regenerate top-level help snapshot for the architecture command |
| golden3 | chore(cli): regenerate cli_golden top-level help fixture for architecture |
| `8ab20cc` | chore(release): bump version 1.169.31 -> 1.169.32 |

## Carry-over debt

None.

## Next

1. **`sddk architecture --changed`** — git-diff → affected units, giving the
   change-scoped delta a real basis (currently always empty).
2. **The seven read surfaces** from `12-CLI-AGENT-UX.md`
   (`architecture contracts|authorities|ownership|compatibility|paradigms|behavior-map|graph`).
3. **AC6/AC7 data as declaration inputs** so `missing_negative_evidence`
   reproduces on CLI runs instead of reporting `not_reproduced`.
