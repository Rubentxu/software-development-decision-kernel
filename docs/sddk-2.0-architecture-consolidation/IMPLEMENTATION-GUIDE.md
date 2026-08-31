# Implementation Guide — First 10 Pull Requests

This is a suggested merge sequence that produces value early while minimizing flag-day changes.

1. **PR-01 Docs/guardrails:** add architecture rules + document role split; no runtime behavior change.
2. **PR-02 Baseline tests:** CLI/UAT compatibility fixtures + entropy baseline.
3. **PR-03 Application ports:** introduce inward repository/EventStore traits and adapters around existing code.
4. **PR-04 Break engine->storage:** migrate one use case and turn ARCH001 green.
5. **PR-05 Thin CLI slice:** remove direct persistence from one CLI command family.
6. **PR-06 CEP v1:** Rust EventEnvelope + JSON schema + canonical hash vectors.
7. **PR-07 SQLite ledger:** append/replay/export + projection checkpoint skeleton.
8. **PR-08 Evidence v1:** universal EvidenceItem with UAT compatibility adapter.
9. **PR-09 Proposal/receipt path:** route one governed capability through proposal-policy-postcondition receipt.
10. **PR-10 Pack manifest v2:** schema/validator/registry skeleton; no UAT move yet.

Only after these seams are stable should the UAT extraction start. This avoids moving a large module into another large module without solving the underlying dependency problem.
