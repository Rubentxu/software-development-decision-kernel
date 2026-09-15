---
id: arch-spec-A3-S10-architecture-cli-surface
status: cycle-bounded
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: null
proposed_by_cycle: p-63676b11dc0ef88f/a3-10-ac-cli-surface
source: docs/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/12-CLI-AGENT-UX.md + arch-spec-041 + arch-spec-033
based_on: ADR-0119-ARCHITECTURE-CONFORMANCE-RECEIPT
---

# arch-spec-A3-S10 — Architecture CLI surface (`sddk architecture receipt`)

## Intent

`12-CLI-AGENT-UX.md` names the verification entry point for the AC track, and
`10-UAT.md` §Production acceptance requires an operator-visible
`ARCHITECTURE-CONFORMANCE-RECEIPT`. AC8 built the receipt capability; the CLI
surface that emits it was deferred across AC4–AC8. This cycle closes that gap:

```text
sddk architecture receipt [--contracts <path>] [--format text|json]
```

## Scope (must)

- Engine `architecture_declaration` module: parse + **validate** a declarative
  YAML contract file into AC1 `ArchitecturalContract`s, fail-closed
  (AC-UAT-001). Declarations are input, never authority.
- CLI `architecture receipt`: load the declaration, build the AC2 overlay, run
  AC4 (verify) and AC5 (audit), compose the AC8 receipt, print it.
- Closed verdict → exit code mapping; no numeric score.
- CommandRegistry entry so `sddk agent-help` documents the command (SPEC-015).

## Scope (must NOT)

- MUST NOT write any file, mutate the graph, or persist the receipt.
- MUST NOT grant capability, construct `ArchitectureClaim`, or call a provider.
- MUST NOT emit a numeric/aggregate score.
- MUST NOT add `--changed` (git-diff→unit mapping is its own cycle).
- MUST NOT add the seven read surfaces (`contracts|authorities|ownership|…`)
  in this cycle; they are follow-ups.

## Requirements (REQ-A3S10-NNN)

### Declaration (engine)

- **REQ-A3S10-001** — `load_declaration(yaml: &str) ->
  Result<DeclaredArchitecture, DeclarationError>` is pure (no IO).
  Pin: `acceptance_load_declaration_is_pure`.
- **REQ-A3S10-002** — A valid declaration yields AC1 `ArchitecturalContract`s
  whose ids, kinds and payloads match the file.
  Pin: `acceptance_valid_declaration_builds_contracts`.
- **REQ-A3S10-003** — Fail-closed: an unknown `kind`, a missing required field,
  a duplicate contract id, or a payload kind/field mismatch is an error.
  Pin: `acceptance_fail_closed_cases`.
- **REQ-A3S10-004** — `revision` is required and non-empty (AC-041-001).
  Pin: `acceptance_revision_required`.
- **REQ-A3S10-005** — `knowledge_basis` defaults to the declaration's source
  label when absent. Pin: `acceptance_knowledge_basis_default`.
- **REQ-A3S10-006** — Declared units convert to AC2 `SoftwareUnit`s.
  Pin: `acceptance_units_convert`.
- **REQ-A3S10-007** — Declared waivers are carried verbatim.
  Pin: `acceptance_waivers_carried`.
- **REQ-A3S10-008** — All five contract kinds used by the historical classes are
  declarable: `single_authority`, `unique_owner`, `forbidden_dependency`,
  `projection_only`, `bounded_compatibility`.
  Pin: `acceptance_all_five_kinds`.

### CLI

- **REQ-A3S10-009** — `sddk architecture receipt` reads
  `.sddk/architecture/contracts.yaml` by default; `--contracts <path>` overrides.
  Pin: e2e `architecture_receipt_reads_declaration`.
- **REQ-A3S10-010** — `--format text` prints the verdict, class coverage,
  unresolved MUST findings and the receipt id; `--format json` prints the
  receipt as JSON. Pin: e2e `architecture_receipt_json_has_verdict`.
- **REQ-A3S10-011** — Exit codes: `0` for Pass and PassWithWaivers, `1` for
  Blocked, `2` for a declaration error. Pin: e2e
  `architecture_receipt_exit_codes`.
- **REQ-A3S10-012** — A missing declaration file is an error (exit 2), not an
  empty receipt. Pin: e2e `architecture_receipt_missing_file_fails`.
- **REQ-A3S10-013** — The output contains no numeric score field.
  Pin: e2e `architecture_receipt_has_no_score`.
- **REQ-A3S10-014** — The command is discoverable via
  `sddk agent-help agent`. Pin: e2e `architecture_command_is_registered`.

### Anti-encroachment

- **REQ-A3S10-015** — The engine declaration module performs no IO and imports
  no authority/capability/provider surface.
  Pin: `anti_encroachment_declaration_is_pure`.
- **REQ-A3S10-016** — The CLI handler writes no file.
  Pin: `anti_encroachment_cli_is_read_only`.

## Acceptance tests (planned)

Engine (`architecture_declaration/tests.rs`):
1. `acceptance_load_declaration_is_pure`
2. `acceptance_valid_declaration_builds_contracts`
3. `acceptance_fail_closed_cases`
4. `acceptance_revision_required`
5. `acceptance_knowledge_basis_default`
6. `acceptance_units_convert`
7. `acceptance_waivers_carried`
8. `acceptance_all_five_kinds`
9. `anti_encroachment_declaration_is_pure`

CLI (`tests/architecture_cli_e2e.rs`):
10. `architecture_receipt_reads_declaration`
11. `architecture_receipt_json_has_verdict`
12. `architecture_receipt_exit_codes`
13. `architecture_receipt_missing_file_fails`
14. `architecture_receipt_has_no_score`
15. `architecture_command_is_registered`

## Declaration format (v1)

```yaml
revision: <string>                 # required
knowledge_basis: <string>          # optional
units:
  - id: <string>
    kind: module|crate|function|endpoint|service|schema|table   # default module
    locator: <string>
contracts:
  - id: <string>                   # required, unique
    kind: single_authority|unique_owner|forbidden_dependency|projection_only|bounded_compatibility
    # kind-specific fields, see the loader
    decided_by: <string>
    specified_by: <string>
    revision: <string>
    declared_at_ms: <int>          # optional, default 0
waivers:
  - <string>
```

## Exit-code contract

| Verdict | Exit |
|---|---|
| `Pass` | 0 |
| `PassWithWaivers` | 0 (governed pass) |
| `Blocked` | 1 |
| declaration/IO error | 2 |

## Determinism

Identical declaration + identical clock ⇒ identical receipt id and identical
printed output.
