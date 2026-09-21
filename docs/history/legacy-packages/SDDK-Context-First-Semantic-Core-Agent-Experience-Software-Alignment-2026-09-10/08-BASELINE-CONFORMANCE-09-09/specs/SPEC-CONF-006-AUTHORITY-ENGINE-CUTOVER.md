# SPEC-CONF-006 — AuthorityEngine Single Admission Cutover

## Baseline contract

Closes ADR-009, SPEC-008, M5 and M9 retirement of competing side-effect authority paths.

## Required semantic path

```text
ActionProposal + Actor + Facts + PolicySnapshot
    -> AuthorityEngine
        -> Allow | Deny | RequireApproval
            -> admitted Capability execution
                -> Receipt/Evidence
```

Permission, gate, risk and approval subsystems may remain internal contributors. They MUST NOT independently authorize a governed external effect.

## Requirements

- all governed side effects pass through the AuthorityEngine application facade;
- legacy authority APIs either delegate to AuthorityEngine or become unreachable;
- `RequireApproval` cannot execute before an approval fact satisfies the admission contract;
- `Deny` produces zero side effect;
- Capability possession is not equivalent to admission;
- Skills/instructions/command visibility cannot grant authority;
- decision is explainable from policy/facts/evidence snapshots;
- old facades have no new consumers and are protected by deny-new-dependency fitness rules.

## Migration sequence

1. enumerate effectful call sites;
2. map each to ActionProposal;
3. route through AuthorityEngine;
4. parity-test old facade against new decision;
5. freeze old entrypoints;
6. delete/internalize when no production caller remains.

## Acceptance

- UAT-04, UAT-16 and UAT-22 pass;
- dependency graph contains no direct effect execution bypass;
- injected denied action records decision but mutates nothing external;
- approval-required fixture proves no early execution;
- architecture test detects any new direct dependency on legacy authority API.
