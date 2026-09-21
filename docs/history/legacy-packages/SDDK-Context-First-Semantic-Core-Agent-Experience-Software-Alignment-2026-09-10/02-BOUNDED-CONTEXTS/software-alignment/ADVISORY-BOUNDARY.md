# Advisory boundary — contrato fuerte

## Separación Agent Experience

```text
PromptAssembly
  NormativeInstructions
    kernel rules
    project instructions
    policy instructions
    task contract
    skill procedural fragments

  AdvisoryContext
    alignment summaries
    tensions
    opportunities
    workbooks
    tradeoff reminders
```

**Prohibido:** convertir automáticamente `AlignmentAssessment`, `Tension` u `Opportunity` en `NormativeInstructions`.

## Qué sí puede ser imperativo

Sólo elementos cuya autoridad viene de otro contexto:

- TaskContract explícito;
- Policy/Authority requirement;
- Project instruction normativa;
- Invariant/contract `MUST` declarado.

Alignment puede presentar la contradicción, pero no crear el MUST.

## UAT obligatorio

Serializar `PromptAssembly` y demostrar que una suggestion de Alignment aparece bajo `advisory_context` y no altera `effective_instruction_set_hash`.

Sí debe afectar `context_capsule_hash`, porque el agente recibió esa información.
