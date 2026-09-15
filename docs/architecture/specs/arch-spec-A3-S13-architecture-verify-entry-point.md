---
id: arch-spec-A3-S13-architecture-verify-entry-point
title: The architecture verification entry point
status: accepted
cycle: p-63676b11dc0ef88f/a3-13-architecture-verify
based_on: arch-spec-A3-S10-architecture-cli-surface + arch-spec-A3-S12-architecture-changed
---

# arch-spec-A3-S13 — The architecture verification entry point

`12-CLI-AGENT-UX.md` names `sddk verify architecture [--changed] [--contract ID]`.

A3-S12 built `--changed`. This cycle adds `--contract ID`, makes the receipt
addressable as an artifact, and fixes a read surface that advertises flags it
ignores.

## Naming disposition (recorded, not reopened)

**There is no new verb.** ADR-0120 already placed the verification entry point
under the `architecture` namespace, which is why A3-S10 shipped
`sddk architecture receipt`. A second verb (`sddk verify architecture` or
`sddk architecture verify`) calling the same code would put two canonical
surfaces on one concept (AGENTS.md §2.7, §2.9); a thin alias is forbidden
outright (§2.0, "Sin aliases").

The verb already gates rather than reports: `run_receipt` maps
`Pass | PassWithWaivers → 0` and `Blocked → 1`, with `2` for usage and resolution
errors. What it lacked was the ability to be asked a narrower question, and to
leave an artifact behind.

## Scope

**In:** `--contract ID` on `architecture receipt`; `contract_filter` on the
receipt; `--out PATH`; removing `--changed`/`--base` from the read surfaces.

**Out:** a `verify` verb or alias; a repeatable `--contract`; `deb-verify
architecture`; `why architecture`; `plan architecture`; the `paradigms` and
`behavior-map` read surfaces; any numeric conformance score (AC-UAT-043).

## Constraints

- MUST NOT add a new root module (no ADR required).
- MUST NOT change the global run (no new flag ⇒ exactly as v1.169.34).
- MUST NOT change the change-scoped run when `--contract` is absent.
- MUST NOT introduce a numeric or aggregate score.
- MUST NOT silently ignore a flag a surface advertises.
- MUST keep the receipt the single authority for its own scope: filtering
  happens where the scope is decided, not in a second pass downstream.

## Requirements (REQ-A3S13-NNN)

- **REQ-A3S13-001** — `architecture receipt --contract <id>` scopes the delta to
  the contracts whose id matches, independently of `--changed`. With no
  `--changed`, the scope units are the named contract's subject unit, so the
  run *evaluates* that contract instead of returning an empty (and therefore
  passing) delta. A filter can only narrow, never widen. Pin: engine
  `acceptance_filter_narrows_to_one_contract`,
  `acceptance_filter_never_widens_the_scope`, e2e
  `architecture_contract_filter_scopes`.
- **REQ-A3S13-002** — Naming a contract that is not declared fails closed:
  exit 2, no receipt, and the error names the id. Pin: e2e
  `architecture_contract_filter_fails_closed`.
- **REQ-A3S13-003** — The receipt records `contract_filter` when `--contract`
  was used and omits it otherwise, so an empty scope is always attributable to a
  stated scope. Pin: engine `acceptance_filter_is_recorded`, e2e
  `architecture_contract_filter_is_recorded`.
- **REQ-A3S13-004** — `--contract` and `--changed` compose as an intersection,
  and the receipt states both. Pin: e2e
  `architecture_contract_filter_intersects_with_changed`.
- **REQ-A3S13-005** — The text renderer prints the contract filter. Pin: e2e
  `architecture_contract_filter_text_names_it`.
- **REQ-A3S13-006** — `--out <path>` writes the receipt to that path with the
  same bytes as stdout, and reports the path. Pin: e2e
  `architecture_out_writes_receipt`.
- **REQ-A3S13-007** — `--out` fails closed when the parent directory does not
  exist: exit 2, no receipt, and no directory is created. Pin: e2e
  `architecture_out_missing_parent_fails_closed`.
- **REQ-A3S13-008** — The read surfaces no longer accept `--changed` or
  `--base`: clap rejects them, and `--help` no longer advertises them. Pin: e2e
  `read_surfaces_reject_change_flags`.
- **REQ-A3S13-009** — A receipt produced with no new flag is unchanged from
  v1.169.34. Pin: the A3-S12 e2e suite passing unmodified, plus
  `architecture_global_run_unchanged`.
- **REQ-A3S13-010** — The receipt id identifies the receipt, not just the
  declaration: the scope (change basis and contract filter) is part of the
  derivation. Pin: engine `acceptance_scope_is_part_of_the_receipt_id`.
  *Found in build: the `v1` derivation hashed only basis + verdict, so a global
  run, a `--changed` run and a `--contract` run over one declaration shared an
  id while reporting different scopes and different verdicts. The id is the
  receipt's address (`--out`), so it has to distinguish them.*
- **REQ-A3S13-011** — `--contract` naming a contract with no evaluable subject
  unit (any kind other than `single_authority` / `unique_owner`) is a usage
  error, not an empty scope. Pin: e2e
  `architecture_contract_filter_rejects_unevaluable_kinds`. *Found in build:
  "verify X" for a global-kind contract is an unanswerable question, and
  answering it with zero rows reads as a pass.*
- **REQ-A3S13-012** — The renderer's scope note names the scope actually asked
  for (`change-scoped`, `contract-scoped`, `change-scoped and filtered`, or
  `empty without --changed or --contract`). Pin: e2e
  `architecture_contract_filter_text_names_it`. *Found in build: the note was
  hardcoded to "change-scoped; empty without --changed", which misdescribes a
  filtered run with no diff.*

## Design sketch

`ConformanceInputs` gains one field:

```rust
/// When set, the delta considers only this contract.
pub contract_filter: Option<&'a ContractId>,
```

`compute_conformance_delta` applies it while building `affected`, so
`claim_results`, `unknowns`, `contradictions` and `class_coverage` all follow
without a second filtering pass. `ConformanceInputs` has no closed-variant pin
(unlike AC3's `EvidenceBasis`, frozen at five), so this is an additive field and
the compiler surfaces every construction site.

`ArchitectureConformanceReceipt` gains `contract_filter: Option<String>`
alongside `change_basis`, and the renderer prints it next to `change_basis:` so
the two scope statements read together.

## Acceptance tests (shipped, 12)

Engine (`architecture_conformance/tests.rs` or `architecture_receipt/tests.rs`):
1. `acceptance_filter_narrows_to_one_contract`
2. `acceptance_filter_is_recorded`

CLI e2e (`tests/architecture_verify_cli_e2e.rs`):
3. `architecture_contract_filter_scopes`
4. `architecture_contract_filter_fails_closed`
5. `architecture_contract_filter_is_recorded`
6. `architecture_contract_filter_intersects_with_changed`
7. `architecture_contract_filter_text_names_it`
8. `architecture_out_writes_receipt`
9. `architecture_out_missing_parent_fails_closed`
10. `read_surfaces_reject_change_flags`
11. `architecture_contract_filter_rejects_unevaluable_kinds` (REQ-011)
