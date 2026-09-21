# SPIKE-007 — SDD Adaptive Ablation Study

## Question
Which current SDD phase boundaries are genuinely load-bearing for quality, and which can be merged into SHAPE/CONVERGE without regression?

## Baseline
Current A-full on a selected set of representative repository tasks.

## Variants
- V1 merge Explore + Propose;
- V2 single SHAPE produces ChangeContract/spec/design projections;
- V3 remove separate Tasks agent; derive WorkGraph directly;
- V4 adaptive debt verification;
- V5 adaptive specialist/evaluator activation;
- V6 full `sdd-adaptive`.

## Task set
Include at least:
- tiny bug/change;
- medium feature;
- architecture refactor;
- security-sensitive change;
- cross-module migration.

## Metrics
Quality: acceptance, tests, regressions, architecture/security findings, human corrections, evidence coverage.

Efficiency: lead time, tokens, cost, agent calls, handoffs, context reads, convergence rounds.

## Decision
Promote no simplification that shows meaningful quality regression. Keep results in Workflow Laboratory rather than relying on subjective impressions.
