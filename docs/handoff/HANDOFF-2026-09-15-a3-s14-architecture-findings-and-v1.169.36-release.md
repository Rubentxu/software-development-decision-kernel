# HANDOFF — A3-S14 architecture findings + v1.169.36 release

- **Date:** 2026-09-15
- **Cycle:** `p-63676b11dc0ef88f/a3-14-architecture-findings` (A-lite)
- **Status:** **CLOSED** (sequence 15)
- **Release:** `v1.169.36` — https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.36
- **HEAD:** `a6bafe3` (== `origin/main`)
- **Binary / bundle / framework:** all `1.169.36`

## What shipped

```text
sddk architecture findings [--kind <tag>] [--format json|text]
```

The sixth read surface. `sddk architecture receipt` already ran AC5's global audit
on every invocation and kept only a count per kind:

```text
audit_findings: { "shadow_authority": 1, "stale_compatibility": 1 }
```

So `severity`, `contract_ids` and `message` were computed and discarded. The
receipt said `shadow_authority: 1, subjects: ["comp:b"]` and never said that
`c-auth-1` and `c-auth-2` were the contracts in conflict — the actionable fact.
It also could not express that a shadow authority is `Critical` while a stale
window is `Medium`: both arrived with the same shape.

Now:

```text
architecture findings — revision smoke, now=5000
  shadow_authority      critical  subjects=[comp:dup]                     contracts=[auth-1, auth-2]
    2 contracts claim SingleAuthority over component `comp:dup`
  missing_owner         high      subjects=[comp:nowhere]                 contracts=[orphan]
    contract `orphan` names component `comp:nowhere` which is absent from the graph
  authority_bypass      critical  subjects=[comp:domain -> comp:provider] contracts=[no-edge]
    forbidden dependency `comp:domain -> comp:provider` is present in the graph
  stale_compatibility   medium    subjects=[window]                       contracts=[window]
    compatibility window for `window` elapsed at 500 with no replacement
  contradiction         high      subjects=[comp:dup]                     contracts=[auth-1, auth-2, proj-1]
    subject `comp:dup` is declared both authority-owned and projection-only
  5 finding(s) over 6 audited contract(s)
```

## Naming: no `deb-verify` verb

`12-CLI-AGENT-UX.md` groups `sddk deb-verify architecture` with
`sddk verify architecture` under Verification. Neither name was adopted:

- `receipt` **runs and gates on** the audit (mandatory findings drive `Blocked`).
- what was missing is **inspection** — the shape the five A3-S11 read surfaces
  already have.

A `deb-verify` verb would duplicate `receipt` (runs it) and `findings` (prints
it), against §2.7/§2.9; an alias is forbidden (§2.0). Same disposition as A3-S13,
extending ADR-0120's addendum.

## Design decisions worth keeping

| Decision | Why |
|---|---|
| `--kind` tags derived from the closed enum | a new kind is filterable the moment it exists; the accepted set in the error cannot go stale |
| Unknown `--kind` → exit 2 | an empty listing is indistinguishable from "this declaration is clean" |
| **No `--severity`** | severity is a pure function of kind, so it is a second way to ask one question (still reported per finding, since that is what you triage on) |
| Drift risk removed **structurally** | both surfaces call `load_declaration` + `declare_overlay` + `run_debverify_audit` unchanged, no preprocessing; the invariant is then asserted from outside at five clocks |
| Wall clock by default, `--now-ms` to pin | `stale_compatibility` is time-dependent; a pinned `now = 0` would report every elapsed window as open |

## Verification found no defects

Six probes across three failure groups (it lies / it is unstable / it is brittle),
all passing:

| # | Probe | Result |
|---|---|---|
| 1 | two identical invocations | byte-identical |
| 2 | agreement with `receipt` at 5 clocks (100, 500, 501, 5000, 9999999999) | agree everywhere |
| 3 | `--kind` case/format variants | all exit 2, no listing |
| 4 | valid `--kind` with no findings | exit 0, `[]` |
| 5 | a declaration where `receipt` cannot compute a delta | `findings` works anyway |
| 6 | a declaration driving **all five** kinds | **coverage gap** |

The useful result was **6**: the build fixture reached two of five kinds, and each
kind has its own severity and its own `contract_ids` **arity**, so a renderer that
special-cased one shape would have passed every build test. Verification pinned all
five plus the audit's canonical ordering, and caught two facts the narrow fixture
structurally could not:

- **`contract_ids` arity is per-kind**: 1, 1, 2, 3, 3.
- **Two kinds fire on the same subject**: `comp:dup` yields both a
  `shadow_authority` and a `contradiction`. Keying findings by subject would
  collapse them into the single row this cycle exists to undo.

## Two observations recorded (not debt)

- **`OBS-A3-S14-1`** — `ProjectionOnly` carries `source_kind`, and the
  contradiction detector compares that string against the *authority subject
  name*. The same-subject match across the two contract families is a nominal
  string identity. Existing AC5 semantics, unchanged, but not visible from either
  interface alone.
- **`OBS-A3-S14-2`** — `findings` computes no delta, so it reports the audit on
  declarations where `receipt` errors out. Inspection being more robust than
  composition is the right asymmetry, but it is a real behavioural difference
  between the two surfaces.

## Gates

| Gate | Outcome |
|---|---|
| `command_spec` | 13 passed (registry entry added) |
| `agent_surface_golden` | regenerated: `total` 56 → 57, one `pure` / `none`-authority entry |
| `cli_golden`, `cli_compatibility` | **unchanged** (nested subcommand) |
| `fmt --check` / `clippy -D warnings` | clean |
| `cargo test --workspace` | 204 blocks / **4240 passed** / 0 failed |

Baseline 203 / 4229 → +1 block, **+11 tests**. The golden movement matched the
prediction exactly; A3-S11 saw the same shape when it added five read surfaces
(51 → 56).

## The AC track now

| Verb | Status |
|---|---|
| `architecture contracts / authorities / ownership / compatibility / graph` | A3-S11 read surfaces |
| `architecture findings` | **A3-S14 read surface** |
| `architecture receipt [--changed] [--contract] [--out]` | A3-S10/12/13, the verification entry point |

Root surface description still reads "emit the architecture-conformance receipt" —
worth widening now that six read surfaces share the namespace.

## Next

1. **`why architecture CONTRACT_OR_FINDING`** — finding → contract →
   decision/spec → evidence. The `contract_ids` on each finding are its first
   edges, so this is the natural successor.
2. `plan architecture --move/--dependency` (counterfactual, advisory only).
3. `architecture paradigms` (needs AC3 declaration input).
4. `architecture behavior-map` (needs generation semantics).
5. `diff architecture REV_A..REV_B` (needs a revision substrate for declarations).
