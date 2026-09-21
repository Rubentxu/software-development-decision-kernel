# PRD — SDDK Companion / Human-Agent Collaboration

## 1. Problema

A medida que SDDK gana agentes, skills, gates, lenses, telemetría y workflows, aumenta la distancia entre la riqueza del runtime y la comprensión del humano. Un usuario puede recibir un resultado técnicamente correcto y aun así perder el contexto: qué fase está activa, qué cambió, por qué cambió, qué riesgo existe y si debe intervenir.

La solución no es hacer prompts más largos. La solución es formalizar la interacción humano-agente como un plano arquitectónico.

## 2. Visión

SDDK debe comportarse como un colaborador durable:

- siempre orienta;
- explica sólo lo importante;
- diferencia hechos, decisiones, hipótesis y cambios de rumbo;
- pregunta únicamente cuando la autoridad humana aporta valor;
- adapta tono y profundidad sin alterar la semántica;
- puede reanudar una sesión sin depender del historial de chat;
- aprende preferencias de interacción de forma explícita, editable y reversible.

## 3. Objetivos

### O1 — Never Lost
Tras cualquier transición relevante, el usuario puede identificar fase, objetivo, último resultado, siguiente paso y necesidad de intervención.

### O2 — Explainable Evolution
Todo cambio de dirección material se representa como `Reframe` con evidencia, impacto y necesidad de aprobación.

### O3 — Low-Friction HITL
Informar no implica bloquear. Las aprobaciones se reservan para decisiones de riesgo, autoridad humana o irreversibilidad.

### O4 — Personalización segura
Audience, personality y autonomy son ejes independientes.

### O5 — Durable Collaboration
Resume se reconstruye desde estado y artifacts autoritativos, no desde memoria libre de chat.

### O6 — Learnable UX
La fricción humana produce telemetría útil para F3 sin auto-modificar políticas de seguridad.

## 4. Defaults del producto

```yaml
interaction:
  audience: novice
  autonomy: balanced
  personality:
    preset: wisecracking_robot
    sarcasm: 0.55
    dry_humor: 0.70
    warmth: 0.55
    directness: 0.85
```

El preset por defecto es didáctico, directo, sarcástico y con humor seco. El humor se suprime automáticamente ante seguridad, pérdida de datos, operaciones destructivas, bloqueos repetidos o frustración explícita.

## 5. Requisitos funcionales

- RF-HX-001 — `CurrentRunView` determinista.
- RF-HX-002 — `InteractionEvent` común.
- RF-HX-003 — `StageReport` por eventos relevantes.
- RF-HX-004 — breadcrumb `cycle/path/phase/progress/attention`.
- RF-HX-005 — Resume Summary.
- RF-HX-006 — Decision/Reframe/Assumption records.
- RF-HX-007 — `DecisionRequired` con opciones, recomendación y default.
- RF-HX-008 — risk-based interruption policy.
- RF-HX-009 — audience renderers.
- RF-HX-010 — personality renderer.
- RF-HX-011 — autonomy profiles.
- RF-HX-012 — preference memory con confidence/promotion.
- RF-HX-013 — memoria inspeccionable/editable/forget.
- RF-HX-014 — Attention Router.
- RF-HX-015 — narration/cognitive budget.
- RF-HX-016 — `/status|where|why|plan|decisions|risks|artifacts|memory` semánticos.
- RF-HX-017 — friction telemetry.
- RF-HX-018 — semantic parity entre renderers.
- RF-HX-019 — locale preservada.
- RF-HX-020 — integración con UAT Human Review Queue.
- RF-HX-021 — facade/goal surface preserva behavioral parity.
- RF-HX-022 — low-level CLI sigue accesible.
- RF-HX-023 — cero intrusión.

## 6. Requisitos no funcionales

- RNF-HX-001 — 100% de hechos críticos invariantes entre personalidades.
- RNF-HX-002 — 0 nuevas llamadas lifecycle desde renderers.
- RNF-HX-003 — rendering local P95 < 50 ms para un evento típico.
- RNF-HX-004 — summary de resume <= 150 palabras por defecto.
- RNF-HX-005 — perfil de usuario exportable y eliminable.
- RNF-HX-006 — schemas versionados.
- RNF-HX-007 — compatibilidad backward con artifacts actuales.
- RNF-HX-008 — failure closed si el estado autoritativo no puede reconstruirse.
- RNF-HX-009 — no dependencia obligatoria de MCP/servicio externo.
- RNF-HX-010 — same input facts => deterministic neutral rendering.

## 7. No objetivos

- No crear un nuevo orchestrator.
- No convertir personality en policy.
- No guardar chain-of-thought.
- No convertir Engram en autoridad runtime.
- No crear una segunda base de datos de telemetría.
- No añadir prompts de personalidad a cada agente.
- No hacer que UAT y Companion tengan modelos humanos divergentes.
- No retirar comandos low-level.

## 8. Métricas de éxito

- >= 90% UAT responde correctamente “dónde estamos / qué pasó / qué sigue / necesito intervenir”.
- `where_am_i_queries_per_cycle < 0.2` tras estabilización.
- `unnecessary_approval_requests <= 1/cycle`.
- 100% reframes materiales reportados.
- 100% semantic parity en golden tests de persona/audience.
- 100% high-risk human decisions con receipt.
- >= 95% usuarios pueden reanudar un ciclo con sólo Resume Summary.
