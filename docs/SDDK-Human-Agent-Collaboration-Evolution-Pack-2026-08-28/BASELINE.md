# Baseline y análisis de gaps

## Baseline auditada

- Repositorio: `Rubentxu/sddk-framework`
- Fecha: 2026-08-28
- Versión workspace: `1.50.0`
- Commit: `643180a21ab1c9e7a63758ad221d97ec1640ae5a`
- Rust edition: 2024
- Rust MSRV declarado: 1.91
- Crates relevantes: `sddk-domain`, `sddk-storage`, `sddk-engine`, `sddk-gateway`, `sddk-vault`, `sddk-cli`, `sddk-testkit`, `sddk-pack-uat`.

## Capacidades existentes que NO deben duplicarse

### Kernel y estado
SDDK ya tiene ciclo, ledger, leases/fencing, CAS, gates, receipts y reconstrucción. El Companion no puede convertirse en una segunda autoridad.

### Agent-first CLI
La baseline ya dispone de una `Instruction Contract Matrix` y una superficie facade de primer nivel:

- `sddk status`
- `sddk plan`
- `sddk run`
- `sddk ship`
- `sddk recover`

Por tanto HX debe consumir esa capa y sólo proponer nuevos verbos cuando aporten una semántica humana distinta, por ejemplo `explain`, `decisions` o `memory`.

### Reporting
Los contratos ya conocen `report_audience: novice | standard | expert`, localización y una salida humana al cierre. El gap está en **durante el ciclo**, no sólo al terminar.

### Telemetría y F3
Existe control plane local, SQLite reconstruible, agregados, dashboard y research packet. HX6 debe extender esta infraestructura; no crear otro analytics store.

### UAT
El roadmap ya contiene `UAT-2026-08-v3 — Human-Governed AI Quality Control Plane`, con separación executor/evidence/oracles/review, Human Review Queue, disagreement dataset, event log y Guided Runner. HX debe ser infraestructura común de interacción, no un UAT alternativo.

## Gaps detectados

### GAP-01 — Estado humano ambiguo
El kernel conoce su estado, pero el humano no recibe una proyección continua y estable de ese estado.

### GAP-02 — Fuente de verdad inconsistente en prompts
`orchestrator.md`/MCW establecen CLI/ledger como autoridad runtime, mientras `status-query.md` aún conserva lenguaje que presenta el vault como autoridad primaria. Debe corregirse antes de construir `/where` o Resume.

### GAP-03 — Reporting concentrado en el cierre
Existe human summary final, pero no contrato común para `phase.started`, progreso significativo, `plan.reframed`, bloqueos o decisiones.

### GAP-04 — Telemetría rica no convertida en narrativa
`apply-progress`, verify/debt reports y receipts contienen datos suficientes para explicar el ciclo, pero no existe una proyección humana uniforme.

### GAP-05 — HITL binario
`interactive` vs `auto` no expresa bien la diferencia entre *ser informado* y *tener que aprobar*. Se necesita autonomía basada en riesgo.

### GAP-06 — Personalidad mezclable con razonamiento
Si la personalidad se implementa copiando instrucciones a decenas de agentes, aumentan divergencia, tokens y riesgo semántico. Debe vivir en presentation.

### GAP-07 — Memoria de usuario no separada
Operational state, project knowledge y preferencias personales deben ser bounded contexts distintos.

### GAP-08 — No existe un concepto de `Reframe`
Los agentes pueden descubrir evidencia que cambia el enfoque, pero el humano necesita una explicación explícita y auditable del cambio.

### GAP-09 — No medimos confusión humana
Preguntas como “¿dónde estamos?”, “¿por qué haces esto?” o “¿qué falta?” son señales de UX que hoy no alimentan F3.

## Constraints no negociables

1. Ledger/CLI siguen siendo autoridad runtime.
2. Personalidad nunca altera gates, facts, verdicts ni decisiones deterministas.
3. La memoria conversacional nunca sustituye a cycle state.
4. Cero archivos nuevos en el workspace adoptado salvo opt-in explícito ya permitido por SDDK.
5. No se pierde ninguna evidencia al usar facade/goal surface.
6. Low-level CLI permanece disponible para debug, tests y recuperación.
7. Todos los cambios deben poder validarse local-first.
