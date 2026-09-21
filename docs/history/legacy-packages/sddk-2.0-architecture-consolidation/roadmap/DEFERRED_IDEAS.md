# Deferred Ideas with Revisit Triggers

These ideas are intentionally not blockers for the consolidation cycle.

| Idea | Why deferred | Revisit when |
|---|---|---|
| Production graph DB as default | premature backend lock-in | local GraphStore misses measured latency/memory targets |
| Full Cypher implementation | high parser/runtime surface | minimal pattern IR cannot express 3 real required patterns |
| Distributed/team event store | local-first is current authority | multi-user concurrent write requirement appears |
| Automatic self-modifying pack promotion | governance risk | fork/replay, held-out gates and verified human approval are stable |
| Agentic relation behaviors broadly enabled | harder to reason about | deterministic relation behaviors have production evidence and proposal boundary is proven |
| Full editable architecture canvas | visualization complexity | read-only Explorer has concrete editing workflows/users |
| Universal software ontology | likely rigid/overengineered | three unrelated packs require the same missing universal semantic noun |
| Second GraphStore adapter | no concrete need yet | conformance suite exists and benchmark shows local adapter limitation |
| Remote hosted SDDK service | conflicts with local-first focus | team synchronization/hosted control-plane requirement is explicit |
| Automatic semantic promote of code | risk of hidden side effects | Git capability/worktree experiments + human governance prove safe process |
| Advanced graph ML/embeddings in core | not needed for correctness | retrieval quality data proves deterministic graph queries insufficient |
