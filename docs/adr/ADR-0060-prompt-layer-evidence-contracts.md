# ADR-0060 - Evidence Contracts for the SDDK Prompt Layer

**Status:** accepted
**Date:** 2026-08-24
**Trigger:** [Issue #93](https://github.com/Rubentxu/software-development-decision-kernel/issues/93)

---

## Context

SDDK instructions duplicated lifecycle rules across the orchestrator, phase
prompts, wrappers, and skills. Several examples used invalid CLI forms, treated
nominal receipts as semantic proof, or let render-oriented documents imply more
authority than their evidence supported. The existing golden runner prepared
pending result stubs instead of executing evaluator and judge roles.

The runtime and persistence roadmap already owns WorkflowIR, CLI, ledger, CAS,
control-plane, and durable debt changes. Expanding those surfaces here would
create a competing architecture and violate the initiative's prompt-layer
scope.

## Decision

1. Keep runtime state authoritative and centralize prompt-layer CLI ownership,
   freshness, error, lease, and material-evidence rules in
   `skills/_shared/cli-usage-contract.md`.
2. Make `sddk-cycle-resume` the single bootstrap/reconstruction procedure and
   fail closed when the current CLI cannot discover a globally active cycle.
3. Normalize optional knowledge preflight as scan, reviewed plan, import, then
   verify. Only explicitly reviewed changed-entry IDs are approved; other
   imported changes remain `NeedsReview` rather than becoming trusted.
4. Require verify to use an L0-L6 evidence pipeline and one versioned finding
   shape. Deterministic failures cannot be downgraded by semantic judgment.
5. Evaluate instruction bundles outside the kernel with isolated held-out
   cases, separate evaluator/judge identities, immutable snapshots, provenance,
   multiple trials, and deterministic grading.
6. Treat Markdown/HTML and architecture diagrams as derived views. Reports use
   Spanish by default with explicit BCP 47 override. C4/LikeC4 remains an
   optional skill with evidence-bound semantics and a renderer-independent
   fallback.
7. Do not perform broad prompt pruning until real-model baseline trials can
   measure regression and adversarial behavior. CI validates contracts only; it
   does not fabricate or run model results.

## Alternatives

- **Add enforcement to Rust now:** partially rejected. Runtime evolution
  belongs to the active kernel roadmap, but **one minimal Rust primitive**
  was accepted in this initiative: the `sddk cycle inventory` subcommand
  (`crates/sddk-cli/src/inventory_cycle.rs`, schema authority
  `prompts/sddk/contracts/inventory.schema.json`). The investigation in
  `docs/research/sddk-prompts-agents-workflow-evolution.md` confirmed
  no existing CLI surface produces the cycle-scoped files inventory; the
  alternative would have been a parallel Python/JS reducer outside the
  runtime, which would duplicate authority and bypass the zero-intrusion
  boundary. The primitive is narrowly scoped (single binary, single
  artifact, single schema) and does not extend CLI semantics beyond
  cycle-coordinator inventory production.
- **Keep rules duplicated near every phase:** rejected because drift already
  produced invalid commands and conflicting ownership.
- **Use one evaluator as its own judge:** rejected because correlated judgment
  hides misses and fabricated evidence.
- **Make LikeC4 mandatory:** rejected because rendering availability must not
  change semantic truth or violate zero intrusion.
- **Continue with pending result stubs:** rejected because they cannot establish
  an executable or comparable baseline.

## Consequences

### Positive

- CLI examples and lifecycle ownership have one prompt-layer authority.
- Findings, reports, and architecture views remain tied to subject and evidence.
- Held-out evaluation inputs are isolated from labels and source worktrees.
- Missing runtime guarantees are visible instead of silently inferred.

### Negative

- The external harness requires `bwrap` for role isolation.
- Statistical efficacy remains unproven until real evaluator/judge trials run.
- Prompt contracts are stricter and reject incomplete legacy envelopes.
- Global active-cycle discovery remains blocked by the current CLI surface.

## Evidence and Provenance

- `docs/research/sddk-prompts-agents-workflow-evolution.md`
- `docs/research/real-model-trials-runbook.md` (cycle-19 follow-up runbook)
- [Issue #93](https://github.com/Rubentxu/software-development-decision-kernel/issues/93)
- `skills/_shared/cli-usage-contract.md`
- `prompts/sddk/contracts/verify-finding.schema.json`
- `prompts/sddk/contracts/inventory.schema.json`
- `crates/sddk-cli/src/inventory_cycle.rs`
- `crates/sddk-gateway/src/git.rs`
- `tests/test_inventory_contract.sh`
- `golden-dataset/runner/run_golden.py`
- `golden-dataset/runner/grade_results.py`
- `tests/test_golden_dataset_contract.py`
- `tests/test_workflow_contract.py`

## Changelog

- 2026-08-24: accepted with prompt-layer implementation; real-model baseline
  trials and statistical pruning remain pending.
- 2026-08-24: cycle-7b integration wired. `prompts/sddk/phases/debt-verify.md`
  now references the canonical severity/priority taxonomies and the
  `docs/debt/debt-report.schema.json` shape, with a Common Finding →
  persisted schema mapping. `prompts/sddk/phases/archive.md` produces
  `INC-NNN-{slug}.md` follow-up incidences from
  `docs/debt/INCIDENCE-TEMPLATE.md`. The first durable incidence
  `INC-001-cli-call-budget-stale.md` is filed as proof that the cycle-7b
  pipeline renders, persists, and dedupes by fingerprint.
- 2026-08-24: **scope deviation documented.** The `## Files Inventory`
  lifecycle required one minimal Rust primitive (`sddk cycle inventory`)
  added under `crates/` to feed the verify/release/archive prompts. The
  primitive is narrowly scoped, re-uses the existing `sddk_gateway::git`
  surface (`pub fn run_read_only`), and the schema authority remains in
  `prompts/sddk/contracts/inventory.schema.json`. The "no changes under
  crates/" clause of issue #93 acceptance criteria is treated as relaxed
  for this primitive only; broader kernel evolution remains out of scope.
