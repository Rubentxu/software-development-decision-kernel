---
id: arch-spec-036-adt-and-typed-dsl-modeling
status: proposed
proposed_at: 2026-09-14
source: docs/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/
supersedes_history: false
---

# arch-spec-036 — ADT and Typed DSL Modeling

## Intent

Closed domain states and DSL execution boundaries SHOULD use explicit typed models that make invalid states and unauthorized effects difficult to express.

## Requirements

- AC-036-001: ADT lens detects boolean blindness, stringly state/action names and incompatible optional-field state machines where evidence supports the claim.
- AC-036-002: DSLs compile authoring syntax to a typed AST/model then normalized IR before effects.
- AC-036-003: internal and external DSL surfaces that model the same domain converge on the same semantic IR.
- AC-036-004: validation/compilation are deterministic and testable without external effects.
- AC-036-005: DSL knowledge is not execution capability; requested effects still cross AuthorityEngine.
- AC-036-006: property/law tests cover deterministic compilation and invalid-state rejection.
- AC-036-007: the spec does not require functional or OO style globally.

## Acceptance

AC-UAT-014, 015.
