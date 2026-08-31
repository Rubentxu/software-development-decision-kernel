# Architecture Spikes

Spikes answer uncertain questions before committing production architecture. Each spike must end in a short decision record, benchmark/test fixture and explicit recommendation.

## SPK-001 — Event canonicalization and hash stability

**Question:** Which canonical JSON implementation is robust and ergonomic in Rust?  
**Prove:** equivalent maps/hash inputs produce identical SHA-256 across processes/platforms.  
**Exit:** selected serialization version + golden vectors.

## SPK-002 — GraphStore local adapter envelope

**Question:** Can petgraph + SQLite checkpoints/indexes handle representative SDDK/code graphs?  
**Dataset:** small, medium and large representative repositories.  
**Measure:** rebuild time, memory, 1-hop/3-hop queries, pattern latency, diff latency.  
**Revisit external DB when:** P95 interactive query > 200 ms for expected Explorer scopes or memory exceeds agreed desktop budget.

## SPK-003 — Pattern language

**Question:** Small custom graph pattern IR vs Cypher subset parser?  
**Need:** node predicates, relation chains, NOT EXISTS, bounded depth, temporal/version predicate.  
**Bias:** minimal IR first; compile to GraphStore capabilities.

## SPK-004 — Fork/replay shared-prefix cache

**Question:** Can SDDK reconstruct and fork a run without re-calling LLM/tools?  
**Prove:** shared prefix event/state hashes match and cached I/O is content-bound.

## SPK-005 — UAT extraction seam

**Question:** Separate crates vs modules inside a pack?  
**Measure:** compile time, cyclic dependency pressure, test ergonomics, public API surface.  
**Invariant:** bounded context must remain independent of CLI/storage regardless of crate count.

## SPK-006 — Signed local gate receipt

**Question:** How best to map receipt payloads onto existing cosign/in-toto style attestations?  
**Prove:** remote verifier rejects wrong commit, stale receipt, altered gate list and invalid signature.

## SPK-007 — Explorer rendering

Evaluate representative graph sizes with:

- WebGL graph renderer (e.g. Cosmos-style approach);
- SVG for bounded C4 views;
- tldraw-style canvas for editable overlays.

**Decision:** renderer adapter matrix by view type, not one renderer for all views.

## SPK-008 — Cognicode bridge

Map code graph concepts to universal SDDK primitives without importing AST ontology into core. Prove architecture-diff and impact-query use cases.

## SPK-009 — Behavioral IR authoring UX

Compare sidecar YAML vs Markdown frontmatter as canonical authoring surface. Generate permission inventory and docs from both prototypes.

## SPK-010 — Entropy metrics calibration

Run metrics against current large UAT/CLI modules and known healthy modules. Define advisory bands that identify responsibility/coupling problems without rewarding meaningless file splitting.
