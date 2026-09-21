# PATCH MANIFEST — SDDK Dynamic Workflows / SDD Adaptive

Date: 2026-08-19

## How to apply

Unzip this delta **over the root that already contains `sddk-decision-kernel-architecture/`**. Files under the same paths are complete replacements. No file from the previous package must be deleted.

```bash
unzip SDDK-Dynamic-Workflows-SDD-Adaptive-Delta.zip -d <parent-of-sddk-decision-kernel-architecture>
```

## MODIFIED — replace existing files

- `sddk-decision-kernel-architecture/README.md`
- `sddk-decision-kernel-architecture/DOCUMENT-INDEX.md`
- `sddk-decision-kernel-architecture/GLOSSARY.md`
- `sddk-decision-kernel-architecture/00-vision/PRD.md`
- `sddk-decision-kernel-architecture/00-vision/PRINCIPLES.md`
- `sddk-decision-kernel-architecture/00-vision/STUDY-GUIDE.md`
- `sddk-decision-kernel-architecture/01-architecture/DESIGN.md`
- `sddk-decision-kernel-architecture/01-architecture/CONTROL-FLOWS.md`
- `sddk-decision-kernel-architecture/01-architecture/DOMAIN-MODEL.md`
- `sddk-decision-kernel-architecture/01-architecture/EVENT-AND-GRAPH-MODEL.md`
- `sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md`
- `sddk-decision-kernel-architecture/02-roadmap/BACKLOG.md`
- `sddk-decision-kernel-architecture/02-roadmap/MIGRATION-PLAN.md`
- `sddk-decision-kernel-architecture/03-adrs/ADR-023-SUPERVISOR-RUNTIME-SEPARATION.md`
- `sddk-decision-kernel-architecture/03-adrs/ADR-024-GENERIC-WORKFLOW-IR.md`
- `sddk-decision-kernel-architecture/03-adrs/ADR-035-EVALUATION-FEEDBACK.md`
- `sddk-decision-kernel-architecture/03-adrs/README.md`
- `sddk-decision-kernel-architecture/04-specs/SPEC-019-SUPERVISOR-RUNTIME.md`
- `sddk-decision-kernel-architecture/04-specs/SPEC-023-WORKFLOW-RUNTIME-V2.md`
- `sddk-decision-kernel-architecture/04-specs/SPEC-024-AGENT-EVALUATION.md`
- `sddk-decision-kernel-architecture/04-specs/SPEC-028-REACTIVE-BEHAVIORS.md`
- `sddk-decision-kernel-architecture/04-specs/README.md`
- `sddk-decision-kernel-architecture/05-workflows/README.md`
- `sddk-decision-kernel-architecture/05-workflows/SDD-WORKFLOW.md`
- `sddk-decision-kernel-architecture/05-workflows/WORKFLOW-CATALOG.md`
- `sddk-decision-kernel-architecture/08-spikes/README.md`
- `sddk-decision-kernel-architecture/09-implementation/ARCHITECTURE-FITNESS-FUNCTIONS.md`
- `sddk-decision-kernel-architecture/09-implementation/IMPLEMENTATION-BACKLOG.md`
- `sddk-decision-kernel-architecture/09-implementation/TEST-STRATEGY.md`

## ADDED — new files

- `sddk-decision-kernel-architecture/CHANGESET-2026-08-19-DYNAMIC-WORKFLOWS.md`
- `sddk-decision-kernel-architecture/03-adrs/ADR-037-DYNAMIC-WORKFLOW-COMPILATION.md`
- `sddk-decision-kernel-architecture/03-adrs/ADR-038-INVARIANT-DRIVEN-SDD.md`
- `sddk-decision-kernel-architecture/03-adrs/ADR-039-ADAPTIVE-VERIFICATION.md`
- `sddk-decision-kernel-architecture/04-specs/SPEC-037-DYNAMIC-WORKFLOW-COMPILER.md`
- `sddk-decision-kernel-architecture/04-specs/SPEC-038-SDD-ADAPTIVE.md`
- `sddk-decision-kernel-architecture/04-specs/SPEC-039-WORKFLOW-PATTERN-ALGEBRA.md`
- `sddk-decision-kernel-architecture/04-specs/SPEC-040-WORKFLOW-LABORATORY.md`
- `sddk-decision-kernel-architecture/05-workflows/SDD-ADAPTIVE-WORKFLOW.md`
- `sddk-decision-kernel-architecture/05-workflows/WORKFLOW-PATTERNS.md`
- `sddk-decision-kernel-architecture/07-reference/RESEARCH-SOURCES-DYNAMIC-WORKFLOWS.md`
- `sddk-decision-kernel-architecture/08-spikes/SPIKE-006-DYNAMIC-WORKFLOW-RUNTIME.md`
- `sddk-decision-kernel-architecture/08-spikes/SPIKE-007-SDD-ADAPTIVE-ABLATION.md`
- `sddk-decision-kernel-architecture/09-implementation/DYNAMIC-WORKFLOW-IMPLEMENTATION-PLAN.md`

## DELETED

**None.** This refinement is intentionally additive/migratory. A-full remains the reference workflow.

## Scope explicitly unchanged

- Existing UAT pack/specs except their future use as an adaptive convergence capability.
- Provider failover/AgentHost design.
- Supply-chain/SBOM design.
- Static Cockpit architecture (only future views are added conceptually).
- Existing spikes 001–005.
