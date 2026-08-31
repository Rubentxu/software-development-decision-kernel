---
name: deep-research-orchestrator
description: "Trigger: investigar tema a fondo, investigación profunda para capítulo, evidencia rigurosa, fuentes primarias, corpus de conocimiento, agenda de investigación, triangulación, knowledge gaps, dinámica de sistemas, leverage points, feedback loops, stocks and flows, system archetypes, Donella Meadows. Orquestador de investigación profunda para CUALQUIER tema de capítulo de libro técnico. Aplica el marco metodológico de Donella Meadows (Thinking in Systems, Leverage Points 1997, Limits to Growth 1972, Dancing with Systems 2001) como lente transversal: tratar el tema como un sistema, mapear sus elementos/loops/paradigmas, identificar leverage points, evitar los system traps. Coordina 21 skills. Nunca investiga inline."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: gate
  author: rubentxu
  version: "1.0"
  domain: deep-research
  methodology: donella-meadows-systems-thinking
  consumers: [book-orchestrator, orchestrator]
  output: claims-yml, blueprints-yml, code-patterns, knowledge-graph
  based_on: "Donella Meadows (Thinking in Systems, Leverage Points), Jay Forrester (System Dynamics), Peter Senge (Fifth Discipline), Daniel Kim (Systems Archetypes Basics)"
---

## Activation Contract

Úsalo cuando el usuario pida **investigar a fondo cualquier tema** que pueda ser capítulo de un libro técnico. Aplica el **marco metodológico de Donella Meadows** como lente transversal:

- **Definir el sistema** (elementos, interconexiones, propósito) — antes de buscar datos.
- **Modelar los feedback loops** (balancing/reinforcing, delays) — para entender dinámica, no eventos.
- **Identificar leverage points** (12 niveles: paradigmas > goals > estructura > reglas > info > loops > delays > stocks > buffers > parámetros) — para saber DÓNDE intervenir.
- **Detectar system traps** (Policy Resistance, Tragedy of the Commons, Shifting the Burden, etc.) — para no caer en anti-patrones conocidos.
- **Persistir el corpus** con decaimiento y gaps — la evidencia caduca, hay que re-verificar.

Coordina 21 skills especializadas (ver §11). Nunca investiga inline.

## Por qué Meadows es la metodología transversal

Meadows no es solo "un tema" entre muchos. Su marco es **la metodología** que aplicamos para investigar bien cualquier tema:

> "Every system has a purpose. Every system has feedback loops. Every system has leverage points. Every system is prone to certain traps." — Síntesis operativa de Meadows.

Cuando investigamos una tecnología, un campo científico, una empresa o un capítulo histórico:
- **Definir el sistema** = ¿qué actores, qué reglas, qué propósito?
- **Modelar loops** = ¿qué dinámicas lo gobiernan? (network effects, lock-in, switching costs, etc.)
- **Leverage points** = ¿dónde un cambio pequeño produce un cambio grande? (¿es la documentación? ¿es el lenguaje? ¿es el modelo mental?).
- **Traps** = ¿está cayendo en policy resistance, shifting the burden, tragedy of the commons?

Esto aplica a un capítulo sobre Rust, un capítulo sobre World3, un capítulo sobre la Revolución Francesa, o un capítulo sobre transformers en LLM.

## Doble orientación: LIBRO + SOFTWARE

| Aspecto | Modo LIBRO (book-orchestrator) | Modo SOFTWARE (orchestrator) |
|---------|-------------------------------|------------------------------|
| **Consumidor** | `book-orchestrator` → `chapter-writer`, `evidence-manager` | `orchestrator` → agentes de implementación |
| **Artefacto principal** | `research/evidence/{topic}.yml` (claims con citas) + `research/drafts/{topic}.md` (borradores AsciiDoc) | `research/blueprints/{topic}.yml` (entrada, salida, algoritmo, restricciones, código de ejemplo) + `research/code-patterns/{pattern}.md` |
| **Granularidad** | Citas textuales, páginas referenciadas, figuras con permiso | Interfaces, tipos, ecuaciones, librerías |
| **Verificación** | `evidence-manager` valida que toda afirmación tiene `claim_id verified` | Tests reproducibles; código que compila y pasa tests |
| **Output típico** | Sección `=== Leverage Points ===` de un capítulo | Función `simulate_reinforcing_loop(...)` con tests |
| **Métrica de éxito** | Cero `hallucination` + cero `manual_copy` + diagramas Mermaid renderizan | Tests pasan + simulación reproduce comportamiento documentado |

## Hard Rules

- **Evidencia primaria primero.** Toda afirmación verificable tiene al menos una fuente L1 (paper revisado, libro original, código fuente verificado) o L2 (doc oficial institucional).
- **Triangulación obligatoria.** Una sola fuente no basta; toda claim `risk: critical` requiere ≥ 2 fuentes independientes o marcado `disputed` con disclaimer.
- **Sin alucinaciones verificables.** Toda cita tiene página/sección exacta; toda cifra tiene fuente trazable. `hallucination-auditor` (en book-orchestrator) marcará como `critical` cualquier afirmación sin `claim_id` con `status: verified`.
- **Corpus persistente.** Todo se consolida en `research/corpus.yml` (alineado con `book-memory-keeper`). El corpus es **transversal a capítulos** y ediciones.
- **Gaps explícitos.** Lo que NO sabemos se trackea en `research/gaps.yml`; nunca se oculta ni se rellena con especulación.
- **Decaimiento temporal.** Cada claim tiene `decay_date`; la evidencia tecnológica caduca y debe re-verificarse.
- **Pensar sistémicamente.** Antes de investigar, define el sistema del tema. Esto NO es opcional — es parte del método. "Don't push the change in the wrong direction" (Meadows).

## Execution Steps

### 1. Recall (arranque)

- Lee `research/LEDGER.md` (si existe) y Engram (topic_keys: `research-{topic}`, `corpus-{topic}`, `system-{topic}`).
- Sintetiza: qué tema se investiga, fase del pipeline R, modo activo (LIBRO/SOFTWARE/DUAL).
- Si no hay memoria: sesión nueva → empezar con R0 (definir el sistema del tema).

### 2. R0 — Definir el sistema del tema (obligatorio)

Aplicar los principios de Meadows *antes* de buscar fuentes:

1. **Identifica el propósito** del sistema/tema. ¿Por qué existe? ¿Qué objetivo persigue?
2. **Mapea los elementos**: actores, entidades, variables clave.
3. **Mapea las interconexiones**: ¿cómo se afectan mutuamente? ¿qué flujos hay (información, recursos, personas)?
4. **Identifica feedback loops dominantes** (preguntar: si X sube 10%, ¿qué pasa con Y? → eso es un loop).
5. **Identifica el leverage point del tema**: ¿dónde un cambio pequeño tendría impacto grande? (¿es documentación? ¿es la arquitectura? ¿es el modelo mental compartido?).
6. **Detecta traps potenciales**: ¿es el tema propenso a Policy Resistance (gente que empuja en direcciones opuestas)? ¿Shifting the Burden (solución sintomática)? ¿Eroding Goals (estándar a la baja)?

Output: `research/system-map/{topic}.yml` (incluye propósito, elementos, loops iniciales, leverage point tentativo).

Sin este paso, la investigación es recolección de datos sin lente analítico. **R0 es la aplicación práctica del marco Meadows**.

### 3. Selección del modo de salida

```
┌─────────────────────────────────────────────────────────────────┐
│ ¿Quién consumirá el resultado?                                  │
│  book-orchestrator   → MODO LIBRO                               │
│  orchestrator        → MODO SOFTWARE                            │
│  Ambos               → MODO DUAL                                │
└─────────────────────────────────────────────────────────────────┘
```

### 4. Pipeline R (investigación profunda)

```
R0  Aplicar marco Meadows: definir el sistema del tema
    → research/system-map/{topic}.yml

R1  deep-research-strategist           (agenda: preguntas + nivel de evidencia)
R2  deep-source-discovery-specialist  (candidate-pool: papers/docs/código/libros)
R3  deep-source-credibility-assessor  (ranking L1-L7, sesgo, COI, frescura)
   ∥ deep-reference-validator         (validación viva: URL HEAD, DOI, version drift)
R4  deep-evidence-triangulator        (combina R3: cruza fuentes, marca conflictos)
R5  deep-knowledge-corpus-curator     (consolida corpus + snapshot + gaps)
    ↑── feedback loop: gaps → vuelve a R1
R6  deep-claim-extractor              (extrae evidence cards desde corpus verified)
```

Las fases son **secuenciales** entre R0-R6; **paralelizables** en R3 (credibilidad ∥ validación viva, ver §6.4).

### 5. Modos de ejecución

- **R-completa**: tema nuevo o re-investigación mayor. Los 7 pasos (R0 + R1-R6).
- **R-incremental**: tras nueva evidencia o cambio de versión. Solo preguntas/temas afectados (curator marca qué re-verificar).
- **R-focal**: pregunta puntual ("¿es verdad que X?"). Ejecuta solo los pasos necesarios.
- **R-claim-only**: producir un `claim.yml` para una afirmación concreta (consumido por `evidence-manager`).
- **R-blueprint-only**: producir un `blueprint.yml` para una pieza de software (consumido por code generation).

### 6. Sub-pipelines opcionales (activar según necesidad)

| Sub-pipeline | Cuándo se activa | Skills |
|--------------|------------------|--------|
| **Dominio Systems Thinking explícito** | El tema trata sobre sistemas complejos, dinámica, leverage points, paradigmas | `deep-coach-systems-thinking` + skills del dominio |
| **Modelado conceptual** | El tema requiere mapear entidades, relaciones, propiedades | `deep-domain-modeler` |
| **Investigación de software** | El tema es una tecnología/framework/lenguaje y necesitamos patrones de implementación | `deep-software-research` + `deep-pattern-extractor` |
| **Knowledge graph** | El tema requiere mapa de relaciones (autores, papers, conceptos, dependencias) | `deep-knowledge-graph-builder` |
| **Línea histórica** | El tema tiene dimensión temporal (cómo evolucionó un campo, quién influyó en quién) | `deep-historical-lineage-tracer` |
| **Escenarios futuros** | El tema requiere proyección (futuro de una tecnología, escenarios alternativos) | `deep-scenarios-explorer` |
| **Detección de traps** | El tema tiene trampas conocidas (anti-patterns, errores comunes) | `deep-traps-detector` |
| **Exploración de paradigmas** | El tema tiene dimensión cultural/modelos mentales | `deep-paradigms-explorer` |

### 7. Salidas por modo

#### Modo LIBRO (consumido por `book-orchestrator`)
```
research/
├── system-map/{topic}.yml            # R0 (marco Meadows)
├── agenda.yml                        # R1
├── candidate-pool.yml                # R2
├── credibility/{source-id}.yml       # R3a
├── reference-validation.jsonl        # R3b
├── triangulation/{claim-id}.yml      # R4
├── corpus.yml                        # R5
├── corpus-snapshot-{date}.yml        # R5 (audit)
├── gaps.yml                          # R5
├── evidence-cards/{topic}.yml        # R6 (consumido por chapter-writer)
├── drafts/{chapter-slug}.md          # borradores AsciiDoc
└── diagrams/{topic}.mmd              # Mermaid para include::
```

#### Modo SOFTWARE (consumido por `orchestrator`)
```
research/
├── system-map/{topic}.yml
├── agenda.yml
├── candidate-pool.yml
├── credibility/
├── reference-validation.jsonl
├── triangulation/
├── corpus.yml
├── blueprints/{component}.yml        # API/función con interface, algorithm, references, test_acceptance
├── code-patterns/{pattern}.{py,rs,md}  # snippets de implementación con citas
├── knowledge-graphs/{topic}.ttl      # RDF/Turtle para razonamiento
└── test-fixtures/{model}-expected.json  # valores de referencia para tests
```

#### Modo DUAL
Todos los archivos anteriores; regla de coherencia: toda cifra en el libro tiene contraparte en `test-fixtures` o `blueprints`.

### 8. Checkpoint (cierre de sesión)

- Actualiza `research/LEDGER.md` (fase R actual, artefactos producidos, modo LIBRO/SOFTWARE/DUAL).
- `mem_session_summary` con Goal/Discoveries/Accomplished/Next Steps.
- `mem_save` por cada decisión clave (topic_key: `research-{topic}-{concepto}`).
- Si modo LIBRO: notificar al `book-orchestrator` con la lista de `evidence-cards` listas para `chapter-writer`.

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Pregunta `risk: critical` con 0 fuentes L1-L2 | Bloquear; escalar al autor o re-formular |
| Claim con fuentes en conflicto | Pasar a `deep-evidence-triangulator` (R4); marcar `disputed` con explicación |
| Tema sin literatura primaria suficiente | Escalar: ¿reducir alcance o esperar más evidencia? |
| Concepto requiere modelado (entidades, relaciones) | Activar `deep-domain-modeler` |
| Tecnología requiere patrones de implementación | Activar `deep-software-research` + `deep-pattern-extractor` |
| claim con `decay_date` vencida | `deep-knowledge-corpus-curator` dispara re-check |
| Usuario quiere proyección/futuro | Activar `deep-scenarios-explorer` |
| Tema trata de sistemas complejos, leverage, paradigmas | Activar `deep-coach-systems-thinking` (sub-pipeline dominio) |
| **Antes de investigar**: ¿se ha aplicado R0 (definir el sistema del tema)? | Si no, ejecutar R0 primero. "Without a system map, you're collecting data without a lens." (Meadows) |
| **Durante la investigación**: ¿se está cayendo en un system trap? | Consultar `deep-traps-detector`. Por ejemplo: Policy Resistance (muchas fuentes dicen cosas distintas) puede ser normal; pero si hay consenso aparente sin triangulación, es sospechoso. |

## Output Contract

- `research/system-map/{topic}.yml` (R0).
- `research/agenda.yml` con preguntas priorizadas (R1).
- `research/candidate-pool.yml` con fuentes descubiertas (R2).
- `research/credibility/{id}.yml` con ranking L1-L7 (R3).
- `research/reference-validation.jsonl` con validación viva (R3).
- `research/triangulation/{claim-id}.yml` con cruces (R4).
- `research/corpus.yml` consolidado (R5).
- `research/gaps.yml` con conocimiento faltante (R5).
- `research/evidence-cards/{topic}.yml` listos para `chapter-writer` (R6, modo LIBRO).
- `research/blueprints/{component}.yml` listos para code generation (modo SOFTWARE).
- `research/knowledge-graphs/{topic}.ttl` cuando aplique.
- `research/LEDGER.md` siempre actualizado.

## Las 21 skills (catálogo)

### Pipeline R principal (siempre activo)
| # | Skill | Fase | Qué hace |
|---|-------|------|----------|
| 1 | `deep-research-strategist` | R1 | Agenda priorizada por riesgo + nivel de evidencia |
| 2 | `deep-source-discovery-specialist` | R2 | Descubrimiento multi-modal (papers, docs, código, libros, blogs, datos) |
| 3 | `deep-source-credibility-assessor` | R3a | Ranking L1-L7, sesgo, COI, frescura |
| 4 | `deep-reference-validator` | R3b | Validación viva (URL HEAD, DOI, version drift) |
| 5 | `deep-evidence-triangulator` | R4 | Cruza fuentes, detecta conflictos, calcula `confidence_score` |
| 6 | `deep-knowledge-corpus-curator` | R5 | Consolida corpus, deduplica, detecta gaps, gestiona decay |
| 7 | `deep-claim-extractor` | R6 | Genera evidence cards listas para `chapter-writer` |

### Sub-pipelines opcionales (activar según dominio del capítulo)
| # | Skill | Cuándo | Modo de salida |
|---|-------|--------|----------------|
| 8 | `deep-software-research` | Tema es tecnología/framework | LIBRO + SOFTWARE |
| 9 | `deep-pattern-extractor` | Hay que extraer patrones de implementación | SOFTWARE |
| 10 | `deep-domain-modeler` | Conceptos requieren entidades/relaciones | LIBRO + SOFTWARE |
| 11 | `deep-knowledge-graph-builder` | Mapa de relaciones (autores, papers, conceptos) | LIBRO + SOFTWARE |
| 12 | `deep-historical-lineage-tracer` | Dimensión temporal (evolución de un campo) | LIBRO |
| 13 | `deep-scenarios-explorer` | Proyección / futuros alternativos | LIBRO + SOFTWARE |
| 14 | `deep-paradigms-explorer` | Sistemas de pensamiento, cultura, modelos mentales | LIBRO |
| 15 | `deep-traps-detector` | Errores comunes del dominio (anti-patterns) | LIBRO |

### Skills de dominio: Systems Thinking (Meadows) — núcleo metodológico
| # | Skill | Cuándo | Modo |
|---|-------|--------|------|
| 16 | `deep-coach-systems-thinking` | Tema = Donella Meadows, System Dynamics, leverage points; o cualquier tema que requiera el marco completo | Activa el sub-pipeline: |
| 17 | `deep-leverage-points-analyst` | Aplicar los 12 leverage points al tema | LIBRO + SOFTWARE |
| 18 | `deep-system-archetypes-mapper` | Mapear el tema a los 8 arquetipos de Senge/Kim | LIBRO + SOFTWARE |
| 19 | `deep-feedback-loops-analyzer` | Modelar causal-loop diagrams del tema | LIBRO + SOFTWARE |
| 20 | `deep-stocks-flows-diagrammer` | Modelar stocks-and-flows del tema (cuando hay cuantificación) | LIBRO + SOFTWARE |
| 21 | `deep-software-research` (también aquí) | Implementar simulación World3/stock-flow en código | SOFTWARE |

## Anti-patrones (con etiqueta Meadows)

- ❌ **"Recolectar datos sin definir el sistema"** — anti-patrón fundamental: equivale a R0 omitido.
- ❌ Confundir L3 (Wikipedia) con L1 (paper original). Wikipedia es punto de navegación, no fuente.
- ❌ Saltarse la triangulación: una sola fuente para claims `critical`.
- ❌ Inventar cuantificaciones. Si no hay dato, documentar el gap.
- ❌ Rellenar gaps con opinión ("creo que..."). Marcar como `L7` y `disputed`.
- ❌ Re-descubrir lo ya consolidado. Si el corpus tiene la claim `verified`, extraer evidence card; no re-investigar.
- � Citar sin página/sección. El `hallucination-auditor` no admite citas vagas.
- ❌ **Caer en Policy Resistance** (Meadows): muchas voces diciendo cosas distintas sin alinear metas. Síntoma: debate eterno sobre definiciones. Solución: alinear goals del equipo de investigación.
- ❌ **Caer en Shifting the Burden** (Meadows): re-citar fuentes secundarias en lugar de leer las primarias. Síntoma: el corpus tiene muchas fuentes L3-L5 pero pocas L1. Solución: priorizar descubrimiento de L1.
- ❌ **Buscar el leverage point equivocado** (Meadows): invertir mucho en parámetros (nivel 12) cuando el problema es de paradigma (nivel 2). Síntoma: agendas largas de "qué versión", "qué número". Solución: re-evaluar el `system-map`.

## References

- `references/deep-research-workflow.md` — normativa detallada (vinculante).
- `references/meadows-canon.md` — bibliografía primaria de Donella Meadows.
- `references/supported-domains.md` — dominios soportados con ejemplos.
- `references/dual-output-contract.md` — contrato de salida LIBRO + SOFTWARE.
- `references/evidence-levels.md` — niveles L1-L7 universales.
- `references/claim-types.md` — tipos de claim por dominio.
- `assets/agenda.schema.yml`, `assets/candidate-pool.schema.yml`, `assets/corpus.schema.yml`, `assets/evidence-card.schema.yml`, `assets/blueprint.schema.yml`, `assets/knowledge-graph.schema.yml`, `assets/system-map.schema.yml`.
