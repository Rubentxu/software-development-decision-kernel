# Focused spikes

Spikes are time-boxed research with a binary decision/output; they are not hidden implementation projects.

## SP-01 Canonical ledger convergence

Question: which current table/port becomes the long-term CanonicalEventLog with least migration risk?  
Output: ADR amendment + write/read migration map + compatibility deadline.

## SP-02 Revision substrate extraction

Question: can PlanRevision and Decision Memory share object/ref/reachability primitives without forcing plan semantics into memory?  
Prototype: generic ObjectId/Revision/Ref store with CAS and CAS-update concurrency tests.

## SP-03 Semantic graph v2 mapping

Question: map current GraphProjection + ActiveGraph semantics to one typed schema without losing rebuild parity.  
Output: relation taxonomy and compatibility projection fixture.

## SP-04 Cycle slimming

Question: which CycleStatus variants are truly delivery states versus derived runtime states?  
Output: state transition crosswalk and migration fixture.

## SP-05 Authority facade

Question: can existing permission/gate/risk/approval components be composed behind one application service before invasive refactoring?  
Output: one governed capability migrated end-to-end.

## SP-06 Context retrieval evaluation

Question: does graph/exact/FTS ranking provide sufficient context quality before adding embeddings?  
Dataset: 20–30 real recovery questions; measure recall, noise and token budget.

## SP-07 CLI target ergonomics

Prototype `change` and `verify` Targets over existing commands. Compare number of commands/flags and error recovery versus current manual flow.


## AX-S1 — CommandRegistry generation from current CLI definition

Question: can the existing CLI definition substrate expose typed command/arg metadata without parsing rendered help? Prototype extraction/generation, stable command IDs, machine schemas and example fixtures. Reject any design whose source of truth is scraped help text.

## AX-S2 — Instruction conflict algebra

Prototype typed semantic keys and strength classes. Test policy-vs-project, task-vs-skill and duplicate-equivalent directives. Output must be deterministic and fail closed for normative contradictions.

## AX-S3 — Contextual command-surface token budget

Measure global CLI contract vs task-specific surfaces for common `change`, `verify`, `ship` and recovery agents. Select minimal surface heuristics that preserve task success while reducing irrelevant commands.

## AX-S4 — Provider adapter portability

Run one reviewer profile/task/fixture through two adapters (real or fake-compatible) using identical semantic contracts. Identify provider-specific data that truly belongs outside AgentProfile.

## AX-S5 — Agent asset static scanner

Prototype detection of deprecated commands, raw store/table references, authority language, duplicated prompt fragments and unregistered examples in Markdown/YAML/Rust literals. Keep advisory until false-positive rate is acceptable.
