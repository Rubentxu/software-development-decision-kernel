# AIW-SPEC-02 — Agenda y handoff fundamentados

**Estado: Proposed.**

- **AIW-REQ-H01 MUST** construir atención/agenda como consulta de Planning/Execution/Verification/Knowledge identificados; NO nueva autoridad Agenda ni queue que se desincronice del ledger.
- **H02 MUST** separar `planning candidate`, `admitted by runtime/Authority`, `requires decision`, `evidence gap` y `blocked`; `project_next` spine ≠ scheduler. Multiple Active, dep missing, snapshot no reconciliado o contradiction MUST emerger explícitamente, nunca «READY» de forma silenciosa.
- **H03 MUST** demostrar `Secretary L0/L1/L2` con un consumidor productivo real, no solo tests de módulo; reglas/propuestas no obtienen autoridad `release/gate/lease/receipt`. Orchestrator conserva decisión delegable y Authority conserva efectos.
- **H04 MUST** construir próximo encargo a partir de tipos efectivos `AgentContributionEnvelope`/outcomes/receipts/referencias y `context_capsule::ContextCompiler` del engine (no otro compilador homónimo); `arch-spec-007` necesita crosswalk de nombres documentales vs Rust actual.
- **H05 MUST** conservar objetivos, restricciones, accepted/rejected decisions, evidence/artifact refs, missing inputs, assumptions/dissent y revisión; context pack acotado y sin transcript opaco. Los resultados parciales no equivalen a `Completed`.
- **H06 MUST** reiniciar proceso y reconstruir cápsula/continuación desde storage real; refs stale/ausentes o evidence mandatory omitida ⇒ refusal/needs-evidence. InMemoryStore no demuestra durabilidad.
- **H07 MUST** enlazar WorkItem→plan revision→node→attempt→evidence/decision con referencias estables y sin otro store propietario. Una tarea puede persistir a través de varias revisiones; no crear clones por vista o por ejecución.
- **H08 SHOULD** reutilizar `intelligence_loop`/`advisory_context` como composición de resultados evaluados; no usar como motor de probes, global score ni decisión de gobernanza.
- **H09 MUST** el siguiente agente no aceptará «tests pasados» como hecho por aparecer en Contribution si falta resultado verificable y alcance válido.
