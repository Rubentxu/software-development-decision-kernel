---
name: chapter-writer
description: "Trigger: redactar capítulo, escribir capítulo, escribir contenido, escribir sección, redactar explicaciones, escribir libro. Redacta explicaciones, ejemplos, transiciones y resúmenes de un capítulo ASCIIDOC respetando estrictamente el contrato del capítulo y las evidence cards verificadas."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo **solo** cuando `chapter-planner` ha producido un contrato `READY_FOR_WRITER` para ese capítulo. No redactas nada sin contrato.

No lo uses para investigar (`source-researcher`), ni para verificar código (`code-example-verifier`), ni para revisar (`technical-reviewer`).

## Hard Rules — la regla de oro

> **Ninguna afirmación técnica se considera correcta por haber sido bien redactada.** Cada afirmación debe estar respaldada por una evidence card verificada, código ejecutable, o revisión explícita.

- **Carga `planning/voice-profile.yml`** (de `editorial-voice-designer`); el capítulo suena como el voice-profile declara, no como suena por defecto.
- **Carga `book-context/GLOSSARY.md`**; usa los términos canónicos (no inventes sinónimos).
- Solo puedes citar afirmaciones presentes en `research/evidence-cards/` con `status: verified`.
- Si necesitas afirmar algo sin card → **parar** y encargar a `source-researcher`.
- El código mostrado se **incluye desde los proyectos reales** (`include::`), no se copia a mano.
- Idioma: castellano de España por defecto. Voz editorial definida en `editorial-style.yml`.
- Respeta el glosario canónico; no inventes traducciones.

## Execution Steps

1. Leer `planning/chapters/{id}.yml` (contrato).
2. Leer evidence cards vinculadas.
3. Leer `audience-profile.yml` para calibrar tono/profundidad.
4. Estructurar el capítulo en AsciiDoc siguiendo `assets/chapter-template.adoc`:
   - Resumen (1 párrafo).
   - Objetivos de aprendizaje.
   - Secciones (una por concepto del contrato).
   - Cada sección: explicación → ejemplo incluido → transición.
   - Resumen final + glosario del capítulo.
5. Para cada afirmación técnica, anotar el `card_id` en un comentario AsciiDoc (`// evidence: ev-xxx`).
6. Incluir ejemplos con `include::../../examples/...[tags=...]`.
7. Dejar marcadores para diagramas (`// diagram: ecs-scheduler`).
8. Entregar el `.adoc` y pasar a `technical-reviewer`.

## Decision Gates

| Necesidad | Acción |
|-----------|--------|
| Afirmación sin card | Parar → `source-researcher` |
| Ejemplo no creado | Parar → `code-example-generator` |
| Diagrama necesario | Marcar placeholder → `diagram-architect` |
| Duda terminológica | Consultar glosario → `editorial-reviewer` |

## Output Contract

- `src/chapters/{chapter-id}.adoc`.
- Mapa de afirmaciones → `card_id` (para `hallucination-auditor`).
- Lista de placeholders pendientes (diagramas/ejemplos sin cubrir).
- Estado: `READY_FOR_REVIEW`.

## References

- `assets/chapter-template.adoc` — plantilla AsciiDoc del capítulo.
- `references/writing-style.md` — voz, tono y convenciones de castellano.
