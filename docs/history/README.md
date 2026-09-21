# Archivo histórico de SDDK — no es backlog activo

**Autoridad de continuación:** [`../roadmap/README.md`](../roadmap/README.md) → [`../roadmap/ROADMAP.md`](../roadmap/ROADMAP.md). Este directorio conserva contexto y decisiones antiguas; **NO** promueve specs/roadmaps/recibos por sí solo.

## Traslado físico 2026-09-21 (sesión de gobernanza documental)

Objetivo: garantizar que **un solo roadmap ejecutable** (`docs/roadmap/ROADMAP.md`) y **un solo puntero de continuidad** (`docs/roadmap/CURRENT.md` + `STATE.yaml` + `SESSION-JOURNAL.md`) son los únicos documentos con autoridad operativa.

### Mapa de traslados `git mv`

| Ruta original (eliminada, salvo excepciones marcadas) | Nueva ruta | Tipo |
|---|---|---|
| `docs/handoff/` (113 archivos) | `docs/history/handoffs/all-handoffs/` | HISTORICAL_PLAN |
| `docs/proposals/` (incl. 2026-09-19-adaptive-inputs-workflows) | `docs/history/proposals/all-proposals/` | HISTORICAL_PLAN / HISTORICAL_REFERENCE |
| `docs/research/` | `docs/history/research/all-research/` | HISTORICAL_REFERENCE |
| `docs/cycles/` | `docs/history/cycles/all-cycles/` | HISTORICAL_PLAN |
| `docs/uat/PLAN-uat-v3-quality-control-plane.md` | `docs/history/uat-plans/PLAN-uat-v3-quality-control-plane.md` | HISTORICAL_PLAN |
| `docs/sddk-complete-evolution-2026-08-23/` | `docs/history/legacy-packages/sddk-complete-evolution-2026-08-23/` | HISTORICAL_PLAN |
| `docs/sddk-decision-kernel-architecture/` | `docs/history/legacy-packages/sddk-decision-kernel-architecture/` | HISTORICAL_PLAN |
| `docs/sddk-2.0-architecture-consolidation/` | `docs/history/legacy-packages/sddk-2.0-architecture-consolidation/` | HISTORICAL_PLAN |
| `docs/SDDK-Human-Agent-Collaboration-Evolution-Pack-2026-08-28/` | `docs/history/legacy-packages/SDDK-Human-Agent-Collaboration-Evolution-Pack-2026-08-28/` | HISTORICAL_PLAN |
| `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/` | `docs/history/legacy-packages/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/` | HISTORICAL_PLAN |
| `docs/SDDK-Context-First-Semantic-Core-Agent-Experience-Software-Alignment-2026-09-10/` | `docs/history/legacy-packages/SDDK-Context-First-Semantic-Core-Agent-Experience-Software-Alignment-2026-09-10/` | HISTORICAL_PLAN |
| `docs/SDDK-CogniCode-Chronos-Context-First-Integration-Bundle-2026-09-10/` | `docs/history/legacy-packages/SDDK-CogniCode-Chronos-Context-First-Integration-Bundle-2026-09-10/` | HISTORICAL_REFERENCE |
| `docs/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/` | `docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/` | HISTORICAL_PLAN |
| `docs/SDDK-Production-Readiness-Alignment-2026-09-14/` | `docs/history/legacy-packages/SDDK-Production-Readiness-Alignment-2026-09-14/` | HISTORICAL_PLAN |
| `docs/sddk-stabilization-plan/` | `docs/history/legacy-packages/sddk-stabilization-plan/` | HISTORICAL_PLAN |
| `docs/architecture/a5/` | `docs/history/legacy-packages/architecture-a5-a6/architecture-a5/` | HISTORICAL_PLAN |
| `docs/architecture/a6/` | `docs/history/legacy-packages/architecture-a5-a6/architecture-a6/` | HISTORICAL_PLAN |
| `docs/architecture/a4-4c-acceptance-receipt.md` | `docs/history/legacy-packages/architecture-a5-a6/` | HISTORICAL_PLAN |
| `docs/architecture/a4-4m-migration-matrix.md` | `docs/history/legacy-packages/architecture-a5-a6/` | HISTORICAL_PLAN |
| `docs/architecture/a5/cycle-artifacts/` | `docs/architecture/cycle-artifacts/a5/` (extraído antes del movimiento) | ACTIVE_EVIDENCE |
| `docs/A3-MILESTONE-RECEIPT.md` | `docs/history/handoffs/all-handoffs/A3-MILESTONE-RECEIPT.md` | HISTORICAL_PLAN |
| `docs/ARCHITECTURE-MODEL.md` | `docs/history/ARCHITECTURE-MODEL.md` | HISTORICAL_REFERENCE |
| `docs/agent-models-registration.md` | `docs/history/agent-models-registration.md` | HISTORICAL_REFERENCE |
| `docs/deep-research-integration.md` | `docs/history/deep-research-integration.md` | HISTORICAL_REFERENCE |
| `docs/skill-categorization.md` | `docs/history/skill-categorization.md` | HISTORICAL_REFERENCE |
| `docs/evolutivo-*.md` (4 archivos, stubs de compat) | eliminados (contenido ya en `legacy-evolutivos-2026/`) | n/a |

### Estructura final del histórico

```text
docs/history/
├── README.md                                          # este catálogo
├── AGENTS-history.md                                  # historial del proyecto (ya existía)
├── ARCHITECTURE-MODEL.md                              # movido
├── agent-models-registration.md                       # movido
├── deep-research-integration.md                       # movido
├── skill-categorization.md                            # movido
├── legacy-evolutivos-2026/                            # contenido previo (ya existía)
├── legacy-packages/                                   # paquetes históricos consolidados
│   ├── architecture-a5-a6/                            # subdir con architecture-a5 + architecture-a6 + a4 docs
│   ├── sddk-complete-evolution-2026-08-23/
│   ├── sddk-decision-kernel-architecture/
│   ├── sddk-2.0-architecture-consolidation/
│   ├── SDDK-Human-Agent-Collaboration-Evolution-Pack-2026-08-28/
│   ├── SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/
│   ├── SDDK-Context-First-Semantic-Core-Agent-Experience-Software-Alignment-2026-09-10/
│   ├── SDDK-CogniCode-Chronos-Context-First-Integration-Bundle-2026-09-10/
│   ├── SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/
│   ├── SDDK-Production-Readiness-Alignment-2026-09-14/
│   └── sddk-stabilization-plan/
├── handoffs/all-handoffs/                             # 113 handoffs + A3-MILESTONE-RECEIPT
├── proposals/all-proposals/                           # propuestas 2026-09-19-* y archivos sueltos
├── research/all-research/                             # investigación pasada
├── cycles/all-cycles/                                 # planes de ciclo
└── uat-plans/                                         # planes UAT históricos
```

### Material que NO se archivó (sigue activo)

| Ruta | Por qué permanece |
|---|---|
| `docs/roadmap/` | Autoridad operativa única |
| `docs/architecture/adrs/` | ADRs aceptados con autoridad |
| `docs/architecture/specs/` | Specs vigentes referenciadas por código y tests |
| `docs/architecture/lints/` | Reglas contractuales de arquitectura |
| `docs/architecture/inventories/` | Inventarios estructurales vigentes |
| `docs/architecture/diagrams/` | Diagramas vigentes |
| `docs/architecture/registries/` | Registros vigentes |
| `docs/architecture/tests/` | Tests contractuales de arquitectura |
| `docs/architecture/spikes/` | Spikes vigentes (work-in-progress) |
| `docs/architecture/receipts/` | Recibos vigentes (no A5/A6, que sí se movieron) |
| `docs/architecture/cycle-artifacts/a5/` | Evidencia operativa de ciclo (extraído antes del traslado) |
| `docs/audit/error-variants.md` | Catálogo vigente de errores del sistema |
| `docs/control-plane/` | Especificación vigente del control-plane |
| `docs/debt/` | Incidencias de deuda activa |
| `docs/generated/` | Inventarios regenerados vigentes |
| `docs/releases/` | Notas release |
| `docs/responsibility-separation/` | SPEC vigente |
| `docs/uat/GUIDED-UAT-DESIGN.md` | Diseño de UAT guiada (IMPLEMENTED) |
| `docs/validation/` | Planes de validación |
| `docs/agent-reconciliation.md`, `docs/reconciliation-spec.md` | Specs vigentes |
| `docs/RELEASING.md` | Procedimiento release vigente |

### Sustituto vigente

Para todo lo movido a histórico, el sustituto normativo es:

- **Planificación:** `docs/roadmap/ROADMAP.md`
- **Estado:** `docs/roadmap/CURRENT.md` + `STATE.yaml`
- **Certificaciones:** `docs/roadmap/CERTIFICATIONS.md`
- **UAT:** `docs/roadmap/UAT-MATRIX.md`
- **Arquitectura:** `docs/architecture/README.md`
- **ADRs:** `docs/architecture/adrs/`

### Referencias que requieren migración posterior

- Cualquier `tests/` o `scripts/` que importe rutas absolutas `docs/handoff/...`, `docs/proposals/...`, `docs/sddk-complete-evolution-2026-08-23/...` necesitará actualización si se reactiva.
- `crates/` y `src/` se verificaron: no contienen referencias absolutas a estas rutas.
- Scripts: `scripts/release.sh`, `scripts/lib/release_admission.sh`, `githooks/pre-push`, `scripts/lib/*` se verificaron tras el traslado (siguiente sección).

### Verificación post-traslado

Realizada en commit `docs(history):` que acompaña este catálogo:

1. `git ls-files docs/` muestra solo subdirectorios activos listados arriba.
2. `find docs/history -maxdepth 2 -type d` muestra la nueva estructura completa.
3. `AGENTS.md` continúa apuntando a `docs/roadmap/ROADMAP.md` como autoridad única (sin cambios necesarios — ya era correcto).
4. No se modificó código funcional ni se publicó una release.

### Estado

- **Status:** Traslado ejecutado, working tree en cambios `git mv`.
- **Fecha:** 2026-09-21T14:26:00Z
- **Autor:** sesión SDDK (orquestador).
- **Aprobación:** decisión del operador (instrucción `2026-09-21T14:24:31`).
- **Próxima acción:** commit del catálogo + reconciliación de CURRENT/STATE/SESSION-JOURNAL.
