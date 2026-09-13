# State and authority matrix

| Concepto | Owner | State class | Authority / source | Rebuildable |
|---|---|---|---|---|
| CanonicalEvent | Shared Kernel | Fact | Canonical Event Log | no |
| Evidence body | Shared Kernel | Object | CAS | no |
| Decision/Tradeoff revision | Decision | Object + fact refs | Decision Memory | no |
| KnowledgeAssertion payload | Knowledge | Object + lifecycle facts | Evidence/Decision/producers | current-view sí |
| KnowledgeBasis/KMT node | Knowledge | Projection/Object snapshot según uso | source/evidence roots | sí/derivable |
| SemanticGraph | Knowledge | Projection | canonical facts/objects | sí |
| AlignmentAssessment | Alignment | Object/projection with provenance | Knowledge + lens | recomputable cuando inputs disponibles |
| Workbook/Grid/Chart | Alignment/Knowledge | Projection | assessments/assertions/metrics | sí |
| VerifyReceipt | Verification | Fact/Object receipt | verification run | no |
| DebVerifyBaselineReceipt | Verification | Fact/Object receipt | full reconciliation | no |
| AdmissionDecision | Governance | Fact | AuthorityEngine + policy snapshot | no |
| ContextCapsule | Agent Experience | Ephemeral/Object receipt ref | ContextCompiler | no authority |

## Regla

Un chart nunca puede ser la única fuente de un gate; el gate consume facts/evidence o un explicit contract result con proof refs.
