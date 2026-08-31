# Pipeline R0-R6 — Guía detallada

El pipeline R0-R6 es el método estándar para investigación profunda en este bundle. Cada fase tiene una sub-skill específica.

## R0 — Definir el sistema del tema (obligatorio)

**Objetivo**: antes de buscar fuentes, entender el sistema.

**Preguntas obligatorias**:
1. ¿Cuál es el propósito del sistema?
2. ¿Cuáles son los elementos clave (5-15)?
3. ¿Cuáles son las interconexiones?
4. ¿Cuáles son los feedback loops dominantes?
5. ¿Dónde está el leverage point tentativo? (1-12 de Meadows)
6. ¿Qué paradigmas están en juego?
7. ¿Qué system traps son probables?

**Output**: `research/system-map/{topic}.yml`

## R1 — Build agenda

**Sub-skill**: `sub/deep-research-strategist`

**Qué hace**: clasifica preguntas por tipo (api-existence, behavior, historical, etc.), nivel de evidencia requerido (L1-L7), y riesgo (critical, normal, low).

**Output**: `research/agenda.yml`

## R2 — Discover sources

**Sub-skill**: `sub/deep-source-discovery-specialist`

**Qué hace**: búsqueda multi-modal (papers oficiales, libros, código fuente, datasets, blogs técnicos). Prioriza L1 (primarias).

**Output**: `research/candidate-pool.yml`

## R3 — Evaluar credibilidad + validar referencias (en paralelo)

**Sub-skills**:
- `sub/deep-source-credibility-assessor` (R3a): autoridad, metodología, COI, frescura, trazabilidad
- `sub/deep-reference-validator` (R3b): HEAD request a URLs, Wayback fallback, version drift

**Output**: `research/credibility/{source-id}.yml` + `research/reference-validation.jsonl`

## R4 — Triangular evidencia

**Sub-skill**: `sub/deep-evidence-triangulator`

**Qué hace**: para cada claim, calcula `confidence_score` basándose en:
- Fuentes L1 independientes
- Detecta cascada de citas
- Detecta conflictos

**Output**: `research/triangulation/{claim-id}.yml`

## R5 — Consolidar corpus

**Sub-skill**: `sub/deep-knowledge-corpus-curator`

**Qué hace**: deduplica, aplica `decay_date`, detecta gaps, genera snapshot.

**Output**: `research/corpus.yml` + `research/corpus-snapshot-{date}.yml` + `research/gaps.yml`

## R6 — Extraer deliverables

**Sub-skill**: `sub/deep-claim-extractor`

**Qué hace**: genera evidence cards (modo LIBRO) o blueprints (modo SOFTWARE), listos para `chapter-writer` o code generation.

**Output**: `research/evidence-cards/{topic}.yml` (LIBRO) o `research/blueprints/{component}.yml` (SOFTWARE)

## Decisión Gates por fase

| Fase | Gate de salida |
|------|---------------|
| R0 → R1 | system-map completo con propósito, elementos, loops |
| R1 → R2 | agenda con preguntas priorizadas y fuentes admisibles |
| R2 → R3 | candidate-pool con ≥ 2 fuentes por pregunta `critical` |
| R3 → R4 | credibilidad y validación completadas, ≥ 80% URLs `live` |
| R4 → R5 | triangulación sin conflictos sin resolver |
| R5 → R6 | corpus consolidado, gaps documentados, decay aplicado |
| R6 → fin | deliverables generados, references a claims verificados |

## Sub-pipelines paralelos

Cualquiera de las 6 sub-pipelines opcionales puede activarse en paralelo cuando el tema lo requiera. Se documentan en `references/index.md`.
