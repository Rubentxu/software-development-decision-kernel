# SPEC-005 — Semantic Graph and causal explanation

## Graph contract

SemanticGraph is a rebuildable typed projection. Nodes/edges reference canonical facts/objects.

Core node families: project, goal, work_item, cycle, workflow, plan_revision, run, action, decision, alternative, assumption, evidence, risk, debt, contribution, synthesis, dissent, artifact.

Core relations include `depends_on`, `caused_by`, `supports`, `contradicts`, `justifies`, `selected_over`, `supersedes`, `produced_by`, `consumed_by`, `gates`, `references`, `affects`.

## Typed extension

```text
NodeKind = Core(kind) | Extension("namespace.kind")
RelationKind = Core(kind) | Extension("namespace.relation")
```

## Why result

```text
ExplanationProof {
 target
 answer
 proof_paths[]
 evidence[]
 decisions[]
 assumptions[]
 dissent[]
 counter_evidence[]
 unknowns[]
 as_of
 graph_revision
 memory_head
}
```

## Queries

- `sddk why <entity>` general causal explanation;
- `sddk why decision:<id>` decision provenance;
- `sddk why debt:<id>` debt cause/evidence/impact;
- `sddk why-not alternative:<id>` rejection reasoning;
- `sddk impact <entity>` forward impact traversal;
- historical `--at <event-seq|memory-ref>` where supported.

A why answer MUST preserve unknowns; absence of a path is not proof of non-causality.

## 2026-09-10 amendment — Alignment explanation

WHY may traverse AlignmentAssessment -> KnowledgeAssertion -> Evidence/Decision/Tradeoff. It must distinguish OBSERVED facts from INFERRED assessments and advisory SUGGESTION. No causal path may be invented.
