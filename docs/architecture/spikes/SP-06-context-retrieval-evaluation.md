# SPIKE SP-06 — Context retrieval evaluation

> **Status:** completed 2026-09-11 (v1.168.9 workspace).
> **Question (per SPIKES.md):** does graph/exact/FTS ranking provide
> sufficient context quality before adding embeddings?
> **Harness:** `crates/sddk-engine/src/spike_sp06.rs` — deterministic,
> reproducible via `cargo test -p sddk-engine --lib spike_sp06 -- --nocapture`.

## Method

- **Corpus:** 18 synthetic documents mirroring real vault record shapes
  (cycles, ADRs, INCs, SPECs) with citation edges between them.
- **Dataset:** 24 recovery-style questions, each with 1–2 ground-truth
  relevant documents. Questions deliberately include synonym/paraphrase
  gaps (e.g. "why was PlanningEvidenceKind deprecated" vs docs that speak
  of "EvidenceRef canonical authority").
- **Rankers compared:**
  1. `exact` — full-query substring match.
  2. `keyword` — tokenized overlap, title-boosted (approximated FTS).
  3. `graph` — keyword seed + one-hop expansion over citation edges (0.5 decay).
  4. `keyword+aliases` — keyword plus tiny per-doc alias lists (mitigation probe).

## Results

| ranker | r@1 | r@3 | r@5 | noise@3 | noise@5 | tokens@3 |
|---|---|---|---|---|---|---|
| exact | 0.0% | 0.0% | 0.0% | — | — | 0 |
| keyword | 60.4% | 93.8% | 95.8% | 43.1% | 49.6% | 138 |
| graph | 60.4% | 93.8% | 100.0% | 45.1% | 56.1% | 141 |
| keyword+aliases | 60.4% | **95.8%** | ~100% | ~43% | ~50% | ~145 |

## Findings

1. **Lexical retrieval is already strong.** Plain keyword ranking achieves
   93.8% recall@3 on this dataset. The marginal contribution of graph
   expansion is +4.2pp at recall@5, paid for with +2pp noise.
2. **The residual misses are synonym gaps, and they are cheap to fix.**
   Both recall@3 misses traced to vocabulary not shared between query and
   document (PlanningEvidenceKind vs EvidenceRef; "promoted to deny" vs
   "blocking enforcement"). A 4-entry-per-document alias list recovered
   most of the gap (93.8% → 95.8%) at near-zero cost.
3. **Token budget is a non-issue at this scale.** ~140 tokens for top-3
   context. Even with 10× corpus growth the budget stays an order of
   magnitude below any practical context window concern.
4. **Exact matching is useless as a ranker** (0% recall) — recovery
   questions never quote documents verbatim. Keep it only as a
   zero-cost fast path for id lookups.
5. **Noise@3 ~45% is the real quality frontier** — not recall. Half the
   retrieved context is irrelevant, which dilutes agent attention. This
   is a ranking-calibration problem (better weighting, recency, kind
   filters), not a semantic-representation problem.

## Decision: DEFER embeddings

**Embeddings are not justified now.** Rationale:

- The 95%+ recall achievable with keyword + aliases + optional one-hop
  graph expansion leaves no meaningful quality gap for embeddings to
  close on this class of queries.
- Embeddings would add: an index-build step (staleness coupling with the
  fact log), a vector store dependency, nondeterminism pressure against
  the "deterministic, provenance-aware" capsule contract (SPEC-021 /
  ADR-081), and an API/token cost — for a projected <5pp gain.
- The documented failure mode (synonym gap) is better served by curated
  aliases/tags, which the vault already partially carries (titles, ADR
  cross-references, `replacement` fields in the lints registry).

**Revisit triggers** (when embeddings become worth re-evaluating):

- Corpus grows past ~1k documents and noise@3 worsens despite ranking
  calibration.
- Recovery questions become predominantly natural-language paraphrase
  with <80% lexical overlap with their targets (measure: rerun this
  harness with live query logs).
- Cross-language or cross-modal retrieval enters the requirements.

## Follow-ups (optional, cheap)

1. Ship per-doc `aliases` as an optional field in vault records / graph
   nodes and thread it through the ContextCapsule adapter (small cycle).
2. Add noise-aware re-ranking (kind filter + recency) to the capsule
   compiler — attacks the actual frontier (noise@3), not recall.
3. Preserve this harness as the regression baseline: any future
   retrieval change must match or beat the recorded numbers on this
   dataset.

## Scope note

The synthetic corpus intentionally mirrors the vault's *shapes* (cycle
manifests, ADRs, INCs, SPECs with citation edges), not its full content.
The numbers are directionally valid for the retrieval-strategy question;
absolute percentages on the live vault will differ and should be
re-measured with the harness pointed at real ActiveGraphInput data
before any production ranking change.
