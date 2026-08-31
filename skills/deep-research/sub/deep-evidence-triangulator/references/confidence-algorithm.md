# Algoritmo de confidence_score

`deep-evidence-triangulator` calcula `confidence_score` (0.0-1.0) por claim combinando:
- Número y calidad de fuentes independientes.
- Conflictos entre fuentes.
- Cascada de citas.
- Frescura.

## Algoritmo

```
score = 0
+ (0.3 * número_de_fuentes_L1_independientes, max 0.9)
+ (0.15 * número_de_fuentes_L2_independientes, max 0.45)
+ (0.05 * número_de_fuentes_L3_independientes, max 0.15)
- 0.2 si hay conflicto con fuente admitida
- 0.1 si todas las fuentes vienen del mismo grupo/autor
- 0.1 si la fuente primaria es de hace >5 años (tech) sin re-test
- 0.1 si claim_type es `performance` y no hay benchmark reproducible
clamp(score, 0.0, 1.0)
```

## Interpretación

| Score | Interpretación | Status |
|-------|----------------|--------|
| 0.0-0.5 | Insuficiente | `unverified` |
| 0.5-0.7 | Marginal | `verified-with-disclaimer` (requiere disclaimer explícito) |
| 0.7-0.85 | Media | `verified-with-disclaimer` |
| 0.85-0.95 | Alta | `verified` |
| 0.95-1.0 | Muy alta | `verified` |

## Independencia

Dos fuentes son **independientes** si:
- Autores diferentes (no co-autores).
- Instituciones diferentes.
- Sin citación cruzada directa (una no cita a la otra como fuente de la afirmación).

## Cascada de citas

Si N fuentes citan todas a la misma fuente primaria, contar como 1.

**Ejemplo**:
- Paper A dice "X".
- Paper B dice "X" y cita A.
- Paper C dice "X" y cita B.
- Paper D dice "X" y cita A.
- Fuentes independientes: 1 (solo A cuenta).
- Score: 0.3, no 1.2.

## Detección de conflictos

Si una fuente admitida contradice la claim:
- Marcar `conflict: true`.
- Restar 0.2 del score.
- Asignar `status: disputed` (no `verified`).
- Documentar AMBAS posiciones en el campo `conflict_summary`.

## Frescura

Para tech (cualquier framework, librería, API):
- Fuente primaria > 5 años → restar 0.1 si no hay re-test posterior.
- Sin re-test en ≥ 2 años → marcar `needs_recheck`.

## Ejemplos

### Caso 1: Claim bien respaldada

- Fuentes: 2 L1 independientes (paper original + paper de replication).
- Sin conflictos.
- Score: 0.3 + 0.3 = 0.6 → 0.7-0.85 con L2 adicional.

### Caso 2: Claim con conflicto

- Fuentes: 1 L1 a favor + 1 L1 en contra.
- Conflicto detectado.
- Score: 0.3 - 0.2 = 0.1 → `disputed`.

### Caso 3: Claim con cascada

- Fuentes: 3 blogs que citan al mismo paper.
- Cascada: 1 fuente real.
- Score: 0.05 (L3, una sola real) → `unverified`.

### Caso 4: Claim con fuente única creíble

- Fuentes: 1 L1 (paper original del autor del concepto).
- Score: 0.3 → `verified-with-disclaimer` (1 fuente única).

## Anti-patrones

- ❌ Asignar score alto por "consenso" sin verificar independencia.
- ❌ Ignorar conflictos cuando las fuentes "mayoritariamente" están de acuerdo.
- ❌ Aplicar mismo score a tech (caduca) que a foundational (no caduca).
- ❌ Marcar `verified` sin `decay_date`.
