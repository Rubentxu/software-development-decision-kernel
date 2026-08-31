---
name: book-memory-keeper
description: "Trigger: guardar contexto del libro, recordar dónde quedamos, persistir progreso, memoria del libro, checkpoint, sesión, contexto entre sesiones, guardar glosario, guardar decisión editorial, recuperar estado del libro, reanudar libro. Mantiene la memoria persistente del libro entre sesiones: estado del pipeline, voz/glosario, decisiones editoriales y contexto ejecutivo. Combina documentos vivos del proyecto con Engram para no perder nada entre días y sesiones."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo de forma **transversal y recurrente**: en cada checkpoint del workflow (al cerrar una macro-fase o sesión) y al arrancar (para recuperar estado). Es el **sistema nervioso de memoria** del libro: sin él, cada sesión empieza desde cero y se pierde contexto acumulado.

El `book-orchestrator` lo invoca al inicio (recuperar) y al cierre (persistir) de cada sesión.

No lo uses para ejecutar fases del pipeline (eso son las skills de fase), ni para tomar decisiones editoriales (eso `editorial-voice-designer`). Esta skill **recuerda y persiste** lo que otros producen.

## Hard Rules

- **Doble capa de memoria**:
  - **Documentos vivos del proyecto** (`book-context/`) — fuente de verdad legible y versionable.
  - **Engram** (`mem_save`/`mem_search`/`mem_session_summary`) — índice semántico recuperable entre sesiones.
- Ambas capas se mantienen **sincronizadas**: un cambio en documentos dispara un `mem_save`; una recuperación empieza por Engram y aterriza en documentos.
- El contexto ejecutivo de sesión se guarda **siempre al cerrar** (checkpoint), incluso si la sesión se interrumpe.
- Las decisiones editoriales/de diseño se persisten como ADRs (no solo en conversación).
- El glosario y el voice-profile son **vivos**: se actualizan al redactar, no se congelan.

## Las 4 memorias del libro

| Memoria | Documento | Engram | Cuándo se actualiza |
|---------|-----------|--------|---------------------|
| **Estado del pipeline** | `book-context/LEDGER.md` | `mem_save` type=decision | Cada cambio de estado de capítulo |
| **Voz + glosario** | `book-context/VOICE.md`, `book-context/GLOSSARY.md` | `mem_save` topic_key=voice/glossary | Al definir voz y al añadir términos |
| **Decisiones de diseño** | `book-context/adr/NN-slug.md` | `mem_save` type=decision | Cuando se toma una decisión no trivial |
| **Contexto de sesión** | `book-context/SESSION-LOG.md` | `mem_session_summary` | Al cerrar cada sesión |

## Execution Steps

### Al iniciar sesión (`recall`)
1. `mem_current_project` → detectar el proyecto de libro activo.
2. `mem_context` (scope project) → últimas observaciones y sesiones.
3. Leer `book-context/LEDGER.md` → estado actual del pipeline.
4. Leer `book-context/SESSION-LOG.md` → último resumen ejecutivo.
5. Sintetizar: "Estábamos en Macro-fase C, cap-12 en IN_REVIEW bloqueado por X; la voz es cero-a-experto con humor; el glosario tiene 47 términos."
6. Devolver el contexto al orchestrator para que continúe sin reinvestigar.

### Al cerrar sesión o macro-fase (`checkpoint`)
1. Actualizar `book-context/LEDGER.md` (estado por capítulo, bloqueos).
2. Generar resumen ejecutivo con `mem_session_summary` (Goal/Discoveries/Accomplished/Next Steps).
3. `mem_save` de las decisiones clave de la sesión (type=decision).
4. Si la voz o el glosario cambiaron, `mem_save` con topic_key estable (voice/glossary) para upsert.
5. Dejar `book-context/SESSION-LOG.md` con el "qué hacer mañana" explícito.

### Al tomar una decisión (`record_decision`)
1. Escribir un ADR en `book-context/adr/NN-{slug}.md` (formato en `references/adr-template.md`).
2. `mem_save` type=decision con el resumen + ruta del ADR.

### Al añadir término al glosario (`record_term`)
1. Actualizar `book-context/GLOSSARY.md`.
2. `mem_save` con topic_key=glossary-{libro} para upsert del glosario vivo.

## Por qué doble capa (documentos + Engram)

- **Documentos**: legibles por humanos, versionables en git, auditables. La fuente de verdad.
- **Engram**: recuperable semánticamente entre sesiones sin releer todo. El índice.
- Un documento puede perderse en 100 commits; Engram lo recupera por significado. Un Engram sin documento es volátil; el documento lo ancla.

## Output Contract

- Mantiene actualizados los documentos de `book-context/`.
- Genera observaciones Engram trazables (con rutas a los documentos).
- Al `recall`: devuelve contexto ejecutivo listo para continuar.
- Al `checkpoint`: deja el libro en estado reanudable.

## References

- `references/book-context-structure.md` — estructura estándar del directorio `book-context/` (tech-agnostic).
- `references/adr-template.md` — plantilla de ADR editorial.
- `references/session-checkpoint-protocol.md` — qué guardar al cerrar sesión.
