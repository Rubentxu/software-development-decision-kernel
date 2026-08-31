# Heurísticas de revisión pedagógica

## Salto conceptual (missing_prerequisite)
- Un término técnico aparece sin definición previa ni enlaces a ella.
- Se asume conocimiento marcado `not_in_scope` en el perfil del lector.
- Se usa un concepto del grafo que se introduce en un capítulo **posterior**.

## Explicación circular (circular_definition)
- A se define en función de B y B en función de A.
- Un glosario define un término con el propio término o un sinónimo cercano.

## Objetivo no cubierto (uncovered_objective)
- El contrato lista "Diseñar sistemas ejecutables en paralelo" pero el capítulo solo los describe, no guía al lector a diseñarlos.

## Sobrecarga cognitiva (cognitive_overload)
- Una sección introduce más de 3 conceptos `core` nuevos sin ejercicio intermedio.
- Aparecen muchos términos en inglés no traducidos y no introducidos de golpe.

## Concepto huérfano (orphan_concept)
- Se introduce una idea que luego no se vuelve a usar en el capítulo ni en posteriores.

## Regla de severidad
- `high`: impide entender el resto del capítulo (salto sobre concepto core).
- `med`: ralentiza pero no bloquea.
- `low`: pulido (huérfanos leves).
