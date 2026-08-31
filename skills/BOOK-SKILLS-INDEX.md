# Libros técnicos con LLM — Sistema de skills y agente

Sistema agéntico para crear **libros técnicos de cualquier tecnología** generados con LLM, con cualquier voz editorial (con humor, de cero a experto, para dummies, referencia formal). La calidad se garantiza por **evidencia, código ejecutable y revisión explícita**, no por buena redacción.

## Regla de oro

> Ningún contenido generado por el LLM se considera correcto por haber sido bien redactado; debe estar respaldado por evidencia, código ejecutable o revisión explícita.

Corolario para libros técnicos: **el código del workspace de ejemplos es el centro de la fiabilidad**. La prosa explica y referencia ese código; no lo duplica.

## Stack-agnosticismo

El sistema sirve para **cualquier libro futuro** (Rust, Python, Go, JS, Java, C++...). El stack concreto lo detecta `book-stack-detector` y lo configuran `code-example-generator`/`code-example-verifier`. Nada del workflow depende de un lenguaje fijo.

## Memoria persistente

Un libro se construye a lo largo de días y sesiones. El estado, la voz, el glosario, las decisiones y el contexto se persisten en `book-context/` + Engram (`book-memory-keeper`) para no perder nada entre sesiones.

## Agente principal

**`book-orchestrator`** (`~/.zcode/agents/book-orchestrator.md`) — coordinador que ejecuta el workflow delegando en 28 skills. Nunca redacta ni verifica inline.

## Contexto del repositorio (fuente de verdad del código)

Lee **siempre** `~/.zcode/skills/BOOK-REPO-CONTEXT.md` antes de tocar código. Resume:
- **Workspace**: `<your-book-repo-path>/` (Cargo, Bevy `=0.19.0`).
- **Libro HTML**: `<your-book-output-path>/`.
- **Convención**: `chapters/chapter-{NN}-{slug}/`, crate `bevy-book-chapter-{NN}`.
- **CI**: `cargo fmt/check/test/clippy --workspace --locked`.

## Stack editorial por defecto

**AsciiDoc + Asciidoctor**. El código se integra vía `include::` desde el workspace.

## Las 37 skills

### MVP (8) — núcleo operativo
| Skill | Fase | Qué hace |
|-------|------|----------|
| `book-outline-architect` | A4 | Diseña partes/capítulos/secciones |
| `source-researcher` | B2 | Evidence cards desde el corpus (o focal si no hay) |
| `chapter-planner` | B1 | Contrato verificable de capítulo |
| `chapter-writer` | B5 | Redacta el `.adoc` solo con evidence cards |
| `code-example-generator` | B3 | Crea ejemplos **en el stack detectado** (cargo/uv/go/npm/maven...) |
| `code-example-verifier` | C1a | Ejecuta la cadena de verificación del stack-profile |
| `technical-reviewer` | C2 | Exactitud + arquitectónica + adversarial |
| `book-builder` | D1 | Render HTML/PDF/EPUB tras preflight |

### Macro-fase R — Investigación profunda (6) — el corpus de conocimiento
| Skill | Fase | Qué hace |
|-------|------|----------|
| `research-strategist` | R1 | Agenda: qué investigar, nivel de evidencia por afirmación |
| `source-discovery-specialist` | R2 | Descubrimiento multi-modal (docs/RFC/papers/código/libros/blogs) |
| `source-credibility-assessor` | R3 | Ranking L1–L7, sesgo, COI, frescura, link rot |
| `reference-validator` | R4 | Verificación viva (URL/DOI/crates.io/docs.rs/GitHub) |
| `evidence-cross-validator` | R5 | Triangulación, conflictos, confidence_score |
| `research-knowledge-curator` | R6 | Corpus persistente, dedup, gaps, decaimiento |

### Bloque dedicado al código (3) — el centro del libro
| Skill | Fase | Qué hace |
|-------|------|----------|
| `code-integration-architect` | A5 | Mapa bidireccional libro↔workspace + includes |
| `code-pedagogy-justifier` | B1b | Code cards: por qué este ejemplo, qué NO muestra |
| `code-prose-coherence-checker` | C2 | Coherencia prosa↔código + drift (review/drift) |

### Principales (10) — pipeline completo
| Skill | Fase | Qué hace |
|-------|------|----------|
| `book-project-initializer` | A1 | Repo, estructura, convenciones, CI |
| `audience-profiler` | A2 | Lector objetivo, profundidad |
| `editorial-voice-designer` | A2b | Arquetipo editorial (humor/cero-a-experto/dummies/referencia) + voice-profile |
| `book-stack-detector` | A2c | Detecta stack (Rust/Py/Go/JS/Java...) → stack-profile |
| `curriculum-designer` | A3 | Grafo de conceptos |
| `evidence-manager` | C3 | Índice de claims (afirmación↔fuente) |
| `diagram-architect` | B6 | Diagramas como código (Mermaid/PlantUML/C4/DOT) |
| `pedagogical-reviewer` | C2 | Saltos conceptuales, explicaciones circulares |
| `editorial-reviewer` | C2 | Terminología, voz, muletillas (Vale) — valida contra voice-profile |
| `hallucination-auditor` | C2 | APIs inventadas, crates inexistentes, referencias falsas |
| `exercise-designer` | B7 | Ejercicios con pistas, soluciones, criterios |
| `release-maintainer` | D2 | Nuevas ediciones tras drift |

### Avanzadas (7) — calidad superior
| Skill | Fase | Qué hace |
|-------|------|----------|
| `citation-manager` | D1b | `references.bib` y bibliografía CSL |
| `repository-knowledge-extractor` | R/D3 | Análisis de repo con AST/LSP |
| `version-drift-detector` | D2 | Drift de **versiones/dependencias** |
| `example-complexity-controller` | B4 | Evita ejemplos LLM sobredimensionados |
| `analogy-auditor` | C2 | Analogías que no induzcan error |
| `security-reviewer` | C2 | Secretos, permisos, comandos destructivos |
| `accessibility-reviewer` | C2 | alt, contraste, jerarquía (WCAG AA) |

### Memoria y voz (2) — el sistema nervioso
| Skill | Rol | Cuándo |
|-------|-----|--------|
| `book-memory-keeper` | Persiste estado, voz, glosario, ADRs y contexto entre sesiones (doble capa: `book-context/` + Engram) | Al arrancar (recall) y al cerrar (checkpoint) cada sesión/macro-fase |
| `editorial-voice-designer` | (también arriba) Define la voz editorial del libro | A2b |

## División de responsabilidades (frecuentes)

| Necesidad | Skill |
|-----------|-------|
| Definir voz editorial y glosario inicial | `editorial-voice-designer` |
| Detectar el stack del libro | `book-stack-detector` |
| Recuperar o persistir contexto entre sesiones | `book-memory-keeper` |
| Planificar qué investigar y con qué rigor | `research-strategist` |
| Descubrir fuentes amplio (libros/blogs/docs) | `source-discovery-specialist` |
| Puntuar credibilidad de una fuente | `source-credibility-assessor` |
| Validar que una referencia existe vivo | `reference-validator` |
| Triangular afirmación entre fuentes | `evidence-cross-validator` |
| Mantener el corpus de conocimiento | `research-knowledge-curator` |
| Extraer evidence cards para un capítulo | `source-researcher` (desde corpus) |
| Decidir qué código va en cada sección | `code-integration-architect` |
| Justificar por qué ese ejemplo | `code-pedagogy-justifier` |
| Crear/modificar el código | `code-example-generator` (usa stack-profile) |
| Compilar y testear | `code-example-verifier` (usa stack-profile) |
| Validar que prosa↔código coinciden | `code-prose-coherence-checker` |
| Drift de versiones de crates/deps | `version-drift-detector` |
| Drift código-libro (al cambiar el repo) | `code-prose-coherence-checker` (modo drift) |

## Puertas de calidad (no negociables)

Un capítulo pasa a `done` solo cuando:

1. `code-example-verifier` = `ALL_GREEN` para los crates que referencia.
2. `hallucination-auditor` = `PASS` (cero `critical`).
3. `code-prose-coherence-checker` = `PASS` (cero `MANUAL_COPY`, `DIVERGENCE`, `BROKEN_INCLUDE`).
4. `technical-reviewer` ≠ `BLOCKED`.
5. `pedagogical-reviewer` ≠ `BLOCKED`.
6. Toda afirmación técnica con `claim_id` `status: verified`.
7. `editorial-reviewer` sin `llm_tell` sin corregir.

`book-builder` rechaza el render si algún capítulo está `BLOCKED`.
