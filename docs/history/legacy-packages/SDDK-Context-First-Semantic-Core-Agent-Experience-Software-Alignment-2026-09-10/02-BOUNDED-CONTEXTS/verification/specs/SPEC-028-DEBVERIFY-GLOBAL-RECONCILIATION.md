# SPEC-028 — DebVerify Global Reconciliation

`deb-verify` asks: **does accumulated knowledge still describe the whole project, and where is implementation/architecture/knowledge debt?**

Pipeline:

```text
project manifest reconciliation
 -> KMT validation/rebuild branches
 -> full/stratified static analysis
 -> expected-vs-observed architecture
 -> risk×uncertainty selection
 -> progressive deep reviews
 -> selected runtime scenarios
 -> tradeoff/waiver/invariant revisit
 -> contradiction reconciliation
 -> all workbooks refresh
 -> ProjectKnowledgeBaseline
```

DebVerify can discover stale/debt in files not changed recently. It is not `verify --all`.
