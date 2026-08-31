---
id: INC-NNN-{slug}
title: "{one-line summary}"
status: open
severity: critical|high|medium|low
priority: P0|P1|P2|P3
fingerprint: "{hex}"
fingerprint_aliases: []
cluster_id: CL-NN
created: YYYY-MM-DD
created_by: actor-name
owner: actor-name
---

# INC-NNN-{slug} — {title}

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

<problem statement: what's wrong, where, why it matters>

## Rationale

<why this severity + priority + cluster_id; cite evidence>

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| YYYY-MM-DD | creator | created | {finding-id} from cycle-{N} |

## References

<finding.evidence_refs: links to commits, docs, or run artifacts>

> Filled by `sddk-archive` (cycle-8+); consumed by `sddk-debt-verify` for cross-cycle correlation via fingerprint.
