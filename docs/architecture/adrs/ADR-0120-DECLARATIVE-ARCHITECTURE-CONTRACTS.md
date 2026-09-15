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
