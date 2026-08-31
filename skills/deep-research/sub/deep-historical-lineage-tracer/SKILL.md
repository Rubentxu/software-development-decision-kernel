---
name: deep-historical-lineage-tracer
description: "Trigger: historia de un campo, quién fundó X, evolución de Y, línea temporal de Z, ¿cómo surgió W?. Traza la evolución histórica de un campo o tecnología: autores seminales, papers fundacionales, controversias, refutaciones, escuelas de pensamiento. Produce una línea temporal con fuentes primarias. Para LIBRO: secciones históricas; para SOFTWARE: contexto histórico en decisiones de diseño."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: domain-pipeline
  author: rubentxu
  version: "1.0"
  domain: deep-research, history
  consumers: [book-orchestrator]
---

## Activation Contract

Úsalo cuando el tema tiene **dimensión temporal**: cómo evolucionó un campo, quién influyó en quién, qué controversias hubo. Produce líneas temporales y árboles genealógicos conceptuales con fuentes primarias.

No lo uses para: estado del arte actual (eso es R1-R6 estándar), comparativa de alternativas actuales (`deep-knowledge-graph-builder`).

## Hard Rules

- **Cada evento/fecha tiene fuente primaria**: paper original, autobiografía, archivo.
- **No inventar fechas**: si no se sabe, marcar `date: approximate` con rango.
- **Influencias requieren paper que las documente**: "X influyó en Y" requiere cita de Y diciendo que fue influenciado por X (o análisis peer-reviewed).
- **Controversias explícitas**: si hubo debate, documentar AMBAS posiciones con sus fuentes.
- **Línea temporal coherente**: las fechas deben cuadrar (no se puede citar un paper de 1985 en 1970).

## Execution Steps

1. Activar pipeline R para el campo.
2. Identificar los **eventos fundacionales**:
   - Primer paper seminal.
   - Primeros autores.
   - Primeros prototipos/implementaciones.
3. Identificar las **influencias**:
   - ¿Qué campos previos influyeron? (e.g., System Dynamics influyó en Limits to Growth).
   - ¿Quiénes se citan mutuamente? (citation graph).
4. Identificar **controversias y refutaciones**:
   - ¿Hubo debates famosos?
   - ¿Algún paper fue refutado?
5. Construir la línea temporal:
   - Lista cronológica de eventos con fecha, descripción, fuentes.
   - Grafo de influencias (quién → quién).
6. Identificar **escuelas de pensamiento**:
   - Diferentes enfoques del mismo problema (e.g., en ML: simbólico vs. conexionista vs. bayesiano).

## Esquema de línea temporal

```yaml
timeline:
  topic: "System Dynamics"
  events:
    - date: 1956
      type: institutional
      event: "Jay Forrester acepta posición en MIT Sloan"
      sources: [src-systemdynamics-org-origin]
    - date: 1961
      type: publication
      event: "Publicación de Industrial Dynamics"
      author: "Jay W. Forrester"
      sources: [src-forrester-1961]
    - date: 1972
      type: publication
      event: "Publicación de The Limits to Growth"
      authors: ["Donella Meadows", "Dennis Meadows", "Jørgen Randers", "William Behrens"]
      sources: [src-limits-to-growth-1972]
  influences:
    - {from: src-forrester-1961, to: src-limits-to-growth-1972, type: foundation}
  controversies:
    - topic: "Limits to Growth predictions"
      positions:
        - {claim: "Las predicciones de World3 son correctas", sources: [src-turner-2014]}
        - {claim: "Las predicciones de World3 son alarmistas", sources: [src-growth-lobby]}
  schools:
    - {name: "System Dynamics (Forrester/Meadows)", focus: "feedback loops cuantitativos"}
    - {name: "Soft Systems (Checkland)", focus: "interpretación cualitativa"}
```

## Output según modo

### Modo LIBRO
- `research/timelines/{topic}.yml`.
- `research/drafts/{topic}-history-section.md` (borrador AsciiDoc).
- `research/diagrams/{topic}-timeline.mmd` (Mermaid timeline).

### Modo SOFTWARE
- `research/timelines/{topic}.yml` para contexto histórico en `blueprints`.

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Fecha sin fuente | NO inventar; marcar `date: approximate` con rango |
| Influencia sin cita | NO incluir; o marcar `evidence: anecdotal` |
| Controversia con una sola posición documentada | STOP: buscar la otra posición; o marcar `unbalanced` |
| Evento con > 3 fuentes L1 | Consenso fuerte; usar la fuente más temprana o canónica |
| Cronología inconsistente (fechas que no cuadran) | Re-evaluar; STOP hasta resolver |

## Output Contract

- `research/timelines/{topic}.yml`.
- `research/drafts/{topic}-history-section.md` (LIBRO).
- `research/diagrams/{topic}-timeline.mmd` (LIBRO).
- Actualizar `research/corpus.yml`.

## References

- Fuentes históricas generales:
  - Wikipedia (L3): para navegar a fuentes primarias.
  - Archivos institucionales (MIT, Stanford, ACM Digital Library).
  - Autobiografías y biografías peer-reviewed.
- `references/timeline-format.md` — formatos de timeline (Mermaid, Markdown, etc.).
