---
id: arch-spec-005-semantic-graph-and-why
package_local_id: SPEC-005
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-005-SEMANTIC-GRAPH-AND-WHY.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-005 — SEMANTIC-GRAPH-AND-WHY

> **Mirror of package SPEC `SPEC-005`.** Repository-native identifier is `arch-spec-005-semantic-graph-and-why` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-005` |
| Repository native | `arch-spec-005-semantic-graph-and-why` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-005-SEMANTIC-GRAPH-AND-WHY.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

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
