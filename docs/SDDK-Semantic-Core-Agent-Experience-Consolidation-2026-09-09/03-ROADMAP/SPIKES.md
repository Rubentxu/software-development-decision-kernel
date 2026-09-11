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

## SP-06 Context retrieval evaluation — **COMPLETED 2026-09-11**

Question: does graph/exact/FTS ranking provide sufficient context quality before adding embeddings?  
Dataset: 20–30 real recovery questions; measure recall, noise and token budget.

**Outcome: DEFER embeddings.** Keyword ranking alone reaches 93.8% recall@3
(95.8% with per-doc aliases); graph expansion adds +4.2pp at recall@5 with
+2pp noise. The residual misses are cheap synonym gaps, not semantic
representation gaps. Full findings and revisit triggers:
`docs/architecture/spikes/SP-06-context-retrieval-evaluation.md`.
Reproducible harness: `crates/sddk-engine/src/spike_sp06.rs`.

## SP-07 CLI target ergonomics — **COMPLETED 2026-09-11**

Prototype `change` and `verify` Targets over existing commands. Compare number of commands/flags and error recovery versus current manual flow.

**Outcome:** Target surface wins on mechanics (1 command vs 2, no `--cycle-id` threading) but the spike exposed a false-success defect — stub targets reported `succeeded`/`executed` without performing work. Fixed in-cycle: honest `not_implemented`/`degraded` reporting shipped. `verify` is already target-equivalent via the `sddk verify` facade. Full findings: `docs/architecture/spikes/SP-07-cli-target-ergonomics.md`. Remaining: M6.3 wires real `change` task bodies (`has_body: true`).


## AX-S1 — CommandRegistry generation from current CLI definition — **COMPLETED 2026-09-11**

Question: can the existing CLI definition substrate expose typed command/arg metadata without parsing rendered help? Prototype extraction/generation, stable command IDs, machine schemas and example fixtures. Reject any design whose source of truth is scraped help text.

**Outcome: ALREADY DELIVERED (M7.1/M7.3), drift closed.** The substrate (`command_spec.rs` `CommandSpec` table + `command_surface.rs` + `agent-help` rendering) already exposes typed command/arg metadata from a structured table, not scraped help — the AX-S1 question was answered by construction during M7. The remaining gap was silent drift between the clap surface and the spec table (4 commands missing from specs: `agent-help`, `agent-result`, `run-view`, `target`; 2 stale spec names: `docs`/`inventory` pointing at `generate` subcommands). Closed in v1.168.14: specs completed/corrected, plus a permanent drift-guard test (`clap_surface_and_command_specs_are_in_sync`) that fails CI when the clap surface and the spec table diverge.

## AX-S2 — Instruction conflict algebra — **COMPLETED 2026-09-11**

Prototype typed semantic keys and strength classes. Test policy-vs-project, task-vs-skill and duplicate-equivalent directives. Output must be deterministic and fail closed for normative contradictions.

**Outcome: SUBSTRATE ALREADY DELIVERED (M7.5), conflict classes pinned by tests.** The `InstructionCompiler` (SPEC-014) already implements typed semantic keys, strength classes (`strength_rank`: Invariant 5 > Policy 4 > ProjectMandatory 3 > Task 2 > Skill 1 > Advisory/Hint 0) and fail-closed conflicts (`InvariantViolation`, `PolicyNarrowingViolation`, `TaskMissing`). The AX-S2 cycle added the missing conflict-class tests in `instruction_compiler.rs` (prefix `axs2_`): policy narrowing by project is allowed (positive case), a skill cannot satisfy a mandatory task requirement (Skill != Capability), duplicate-equivalent directives resolve case/whitespace-insensitively, conflict identity is order-independent, and the strength ladder is a verified total order. No production changes were needed: the algebra was already deterministic and fail-closed; it was unpinned.

## AX-S3 — Contextual command-surface token budget — **COMPLETED 2026-09-11**

Measure global CLI contract vs task-specific surfaces for common `change`, `verify`, `ship` and recovery agents. Select minimal surface heuristics that preserve task success while reducing irrelevant commands.

**Outcome: MINIMAL HEURISTIC WINS — keep depth-0 surfaces.** Task-specific surfaces cost 8-40 tokens vs 1073 for the global contract (96-99% reduction); the existing `command_matches_target` heuristic already delivers them. The real frontier is the `related` graph: the four facade specs have no `related` edges to their delegate commands, so depth-1 expansion adds nothing today. Recommendation adopted: keep minimal surfaces; enrich facade `related` edges lazily when a real workflow needs cross-command discovery. Measurements and revisit triggers: `docs/architecture/spikes/AX-S3-command-surface-token-budget.md`. Harness: `crates/sddk-cli/src/spike_axs3.rs` (4 pinned tests).

## AX-S4 — Provider adapter portability

Run one reviewer profile/task/fixture through two adapters (real or fake-compatible) using identical semantic contracts. Identify provider-specific data that truly belongs outside AgentProfile.

## AX-S5 — Agent asset static scanner

Prototype detection of deprecated commands, raw store/table references, authority language, duplicated prompt fragments and unregistered examples in Markdown/YAML/Rust literals. Keep advisory until false-positive rate is acceptable.
