# ADR-035 — Evaluation Feedback for Agents, Routes and Workflow Strategies

**Status:** Accepted (refined 2026-08-19)

## Context
Routing decisions and workflow structure both affect correctness, cost, time and human intervention. Optimizing only provider/model selection misses a larger source of overhead: unnecessary phases, handoffs, reviewers and context transfers.

## Decision
SDDK SHALL collect outcome evidence at three levels:
- capability/agent execution;
- execution route/model/provider;
- workflow strategy/graph.

Historical data MAY propose routing or workflow-policy changes, but automatic promotion requires controlled evaluation: offline replay/simulation where possible, shadow mode, bounded rollout, comparison and rollback.

A Workflow Laboratory SHALL support baseline vs adaptive workflows and ablation experiments.

## Metrics
At minimum:
- acceptance/verifier success;
- regression and policy violations;
- first-pass rate;
- retries/remediation/convergence rounds;
- tokens/cost/latency;
- agent calls and handoffs;
- context bytes/tokens/read reuse;
- evidence/invariant coverage;
- human corrections/escalations.

## Consequence
SDDK can simplify its own harness over time based on evidence instead of intuition, while avoiding blind self-optimization.
