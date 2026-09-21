# Lens system — paradigm agnostic by construction

## Capas

```text
Universal Concern
      +
Selected Lens
      +
Project Intent
      +
Evidence
      -> AlignmentAssessment
```

## LensDefinition

```text
id + version
applies_when
concerns[]
required_evidence[]
optional_evidence[]
evaluator_contract
workbook_contributions[]
explanation_template
```

## Reglas

- lens puede devolver `NOT_APPLICABLE`;
- autodetección recomienda, no activa reglas bloqueantes;
- varias lenses pueden evaluar el mismo concern y discrepar;
- la discrepancia se preserva, no se fuerza un score medio;
- cada lens debe declarar qué evidencia necesita;
- la ausencia de evidencia produce UNKNOWN, no MISALIGNED.

## Primer set

1. `core.universal`
2. `architecture.hexagonal`
3. `ddd.bounded-context`
4. `oo.solid-grasp`
5. `functional.core`
6. `types.adt-state-modeling`
7. `quality.smells`
8. `design.connascence`

Otros paradigmas entran como packs cuando haya consumidores reales.
