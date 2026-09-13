# UAT 09/09 Conformance Master

This file carries the original UAT-01..22 forward without weakening their semantics. During C0, replace every `TBD` with concrete test/fixture/command refs.

| UAT | Scenario | Primary specs | Required level | Test/command | Status |
|---|---|---|---|---|---|
| UAT-01 | Fresh project adoption without duplicate authorities | 001, 009, 010, 012 | T4/T5 | TBD | UNKNOWN |
| UAT-02 | Deterministic/idempotent planning reconciliation | 002, 009 | T2/T5 | TBD | UNKNOWN |
| UAT-03 | Multi-run Cycle with fail/wait/pass and derived Cycle summary | 002 | T2/T5 | TBD | UNKNOWN |
| UAT-04 | Governed side effect requires approval and emits receipt/evidence | 001, 008 | T2/T5 | TBD | UNKNOWN |
| UAT-05 | Conflicting agent Contributions preserve material dissent/dispositions | 007 | T2/T5 | TBD | UNKNOWN |
| UAT-06 | Decision Memory fresh-process recovery + bounded context | 003, 004, 006 | T3/T6 | TBD | UNKNOWN |
| UAT-07 | Delete/rebuild SemanticGraph and WHY equivalence | 005 | T3/T6 | TBD | UNKNOWN |
| UAT-08 | Vault edit cannot mutate canonical Decision/Admission | 004, 006, 008 | T2/T5 | TBD | UNKNOWN |
| UAT-09 | Pack isolation leaves core operational | 010 | T2/T5 | TBD | UNKNOWN |
| UAT-10 | Legacy command compatibility/deprecation guidance | 009, 012 | T4/T5 | TBD | UNKNOWN |
| UAT-11 | Drift guard rejects second graph/evidence/revision/event authority | 001, 003, 005 | T0 | TBD | UNKNOWN |
| UAT-12 | Long-session recovery via status/next/why/context explain | 002, 004, 005, 006, 011 | T5/T6 | TBD | UNKNOWN |
| UAT-13 | Agent completes verify using supplied command surface, no exploratory help | 013, 015 | T5 | TBD | UNKNOWN |
| UAT-14 | Every generated cheat-sheet example parses and schema matches | 015 | T4/T5 | TBD | UNKNOWN |
| UAT-15 | CLI change without CommandSpec update breaks contract test | 015 | T0/T4 | TBD | UNKNOWN |
| UAT-16 | Skill selected but missing Capability cannot write | 008, 016 | T2/T5 | TBD | UNKNOWN |
| UAT-17 | Conflicting normative instruction/policy fails closed | 014 | T2/T5 | TBD | UNKNOWN |
| UAT-18 | Same profile works across two provider/fake adapters | 017 | T2/T5 | TBD | UNKNOWN |
| UAT-19 | Execution provenance resolves exact contract hashes/refs | 018 | T2/T5 | TBD | UNKNOWN |
| UAT-20 | Active agent asset with deprecated command/store mutation rejected | 013-017 | T0 | TBD | UNKNOWN |
| UAT-21 | Contextual command surface minimizes unrelated commands | 015 | T2/T5 | TBD | UNKNOWN |
| UAT-22 | Command knowledge does not grant shipping authority | 008, 015-016 | T2/T5 | TBD | UNKNOWN |

## Additional closeout scenarios

These tests do not replace UAT-01..22; they prove M9 removal semantics.

| ID | Scenario | Expected |
|---|---|---|
| CLOSE-01 | Attempt legacy event write | rejected/unreachable |
| CLOSE-02 | Attempt new `PlanningEvidenceKind` production construction | architecture/compile guard fails |
| CLOSE-03 | Approval wait transition | Run/Authority changes; no canonical runtime Cycle write |
| CLOSE-04 | Decision ref CAS race | one winner, one typed conflict |
| CLOSE-05 | Governed effect bypass AuthorityEngine | architecture test fails |
| CLOSE-06 | Modify stable CLI option without registry contract update | contract test fails |
| CLOSE-07 | Active prompt contains deprecated internal semantic/CLI name | asset lint fails |
| CLOSE-08 | Delete all rebuildable projections | canonical state recovers equivalently |
| CLOSE-09 | Start fresh process on migrated repo | no transcript/legacy authority required |
| CLOSE-10 | Docs status says proposed for implemented normative spec | docs drift test fails |

## Release rule

The baseline receipt may be declared 100% only when all 22 original UATs and all applicable CLOSE scenarios are PASS.
