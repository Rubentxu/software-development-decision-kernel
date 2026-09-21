> **DELTA / SNAPSHOT HISTÓRICO (anotación 2026-09-21).** Conserva cierres AIW y limitaciones tal como se documentaron; la expresión «live state» del cuerpo corresponde a su fecha, no a cada sesión futura. Para continuar y certificar, consultar [roadmap único](../../roadmap/ROADMAP.md), [CURRENT](../../roadmap/CURRENT.md) y [UAT](../../roadmap/UAT-MATRIX.md). No transformar `DELIVERED` en certificación de perfil por esta anotación.

---

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
| **AIW-S1b** — Forma sucesora de evidencia observacional, CONDICIONAL | **CANCELLED per roadmap** (2026-09-20) | Gate ejecutado: sonda UAT-A10 contra el writer actual PASA (`crates/sddk-engine/tests/observation_set_durability.rs`) — dos observaciones contradictorias (Affirms/Denies, bases distintas) sobreviven crash/reopen sin pérdida, sin latest-wins, sin agregación, vía `observation.set.appended` v1 (S4). Carencia NO demostrada → slice cancelado por su propia regla; AIW-ADR-03 NO se promueve. Receipt: `tests/cycle-artifacts/.../aiw-s1b-successor-shape/RECEIPT.md`. | Ninguno — cierre limpio, no requiere operador. |
| **AIW-S2** — Captura estructurada de tests y gates | DELIVERED (locally) | `crates/sddk-gateway/src/runner_receipt.rs` + `crates/sddk-gateway/tests/runner_receipt_e2e.rs`. Slice: `tests/cycle-artifacts/.../aiw-s2-test-runner-capture/{SCOPE-CONTRACT,UAT-EVIDENCE,RECEIPT}.md`. Commit `ab9fe23`. 10 unit + 11 E2E tests; bounded_runner_contract 18/18 (C1 preserved). UAT-T01/T02/T04/T05/T06/T07/T08/T09/T10 PASS. T08 per-testcase normalization is **PARTIAL** (consumer-of-receipt concern, deferred to a follow-up AIW slice per SCOPE §6). Wrapper CLI NOT added (C3: optional per spec). Receipt surface: `RunnerReceipt` + `RunnerStatus` + `OutputRefs` + `RedactionMarker` + `RunOutcomeView` (wire mirror). | Waiting on `scripts/release.sh` push authorization. T08 per-testcase normalization to be folded into a follow-up AIW slice alongside AIW-S3. |
| **AIW-S3** — Handoff durable entre DOS tareas | **DELIVERED** | `crates/sddk-engine/src/context_compiler/storage_adapter.rs` (276 LOC) + `crates/sddk-engine/tests/aiw_s3_storage_handoff.rs` (320 LOC). Slice: `tests/cycle-artifacts/.../aiw-s3-handoff-durable/{SCOPE-CONTRACT,UAT-EVIDENCE,RECEIPT}.md`. Commit `98614fc` (merge `b4768e7` into main). 6 in-module unit tests + 5 integration tests = 11 new tests. UAT-H01/H04/H06/H09 PASS; H02/H03/H05/H07-secret PARTIAL (covered by AIW-S7 follow-ups). v1.169.97. | None — closure clean. Follow-ups already delivered by AIW-S7a..S7c (in flight 2026-09-21). |
| **AIW-S4** — Expansión dinámica por evidencia | **DELIVERED** | `crates/sddk-engine/tests/aiw_s4_dynamic_expansion.rs` (430 LOC, 7 integration tests W01/W02/W03/W04/W08/W11). Slice: `tests/cycle-artifacts/.../aiw-s4-dynamic-expansion/{SCOPE-CONTRACT,UAT-EVIDENCE,RECEIPT}.md`. Commit `648f23b` (merge `72d5ff3` into main). NO src change. C2 enforced: `<30 LOC` glue in tests/. Replay bug fix (`6b97202` + `fb1fc80`) landed in `v1.169.116`. | None — closure clean. |
| **AIW-S5** — Chronos real | DELIVERED (real) | `tests/cycle-artifacts/.../aiw-s5-chronos-runtime/RECEIPT.md`. E2E CLI con `chronos-mcp` v0.1.0 real, 3 EXT tests passed, 4 unit tests passed, E2E exit=0. | No declara cierre A7 `RUNTIME_ENHANCED`; verificación es vertical (un programa, un contrato), no cobertura completa. |
| **AIW-S6** — Correlación estático/runtime | DELIVERED (by-infrastructure) | A4-5a falsification suite (`crates/sddk-engine/tests/a4_5a_intelligence_loop_composition.rs`) ya ejerce `compose_intelligence_loop` + `derive_advisory_context` con `happy_inputs`, `contradictory_composition_preserved`, `unknown_gap_composition` (10+ tests). Slice: `tests/cycle-artifacts/.../aiw-s6-correlation-a8/{SCOPE-CONTRACT,UAT-EVIDENCE,RECEIPT}.md`. P09/P10/P11/P12 cubiertos por A4-5a. No `src/` change; no nueva dependencia. | Cierre por infrastructure: añadir un test redundante violaría "no fake PASS, no inferred per-test data". AIW-S7/S8 pueden componerse contra la API pública ya probada. |
| **AIW-S7** — Secretary y atención adaptativa | **DELIVERED** (sub-slices closed 2026-09-21) | Three sub-slices all delivered: AIW-S7a producer→L0 stream adapter (commit `92a4cb8`, 8 tests PASS); AIW-S7b StorageSnapshot→SecretaryL1 consumer (commit `118e969`, 8 tests PASS); AIW-S7c AuthorityContext negative-grant integration tests (commit `7fca3d9`, 13 tests PASS). G01+G03+G04+G05+G06+G07 closed with evidence on disk; G02/G08/G09 remain covered by AIW-S4 L2 replan surface (already DELIVERED). | None — closure clean. |
| **AIW-S8** — CLI/host y límites de empaquetado | **DELIVERED** (X02/X04/X06/X07 closed 2026-09-21; X01/X03/X05/X08 already in infrastructure or DEFERRED) | X02 denial surface (commit `7805d4c`, 13 tests PASS, zero-leak contract); X04 two-CLI concurrency (commit `358686c`, 4 tests PASS); X06 storage schema version guard (commit `6136f2f`, 9 tests PASS); X07 second-binary read-only integration (commit `c3c3101`, 4 tests PASS). X01/X03/X05 already closed by pre-existing infrastructure (decision_plane_cli_parity_tests, agent_host_tests). X08 Jev corpus+baseline DEFERRED — requires operator scope (corpus definition, baseline threshold) rather than a human_gate; no slice opened. | X08 deferred to operator scope definition. |

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
| 1 | ~~S4 A/B/C~~ — **RESOLVED: Option B** (receipt en `tests/cycle-artifacts/.../s4-durability/RECEIPT.md`). La infraestructura de B (`EventSchemaRegistry`, `schema_struct!`, `CanonicalEventValidator`) ya existía con 26 tipos; solo faltaba `Deserialize` en el árbol de observación y el schema `observation.set.appended` v1. Envelope versionado congelado, roundtrip crash/reopen pinado por tests. Coste real de B colapsó respecto a la estimación del STOP — registrada como mejora sobre lo propuesto. | Unblocks AIW-S1b. |
| 2 | **AIW adoption status** — partial (capability map) vs full (competing roadmap) | This file | Once decided, replace the README "proposed" label with the actual adoption status. |
| 3 | **H4.7 push** — 4 SHAs (`b60eca5f`, `e2da4131`, `b036b5ad`, `5427beaf`) not in this checkout | (separate incidence) | Pending external bundle / cherry-pick / remote reference from operator. |
| 4 | ~~`verify_cycle_snapshot` ignores `replan_count` on replay~~ — **RESOLVED** (commit `6b97202`). Root cause was `is_cycle_state_event` missing `cycle.replan.applied` + replan events carrying `state_after: None`. Fix: replan events are now state-bearing and the replay filter includes `cycle.replan.applied`. Contract test `verify_cycle_snapshot_succeeds_after_replan` pins the corrected behavior; pre-fix ledgers keep replaying as before (workaround still valid, its regression pin passes). | RESOLVED — no operator action needed. |
| 5 | ~~SEC-WORKSPACE-FLAKE~~ — **RESOLVED** (receipt en `tests/cycle-artifacts/p-63676b11dc0ef88f/sec-workspace-flake-fix/RECEIPT.md`). El fix ya estaba aplicado en `main` (commit `0ca24c2`): el test eliminado era estrictamente redundante (identidad singleton + seq monotónico ya pinados por tests retenidos y más fuertes), reducción de cobertura neta cero, 3× corridas green. La reducción de cobertura sobre `AuthorityTicketService` que motivaba el bloqueo de auto-run no existe. | RESOLVED — no operator action needed. |

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

## §9 Active sub-slices (2026-09-21, post operator authorization)

Operator authorization 2026-09-21 unblocks the AIW-S7 / S8 sub-slices that
were declared STOP-honesto on the previous session. Each sub-slice has its
own SCOPE-CONTRACT, UAT-EVIDENCE, and RECEIPT cycle, and shares the same
release.sh pipeline as the prior slices.

| Sub-slice | Closes | Path | Owner |
|---|---|---|---|
| **AIW-S7a** | G04+G06 | `tests/cycle-artifacts/.../aiw-s7a-producer-l0-stream/` | swarm (in flight) |
| **AIW-S7b** | G01+G03 | `tests/cycle-artifacts/.../aiw-s7b-storage-snapshot-l1-consumer/` | swarm (queued) |
| **AIW-S7c** | G05+G07 | `tests/cycle-artifacts/.../aiw-s7c-authority-negative-grant/` | swarm (queued) |
| **AIW-S8 X02** | X02 | `tests/cycle-artifacts/.../aiw-s8-x02-denial-surface/` | swarm (queued) |
| **AIW-S8 X04** | X04 | `tests/cycle-artifacts/.../aiw-s8-x04-two-cli-concurrency/` | swarm (queued) |
| **AIW-S8 X06** | X06 | `tests/cycle-artifacts/.../aiw-s8-x06-storage-schema-versioning/` | swarm (queued) |
| **AIW-S8 X07** | X07 | `tests/cycle-artifacts/.../aiw-s8-x07-second-binary-integration/` | swarm (queued) |
| **AIW-S8 X08** | X08 | **DEFERRED** — requires corpus + baseline definition (operator scope, not human_gate) | none |

A6-S5 / A7-S5 EXT (real CogniCode-MCP / Chronos-MCP binaries) are external
to the repo and require package availability; no cycle is opened in this
session.

## §10 Reconciliation rule (v2)

The §1 rule is preserved. Add: a slice's STATE-OF-AIW row is updated to
`DELIVERED` only after the `feat`/`fix` commit lands on `origin/main` and
the RECEIPT cites the real commit SHA. This prevents drift like the S3/S4
marking the 2026-09-15 handoff noted as "still NOT_STARTED" while in fact
landed at `v1.169.97` / `v1.169.98`.

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
