# HANDOFF — A3-S13 architecture verification entry point + v1.169.35 release

- **Date:** 2026-09-15
- **Cycle:** `p-63676b11dc0ef88f/a3-13-architecture-verify` (A-lite)
- **Status:** **CLOSED** (sequence 15)
- **Release:** `v1.169.35` — https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.35
- **HEAD:** `97e08c7` (== `origin/main`)
- **Binary / bundle / framework:** all `1.169.35`

## What shipped

`sddk architecture receipt` is now the verification entry point the upstream
package asks for, with two new flags and **no new verb**.

```text
sddk architecture receipt [--changed] [--contract <id>] [--out <path>]
```

## The naming disposition (this is the interesting part)

`12-CLI-AGENT-UX.md` names `sddk verify architecture [--changed] [--contract ID]`.
That name is **not** adopted:

1. `sddk verify` is already the M6.1 ledger-continuity and capability-policy
   facade (`sddk verify [--format]`).
2. ADR-0120 *already* placed this surface under the `architecture` namespace —
   that is why A3-S10 shipped `receipt`.

A second verb calling the same code would put two canonical surfaces on one
concept (AGENTS.md §2.7, §2.9), and a thin alias is forbidden (§2.0, "Sin
aliases"). So the verb was completed, and the disposition is recorded in the spec
plus an **addendum to ADR-0120** — no new ADR minted for a non-decision.

The verb already gated: `Pass | PassWithWaivers → 0`, `Blocked → 1`, usage
errors → `2`.

## `--contract <id>`

Three deliberate choices:

**It evaluates X rather than narrowing into emptiness.** The delta's affected set
derives from scope units; a filter with no diff would yield an empty delta, and
an empty delta is a *pass*. So with no `--changed`, the scope unit is X's subject
unit and the run reports X's truthful state.

**It fails closed on four unanswerable shapes** — undeclared id, malformed id,
unevaluable kind, dangling subject — all exit 2 with no receipt. Each would
otherwise read as "verified, nothing to report".

**It can only narrow.** `retain` at the engine, so `--changed --contract X` is the
intersection, and both scopes are recorded so the emptiness is attributable:

```
change_basis:      base=<sha> changed_units=1
contract_filter:   c-untouched
affected_contracts: 0 (change-scoped and filtered)
```

## `--out <path>`

Writes the receipt bytes to the path, reported on **stderr** (so stdout stays pure
receipt in both formats — a note appended to JSON would make it unparseable).
Fails closed on a missing parent and does not create it.

## Four defects found and fixed in-cycle

| # | Defect | Where |
|---|---|---|
| 1 | The receipt id ignored the scope: `v1` hashed basis + verdict only, so global, `--changed` and `--contract` runs over one declaration **shared an id** while reporting different scopes and verdicts | build → REQ-010 |
| 2 | `architecture contracts\|authorities\|ownership\|compatibility` advertised `--changed` and `--base` in `--help` and **silently ignored both**; `--base nope` exited 0 | build → REQ-008 |
| 3 | The renderer hardcoded `(change-scoped; empty without --changed)`, misdescribing a filtered run | build → REQ-012 |
| 4 | `--contract` tested the subject's *kind*, not whether the subject is a **declared unit**, so a dangling subject was blocked incidentally by AC5's `missing_owner` rather than by an answer to the question asked | verify → REQ-011 extended |

Defect 1 was pre-existing from A3-S12 and became blocking here: `--out` makes the
id the artifact's address, and an address that does not distinguish its receipts
is not an address. Derivation bumped to `sddk.architecture_receipt.id.v2`.

Defect 4 is the cycle's real finding. Build had *already* taught `--contract` to
reject a contract with no subject unit, and the acceptance test for it passed —
because the test used a global kind, which `unit_subject` rejects anyway.
"Evaluable subject" had two ways to fail and the pin covered one. The bug was in
the **definition**, not the check, which is why the requirement had to be
extended rather than the code merely fixed.

## Deliberately not changed (open question, not debt)

`validate` still accepts a contract naming an undeclared unit. This looks like the
fail-closed rule A3-S11 applied to relation endpoints, but the two differ: a
relation is an edge *between declared units*, while a contract is a claim a
declaration may describe only partially — and AC5's global `missing_owner` audit
is the mechanism that reports what it left undescribed. Tightening `validate`
would change the declaration contract for A3-S10/S11 on evidence that does not
support it. Recorded as observation `OBS-A3-S13-1`.

## Gates

| Gate | Outcome |
|---|---|
| `cli_golden`, `cli_compatibility`, `agent_surface_golden`, `command_spec_tests` | **all unchanged** |
| `fmt --check` / `clippy -D warnings` | clean |
| `cargo test --workspace` | 203 blocks / **4229 passed** / 0 failed |

Baseline 202 / 4215 → +1 block, **+14 tests** (2 conformance, 1 receipt, 11 e2e).
The A3-S12 e2e suite passes **unmodified**, which is what pins REQ-009.

The design note predicted the read-surface help snapshot would lose two entries.
It did not: the fixtures snapshot the root and per-command surfaces, not nested
subcommand help. Prediction wrong, outcome better — recorded rather than
retro-fitted.

## Cycle artifacts

`.sddk/cycles/p-63676b11dc0ef88f-a3-13-architecture-verify/` and the XDG mirror:

| Artifact | |
|---|---|
| `exploration-report.md` | the naming disposition and the `ReadArgs` defect |
| `design-note.md` | the seam choice, semantics table, blast radius |
| `implementation-receipt.md` | incl. remediation round 1 |
| `verification-report.md` | the five probes and defect 4 |
| `merge-receipt.json` | 4 commits, base `2858fd6` → head `97e08c7` |
| `release-receipt.json` | pipeline, hashes, smoke results |
| `archive-manifest.json` | closure + two observations |

Vault: ADR-0120 regenerated with the addendum (create-only mirror, so the file was
replaced and the script re-run, per AGENTS.md §4).

## Next

1. **`deb-verify architecture`** — AC5's global audit as its own verb.
2. **`why architecture CONTRACT_OR_FINDING`** — finding → claim → contract →
   decision/spec → evidence traversal.
3. `architecture paradigms` (needs AC3 declaration input).
4. `architecture behavior-map` (needs generation semantics).
