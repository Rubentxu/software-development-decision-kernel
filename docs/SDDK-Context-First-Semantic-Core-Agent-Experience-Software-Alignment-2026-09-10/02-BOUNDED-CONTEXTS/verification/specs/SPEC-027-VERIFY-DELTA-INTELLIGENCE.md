# SPEC-027 — Verify Delta Intelligence

`verify` asks: **what changed recently, what is impacted, and what knowledge must be refreshed?**

Pipeline:

```text
source/git delta
 -> fingerprint diff
 -> changed units
 -> impact closure
 -> staleness propagation
 -> base analyzers
 -> optional static provider
 -> previous cards/assertions
 -> targeted LLM evaluation
 -> contradictions
 -> affected workbook refresh
 -> external policy controls
 -> VerifyReceipt
```

Runtime provider is trigger-based only (runtime invariant, concurrency, golden scenario impact, static/runtime contradiction or required policy).

Verify MUST NOT full-scan by default.
