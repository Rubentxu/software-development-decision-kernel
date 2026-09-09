# Command Registry design

## Goal

Make CLI implementation, human docs and agent knowledge impossible to drift independently.

## CommandSpec

Conceptual schema:

```yaml
id: run.verify
path: [run, verify]
summary: Execute verification target
stability: stable
side_effect_class: governed
required_authority: execute
arguments: []
options:
  - name: dry-run
    type: bool
outputs:
  text: HumanRunSummary
  json: RunReceiptV1
preconditions:
  - project.adopted
examples:
  - id: verify-dry-run
    command: sddk run verify --dry-run
    expected_exit: 0
    fixture: adopted-rust-project
```

## Stable command IDs

Stable semantic IDs decouple docs/tests from parser implementation details. Renaming a visible path is a compatibility event associated with the same or new command id according to semantics.

## Generated views

```text
CommandSpec registry
 ├─ clap/parser wiring or validation
 ├─ --help human output
 ├─ completion metadata
 ├─ docs tables
 ├─ JSON schema/export
 └─ AgentCommandSurface
```

Exact implementation may adapt to current CLI framework, but rendered help must not become the metadata source.
