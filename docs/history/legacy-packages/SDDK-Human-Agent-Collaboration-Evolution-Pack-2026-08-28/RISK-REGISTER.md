# Risk Register

| ID | Riesgo | Prob. | Impacto | Mitigación |
|---|---|---:|---:|---|
| R-HX-01 | Duplicar autoridad runtime | M | Crítico | CurrentRunView sólo proyección; ADR-HX-001 |
| R-HX-02 | Prompts más grandes | H | Alto | contract central + structured delta |
| R-HX-03 | Humor oculta blocker | M | Alto | semantic invariance + safety tone filter |
| R-HX-04 | Memoria aprende falso patrón | M | Alto | candidate/confidence/pin/edit/forget |
| R-HX-05 | Exceso de reporting | H | Medio | Attention Router + narration budget |
| R-HX-06 | HITL bloquea demasiado | H | Alto | balanced risk policy + métrica approvals |
| R-HX-07 | UAT crea protocolo humano paralelo | M | Alto | HumanDecisionPort común |
| R-HX-08 | Nuevo event store compite con ledger | M | Alto | derive-first; persist minimal human-only data |
| R-HX-09 | Facade oculta evidencia | M | Crítico | behavioral parity; semantic compression rule |
| R-HX-10 | Renderer LLM inventa hechos | M | Crítico | facts envelope + deterministic semantic checks |
| R-HX-11 | Datos personales demasiado persistentes | M | Alto | local-first, inspect/export/forget, minimal storage |
| R-HX-12 | Nuevo crate prematuro | M | Medio | architecture emergence trigger |
