# 09/09 SPEC Conformance Crosswalk

Populate code/test references during C0. Initial status reflects the 2026-09-13 audit and is intentionally conservative.

| SPEC | Contract | Initial status | Closeout action |
|---|---|---|---|
| SPEC-001 | Canonical authority / facts / evidence | FAIL/PARTIAL | event + evidence cutover |
| SPEC-002 | Lifecycle model | FAIL/PARTIAL | remove runtime Cycle authority |
| SPEC-003 | Revision substrate | PARTIAL | classify/consolidate Decision Memory primitives |
| SPEC-004 | Decision Memory | PASS/PARTIAL | preserve domain semantics; prove substrate reuse/recovery |
| SPEC-005 | SemanticGraph + WHY | PASS | protect single graph authority; rebuild proof |
| SPEC-006 | Knowledge/Vault/Context | PASS/PARTIAL | prove Vault cannot mutate authority + recovery provenance |
| SPEC-007 | Agent protocol/handoff | PASS/PARTIAL | explicit UAT-05 traceability |
| SPEC-008 | Authority/side effects | PARTIAL | complete AuthorityEngine strangler cutover |
| SPEC-009 | Target/Task workflow | PASS/PARTIAL | executable UAT traceability |
| SPEC-010 | Pack SDK | PASS/PARTIAL | isolation/conformance test evidence |
| SPEC-011 | Observability views | PASS/PARTIAL | prove projection semantics, no authority |
| SPEC-012 | Configuration | PASS/PARTIAL | precedence/config-explain fixture |
| SPEC-013 | Agent Experience contract | PASS/PARTIAL | asset inventory and migration closure |
| SPEC-014 | Instruction compiler | PASS/PARTIAL | conflict fail-closed UAT evidence |
| SPEC-015 | Command registry/agent surface | PARTIAL | single-source convergence + UAT-13/14/15/21/22 |
| SPEC-016 | Skill contract | PASS/PARTIAL | UAT-16 evidence |
| SPEC-017 | Agent profiles/provider adapters | PASS/PARTIAL | provider portability evidence |
| SPEC-018 | Agent execution provenance | PASS/PARTIAL | exact receipt/hash UAT evidence |

## Interpretation

`PASS/PARTIAL` means the core capability appears present but the complete 09/09 acceptance/migration proof is not yet recorded. It MUST NOT be converted to PASS merely by inspection; link an executable result.

## M9 explicit removal crosswalk

| M9 obligation | Initial finding | Required final state |
|---|---|---|
| remove duplicate event paths | `events_v1` + `ledger_events` coexist | one write authority; legacy read/migration only or removed |
| remove planning Evidence duplicates | `PlanningEvidenceKind` migration deferred | no production writes/type dependency outside compat migration |
| remove ActiveGraph authority path | audited as derived view | keep projection-only and ratchet it |
| remove AgentResult production writes | not fully proven in audit | explicit scan + zero production writers |
| retire runtime-specific Cycle statuses | variants remain | no canonical writes; remove/decode-only |
| retire handwritten cheat sheets/duplicated command docs | parity exists, source duplication remains possible | generated/mechanically bound single source |
| remove obsolete monolithic prompts | not fully proven | inventory + no active obsolete path |
| architecture/agent drift blocking | partially present | mutation-tested blocking rules |
| one normative roadmap/architecture entry point | status drift observed | reconciled docs and supersession |
