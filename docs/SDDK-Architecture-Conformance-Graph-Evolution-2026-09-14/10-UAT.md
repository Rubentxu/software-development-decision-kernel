# Architecture Conformance UAT

## Base UAT (AC1–AC8)

| ID | Scenario | Acceptance |
|---|---|---|
| AC-UAT-001 | ingest typed contract | stable ID/basis; invalid contract fails closed |
| AC-UAT-002 | rebuild architecture overlay | delete/rebuild yields equivalent nodes/relations/digests |
| AC-UAT-003 | single-authority contract | one writer PASS; injected second writer CONTRADICTED |
| AC-UAT-004 | dependency boundary | legal edge PASS; forbidden edge is detected with source evidence |
| AC-UAT-005 | projection-only contract | read projection cannot become canonical writer without finding |
| AC-UAT-006 | compatibility boundedness | missing owner/removal trigger produces finding, not green |
| AC-UAT-007 | missing provider | provider-dependent claim becomes UNKNOWN/NOT_EVALUATED |
| AC-UAT-008 | changed unit | only affected contracts enter delta Verify scope |
| AC-UAT-009 | DebVerify | global duplicate authority found even with empty recent delta |
| AC-UAT-010 | mutation fitness | injected domain→provider SDK dependency causes expected guard failure |
| AC-UAT-011 | advisory boundary | Alignment finding changes advisory context, not EffectiveInstructions |
| AC-UAT-012 | OO lens | declared OO unit receives evidence-based assessment without global paradigm judgment |
| AC-UAT-013 | pure FP lens | hidden mutation contradicts declared pure profile; absent purity declaration is NOT_APPLICABLE |
| AC-UAT-014 | ADT lens | invalid-state/stringly fixture yields opportunity with exact evidence |
| AC-UAT-015 | DSL lens | typed AST→IR fixture deterministic; syntax knowledge grants no execution capability |
| AC-UAT-016 | self-audit | SDDK reproduces agreed historical conformance findings and emits receipt |

## Reactive/Agentic UAT

| ID | Scenario | Acceptance |
|---|---|---|
| AC-UAT-020 | JCode material change | turn_done coalesces changes and evaluates affected contracts |
| AC-UAT-021 | irrelevant change | no noisy ContextDelta |
| AC-UAT-022 | tension | normal Alignment tension is CONTEXTUAL, not forced interrupt |
| AC-UAT-023 | proof-carrying structured work | contribution links conformance receipt but does not self-authorize |

## Enhanced provider UAT

| ID | Scenario | Acceptance |
|---|---|---|
| AC-UAT-030 | CogniCode static evidence | provider result normalized to Evidence; no provider types inside domain |
| AC-UAT-031 | Chronos runtime bypass | runtime path can contradict static assumption without replacing Base truth |
| AC-UAT-032 | provider contradiction | static/runtime evidence coexist; reconciliation preserves contradiction |
| AC-UAT-033 | provider outage | Base architecture Verify remains usable; affected enhanced claims are NOT_EVALUATED |

## Advanced UAT

| ID | Scenario | Acceptance |
|---|---|---|
| AC-UAT-040 | counterfactual move | predicts boundary violation without changing working tree |
| AC-UAT-041 | architecture time travel | revision diff reports semantic ownership/authority/compatibility changes |
| AC-UAT-042 | immune-system ratchet | repaired finding promoted to fitness rule; reintroduction is detected |
| AC-UAT-043 | no universal score | UI/CLI reports conformance vector/statuses, never fabricated aggregate quality score |

## Production acceptance

Base cannot claim the AC capability production-ready until AC-UAT-001..016 pass at a named commit and `ARCHITECTURE-CONFORMANCE-RECEIPT` lists zero unresolved MUST findings.
