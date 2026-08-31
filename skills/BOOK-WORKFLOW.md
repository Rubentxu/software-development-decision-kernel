# BOOK-WORKFLOW — La ley del sistema de libros técnicos

**Normativa vinculante.** El `book-orchestrator` carga este documento y lo hace cumplir. Cualquier conflicto entre una skill y este documento, gana este documento.

Estado: v1.1 · 2026-07-23 · 34 skills · 5 macro-fases (A fundamentos · **R investigación profunda** · B construcción · C validación · D publicación).

---

## 0. Principios rectores

1. **El código es el centro.** La fiabilidad del libro nace del workspace de ejemplos. La prosa explica y referencia ese código; no lo duplica.
2. **Regla de oro.** Ningún contenido LLM es correcto por estar bien redactado: requiere evidencia, código ejecutable o revisión explícita.
3. **Artefacto único.** El código mostrado en el libro y el código probado son la misma región (`tag::`) del workspace.
4. **Nada avanza sin su puerta.** Cada macro-fase produce artefactos verificables; la siguiente no empieza sin ellos.
5. **Stack-agnosticismo.** El sistema sirve para cualquier tecnología (Rust, Python, Go, JS...). El stack concreto se detecta y configura; nada del workflow depende de un lenguaje fijo.
6. **Memoria persistente.** Un libro se construye a lo largo de días y sesiones. El estado, la voz, el glosario, las decisiones y el contexto se persisten en `book-context/` + Engram para no perder nada. Ver §13.
7. **Distinguir libro ↔ código.** La prosa del libro y el repo de ejemplos son entidades distintas pero acopladas por el code-map.

---

## 1. Las 5 macro-fases

```
┌─────────────────────────────────────────────────────────────────┐
│  MACRO-FASE A · FUNDAMENTOS DEL LIBRO (una vez por libro)       │
│  audience → curriculum → outline → code-map                      │
├─────────────────────────────────────────────────────────────────┤
│  MACRO-FASE R · INVESTIGACIÓN PROFUNDA (recurrente, por tema)   │
│  agenda → descubrir → credibilidad → validar refs → triangular  │
│  → consolidar corpus                                             │
├─────────────────────────────────────────────────────────────────┤
│  MACRO-FASE B · CONSTRUCCIÓN (por capítulo, en bucle)           │
│  contrato → evidence(desde corpus) → código → redacción → ...   │
├─────────────────────────────────────────────────────────────────┤
│  MACRO-FASE C · VALIDACIÓN (por capítulo, en bucle)             │
│  verificar + panel de revisión (8 skills en paralelo)           │
├─────────────────────────────────────────────────────────────────┤
│  MACRO-FASE D · PUBLICACIÓN Y MANTENIMIENTO                     │
│  render → mantener (drift versiones + drift código-libro)       │
└─────────────────────────────────────────────────────────────────┘
```

**Orden de ejecución**: A → R → B → C → D. La Macro-fase R es **recurrente**: se re-ejecuta (parcial o completa) cuando el corpus decae, cuando se añaden temas, o cuando `version-drift-detector`/`code-prose-coherence-checker` disparan re-checks.

---

## 2. Macro-fase A — Fundamentos del libro (secuencial, una vez)

Se ejecuta una sola vez al crear el libro. Orden estricto, sin paralelizar (cada paso depende del anterior).

| Paso | Skill | Artefacto de salida | Depende de |
|------|-------|---------------------|------------|
| A1 | `book-project-initializer` | repo + estructura + CI | — |
| A2 | `audience-profiler` | `planning/audience-profile.yml` | A1 |
| A2b | `editorial-voice-designer` | `planning/voice-profile.yml` + glosario inicial | A2 |
| A2c | `book-stack-detector` | `planning/stack-profile.yml` (detecta Rust/Py/Go/JS...) | A1 |
| A3 | `curriculum-designer` | `planning/curriculum-graph.yml` | A2, A2b |
| A4 | `book-outline-architect` | `planning/outline.yml` + `src/book.adoc` esqueleto | A3 |
| A5 | `code-integration-architect` | `planning/code-map.yml` (bidireccional) | A4 |

**Puerta A→R**: A4 y A5 aprobados. Sin code-map no hay agenda de investigación útil.

**A2b y A2c en paralelo**: la voz editorial y la detección de stack son independientes entre sí y de A3; pueden correr junto a A2.

---

## 2b. Macro-fase R — Investigación profunda (recurrente, por tema)

El **ciclo de deep research**. Construye y mantiene el corpus de conocimiento verificado que alimenta toda la redacción. Es independiente de los capítulos: investiga **temas**, no secciones. Se ejecuta tras A (primera vez) y de forma recurrente cuando el corpus decae o se añaden temas.

### Filosofía
Un libro técnico fiable no se basa en "buscar para cada capítulo": se basa en un **corpus de conocimiento verificado** donde cada afirmación está triangulada entre fuentes independientes, clasificada por credibilidad y validada viva. La Macro-fase R construye ese corpus; `source-researcher` (en B) solo extrae evidence cards de él.

### Pipeline R (6 pasos)

| Paso | Skill | Artefacto | Depende de |
|------|-------|-----------|------------|
| R1 | `research-strategist` | `research/agenda.yml` (preguntas + niveles) | A4, A5 |
| R2 | `source-discovery-specialist` | `research/candidate-pool.yml` (multi-modal) | R1 |
| R3 | `source-credibility-assessor` | `research/credibility/{id}.yml` (admitted/rejected) | R2 |
| R4 | `reference-validator` | `research/reference-validation.jsonl` | R3 |
| R5 | `evidence-cross-validator` | `research/triangulation/` (claims + confidence) | R3, R4 |
| R6 | `research-knowledge-curator` | `research/corpus.yml` (vivo) + snapshot | R5 |

### Orden y paralelización
- R1 → R2: secuencial (sin agenda no se descubre con foco).
- R3 ∥ R4: **paralelizables** tras R2 (evaluar credibilidad y validar referencias son independientes).
- R5: tras R3 + R4 (triangula solo fuentes creíbles y vivas).
- R6: tras R5 (consolida lo triangulado).

### Bucle de investigación
La Macro-fase R no es un solo pase: tras R6, el curator puede descubrir **gaps** (preguntas sin claim verificada) o claims `needs_recheck`. Eso dispara otra ronda R1→R6 para esos temas. El bucle termina cuando:
- Todas las preguntas `critical` de la agenda tienen claim `verified`.
- No quedan gaps bloqueantes.

### Modos de ejecución
- **R-completa**: primera vez, o re-investigación mayor. Los 6 pasos para todos los temas.
- **R-incremental**: tras un cambio (nueva versión, nuevo tema). Solo los pasos afectados; el curator identifica qué reclamar `needs_recheck`.

### Output hacia B
El `corpus.yml` es la entrada de `source-researcher` (B2): extrae evidence cards de claims `verified`. Un capítulo no puede empezar B2 si sus conceptos no tienen cobertura en el corpus.

**Puerta R→B**: cobertura del corpus ≥ umbral (todas las preguntas `critical` con claim `verified` o `disputed` con disclaimer). Gaps `low` no bloquean.

---

## 3. Macro-fase B — Construcción del capítulo (en bucle, por capítulo)

Se repite para cada capítulo del outline. Dentro de un capítulo es **secuencial** (el writer necesita evidence + código); entre capítulos puede haber concurrencia (ver §6).

| Paso | Skill | Artefacto | Depende de |
|------|-------|-----------|------------|
| B1 | `chapter-planner` + `code-pedagogy-justifier` | `planning/chapters/{id}.yml` + `planning/code-cards/{crate}.yml` | A5, code-map |
| B2 | `source-researcher` | `research/sources.yaml` + evidence cards | B1 |
| B3 | `code-example-generator` | crate en `chapters/chapter-{NN}-{slug}/` | B1 (code card) |
| B4 | `example-complexity-controller` | `build/reviews/{ex}.complexity.yml` | B3 |
| B5 | `chapter-writer` | `src/chapters/{id}.adoc` (solo con cards; código vía `include::`) | B2, B3, code-map |
| B6 | `diagram-architect` | `diagrams/{id}.{ext}` | B1, B5 |
| B7 | `exercise-designer` | `exercises/{id}/` + `solution/` | B1, B3 |

**Puerta B→C**: B5 completo y B1 está `READY_FOR_REVIEW`. B6 y B7 pueden terminar en paralelo con el inicio de C (no bloquean la verificación de código).

**Notas de orden**:
- B2 y B3 **pueden arrancar en paralelo** una vez B1 está listo (investigar fuentes y generar código son independientes), pero B5 los necesita a ambos.
- B4 puede devolver el ejemplo a B3 (si `OVERCOMPLEX`).

---

## 4. Macro-fase C — Validación (en bucle, por capítulo)

Dos sub-fases: **C1 verificación** (determinista, compilación) y **C2 panel de revisión** (semántica, paralelo).

### C1 — Verificación determinista
| Paso | Skill | Artefacto | Depende de |
|------|-------|-----------|------------|
| C1a | `code-example-verifier` | `build/verify-report.jsonl` (`ALL_GREEN`) | B3 |

**Puerta C1→C2**: `ALL_GREEN`. Sin código que compile, no se revisa semántica.

### C2 — Panel de revisión (8 skills en paralelo)
Todas leen el mismo `.adoc` + evidence cards + code-map. Son **independientes** → se ejecutan concurrentes.

| Skill | Artefacto | Tipo |
|-------|-----------|------|
| `technical-reviewer` | `{id}.review.yml` | técnica |
| `pedagogical-reviewer` | `{id}.pedagogy.yml` | didáctica |
| `hallucination-auditor` | `{id}.hallucination.yml` | fabricaciones |
| `code-prose-coherence-checker` | `{id}.coherence.yml` | prosa↔código |
| `security-reviewer` | `{id}.security.yml` | seguridad (condicional) |
| `accessibility-reviewer` | `{id}.a11y.yml` | a11y |
| `analogy-auditor` | `{id}.analogy.yml` | analogías |
| `editorial-reviewer` | `{id}.editorial.yml` | estilo |

**C2 condicionales**: `security-reviewer` solo si el capítulo toca infra/redes/backend/comandos. El resto siempre.

**Puerta C2→B (bucle de remediación)** o **C2→D**: ver §5 (resolución de veredictos).

### C3 — Sincronización transversal
| Paso | Skill | Artefacto |
|------|-------|-----------|
| C3 | `evidence-manager` | `evidence/claims.jsonl` actualizado |

Corre tras C2, antes de dar el capítulo por `done`.

---

## 5. Resolución de veredictos y máquina de estados del capítulo

```
            ┌──────────────────────────────────────────────┐
            │                                              ▼
PLANNED → DRAFTING → IN_REVIEW → (PASS?) ─── yes ──→ DONE
   │          │           │                             
   │          │           └── no (BLOCKED) ──→ devuelve a DRAFTING
   │          │                                          (con hallazgos)
   └──────── BLOCKED_PERMANENT  (scope imposible, decide autor)
```

**Estados**:
- `PLANNED`: contrato (B1) pendiente o no aprobado.
- `DRAFTING`: B2–B7 en curso.
- `IN_REVIEW`: C1/C2 en curso.
- `BLOCKED`: una puerta falló y exige remediación; el capítulo vuelve a `DRAFTING` con la lista de hallazgos.
- `DONE`: todas las puertas de §8 en verde.
- `BLOCKED_PERMANENT`: alcance imposible (concepto fuera de scope, API que no existe). Requiere decisión del autor.

**Reglas de remediación** (a qué skill vuelve cada fallo):

| Fallo detectado por | Vuelve a | Acción |
|---------------------|----------|--------|
| `code-example-verifier` (compile/test) | `code-example-generator` | reparar crate |
| `code-example-verifier` (versión crate) | `source-researcher` | verificar versión real |
| `hallucination-auditor` (API inventada) | `source-researcher` → `chapter-writer` | evidenciar o reescribir |
| `hallucination-auditor` (afirmación sin card) | `source-researcher` → `chapter-writer` | crear card |
| `code-prose-coherence-checker` (`MANUAL_COPY`) | `chapter-writer` | reemplazar por `include::` |
| `code-prose-coherence-checker` (`DIVERGENCE`) | `chapter-writer` | alinear prosa al código |
| `code-prose-coherence-checker` (`BROKEN_INCLUDE`) | `code-integration-architect` | reparar code-map |
| `technical-reviewer` (`BLOCKED`) | `chapter-writer` | corregir |
| `pedagogical-reviewer` (`BLOCKED`) | `chapter-writer` | tapar saltos conceptuales |
| `editorial-reviewer` (`llm_tell`) | `chapter-writer` | reescribir muletilla |
| `example-complexity-controller` (`OVERCOMPLEX`) | `code-example-generator` | dividir/simplificar |

**Límite de remediación**: 3 ciclos por capítulo por la misma causa. Al 4º, escalar al autor (probablemente un problema de alcance, no de ejecución).

---

## 6. Paralelización y concurrencia

- **Dentro de un capítulo**: B2 y B3 en paralelo; C2 (8 revisores) en paralelo.
- **Entre capítulos**: la macro-fase B/C es independiente por capítulo → el orchestrator puede tener varios capítulos en vuelo simultáneamente.
- **Restricción**: dos capítulos no pueden modificar el **mismo crate** del workspace a la vez (gestionar con el code-map: un crate compartido es sección crítica).
- **Anti-patrón**: no paralelizar pasos con dependencia de datos (ej. C2 antes que C1a, o B5 antes que B2+B3).

---

## 7. Macro-fase D — Publicación y mantenimiento

### D1 — Render
| Paso | Skill | Artefacto | Depende de |
|------|-------|-----------|------------|
| D1 | `book-builder` | `build/html,pdf,epub` + `manifest.json` | TODOS los capítulos `DONE` |
| D1b | `citation-manager` | `evidence/references.bib` + bibliografía | C3 |

`book-builder` hace preflight: rechaza si algún capítulo no está `DONE` o si `verify-report.jsonl` no es `ALL_GREEN`.

**Framework compartido (ADR-15)**: Si el libro usa `book-template/` (modo compartido), `book-builder` debe invocar `book-template/scripts/sync-to-blog.sh --apply` antes del render para asegurar que `blog/static/libros/shared/` está sincronizado. Si no, los libros publicarán con un reader desactualizado. Política:

1. `book-template/assets/` es la **fuente canónica** del reader JS/CSS.
2. `blog/static/libros/shared/` es la versión productiva servida por Hugo.
3. `sync-to-blog.sh --apply` los mantiene en sync; el orchestrator lo ejecuta como preflight de D1 cuando hay un libro en modo compartido.
4. Tras el sync, el orchestrator propone commit con mensaje `chore: sync reader framework from book-template`.

Documentación detallada en `book-template/docs/10-shared-vs-static.md` (modos compartido vs estático).

### D2 — Mantenimiento (periódico o disparado)
Tres tipos de drift, cada uno con su skill:

| Disparador | Skill | Tipo de drift |
|------------|-------|---------------|
| Nueva versión de Bevy/crate | `version-drift-detector` | versiones/dependencias |
| Cambio en el workspace de ejemplos | `code-prose-coherence-checker` (modo `drift`) | código-libro |
| Drift entre `book-template/assets/` y `libros/shared/` | `sync-to-blog.sh` (en modo auditoría) | framework compartido |

Los tres informes alimentan a `release-maintainer`, que orquesta la nueva edición (re-ejecuta B/C solo en capítulos afectados).

### D3 — Conocimiento profundo del framework (on-demand)
| Skill | Cuándo |
|-------|--------|
| `repository-knowledge-extractor` | Cuando el libro debe explicar un framework/repo concreto; alimenta a B2 y A3 |

---

## 7b. Framework compartido de libros (`book-template/`)

A partir de 2026-07-24, todo libro técnico del workflow debe partir del
**template reutilizable** en `<your-book-template-path>/`.
Este template:

- Estandariza estructura, componentes, interacciones, tipografía y
  comportamiento responsive/a11y.
- Permite paletas por libro vía design tokens (`--bg`, `--primary`,
  `--accent`, `--cyan`) sin tocar el reader.
- Distribuye en dos modos (compartido vs estático) según el caso de uso.
- Se sincroniza con `blog/static/libros/shared/` mediante
  `scripts/sync-to-blog.sh`.

**Skills que deben conocer `book-template/`**:

- `book-project-initializer` (A1): incluir el template como input opcional.
- `book-outline-architect` (A4): documentar el outline asumiendo estructura multi-página.
- `chapter-writer` (B5): usar los features del template (lab boxes, admonitions custom).
- `book-builder` (D1): invocar `sync-to-blog.sh --apply` en preflight.
- `version-drift-detector` (D2): detectar drift framework↔shared.

**Contrato entre el template y el orchestrator**:

| Cosa | Quién la posee |
|------|----------------|
| Reader JS/CSS | `book-template/assets/` |
| Sincronización con blog | `book-template/scripts/sync-to-blog.sh` |
| Build multi-página | `book-template/scripts/build-chapters.py` |
| Verificación | `book-template/scripts/verify-book.sh` |
| Tokens de paleta por libro | `static/libros/<slug>/assets/styles.css` |
| Slugs y rutas | `static/libros/<slug>/` |

---

## 8. Puertas de calidad (no negociables)

Un capítulo pasa a `DONE` **solo** cuando las 7 se cumplen:

1. `code-example-verifier` = `ALL_GREEN` para los crates que referencia.
2. `hallucination-auditor` = `PASS` (cero `critical`).
3. `code-prose-coherence-checker` = `PASS` (cero `MANUAL_COPY`, `DIVERGENCE`, `BROKEN_INCLUDE`).
4. `technical-reviewer` ≠ `BLOCKED`.
5. `pedagogical-reviewer` ≠ `BLOCKED`.
6. Toda afirmación técnica con `claim_id` `status: verified`.
7. `editorial-reviewer` sin `llm_tell` sin corregir.

`book-builder` (D1) rechaza render si cualquier capítulo ≠ `DONE`.

---

## 9. Persistencia y reanudación

El estado vive en `book-ledger.md` (en el dir del libro):

```yaml
book: "Patrones 2D y ECS con Bevy 0.19"
current_macro_phase: C
chapters:
  - id: cap-04
    state: DONE
  - id: cap-12
    state: IN_REVIEW      # C2 en curso
    blocked_on: null
  - id: cap-13
    state: BLOCKED
    blocked_on: hallucination-auditor
    remediation_target: chapter-writer
    cycle: 2
last_updated: 2026-07-23T15:00:00Z
```

**Al iniciar sesión**, el orchestrator lee el ledger y reanuda desde el último estado conocido. No reinicia capítulos `DONE`. Reanuda `BLOCKED` aplicando la remediación pendiente.

**Checkpoint**: tras cada paso completado, el orchestrator actualiza el ledger (ligero: estado + `blocked_on` + `cycle`). Así una interrupción no pierde progreso.

---

## 10. Enrutamiento de peticiones del usuario

El orchestrator clasifica cada petición antes de actuar:

| Petición | Macro-fase | Acción |
|----------|------------|--------|
| "Crear libro nuevo" | A desde A1 | Flujo completo |
| "Escribe/avanza capítulo N" | B/C según estado del ledger | Lee ledger, continúa |
| "Revisa capítulo N" | C | C1a → C2 |
| "Publica/renderiza" | D1 | Preflight + `book-builder` |
| "Actualiza el libro" | D2 | drift versiones + drift código → `release-maintainer` |
| "Cambió el código del workspace" | D2 | `code-prose-coherence-checker` modo `drift` |
| "Explica el framework X" | D3 | `repository-knowledge-extractor` |

Si la petición no encaja o el estado es ambiguo, el orchestrator **pregunta** antes de delegar.

---

## 11. Fallos comunes que este workflow previene

| Fallo (caso real del libro Bevy) | Prevención |
|----------------------------------|------------|
| Snippet con sintaxis inventada pegado a mano | B5 exige `include::`; C2 `MANUAL_COPY` lo detecta |
| API que no existe presentada como estable | B2 evidence card; C2 `hallucination-auditor` |
| Crate con versión inventada | B2 verifica versión; C1a `VERSION_MISMATCH` |
| Prosa dice una cosa, código hace otra | C2 `DIVERGENCE` (coherence-checker) |
| Include a un tag que desapareció | C2 `BROKEN_INCLUDE`; D2 drift lo detecta al cambiar el repo |
| Ejemplo demasiado complejo que distrae | B4 `example-complexity-controller` |
| Muletilla LLM sin corregir | C2 `editorial-reviewer` |
| Render con capítulos rotos | D1 preflight rechaza |

---

## Referencia rápida de skills por macro-fase

```
A: book-project-initializer → audience-profiler → curriculum-designer
   → book-outline-architect → code-integration-architect
R: research-strategist → source-discovery-specialist
   → [source-credibility-assessor ∥ reference-validator]
   → evidence-cross-validator → research-knowledge-curator
B: chapter-planner+code-pedagogy-justifier → [source-researcher(desde corpus) ∥ code-example-generator]
   → example-complexity-controller → chapter-writer → diagram-architect ∥ exercise-designer
C: code-example-verifier → [8 revisores en ∥] → evidence-manager
D: citation-manager → book-builder  |  version-drift-detector + code-prose-coherence-checker(drift) → release-maintainer
```

---

## 12. La Macro-fase R en el mantenimiento (D)

El corpus de conocimiento **decae**. La Macro-fase R no es solo de inicio: se re-dispara desde D.

| Disparador | Qué re-ejecuta R | Origen |
|------------|------------------|--------|
| `version-drift-detector` detecta nueva versión | R4+R5+R6 en claims afectadas | drift de versiones |
| `code-prose-coherence-checker` (modo drift) detecta cambio de código | R5+R6 en claims del capítulo | drift código-libro |
| `decay_date` vencida en el corpus | R2→R6 en ese tema | decaimiento temporal |
| Nuevo tema/capítulo añadido | R1→R6 completo para ese tema | nuevo alcance |
| `hallucination-auditor` (C2) detecta afirmación sin card | R1→R6 focal para esa pregunta | gap descubierto en redacción |

Así el conocimiento del libro **no se pudre**: cada afirmación tiene fecha de caducidad y mecanismo de re-verificación.

---

## 13. Memoria persistente entre sesiones (skill: `book-memory-keeper`)

Un libro técnico se construye a lo largo de **días, semanas y muchas sesiones**. Sin memoria persistente, cada sesión pierde el contexto acumulado (voz, glosario, decisiones, dónde quedó cada capítulo). La skill `book-memory-keeper` es el **sistema nervioso de memoria**, transversal a todas las macro-fases.

### Doble capa de memoria

| Capa | Mecanismo | Rol |
|------|-----------|-----|
| **Documentos vivos** | `book-context/` (LEDGER, VOICE, GLOSSARY, ADRs, SESSION-LOG) | Fuente de verdad legible, versionable, auditable |
| **Engram** | `mem_save`/`mem_search`/`mem_session_summary` | Índice semántico recuperable entre sesiones sin releer todo |

Ambas se mantienen **sincronizadas**: un cambio en documentos dispara un `mem_save`; una recuperación empieza por Engram y aterriza en documentos.

### Las 4 memorias

| Memoria | Documento | Engram topic_key |
|---------|-----------|------------------|
| Estado del pipeline | `LEDGER.md` | (observaciones type=decision) |
| Voz + estilo | `VOICE.md` | `voice-{libro}` |
| Glosario canónico | `GLOSSARY.md` | `glossary-{libro}` |
| Decisiones de diseño | `adr/NN-{slug}.md` | (observations type=decision) |

### Checkpoints (puntos de guardado obligatorios)

| Momento | Qué persiste `book-memory-keeper` |
|---------|-----------------------------------|
| Al **cerrar sesión** | LEDGER + SESSION-LOG + `mem_session_summary` + `mem_save` decisiones |
| Al cerrar **macro-fase A** | book-config + voice-profile + code-map |
| Al cerrar **macro-fase R** | corpus.yml + snapshot + gaps |
| Al cerrar **macro-fase B** (capítulo) | estado del capítulo + code cards |
| Al cerrar **macro-fase C** (capítulo) | verify-report + revisión consolidada |
| Al cerrar **macro-fase D** | manifest + CHANGELOG edición |

### Rutina de arranque (recall)

Al iniciar cualquier sesión, el `book-orchestrator` invoca `book-memory-keeper(recall)`:
1. `mem_current_project` → detecta el libro activo.
2. `mem_context` → últimas observaciones/sesiones.
3. Lee `LEDGER.md` + `SESSION-LOG.md`.
4. Sintetiza y devuelve contexto ejecutivo: "Estábamos en macro-fase C, cap-12 bloqueado por X; voz cero-a-experto con humor; glosario 47 términos."

### Rutina de cierre (checkpoint)

Al cerrar sesión o alcanzar parada natural, `book-memory-keeper(checkpoint)`:
1. Actualiza `LEDGER.md` + `SESSION-LOG.md` con "qué hacer mañana".
2. `mem_session_summary` con Goal/Discoveries/Accomplished/Next Steps.
3. `mem_save` por cada decisión clave.
4. Upsert de voz/glosario si cambiaron.

**Sin este checkpoint, la siguiente sesión empieza a ciegas.** Es obligatorio, aunque la sesión haya sido corta.

### Decisiones como ADRs

Cualquier decisión no trivial (stack, arquetipo editorial, simplificación pedagógica, alcance) se registra como ADR en `book-context/adr/NN-{slug}.md` + `mem_save` type=decision. Así "¿por qué enseñamos ChildOf en vez de bsn!?" tiene respuesta trazable meses después.

### Anti-patrón
Confiar en la memoria de la conversación. Una sesión puede resumirse y perder el detalle; un ADR y Engram no. Si una decisión no aterrizó en `book-context/` + Engram, no existe para la próxima sesión.
