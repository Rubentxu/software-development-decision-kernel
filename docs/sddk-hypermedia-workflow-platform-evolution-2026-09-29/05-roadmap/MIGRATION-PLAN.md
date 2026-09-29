# Plan de migración strangler

## M0 — Baseline y ratchets

Antes de cambios:

- capturar exact HEAD/version/tests;
- inventariar consumers de WorkflowManifest/prompts/cycle resume;
- añadir fitness tests para no introducir provider types en domain;
- pin de comportamiento 0/1/N active cycles.

## M1 — Recovery first

C3i/C3j. No tocar control-flow general.

Deliverables:
- bootstrap application service;
- adoption ensure semantics;
- persistent ContextCapsule/Binding;
- hypermedia read-only representation.

Rollback: legacy commands siguen disponibles.

## M2 — New definitions alongside old

Añadir ResourceRef/Affordance/StepDefinition/WorkflowDefinition como experimental internal schemas. No side effects diferentes.

## M3 — Compile shadow

Por cada default workflow seleccionado:

```text
legacy manifest → legacy normalized graph
new adapter     → WorkflowDefinition → IR
compare semantic contract
```

No ejecutar ambos.

Comparar:
- ordering/dependencies;
- path applicability;
- gates;
- artifacts/outputs;
- failure transitions;
- authority requirements.

## M4 — First runtime cutover

Elegir un vertical slice read-mostly o bounded. WorkflowRuntime ejecuta el slice. Legacy path queda feature/compat fallback explícito durante una ventana.

## M5 — Contribution reconciliation

Retirar del prompt del slice:
- evidence attach recipes;
- decision record recipes;
- transition bookkeeping que ya posea runtime;
- handoff summaries como authority.

Agente devuelve typed contributions.

## M6 — Provider augmentation

Poner CapabilityResolver delante de adapters existentes. Primero same provider behavior, después policy de automatic augmentation. No reescribir adapters simultáneamente.

## M7 — Hypermedia mutation navigation

Exponer legal actions. Migrar harness para seguir affordances. Mantener direct commands como compatibility aliases.

## M8 — User workflow surface

Abrir project/user definitions sólo después de:
- validator estable;
- resource/action security UAT;
- step registry precedence UAT;
- output contract enforcement.

## M9 — Software pack strangler

Mover semantics, no ficheros por estética. Cada migration slice demuestra que core no importa software-specific concept.

## M10 — Second domain

Book pack. Fallos se clasifican:

```text
pack bug
missing generic primitive
wrong generic abstraction
software leakage
provider gap
```

Sólo `missing generic primitive` justifica ampliar core.

## Deprecation policy

Una legacy surface sólo se marca removable cuando:

1. no current default workflow la usa;
2. no plugin/skill first-party la usa;
3. compatibility telemetry/tests muestran sustituto;
4. migration note existe;
5. release notes anuncian deprecation;
6. al menos una release compatible ha pasado, salvo security defect.
