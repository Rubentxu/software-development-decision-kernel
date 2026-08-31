# Protocolo de resolución de conflictos

Cuándo las fuentes se contradicen, `evidence-cross-validator` resuelve con este protocolo (siempre documentado).

## Jerarquía de desempate (de mayor a menor prioridad)

1. **Evidencia reproducible (L1-exp)**. Un benchmark, un test, un manifest de crate que se puede ejecutar/verificar. Gana sobre todo.
2. **Especificación/estándar (L1)**. La definición formal.
3. **Documentación oficial vigente (L2)** con `retrieved_at` reciente.
4. **Código fuente (L3)** de la versión concreta de la que se afirma algo.
5. **Paper revisado por pares (L4)**.
6. **Post del mantenedor (L5)** con fecha.
7. **Libro/manual canónico (L6)**.
8. **Comunidad (L7)**.

## Reglas

- Una fuente menor **solo** vence a una mayor si presenta **evidencia reproducible** (L1-exp) que la refuta.
  - Ejemplo: crates.io (manifest ejecutable) vence al README del repo si discrepan.
- Un conflicto entre dos fuentes del **mismo nivel** se marca `disputed` si ninguna tiene evidencia reproducible; no se publica sin disclaimer.
- La **versión** importa: una doc de Bevy 0.17 no refuta una de 0.19. Comparar siempre mismo target.
- La **frescura** importa: a igual nivel, la más reciente (para temas que evolucionan).

## Documentación obligatoria de la resolución

```yaml
resolution:
  winning_source: crates-io-avian
  winning_level: L2
  winning_reason: "manifest del crate es evidencia reproducible; README desactualizado"
  losing_source: avian-readme
  losing_level: L3
  alternative_considered: "NINGUNA; el manifest es verificable"
  resolved_at: "2026-07-23"
```

## Cuándo NO resolver (escalar)
- Conflicto entre dos fuentes L2 oficiales que dicen cosas distintas (ej. doc vs release notes): escalar al autor, puede ser un bug de la doc.
- Conflicto donde la menor tiene L1-exp pero la mayor es la spec: requiere juicio experto humano.

## Regla de honradez
Si la resolución no es clara, `status: disputed` y bloqueante. Es preferible no publicar que publicar algo falso.
