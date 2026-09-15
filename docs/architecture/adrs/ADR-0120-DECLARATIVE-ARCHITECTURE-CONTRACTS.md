---
id: ADR-0120-DECLARATIVE-ARCHITECTURE-CONTRACTS
status: accepted
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: 2026-09-15
accepted_by_cycle: p-63676b11dc0ef88f/a3-10-ac-cli-surface
implementation_evidence:
  - "crates/sddk-engine/src/architecture_declaration/mod.rs (public surface; input-not-authority doc)"
  - "crates/sddk-engine/src/architecture_declaration/types.rs (DeclarationFile, UnitDecl, ContractDecl, DeclaredArchitecture, DeclarationError)"
  - "crates/sddk-engine/src/architecture_declaration/validate.rs (fail-closed validate/contract_from_decl/unit_from_decl)"
  - "crates/sddk-engine/src/architecture_declaration/tests.rs (11 tests)"
  - "crates/sddk-cli/src/architecture_cmd.rs (sddk architecture receipt; YAML parsing; text/JSON render; exit codes)"
  - "crates/sddk-cli/tests/architecture_cli_e2e.rs (9 e2e tests)"
  - "docs/architecture/specs/arch-spec-A3-S10-architecture-cli-surface.md (cycle-bounded spec, 16 REQs)"
superseded_by: []
related_adrs:
  - "ADR-0112-TYPED-ARCHITECTURAL-CONTRACTS"
  - "ADR-0113-ARCHITECTURE-AS-SEMANTIC-GRAPH-OVERLAY"
  - "ADR-0119-ARCHITECTURE-CONFORMANCE-RECEIPT"
  - "ADR-0107-ONE-COMMAND-REGISTRY"
stale_after: 2027-03-14
---

# ADR-0120 — Declarative Architecture Contracts Are Input, Not Authority

## Context

AC1–AC8 built a complete conformance stack, but every capability is reachable
only programmatically: `ArchitecturalContract`s are constructed in Rust. That
left two gaps named by the upstream package:

- `10-UAT.md` requires an operator-visible `ARCHITECTURE-CONFORMANCE-RECEIPT`;
- `12-CLI-AGENT-UX.md` names `sddk verify architecture` as the verification
  entry point.

Both need a **contract source** a repository can carry. AC1's spec anticipates
this:

> Initial adapters may ingest contract declarations from repository-native
> specs/ADRs, but Decision Memory/revisioned objects become semantic authority
> after validation.

The risk: a file becomes a second source of architectural truth, drifting from
the AC1 objects everything else reasons about.

## Decision

**1. The declaration file is INPUT.** A declarative description
(`.sddk/architecture/contracts.yaml`) is a manifest of intent. `validate`
converts it into AC1 `ArchitecturalContract`s and AC2 `SoftwareUnit`s; only
those objects are authority. Nothing else reads the file.

**2. Validation is fail-closed (AC-UAT-001).** A blank id, a duplicate id, a
missing kind-specific field, an unrecognised kind, or an AC1 rejection is an
error. There are no silent skips and no defaults for required payload fields.

**3. The engine is format-agnostic.** `architecture_declaration` defines serde
structs and the validator but never parses YAML: the CLI owns the file format
and passes the parsed value. The engine gains no new dependency and stays IO-free,
so a future TOML/JSON/Kotlin-like front end reuses the same validator.

**4. Exactly one new CLI entry point in this stage:**
`sddk architecture receipt`, which loads the declaration, runs AC4/AC5 and
composes the AC8 receipt, printing it as text or JSON. It is registered in the
CommandRegistry (ADR-0107) so `sddk agent-help` documents it.

**5. The verdict maps to an exit code, never to a score.**
`Pass`/`PassWithWaivers` → 0, `Blocked` → 1, declaration error → 2. The output
carries the closed verdict, the historical-class coverage and the unresolved
MUST findings — no numeric aggregate (AC-UAT-043).

## Consequences

- The AC stack becomes operator-usable end to end: a repository declares what it
  intends, and `sddk architecture receipt` produces the artefact the Base gate
  references.
- Declarations are declarative data, not a language: no expressions, no
  conditionals, no hooks. A contract DSL with semantics remains a later,
  separate decision (the upstream doc says syntax must never own semantics).
- Two shapes now describe an architecture: the declaration (intent) and the AC1
  objects (validated authority). The conversion is one-directional and total, so
  they cannot drift silently — an unrepresentable declaration fails loudly.
- A new root-level engine module requires an ADR under the repository's
  root-module fitness rule.

## Alternatives considered

- **Put YAML parsing in the engine.** Rejected: it would add a format dependency
  to a module whose whole value is a format-independent validator.
- **Read the declaration as authority throughout the runtime.** Rejected: it
  would create a second authority for architectural facts and bypass the AC1
  revisioned objects.
- **Generate the receipt from the existing lint rules instead of declarations.**
  Rejected: lint rules are negative constraints, not typed contracts; they cannot
  express ownership, projection-only or bounded-compatibility intent.
- **Implement all seven read surfaces plus `--changed` now.** Rejected as scope
  creep: the verification entry point is what the Base gate needs; the read
  surfaces and change scoping are follow-ups.
- **Emit a pass/fail score for easy gating.** Rejected: AC-UAT-043 forbids a
  fabricated aggregate; the verdict plus statuses is the honest output.

---

## Addendum (A3-S13, v1.169.35) — the verification entry point is `architecture receipt`

`12-CLI-AGENT-UX.md` names `sddk verify architecture [--changed] [--contract ID]`.
That name is **not** adopted, and this is a deliberate continuation of this
ADR's decision rather than a new one, so no new ADR is minted.

Two reasons:

1. **`sddk verify` is taken.** It is the M6.1 ledger-continuity and
   capability-policy facade (`sddk verify [--format]`). Hanging an `architecture`
   subcommand off it would overload one verb with two unrelated meanings.
2. **This ADR already placed the entry point under `architecture`.** A3-S10
   shipped `sddk architecture receipt` precisely because of the context recorded
   above. Adding `sddk verify architecture` or `sddk architecture verify` now
   would put two canonical surfaces on one concept (AGENTS.md §2.7, §2.9), and a
   thin alias is forbidden outright (§2.0: "Sin aliases").

The verb already gates rather than reports: `Pass | PassWithWaivers → 0`,
`Blocked → 1`, usage and resolution errors → `2`. A3-S13 completed it into the
entry point the package needs:

- `--contract <id>` — answer a question about one contract, independently of any
  diff. Fails closed if the id is undeclared or the contract has no evaluable
  subject unit.
- `--out <path>` — leave the receipt where a harness can find it. Fails closed on
  a missing parent directory; the path is reported on stderr so stdout stays a
  pure receipt in both formats.

Two consequences for the receipt's identity, both fixed in A3-S13:

- The receipt id now hashes the **scope** as well as the basis and verdict
  (`sddk.architecture_receipt.id.v2`). Under `v1`, a global run, a `--changed`
  run and a `--contract` run over one declaration shared an id while reporting
  different scopes and different verdicts. An address that does not distinguish
  its receipts is not an address.
- `contract_filter` is recorded alongside `change_basis`, so an empty scope is
  always attributable to a stated scope. This generalizes the rule A3-S12
  established for `change_basis`.

The `architecture contracts|authorities|ownership|compatibility` surfaces lost
`--changed` and `--base`. They advertised both in `--help` and silently ignored
them; a read surface is not change-scoped, and `graph --scope` already narrows
read-side.
