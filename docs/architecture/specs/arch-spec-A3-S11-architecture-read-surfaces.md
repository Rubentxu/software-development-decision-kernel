---
id: arch-spec-A3-S11-architecture-read-surfaces
status: cycle-bounded
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: null
proposed_by_cycle: p-63676b11dc0ef88f/a3-11-architecture-read-surfaces
source: docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/12-CLI-AGENT-UX.md
based_on: arch-spec-A3-S10-architecture-cli-surface + ADR-0120-DECLARATIVE-ARCHITECTURE-CONTRACTS
---

# arch-spec-A3-S11 — Architecture Read Surfaces

## Intent

`12-CLI-AGENT-UX.md` §Read surfaces lists seven inspection commands. A3-S10
shipped the verification verb (`receipt`); this cycle ships the inspection
family so an operator can ask *what was declared* and *what shape the graph has*.

```text
sddk architecture contracts
sddk architecture authorities
sddk architecture ownership
sddk architecture compatibility
sddk architecture graph [--scope <locator-prefix>]
```

## Prerequisite: declared relations

`dependency_boundary` could never reproduce on a CLI run: the declaration had
`units[]` and `contracts[]` but no **relations**, so AC5's `AuthorityBypass`
detector had no edge to find. This cycle adds `relations[]`.

```yaml
relations:
  - from: comp:graph          # must name a declared unit
    to: comp:canonical-event-log
    kind: writes              # one of nine declarable kinds
```

Declarable kinds (a closed subset of AC2's 14): `owns`, `depends_on`, `writes`,
`reads`, `emits`, `consumes`, `projects`, `derives_from`, `implements`.
Semantic kinds (`decided_by`, `verified_by`, `contradicts_by`, `supersedes_by`,
`architecture_claimed_by`) are produced by the system, never declared by hand.

## Scope (must)

- `RelationDecl { from, to, kind }` + `relations[]` on `DeclarationFile`.
- Fail-closed validation: unknown `kind`, or an endpoint naming no declared
  unit, is an error.
- Five read subcommands (`contracts`, `authorities`, `ownership`,
  `compatibility`, `graph`), text and JSON.
- `graph --scope <prefix>` filters units by locator prefix.
- CommandRegistry entries for the five subcommands.
- `compatibility` annotates stale windows with their AC5 finding.

## Scope (must NOT)

- MUST NOT emit a verdict, a change basis, or a numeric score from a read
  surface.
- MUST NOT write, mutate the graph, or persist anything.
- MUST NOT implement `paradigms` (needs AC3 declaration input) or
  `behavior-map` (needs generation semantics) in this cycle.
- MUST NOT introduce a new root engine module (no new ADR required).

## Requirements (REQ-A3S11-NNN)

### Declared relations

- **REQ-A3S11-001** — `DeclarationFile.relations: Vec<RelationDecl>` with
  `RelationDecl { from: String, to: String, kind: String }`; absent ⇒ empty.
  Pin: `acceptance_relations_default_empty`.
- **REQ-A3S11-002** — `validate` accepts the nine declarable kinds and rejects
  any other with `UnknownRelationKind`.
  Pin: `acceptance_relation_kinds_closed`.
- **REQ-A3S11-003** — An endpoint that names no declared unit id is rejected
  with `UnknownRelationEndpoint`.
  Pin: `acceptance_relation_endpoint_must_exist`.
- **REQ-A3S11-004** — Accepted relations are carried on `DeclaredArchitecture`
  as AC2 `ArchitectureOverlayRelation`s with node refs resolving to the declared
  units.
  Pin: `acceptance_relations_convert`.
- **REQ-A3S11-005** — A declaration with a forbidden-dependency contract **and**
  a live relation joining its endpoints makes AC5 report `AuthorityBypass`.
  Pin: `acceptance_declared_bypass_is_detected` (integration).
- **REQ-A3S11-006** — A declaration with a relation whose kind is non-declarable
  (e.g. `contradicts_by`) is rejected. Pin: `acceptance_semantic_kinds_rejected`.

### Read surfaces

- **REQ-A3S11-007** — `architecture contracts` lists every declared contract
  with id, kind, subject and metadata; sorted by id.
  Pin: e2e `architecture_contracts_lists_declared`.
- **REQ-A3S11-008** — `architecture authorities` lists only `SingleAuthority`
  contracts, grouped by owning component.
  Pin: e2e `architecture_authorities_filters_single_authority`.
- **REQ-A3S11-009** — `architecture ownership` lists only `UniqueOwner`
  contracts, grouped by owned entity.
  Pin: e2e `architecture_ownership_filters_unique_owner`.
- **REQ-A3S11-010** — `architecture compatibility` lists `BoundedCompatibility`
  contracts with `deprecated_after_ms`, `replaced_by` and a `window_status`
  (`open` | `stale`). Pin: e2e `architecture_compatibility_reports_window`.
- **REQ-A3S11-011** — `architecture graph` lists units and declared relations and
  the overlay digest. Pin: e2e `architecture_graph_lists_units_and_relations`.
- **REQ-A3S11-012** — `architecture graph --scope <prefix>` restricts units to
  those whose locator starts with `<prefix>`. Pin: e2e
  `architecture_graph_scope_filters_units`.
- **REQ-A3S11-013** — Every surface honours `--format json` and emits no numeric
  score. Pin: e2e `architecture_read_surfaces_are_score_free`.
- **REQ-A3S11-014** — Every surface fails closed on a missing/invalid
  declaration (exit 2). Pin: e2e `architecture_read_surfaces_fail_closed`.
- **REQ-A3S11-015** — The five subcommands are registered in the
  CommandRegistry. Pin: e2e `architecture_subcommands_are_registered`.

## Acceptance tests (planned)

Engine (`architecture_declaration/tests.rs`):
1. `acceptance_relations_default_empty`
2. `acceptance_relation_kinds_closed`
3. `acceptance_relation_endpoint_must_exist`
4. `acceptance_relations_convert`
5. `acceptance_semantic_kinds_rejected`
6. `acceptance_declared_bypass_is_detected`

CLI (`tests/architecture_read_cli_e2e.rs`):
7. `architecture_contracts_lists_declared`
8. `architecture_authorities_filters_single_authority`
9. `architecture_ownership_filters_unique_owner`
10. `architecture_compatibility_reports_window`
11. `architecture_graph_lists_units_and_relations`
12. `architecture_graph_scope_filters_units`
13. `architecture_read_surfaces_are_score_free`
14. `architecture_read_surfaces_fail_closed`
15. `architecture_subcommands_are_registered`

## Declaration format v2

```yaml
revision: <string>                 # required
knowledge_basis: <string>          # optional
units:
  - id: <string>
    kind: module|crate|function|endpoint|service|schema|table   # default module
    locator: <string>              # default: id
relations:                         # NEW in v2
  - from: <unit id>
    to: <unit id>
    kind: owns|depends_on|writes|reads|emits|consumes|projects|derives_from|implements
contracts: [...]
waivers: [...]
```

## Output contract

Every read surface prints a header naming the surface and the revision, the rows,
and (for `graph`) the overlay digest. `--format json` prints the rows as JSON.
No surface prints a verdict or a score.

## Determinism

Identical declaration + identical arguments ⇒ identical output.
