# Deep Research Workflow — Ley del sistema de investigación profunda

**Normativa vinculante** para `deep-research-orchestrator` y las 21 skills del sistema. Cualquier conflicto entre una skill y este documento, gana este documento.

Inspirado en (la metodología transversal):
- **Donella Meadows** — *Thinking in Systems: A Primer* (Chelsea Green, 2008)
- **Donella Meadows** — "Leverage Points: Places to Intervene in a System" (Whole Earth Review, 1997; Sustainability Institute)
- **Donella Meadows** et al. — *The Limits to Growth* (Universe Books, 1972)
- **Donella Meadows** — "Dancing with Systems" (Whole Earth Review, 2001)
- **Jay W. Forrester** — *Industrial Dynamics* (MIT Press, 1961); *World Dynamics* (1971)
- **Peter Senge** — *The Fifth Discipline* (Doubleday, 1990)
- **Daniel Kim & Virginia Anderson** — *Systems Archetypes Basics* (Pegasus WB002E)
- **Thomas Kuhn** — *The Structure of Scientific Revolutions* (1962) — paradigmas
- **John Sterman** — *Business Dynamics* (2000)
- **Peter Schwartz** — *The Art of the Long View* (1991) — escenarios
- **Kees van der Heijden** — *Scenarios: The Art of Strategic Conversation* (1996)
- **Elinor Ostrom** — *Governing the Commons* (1990) — Nobel 2009

Estado: v1.0 · 2026-08-16 · 1 orquestador + 21 skills · cubre CUALQUIER tema de capítulo.

---

## 0. Principios rectores (de Meadows, aplicados transversalmente)

1. **Define el sistema antes de investigar.** "A system is a set of elements... interconnected to achieve a purpose." (Thinking in Systems, cap. 1). R0 es obligatorio.

2. **Estructura > eventos.** "Most of the big problems in the world are caused by the gradual accumulation of small events." (Meadows). Buscar estructura, no anécdotas.

3. **Las interacciones son más importantes que los elementos.** "Interconnections are also critically important. Changing relationships usually changes system behavior." (Meadows, Thinking in Systems cap. 3).

4. **Higher leverage points resisten más.** "The higher the leverage point, the more the system will resist changing it." (Leverage Points, 1997). No confundir nivel 12 (parámetros) con nivel 2 (paradigma).

5. **No se controla, se baila.** "We can't control systems or figure them out. But we can dance with them!" (Dancing with Systems, 2001). Aplicar a la investigación: no forzar el corpus, dejar que emerja.

6. **Bounded rationality en todos los actores.** Cada decisor/investigador ve solo una parte. El corpus debe agregar perspectivas.

7. **Memoria persistente entre sesiones.** El corpus vive en `research/corpus.yml` + Engram. Las sesiones se reanudan.

8. **Evidencia primaria sobre intuición.** Toda afirmación tiene L1-L7 explícito; sin fuente → gap, no relleno especulativo.

9. **Decaimiento explícito.** La evidencia caduca; cada claim tiene `decay_date`.

10. **Doble orientación (LIBRO + SOFTWARE).** Cada skill produce artefactos consumibles por `book-orchestrator` y por `orchestrator` general.

---

## 1. Las 7 fases del pipeline R

```
┌─────────────────────────────────────────────────────────────────┐
│  R0 · DEFINIR EL SISTEMA DEL TEMA (Meadows)                     │
│  propósito · elementos · loops · leverage points tentativos       │
│  → research/system-map/{topic}.yml                              │
├─────────────────────────────────────────────────────────────────┤
│  R1 · AGENDA                                                    │
│  preguntas + nivel de evidencia + riesgo                         │
│  → research/agenda.yml                                          │
├─────────────────────────────────────────────────────────────────┤
│  R2 · DESCUBRIMIENTO                                            │
│  papers/docs/código/libros/datos/blogs                          │
│  → research/candidate-pool.yml                                  │
├─────────────────────────────────────────────────────────────────┤
│  R3 · EVALUACIÓN (paralela)                                     │
│  R3a credibilidad (L1-L7, sesgo, COI) → credibility/*.yml       │
│  R3b validación viva (URL, DOI) → reference-validation.jsonl    │
├─────────────────────────────────────────────────────────────────┤
│  R4 · TRIANGULACIÓN                                             │
│  cruzar fuentes independientes, detectar conflictos              │
│  → research/triangulation/{claim-id}.yml                        │
├─────────────────────────────────────────────────────────────────┤
│  R5 · CONSOLIDACIÓN                                             │
│  corpus + snapshot + gaps + decay                               │
│  → research/corpus.yml · research/gaps.yml                      │
├─────────────────────────────────────────────────────────────────┤
│  R6 · EXTRACCIÓN                                                │
│  evidence cards (LIBRO) + blueprints (SOFTWARE)                 │
│  → research/evidence-cards/{topic}.yml                          │
│  → research/blueprints/{component}.yml                          │
└─────────────────────────────────────────────────────────────────┘
```

**Orden**: R0 → R1 → R2 → R3 → R4 → R5 → R6. R3a y R3b son paralelizables.

---

## 2. R0 · Definir el sistema del tema (Meadows como metodología)

### 2.1 Por qué es obligatorio

Sin sistema mapeado, "you're collecting data without a lens" (síntesis de Meadows). El R0 es la diferencia entre "buscar en Google" y "investigar".

### 2.2 Cinco preguntas que no pueden faltar

| Pregunta | Respuesta esperada | Sin esto |
|----------|-------------------|----------|
| ¿Cuál es el **propósito** u objetivo del sistema/tema? | Una frase clara, idealmente cuantificable | No se pueden interpretar los balancing loops |
| ¿Cuáles son los **elementos** clave? (stocks, variables, agentes) | Lista priorizada 5-15 | El modelo no tiene nodos |
| ¿Cuáles son las **interconexiones**? (causalidad, flujos) | Diagrama causal + flujos | No hay dinámica |
| ¿Cuáles son los **feedback loops dominantes**? (R, B, polaridad) | Lista de B1, B2, R1... con polaridad | No se puede diagnosticar arquetipos |
| ¿Dónde está el **leverage point** tentativo del tema? | 1-12 de Meadows | No se sabe dónde intervenir |

### 2.3 Anti-patrón

> ❌ Saltarse R0 porque "el tema es obvio". Si es obvio, R0 es rápido; si NO es obvio, R0 revela el gap.

### 2.4 Artefacto

```yaml
# research/system-map/{topic}.yml
system:
  id: {slug}
  name: {humano-legible}
  purpose_declared: {lo que dice perseguir}
  purpose_inferred: {lo que realmente persigue}
  boundary:
    included: [elementos]
    excluded: [elementos fuera]
  stocks: [...]
  key_variables: [...]
  actors: [...]
  feedback_loops_identified: [B1, R1, R2]
  leverage_point_tentative: {1-12}
  paradigm_in_play: {paradigma dominante}
  potential_traps: [lista de system traps sospechosos]
```

---

## 3. R1 · Agenda

Ya documentado en `deep-research-strategist/SKILL.md`. Resumen:
- `claim_type` por dominio (ver `references/claim-types.md` en strategist).
- `evidence_level` L1-L7 (floor por dominio).
- `risk` critical/normal/low.
- `admissible_sources` por tipo de claim.

---

## 4. R2 · Descubrimiento

- `deep-source-discovery-specialist` produce `candidate-pool.yml`.
- Multi-modal: papers, libros, código fuente, modelos, datasets, blogs, podcasts (transcripciones cuando el autor es autoridad).
- Cobertura: ≥ 2 candidatos por pregunta `critical`.
- **Anti-Shifting the Burden**: no quedarse en L3-L5 cuando hay L1.

---

## 5. R3 · Evaluación (paralela)

### 5.1 R3a · Credibilidad (`deep-source-credibility-assessor`)

5 dimensiones:
1. Autoridad del autor.
2. Metodología.
3. Independencia / COI.
4. Frescura.
5. Trazabilidad.

Output: `credibility/{source-id}.yml` con `evidence_level`, `credibility_score`, `admitted: true/false`.

### 5.2 R3b · Validación viva (`deep-reference-validator`)

- HEAD request a URLs.
- Wayback Machine para URLs muertas.
- Version drift para software.

Output: `reference-validation.jsonl`.

### 5.3 Paralelización

R3a y R3b son independientes → ejecutar concurrentes.

---

## 6. R4 · Triangulación

`deep-evidence-triangulator`:
- Calcula `confidence_score` según algoritmo documentado.
- Detecta cascada de citas (múltiples fuentes que vienen de la misma primaria → contar como 1).
- Detecta conflictos entre fuentes L1.
- Asigna `status`: `verified` | `verified-with-disclaimer` | `disputed` | `needs_recheck` | `unverified` | `deprecated`.

---

## 7. R5 · Consolidación

`deep-knowledge-corpus-curator`:
- `research/corpus.yml` consolidado.
- `research/gaps.yml` (lo que NO sabemos).
- `research/corpus-snapshot-{date}.yml` (audit trail).
- `decay_date` por claim (tasas por dominio).
- Conexión viva con R0 (system-map) — el corpus no pierde la lente sistémica.

---

## 8. R6 · Extracción

`deep-claim-extractor`:
- Modo LIBRO: `evidence-cards/{topic}.yml` con excerpt textual, listo para `chapter-writer`.
- Modo SOFTWARE: `blueprints/{component}.yml` con interface, algorithm, references, test_acceptance.
- Solo claims `verified` (o `verified-with-disclaimer` con disclaimer).

---

## 9. Sub-pipelines opcionales

Activación condicional según el tema del capítulo:

| Sub-pipeline | Trigger | Skills |
|--------------|---------|--------|
| Software | Tecnología concreta | `deep-software-research` + `deep-pattern-extractor` |
| Dominio conceptual | Necesita entidades/relaciones | `deep-domain-modeler` |
| Knowledge graph | Muchas entidades relacionadas | `deep-knowledge-graph-builder` |
| Línea histórica | Dimensión temporal | `deep-historical-lineage-tracer` |
| Escenarios | Proyección/futuros | `deep-scenarios-explorer` |
| Paradigmas | Modelos mentales/cultura | `deep-paradigms-explorer` |
| Traps | Errores comunes | `deep-traps-detector` |
| **Systems Thinking** | Tema = Donella Meadows / System Dynamics | `deep-coach-systems-thinking` + sus 4 skills subordinadas |

---

## 10. Puertas de calidad

Una investigación se considera `done` solo cuando:

1. `system-map/{topic}.yml` completo con propósito, elementos, loops, leverage point tentativo.
2. `agenda.yml` con todas las preguntas `critical` clasificadas.
3. `candidate-pool.yml` con cobertura suficiente (≥ 2 candidatos por `critical`).
4. Todas las fuentes del corpus evaluadas (`credibility/*.yml`) y validadas (`reference-validation.jsonl`).
5. Todas las claims `critical` con `confidence_score ≥ 0.7` y `status: verified` (o `disputed` con disclaimer).
6. `corpus.yml` sin claims `needs_recheck` vencidas (salvo declaración explícita).
7. `evidence-cards/{topic}.yml` (LIBRO) o `blueprints/{component}.yml` (SOFTWARE) generados.
8. `gaps.yml` actualizado con conocimiento faltante priorizado.

---

## 11. Persistencia y reanudación

`research/LEDGER.md`:

```yaml
investigation:
  topic: "Bevy ECS scheduling"
  domain: technology
  current_phase: R4
  mode: DUAL  # LIBRO | SOFTWARE | DUAL
  phases:
    - {phase: R0, status: DONE, artefact: research/system-map/bevy-ecs-scheduling.yml}
    - {phase: R1, status: DONE, artefact: research/agenda.yml}
    - {phase: R2, status: DONE, artefact: research/candidate-pool.yml}
    - {phase: R3a, status: DONE, artefact: research/credibility/*.yml}
    - {phase: R3b, status: DONE, artefact: research/reference-validation.jsonl}
    - {phase: R4, status: IN_PROGRESS}
    - {phase: R5, status: PENDING}
    - {phase: R6, status: PENDING}
  corpus:
    version: "2026-08-16-01"
    open_questions: 3
    verified_claims: 12
    disputed_claims: 1
last_updated: 2026-08-16T15:00:00Z
```

Al reanudar, el orquestador lee el LEDGER y continúa desde la fase actual.

---

## 12. Integración con book-orchestrator y orchestrator

### Desde `book-orchestrator`

```
book-orchestrator (Macro-fase R)
  └─> deep-research-orchestrator (R0-R6)
        ├─> research/evidence-cards/{topic}.yml  → chapter-writer
        └─> research/diagrams/{topic}.mmd         → chapter-writer (include::)
```

El `book-orchestrator` invoca el orquestador de deep-research cuando un capítulo requiere evidencia rigurosa. El output se sincroniza con el `corpus.yml` del libro (Macro-fase R estándar).

### Desde `orchestrator` (general)

```
orchestrator (proyecto de software)
  └─> deep-research-orchestrator (R0-R6)
        ├─> research/blueprints/{component}.yml       → code generation
        ├─> research/code-patterns/{pattern}.py       → code generation
        └─> research/test-fixtures/{model}-expected.json → testing agent
```

El `orchestrator` invoca cuando el proyecto es una aplicación de simulación/modelado o cuando necesita patrones de implementación rigurosamente documentados.

---

## 13. Anti-patrones comunes (con etiqueta Meadows)

| Anti-patrón | Por qué falla | Cómo evitarlo |
|-------------|---------------|---------------|
| **Saltarse R0** (recolectar datos sin definir el sistema) | "Collecting data without a lens" | R0 obligatorio, aunque sea rápido |
| **Confundir L3 con L1** | Wikipedia no es fuente | Solo L1 para claims `critical` |
| **Single-source `critical`** | Una fuente no basta | Triangulación obligatoria |
| **Inventar cuantificaciones** | Política de cero alucinaciones | Marcar gap explícito |
| **Re-descubrir lo ya consolidado** | Ineficiencia | Leer `corpus.yml` antes de R2 |
| **Citar sin página/sección** | `hallucination-auditor` falla | Toda cita con página exacta |
| **Caer en Policy Resistance** (muchas voces diciendo cosas distintas sin alinear goals) | Síntoma: debate eterno sobre definiciones | Alinear metas del equipo de investigación |
| **Caer en Shifting the Burden** (re-citar fuentes secundarias sin leer las primarias) | Síntoma: corpus con muchas L3-L5 pero pocas L1 | Priorizar descubrimiento L1 |
| **Buscar el leverage point equivocado** (parámetros cuando el problema es de paradigma) | Síntoma: agendas largas de "qué versión", "qué número" | Re-evaluar `system-map` |
| **Ignorar el decay_date** | Citar evidencia vencida | Re-check antes de R6 |

---

## 14. El "dancing with systems" aplicado a la investigación

Meadows cierra *Dancing with Systems*:

> "We can't control systems or figure them out. But we can dance with them!"

Aplicado a la investigación profunda:
- **No controles el corpus**: deja que emerja de la triangulación.
- **No fuerces la conclusion**: si los datos no soportan X, X no entra.
- **Escucha al sistema**: las preguntas que NO se responden son tan informativas como las que sí.
- **Danza, no empujes**: cuando el corpus se resiste, probablemente tiene razón.

> "Go quiet. Go still. Let it settle." — Donella Meadows.
