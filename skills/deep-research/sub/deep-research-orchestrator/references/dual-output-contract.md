# Contrato de salida dual: LIBRO + SOFTWARE

Cómo `deep-research-orchestrator` produce artefactos consumibles por `book-orchestrator` y por `orchestrator` (general).

## Resumen ejecutivo

| Modo | Quién consume | Qué se produce | Dónde vive |
|------|---------------|----------------|------------|
| **LIBRO** | `book-orchestrator` (Macro-fase R) | Evidence cards, borradores AsciiDoc, diagramas Mermaid | `research/evidence-cards/`, `research/drafts/`, `research/diagrams/` |
| **SOFTWARE** | `orchestrator` general | Blueprints, code patterns, knowledge graphs, test fixtures | `research/blueprints/`, `research/code-patterns/`, `research/test-fixtures/`, `research/knowledge-graphs/` |
| **DUAL** | Ambos | Todo lo anterior | Combinación |

---

## Modo LIBRO — Contrato con `book-orchestrator`

### Entrada (desde book-orchestrator)

El book-orchestrator, en su Macro-fase R, invoca el orquestador de deep-research cuando un capítulo requiere evidencia rigurosa. Pasa como input:
- `planning/outline.yml` (qué capítulo).
- `planning/curriculum-graph.yml` (qué conceptos).
- `research/claims.jsonl` (estado actual).

### Salida (hacia book-orchestrator)

```
research/
├── evidence-cards/
│   └── {topic}.yml          # claims con citas (entrada de evidence-manager)
├── drafts/
│   └── {chapter-slug}.md    # borrador AsciiDoc (entrada de chapter-writer)
├── diagrams/
│   └── {topic}.mmd          # Mermaid (incluido vía include:: en el libro)
└── corpus.yml               # sincronizado con research/corpus.yml del libro
```

### Contrato con `evidence-manager`

Cada claim tiene:
- `claim.id`: `cl-{kebab-slug}` (único en el libro).
- `claim.text`: la frase exacta que aparecerá en el `.adoc`.
- `claim.sources[].excerpt`: cita textual verbatim.
- `claim.sources[].page_reference`: para incluir en el `.adoc`.
- `claim.evidence_level`: L1-L7.
- `claim.status`: `verified` o `verified-with-disclaimer`.
- `claim.decay_date`: fecha en que `version-drift-detector` debe re-verificar.

### Contrato con `chapter-writer`

Los borradores en `research/drafts/*.md` son **punto de partida**, no texto final. El `chapter-writer`:
1. Verifica que cada afirmación tiene `claim_id` con `status: verified`.
2. Reemplaza código por `include::` desde el workspace.
3. Aplica la voz editorial del libro (`voice-profile.yml`).
4. Pasa por el panel C2 (8 revisores).

### Contrato con `hallucination-auditor`

El auditor verifica:
- Citas textuales con página exacta.
- `claim_id` presente en `corpus.yml`.
- `evidence_level ≥ L1` para conceptos atribuidos a autoridad.
- Sin afirmaciones cuantitativas sin `source_id`.

---

## Modo SOFTWARE — Contrato con `orchestrator` (general)

### Entrada (desde orchestrator)

El orchestrator invoca cuando el proyecto implica:
- Construir una simulación de sistemas (World3, climate, supply chain).
- Implementar feedback loops en código.
- Crear APIs para modelado causal (CLD as code).
- Aplicar leverage points a un sistema.

Pasa como input:
- `spec.md` o equivalente.
- Stack tecnológico.
- Restricciones.

### Salida (hacia orchestrator)

```
research/
├── blueprints/
│   ├── {component}.yml          # API/función específica
│   └── {domain}-model.yml       # modelo del dominio (variables, unidades, ecuaciones)
├── code-patterns/
│   ├── {pattern}.md             # descripción + cuándo usar
│   ├── {pattern}.py             # implementación Python
│   └── {pattern}.rs             # (opcional) Rust
├── knowledge-graphs/
│   └── {topic}.ttl              # RDF/Turtle
└── test-fixtures/
    └── {model}-expected.json    # valores esperados para tests
```

### Contrato con code generation

Cada `blueprint.yml` tiene:
- `interface`: signature exacta (tipos, units, errores).
- `algorithm`: pasos numerados con complejidad.
- `references`: papers/secciones que respaldan el algoritmo.
- `test_acceptance`: comportamiento esperado + valor de referencia + método de validación.

### Contrato con testing

Los `test-fixtures/*.json` contienen valores de referencia:
- World3 Standard Run (1972): valores de población, capital, recursos por año.
- Modelo de población de Forrester (1961): ciclo de empleo GE.
- Modelo de pesca (Sterman): referencia para Tragedy of the Commons.

### Librerías de simulación válidas

| Librería | Lenguaje | Modelo que cubre |
|----------|----------|------------------|
| PySD | Python | Vensim/STELLA → Python |
| BPTK-Py | Python | SD + agent-based |
| pysd-jax | Python | PySD con JAX (GPU) |
| Tellurium/libSEDML | Python | Systems Biology Markup Language |
| Vensim DSS | (propietario) | Editor + simulador |
| STELLA/iThink | (propietario) | Editor + simulador |

---

## Modo DUAL — Libro + código

```
research/
├── evidence/                   # para el libro
├── drafts/
├── diagrams/
├── blueprints/                 # para el software
├── code-patterns/
├── knowledge-graphs/
├── test-fixtures/
└── corpus.yml                  # fuente única de verdad
```

**Regla de coherencia**: toda afirmación cuantitativa en el libro (`evidence/{topic}.yml`) debe tener su contraparte en un test fixture (`test-fixtures/*.json`) o en una constante del modelo (`blueprints/{domain}-model.yml`).

Ejemplo:
- Libro: "Meadows identificó el paradigma como leverage point #2".
- Software: `class LeveragePoints: PARADIGM = 2`.
- Test: `assert LeveragePoints.PARADIGM == 2`.

---

## Anti-patrones del doble propósito

| Anti-patrón | Cómo evitarlo |
|-------------|---------------|
| Libro cita código que no existe | Generar código antes que el capítulo |
| Software implementa concepto no documentado | Toda función pública tiene test que mapea a claim del libro |
| Mezclar claims verificadas con opiniones | claim.evidence_level siempre presente |
| Generar código sin blueprint | Blueprint primero, código después |
| Generar claims sin source.excerpt | excerpt obligatorio para L1-L2 |

---

## Política de versionado

- `corpus.yml` tiene `version: {date}-{n}` (snapshot por sesión o re-verificación).
- Las claims con `status: needs_recheck` no bloquean si `risk: low`, pero SÍ bloquean si `risk: critical`.
- Cuando se publica nueva edición del libro o se re-deploya el software, el curator dispara R-incremental.
