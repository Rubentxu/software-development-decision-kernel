# Niveles de evidencia — Universales (L1-L7)

Calibración universal aplicable a cualquier dominio. Las skills de dominio pueden extender o reinterpretar floors.

## L1 — Fuente primaria

**Definición**: documento original del autor canónico, paper revisado por pares, código fuente verificado, fuente primaria histórica.

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
| Filosofía | Texto del filósofo original |

## L2 — Fuente oficial autoritativa

**Definición**: documento institucional, white paper, peer-reviewed que cita L1, datos oficiales con metodología.

**Ejemplos**: docs oficiales de framework, white paper institucional, peer-reviewed secundario, IPCC reports, FDA approvals, Academy for Systems Change, Club of Rome.

## L3 — Fuente secundaria revisada

**Definición**: enciclopedia académica (Wikipedia, Stanford Encyclopedia), paper que cita L1 sin ser del autor original, tesis doctoral.

**Advertencia**: Wikipedia es L3, no L1. Usar como navegador a fuentes primarias.

## L4 — Fuente periodística especializada

**Definición**: revista técnica, periodista con expertise.

**Ejemplos**: MIT Technology Review, The Economist (sección tech/science), Wired (firmados).

## L5 — Fuente terciaria

**Definición**: blog técnico, video educativo, podcast, charla TED.

**Advertencia**: L5 nunca como soporte único de claims `critical`.

## L6 — Anécdota / experiencia personal

**Definición**: experiencia vivida por un autor sin paper que la respalde.

## L7 — Sin fuente

**Definición**: sospecha, intuición, "creo que...".

**Nunca aceptable** como soporte único. Bloqueador por defecto.

---

## Hard floors por dominio

| Dominio | Floor típico para `critical` |
|---------|------------------------------|
| Tecnología / Software | L1 (código, release notes, RFC) |
| AI / ML | L1 (paper con código) |
| Systems Thinking | L1 (Meadows/Forrester/Senge originales) |
| Ciencia | L1 (peer-reviewed) |
| Medicina | L1 (ClinicalTrials.gov, paper) |
| Economía | L2 (datos oficiales) |
| Historia | L1 (archivo primario) |
| Filosofía | L1 (texto del filósofo) |

---

## Cómo clasificar

1. Identifica el `claim_type` (ver `references/claim-types.md` en strategist).
2. Consulta el floor del dominio.
3. Marca `risk` según impacto si la afirmación está mal.
4. Si la fuente no cumple el floor, escala a L5+ con disclaimer explícito.

---

## Tabla rápida de decisión

| claim_type | Mínimo aceptable | Recomendado | Notas |
|------------|-------------------|-------------|-------|
| API/version (tech) | L1 | L1 | Código fuente |
| Comportamiento (tech) | L1 | L1 | Código fuente o test reproducible |
| Performance | L1-exp | L1-exp | Benchmark con datos y método |
| Concepto de Meadows | L1 | L1 | Texto primario |
| Arquetipo de Senge | L1 | L1 | Texto primario |
| Hallazgo científico | L1 | L1 | Peer-reviewed |
| Resultado clínico | L1 | L1 | ClinicalTrials.gov |
| Dato económico | L2 | L1 | INE, BLS, Banco Mundial |
| Evento histórico | L1 | L1 | Archivo primario |
| Opinión general | L5 | L5 | Nunca blocker |
