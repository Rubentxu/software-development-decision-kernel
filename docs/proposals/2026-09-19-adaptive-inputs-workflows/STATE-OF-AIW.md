# AIW — State of adoption (live reconciliation)

> **Purpose:** this document is the live state of the AIW package
> versus actual delivery. It exists to prevent two failure modes:
>
> 1. Treating the README "proposed" label as a statement about
>    capability state (it is not — it is a statement about
>    document adoption).
> 2. Re-implementing capabilities that already exist under a
>    different name in `tests/cycle-artifacts/`.
>
> **Authority:** `docs/SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md`
> remains the live execution roadmap. This file is a **delta**, not
> a competing roadmap. Per the operator's decision, AIW is becoming
> a "map of capabilities and acceptance criteria" reconciled with
> the cycles already executed.

## §1 Reconciliation rule

A capability is **delivered** if and only if:

1. A cycle / slice / commit exists in `tests/cycle-artifacts/`
   or in the workspace tree.
2. The corresponding receipt documents an **OBSERVED** execution
   (real binary, real storage, real CLI) — **or** a documented
   partial state with reason (NOT_EVALUATED, NOT_PROCEED, STOP).
3. The acceptance criteria in `uat/UAT-MATRIX.md` are matched
   either positively (PASS) or honestly (NOT_RUN with reason).

The deliverable **does not need** to live under the AIW-S* id
tree. Macro-cycle A6, A7, A8 WorkItems, and ad-hoc cycles all
count.

## §2 Capability matrix

Legend:

- **DELIVERED (real)** — production binary executed, OBSERVED in receipt.
- **DELIVERED (deterministic)** — closed against Fake/null provider; not promoted to production profile.
- **PARTIAL** — partial closure with documented state.
- **NOT_STARTED** — no committed work.
- **NOT_EVALUATED** — environmental constraint (no binary / no env var).
- **NOT_PROCEED** — audit-only, no code change warranted.
- **STOP** — STOP condition fired pre-implementation; awaiting operator decision.

| Capability (AIW) | State | Evidence (commit / receipt / test) | Carencia concreta |
|---|---|---|---|
| **AIW-S0** — Preflight A6 | DELIVERED (real) | `tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s0-preflight/SCOPE-CONTRACT.md` + `deba961` (commit on `main-temp`) — cognicode-mcp v0.97.1 arrancable via stdio MCP. | None — preflight completo. |
| **AIW-S1** — CogniCode real → Verify | DELIVERED (real) | `tests/cycle-artifacts/.../aiw-s1-cognicode-real/RECEIPT.md` + commits `f65e182`, `427bbf4`, `f27f85b`. E2E CLI exit=0, verdict `Verified`, 10 observaciones, SHA-256 digest. EXT tests 5/5 contra binario real. Base green sin proveedor (A03+A06). Regresión 1251/0 passed. | Limitaciones documentadas: grafo lightweight ≠ absence de impacto; veredicto basado en find_usages. |
| **AIW-S1b** — Forma sucesora de evidencia observacional, CONDICIONAL | STOP-pending | A6 macro-cycle S4 STOP report (`tests/cycle-artifacts/.../a6-static-enhanced-readiness/slices/s4-durability/SCOPE-CONTRACT.md`) propone 3 opciones (A/B/C) que mapean a AIW-ADR-03. | Necesita decisión operador A/B/C para saber si el hueco de shape es real. **No abrir AIW-S1b hasta que S4 cierre.** |
| **AIW-S2** — Captura estructurada de tests y gates | DELIVERED (locally) | `crates/sddk-gateway/src/runner_receipt.rs` + `crates/sddk-gateway/tests/runner_receipt_e2e.rs`. Slice: `tests/cycle-artifacts/.../aiw-s2-test-runner-capture/{SCOPE-CONTRACT,UAT-EVIDENCE,RECEIPT}.md`. Commit `ab9fe23`. 10 unit + 11 E2E tests; bounded_runner_contract 18/18 (C1 preserved). UAT-T01/T02/T04/T05/T06/T07/T08/T09/T10 PASS. T08 per-testcase normalization is **PARTIAL** (consumer-of-receipt concern, deferred to a follow-up AIW slice per SCOPE §6). Wrapper CLI NOT added (C3: optional per spec). Receipt surface: `RunnerReceipt` + `RunnerStatus` + `OutputRefs` + `RedactionMarker` + `RunOutcomeView` (wire mirror). | Waiting on `scripts/release.sh` push authorization. T08 per-testcase normalization to be folded into a follow-up AIW slice alongside AIW-S3. |
| **AIW-S3** — Handoff durable entre DOS tareas | NOT_STARTED | No commit con identificador AIW-S3 en este CWD. Aceptación UAT-H01..H09 sin ejecutar. | Falta adaptador mínimo a `CapsuleInputs` / `ContextCompiler` desde store real + envelope/outcome/receipts efectivos; cierre/reapertura preservando WorkItem/PlanRevision/Attempt/evidence/dissent. |
| **AIW-S4** — Expansión dinámica por evidencia | NOT_STARTED | Aceptación UAT-W01..W11 sin ejecutar. | Falta trigger Secretary → Authority → compiler/validator → PlanRevision → runtime sin repetir nodos. |
| **AIW-S5** — Chronos real | DELIVERED (real) | `tests/cycle-artifacts/.../aiw-s5-chronos-runtime/RECEIPT.md`. E2E CLI con `chronos-mcp` v0.1.0 real, 3 EXT tests passed, 4 unit tests passed, E2E exit=0. | No declara cierre A7 `RUNTIME_ENHANCED`; verificación es vertical (un programa, un contrato), no cobertura completa. |
| **AIW-S6** — Correlación estático/runtime | DELIVERED (by-infrastructure) | A4-5a falsification suite (`crates/sddk-engine/tests/a4_5a_intelligence_loop_composition.rs`) ya ejerce `compose_intelligence_loop` + `derive_advisory_context` con `happy_inputs`, `contradictory_composition_preserved`, `unknown_gap_composition` (10+ tests). Slice: `tests/cycle-artifacts/.../aiw-s6-correlation-a8/{SCOPE-CONTRACT,UAT-EVIDENCE,RECEIPT}.md`. P09/P10/P11/P12 cubiertos por A4-5a. No `src/` change; no nueva dependencia. | Cierre por infrastructure: añadir un test redundante violaría "no fake PASS, no inferred per-test data". AIW-S7/S8 pueden componerse contra la API pública ya probada. |
| **AIW-S7** — Secretary y atención adaptativa | NOT_STARTED | Aceptación UAT-G01..G09 sin ejecutar. | Falta conectar con Planning snapshot reconciliado, Verify gaps, outcomes. |
| **AIW-S8** — CLI/host y límites de empaquetado | NOT_STARTED | Aceptación UAT-X01..X08 sin ejecutar. Evaluativo. | No crear nuevas fronteras hasta acreditar consumidores reales. |

## §3 Cross-mapping with macro-cycle A6

The macro-cycle `a6-static-enhanced-readiness` (closed locally,
in `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-static-enhanced-readiness/`)
addresses **G7 of the production-ready gate** through the fake-driven
deterministic profile. AIW-S1 addresses the same gate through the
real provider. Both satisfy the AC10 contract end-to-end; they
exercise different routes:

| Route | Entry point | Provider | Verifier | Profile |
|---|---|---|---|---|
| A6 / S3 (macro-cycle) | `code_intelligence_port_fake` → `verify_kernel::evidence_source_static_provider::build()` → `StaticProviderDomain::evaluate` | Fake / Null | Rust unit + integration tests | Deterministic (G7 not promoted) |
| AIW-S1 (package) | CLI `sddk verify-kernel --domain static_provider --provider-bin <cognicode-mcp>` | Real (`cognicode-mcp v0.97.1`) | CLI E2E | Real (G7 candidate) |

**Both routes close the in-process contract. AIW-S1 additionally
provides the EXT (real-binary) profile needed for promotion.**
Promotion to `STATIC_ENHANCED_PRODUCTION_READY` requires AIW-S1
plus G7 sub-criteria (provider lifecycle, capability negotiation,
stable refs/digests, etc.).

## §4 Cross-mapping with macro-cycle A7 (Chronos)

A7 is the `RUNTIME_ENHANCED` track. AIW-S5 already delivered a
vertical E2E with `chronos-mcp v0.1.0`. AIW-S5 is **not** a full
A7 closure; it is the first acceptable real run. The full A7
profile (multi-program coverage, runtime provider lifecycle, etc.)
remains pending.

## §5 Where the macro-cycle S4 STOP maps onto AIW

The macro-cycle `a6-static-enhanced-readiness` slice S4 STOP is a
**STOP report with three options**:

- **Option A** — JSON shape contract (`schema_version` on payload).
- **Option B** — typed event class in `event_registry`.
- **Option C** — defer to downstream cycle.

AIW-S1b is the **only** AIW hito that consumes this decision. Per
the AIW roadmap:

> Si no permite guardar dos observaciones contradictorias con
> identity+basis+relation sin pérdida, aprobar AIW-ADR-03 mediante
> número canónico y migración aditiva; dual-read compatibility,
> no segundo store, writer canónico.

If Option A or B is chosen and a real test demonstrates the gap,
AIW-S1b opens. If Option C is chosen (defer), AIW-S1b remains
`PARTIAL` and `NOT_STARTED` until a future cycle.

## §6 What needs operator decision now

| # | Decision | Where | Effect |
|---|---|---|---|
| 1 | **S4 A/B/C** | `tests/cycle-artifacts/.../a6-static-enhanced-readiness/slices/s4-durability/SCOPE-CONTRACT.md` §4 | Unblocks AIW-S1b if A or B. |
| 2 | **AIW adoption status** — partial (capability map) vs full (competing roadmap) | This file | Once decided, replace the README "proposed" label with the actual adoption status. |
| 3 | **H4.7 push** — 4 SHAs (`b60eca5f`, `e2da4131`, `b036b5ad`, `5427beaf`) not in this checkout | (separate incidence) | Pending external bundle / cherry-pick / remote reference from operator. |

## §7 What does not need operator decision (autonomously executable)

Per the operator's instruction, the following are autonomously
executable within existing contracts:

- **AIW-S2** (captura tests/gates) — slice scoped to `verify_kernel_cmd` + `nextest` runner + one structured format.
- **AIW-S3** (handoff durable) — first adapter to `CapsuleInputs`/`ContextCompiler` from real storage.
- **AIW-S4** (expansión dinámica) — one real trigger: Secretary proposes → Authority validates → compiler accepts → runtime executes, no re-runs.
- **AIW-S6** (correlación A8) — uses `intelligence_loop` + relations, no new ADT unless justified.
- **Roadmap reconciliation** updates to this file and to the AIW README.

Each slice must respect the project's hard constraints (no
auto-bumps, push via release.sh, STOP conditions named in the
slice SCOPE, no closed-vocabulary extension unless justified).

## §8 References

- AIW roadmap: `docs/proposals/2026-09-19-adaptive-inputs-workflows/roadmap/MILESTONES.md`
- AIW UAT matrix: `docs/proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md`
- AIW merge plan: `docs/proposals/2026-09-19-adaptive-inputs-workflows/integration/MERGE-PLAN.md`
- AIW legacy disposition: `docs/proposals/2026-09-19-adaptive-inputs-workflows/integration/LEGACY-DISPOSITION.md`
- Macro-cycle A6: `tests/cycle-artifacts/p-63676b11dc0ef88f/a6-static-enhanced-readiness/`
- AIW-S0 receipt: `tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s0-preflight/SCOPE-CONTRACT.md`
- AIW-S1 receipt: `tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s1-cognicode-real/RECEIPT.md`
- AIW-S5 receipt: `tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s5-chronos-runtime/RECEIPT.md`
- Canonical roadmap: `docs/SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md`
- G7 gate: `docs/SDDK-Production-Readiness-Alignment-2026-09-14/03-PRODUCTION-READY-GATE.md` §9
