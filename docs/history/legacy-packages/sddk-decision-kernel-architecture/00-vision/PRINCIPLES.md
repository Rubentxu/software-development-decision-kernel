# Architectural & Product Principles

## P1 — Git y el Event Ledger tienen autoridad explícita
Git conserva la autoridad sobre el código. El Event Ledger conserva la historia operacional de SDDK. Las projections y el graph se reconstruyen.

## P2 — Agente lógico ≠ modelo ≠ proveedor ≠ IDE
No codificar identidades como `architect = claude`. El agente representa una capacidad/rol; el router decide el backend de ejecución.

## P3 — Gobernar invariantes, no prescribir cada paso
SDDK debe asegurar intención, aceptación, arquitectura, evidencia, políticas y verificación. El número exacto de fases/agentes puede adaptarse.

> **Don't prescribe every step. Preserve every guarantee.**

## P4 — El LLM puede proponer el plan; el runtime valida y ejecuta
El Supervisor/Planner puede decidir WHAT y proponer HOW. Scheduling, locks, retries, joins, budgets, capabilities, idempotencia y políticas pertenecen al runtime determinista.

## P5 — Workflow Template ≠ Workflow IR ≠ Execution Graph
- Template: intención e invariantes estables.
- IR: plan validado y ejecutable.
- Execution Graph: instanciación durable que puede crecer durante el run.

## P6 — No convertir cada evento en prompt
Primero clasificar. Observe → deterministic behavior → cognitive signal.

## P7 — Toda reacción y expansión importante deja evento
Si no queda evento, no existe operacionalmente. Añadir nodos dinámicos también es una transición auditable.

## P8 — El graph no es source of truth
Es una proyección temporal, semántica y causal derivada del ledger.

## P9 — Contexto mínimo, relevante y trazable
Evitar context dumps. Context Capsules con must-read, on-demand, deltas, staleness y negative knowledge.

## P10 — Side effects gobernados
Los agentes proponen; una capability autorizada ejecuta; postconditions verifican; receipts acreditan.

## P11 — Failover conserva la identidad lógica del trabajo
Cambiar proveedor genera un nuevo `Attempt`, no un nuevo trabajo independiente.

## P12 — Verificación independiente y adaptativa
El productor no debe ser la única autoridad de aceptación. La profundidad de verificación se ajusta a riesgo, cambios y evidencia, no a un número fijo de agentes.

## P13 — Human-in-the-loop es una primitive
Approval, review, UAT acceptance y sign-off son waits/gates explícitos, pausables y reanudables.

## P14 — Packs, no hardcoding del dominio
SDD/UAT/security/incident/release deben vivir sobre contratos extensibles.

## P15 — Local-first por defecto
Ledger, graph, metrics y cockpit deben funcionar sin servicio remoto obligatorio.

## P16 — El Control Plane debe poder abrirse sin servidor
Generación HTML autocontenida desde persistencia; servidor opcional sólo para experiencias que realmente lo necesiten.

## P17 — La arquitectura se protege con fitness functions
Los ADR no bastan. Dependencias, puertos, side effects, dynamic expansion y reglas de delegación se verifican automáticamente.

## P18 — Aprender de la ejecución real sin auto-optimización ciega
Routing y workflows pueden aprovechar estadísticas históricas, pero los cambios de política pasan por evaluación, shadow mode, bounded rollout y rollback.

## P19 — Canonical-first, dynamic-when-useful
Un workflow conocido y validado se reutiliza. La composición/expansión dinámica se usa cuando reduce overhead o cuando el problema no se puede descomponer completamente de antemano.

## P20 — Los documentos SDD son vistas del contrato, no necesariamente fases
`spec.md`, `design.md` y `tasks.md` pueden seguir existiendo como artefactos humanos; no es obligatorio convertir cada artefacto en un AgentRun independiente.
