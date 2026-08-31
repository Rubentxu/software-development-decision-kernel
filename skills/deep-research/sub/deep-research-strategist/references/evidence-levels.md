# Niveles de evidencia — Universales

Calibración universal L1-L7 aplicable a cualquier dominio. Las skills de dominio pueden extender o reinterpretar floors.

## L1 — Fuente primaria

Definición: documento original del autor canónico, paper revisado por pares, código fuente verificado, fuente primaria histórica.

**Ejemplos por dominio**:

| Dominio | Ejemplo L1 |
|---------|------------|
| Software | Código fuente del repo oficial, RFC, release notes oficial |
| IA/ML | Paper original del modelo (arXiv con código público), paper peer-reviewed |
| Systems Thinking | `Thinking in Systems` (Meadows 2008), `Leverage Points` (1997), Forrester 1961 |
| Ciencia | Paper peer-reviewed en journal indexado |
| Historia | Documento de época, archivo desclasificado, autobiografía |
| Medicina | Clinical trial registrado (clinicaltrials.gov), paper peer-reviewed |
| Economía | Datos oficiales (BLS, INE, Banco Mundial), metodología publicada |
| Tecnología web | MDN, W3C, ECMA-262 |

## L2 — Fuente oficial autoritativa

Definición: documento institucional, white paper, peer-reviewed que cita L1, datos oficiales con metodología.

**Ejemplos**: docs oficiales de framework, white paper institucional, peer-reviewed secundario, IPCC reports, FDA approvals.

## L3 — Fuente secundaria revisada

Definición: enciclopedia académica (Wikipedia, Stanford Encyclopedia), paper que cita L1 sin ser del autor original, tesis doctoral.

**Advertencia**: Wikipedia es L3, no L1. Usar como navegador a fuentes primarias.

## L4 — Fuente periodística especializada

Definición: revista técnica, periodista con expertise.

**Ejemplos**: MIT Technology Review, The Economist (sección tech/science), Wired (artículos firmados).

## L5 — Fuente terciaria

Definición: blog técnico, video educativo, podcast, charla TED.

**Advertencia**: L5 nunca como soporte único de claims `critical`.

## L6 — Anécdota / experiencia personal

Definición: experiencia vivida por un autor sin paper que la respalde.

## L7 — Sin fuente

Definición: sospecha, intuición, "creo que...".

**Nunca aceptable** como soporte único. Bloqueador por defecto.

---

## Hard floors por dominio y skill

| Skill | claim_type más común | Floor |
|-------|---------------------|-------|
| `deep-feedback-loops-analyzer` | `feedback-behavior` | L1 |
| `deep-leverage-points-analyst` | `leverage-rank` | L1 (Meadows) |
| `deep-system-archetypes-mapper` | `archetype-structure` | L1 (Senge/Kim) |
| `deep-traps-detector` | `concept-meadows` | L1 |
| `deep-software-research` (API/version) | `api-existence`, `version` | L1 (oficial) |
| `deep-pattern-extractor` | `architectural-pattern` | L1 (paper) o L2 (doc) |
| `deep-domain-modeler` | `theory` | L1 |
| `deep-historical-lineage-tracer` | `event-date`, `primary-source-quote` | L1 |
| `deep-knowledge-graph-builder` | `relation-claim` | L1 o L2 |

## Cómo clasificar

1. Identifica el `claim_type`.
2. Consulta el floor del dominio.
3. Marca `risk` según impacto si la afirmación está mal.
4. Si la fuente no cumple el floor, escala a L5+ con disclaimer explícito.
