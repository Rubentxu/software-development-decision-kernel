# Deep Research Skills — Catálogo

> **DEPRECADO**: Esta página es el catálogo legacy. La estructura canónica ahora vive en `skills/deep-research/SKILL.md` (skill maestra) con sus 22 sub-skills en `skills/deep-research/sub/`. Este archivo se mantiene por compatibilidad con enlaces existentes.

**1 skill maestra + 22 sub-skills** + 1 agente ejecutor (`deep-research-orchestrator`). Marco metodológico de Donella Meadows como lente transversal.

## Cómo navegar

- **Entry point**: `skills/deep-research/SKILL.md` (la maestra)
- **Mapa de sub-skills**: `skills/deep-research/references/index.md`
- **Pipeline detallado**: `skills/deep-research/references/pipeline-r0-r6.md`
- **Agente ejecutor** (fuera del bundle): `agents/deep-research-orchestrator.md`

## Pipeline R

```
R0  Definir el sistema del tema (Meadows)         [obligatorio]
R1  Build agenda              (sub/deep-research-strategist)
R2  Discover sources          (sub/deep-source-discovery-specialist)
R3  Evaluar credibilidad + validar refs (paralelo)
R4  Triangular                (sub/deep-evidence-triangulator)
R5  Consolidar corpus         (sub/deep-knowledge-corpus-curator)
R6  Extraer deliverables      (sub/deep-claim-extractor)
```

## Estado

v1.0 · Standalone · Patrón Master + sub-skills (LangChain hierarchical skills + RFC-318)
