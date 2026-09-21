# Arquitectura — Human Interaction Plane

## Principio

**El kernel produce verdad; Companion produce comprensión.**

```text
Human
  │
  ▼
Presentation / Companion
  ├─ Audience Renderer
  ├─ Personality Renderer
  ├─ Attention Router
  └─ Decision UX
  │
  ▼
Human Interaction Application
  ├─ BuildCurrentRunView
  ├─ ProjectInteractionEvent
  ├─ RenderStageReport
  ├─ RequestHumanDecision
  ├─ BuildResumeSummary
  └─ LearnPreference
  │
  ▼
Domain contracts
  ├─ InteractionEvent
  ├─ CurrentRunView
  ├─ StageReport
  ├─ DecisionRecord
  ├─ Reframe
  ├─ AssumptionRecord
  └─ InteractionProfile
  │
  ▼
Existing SDDK authorities
  ├─ CLI / Ledger       runtime truth
  ├─ CAS / artifacts    evidence truth
  ├─ Git                code truth
  └─ Vault              durable project knowledge
```

## Hexagonal boundaries

### Domain
Tipos puros, reglas de invariancia, clasificación de atención, risk tier, preference promotion.

### Application
Casos de uso que componen información autoritativa en una vista humana.

### Driven ports
- RuntimeStateReader
- ArtifactReader
- DecisionRecordStore
- PreferenceStore
- TelemetrySink
- Clock
- LocaleResolver

### Driving adapters
- CLI
- OpenCode/ZCode/Claude/Codex integration
- Markdown/chat rendering
- HTML dashboard
- future TUI/web

## Ubicación inicial recomendada

No crear un crate nuevo en HX0.

Empezar con módulos:

```text
sddk-domain/src/interaction/
sddk-engine/src/interaction/
sddk-cli/src/commands/interaction/
```

Crear `sddk-presentation` sólo si HX2 demuestra dependencia/ciclo o necesidad real de reutilización multi-adapter. Esta es una decisión de arquitectura emergente, no una obligación upfront.

## Source of Truth Matrix

| Concern | Authority | Projection |
|---|---|---|
| cycle/phase/status/lease | CLI + ledger | CurrentRunView |
| artifacts/evidence | CAS/XDG artifacts | StageReport |
| code state | Git | CurrentRunView |
| requirements/ADR/domain knowledge | vault | Explain/Why |
| user preferences | InteractionProfile store | renderer config |
| chat history | contextual only | never authority |

## Flujo de evento

```text
phase coordinator completes work
        │
        ├─ persists authoritative artifact
        ├─ evaluates gate / transition
        └─ emits or derives InteractionEvent
                     │
                     ▼
             Attention Router
             /       |       \
          silent  report   decision
                     │
                     ▼
                Renderer
                     │
                     ▼
                    Human
```

`InteractionEvent` no debe convertirse en un event store competidor del ledger. Cuando sea posible se deriva de ledger + artifact. Sólo eventos puramente humanos, como preference feedback, requieren persistencia propia.

## Modelo de decisión humana

`DecisionRequired` incluye:
- question;
- context;
- options;
- recommendation;
- default;
- risk;
- reversibility;
- deadline/timeout policy si existe;
- affected scope;
- evidence references.

La respuesta humana produce receipt/record, no texto ambiguo.

## Personalidad

Pipeline:

```text
facts -> audience transform -> personality transform -> safety tone filter -> output
```

Reglas:
1. Facts inmutables.
2. Personality no puede borrar riesgos, estados, acciones requeridas ni blockers.
3. Humor sólo afecta wording.
4. Safety tone filter tiene precedencia.

## Memoria

Namespaces separados:
- operational: ciclo/progreso;
- project: ADR/REQ/knowledge;
- user-interaction: preferencias.

Un dato de interacción pasa por:
`observation -> candidate -> repeated evidence -> learned -> pinned(optional)`.

## Integración con goal/facade CLI

La facade ya simplifica lifecycle. Companion debe:
- consumir `sddk status` como entrada preferente;
- usar `plan/run/ship/recover` donde corresponda;
- no replicar secuencias low-level en prompts;
- conservar low-level CLI para contract tests, debug y recovery;
- añadir comandos humanos sólo si no son alias de lifecycle.
