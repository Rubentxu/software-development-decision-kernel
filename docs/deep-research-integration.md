# Deep Research Integration — Bundle standalone en SDDK

## Estructura (patrón Master + sub-skills)

```
sddk-framework/
├── agents/deep-research-orchestrator.md      ← agente ejecutor
├── skills/
│   ├── DEEP-RESEARCH-INDEX.md                ← catálogo legacy (deprecado, apunta a la maestra)
│   └── deep-research/                        ← MAESTRA + bundle
│       ├── SKILL.md                           ← entry point (índice + activación)
│       ├── references/                        ← docs compartidos
│       │   ├── index.md                       ← mapa de las 22 sub-skills
│       │   └── pipeline-r0-r6.md             ← guía del pipeline
│       └── sub/                               ← 22 sub-skills especializadas
│           ├── deep-research-orchestrator/    (gate)
│           ├── deep-research-methodology-hub/ (hub renombrado)
│           ├── deep-research-strategist/      (R1)
│           ├── ... (19 más)
│           └── deep-traps-detector/
└── docs/sddk-2.0-architecture-consolidation/
    └── adrs/
        ├── ADR-0016-skill-namespace-categorization.md
        └── ADR-019-workflow-self-discovery.md
```

## Patrón elegido

**Master + sub-skills (jerárquica)**:
- 1 skill entry-point (`skills/deep-research/SKILL.md`) que actúa como índice.
- 22 sub-skills especializadas en `skills/deep-research/sub/` (progressive disclosure por fase del pipeline o por sub-pipeline).
- 2 docs compartidas en `skills/deep-research/references/` (mapa + guía del pipeline).

**Por qué este patrón**:
- Reduce el ruido en la raíz de `skills/` (22 → 1 entry).
- Mantiene la granularidad (cada sub-skill es invocable independientemente).
- Sigue la spec oficial de Agent Skills (1 skill = 1 directorio + SKILL.md).
- Compatible con la dirección de sddk-2.0 (ADR-019, ADR-0016).

## Pipeline R0-R6

```
R0  Definir el sistema del tema (Meadows)         [obligatorio]
R1  Build agenda              (sub/deep-research-strategist)
R2  Discover sources          (sub/deep-source-discovery-specialist)
R3  Evaluar credibilidad + validar refs (paralelo)
R4  Triangular                (sub/deep-evidence-triangulator)
R5  Consolidar corpus         (sub/deep-knowledge-corpus-curator)
R6  Extraer deliverables      (sub/deep-claim-extractor)
```

## Limitación del CLI actual (SDDK 1.13.0)

El CLI (`sddk dev link`, `doctor`, `uninstall`, `bootstrap.sh`) solo escanea 1 nivel en `skills/`. Las sub-skills NO se descubren automáticamente — el orchestrator debe leer `references/index.md` de la maestra para descubrirlas.

**Solución** (en SDDK2-411, sddk-2.0): modificar el CLI para recursar 1 nivel y descubrir sub-skills automáticamente.

## Estado

- **Versión**: 1.1 (patrón Master + sub-skills)
- **Fecha**: 2026-08-16
- **Standalone**: 22 sub-skills + 1 maestra + 1 agente, todos bundled
