---
name: exercise-designer
description: "Trigger: crear ejercicios, diseñar laboratorios, ejercicios del libro, pistas, soluciones, criterios de evaluación, práctica, kata. Crea ejercicios, laboratorios, pistas, soluciones y criterios de evaluación alineados con los objetivos de aprendizaje del contrato del capítulo."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo cuando `chapter-planner` lista `exercises` en el contrato, o al final de un capítulo para consolidar conceptos. Los ejercicios deben ser **verificables** (tienen solución y criterios).

No la uses para generar ejemplos principales del capítulo (`code-example-generator`).

## Hard Rules

- Cada ejercicio se vincula a ≥1 `learning_objective` del contrato.
- Todo ejercicio tiene **solución** (en `exercises/{id}/solution/`) y **criterios de evaluación**.
- Las **pistas** se gradúan (de menos a más reveladoras) y nunca dan la solución completa.
- Las soluciones deben compilar/pasar `code-example-verifier` como cualquier otro ejemplo.
- Un ejercicio introduce **pocos** conceptos nuevos (alineado con `example-complexity-controller`).

## Execution Steps

1. Leer el contrato del capítulo (objetivos y conceptos).
2. Diseñar 3-5 ejercicios graduados por dificultad:
   - **Recordar/aplicar** — uso directo del concepto.
   - **Analizar** — diagnosticar un fragmento dado.
   - **Crear** — construir algo pequeño que combine conceptos.
3. Para cada ejercicio:
   - Enunciado claro y autocontenido.
   - Pistas graduadas (3 niveles máximo).
   - Solución completa y verificable (proyecto bajo `exercises/{id}/solution/`).
   - Criterios de evaluación explícitos (qué se evalúa, qué no).
4. Verificar que las soluciones compilan (delegar a `code-example-verifier`).
5. Registrar en `exercises/index.yml`.

## Esquema de ejercicio

```yaml
exercise:
  id: scheduling-analysis
  chapter: ch05-ecs-scheduling
  objective: Diagnosticar una planificación secuencial accidental
  difficulty: analyze
  hints:
    - "Mira qué recursos muta cada sistema."
    - "Dos sistemas que mutan el mismo recurso no pueden ir en paralelo."
    - "Revisa el orden de add_systems y los conjuntos."
  solution: exercises/scheduling-analysis/solution/
  criteria:
    - "Identifica el conflicto de acceso a Resource X."
    - "Propone reordenar o separar en conjuntos."
```

## Output Contract

- `exercises/{id}/` con enunciado, hints, `solution/` (proyecto verificable).
- `exercises/index.yml` actualizado.
- Resultado de `code-example-verifier` sobre las soluciones (debe ser verde).

## References

- `references/exercise-taxonomy.md` — taxonomía de dificultad (recordar/aplicar/analizar/crear).
