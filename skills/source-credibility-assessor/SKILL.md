---
name: source-credibility-assessor
description: "Trigger: evaluar credibilidad de fuente, ranking de autoridad, sesgo de fuente, conflicto de intereses, frescura de fuente, link rot, link muerto, calidad de libro técnico, calidad de blog. Evalúa la credibilidad de cada fuente candidata: nivel de autoridad, sesgo, conflicto de intereses, frescura y link rot. Especializado en distinguir un libro/blog fiable de uno mediocre."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo **después** de `source-discovery-specialist` (que llenó el candidate-pool) y **antes** de `evidence-cross-validator` (que triangula solo con fuentes creíbles). Filtra el ruido y deja lo fiable.

No lo uses para descubrir (`source-discovery-specialist`), ni para validar que una URL resuelve (`reference-validator`), ni para triangular (`evidence-cross-validator`).

## Hard Rules

- Toda fuente recibe un `credibility_level` (L1–L7) **confirmado**, no estimado.
- El sesgo y conflicto de intereses se declaran explícitamente (no se asume neutralidad).
- La frescura se mide contra el `retrieved_at` y la velocidad de cambio del tema.
- Link rot / paywall / archivado se marca: una fuente inaccesible no es fuente.
- Literatura secundaria (libros/blogs) se puntúa con criterios específicos (ver tabla).

## Dimensiones de evaluación

| Dimensión | Qué mide |
|-----------|----------|
| `authority` | nivel L1–L7 confirmado |
| `bias` | comercial, ideológico, autopromocional |
| `conflict_of_interest` | el autor se beneficia de la afirmación |
| `freshness` | antigüedad vs velocidad de cambio del tema |
| `accessibility` | link vivo, paywall, archivado, rot |
| `verifiability` | ¿se puede comprobar la afirmación independientemente? |

## Criterios para libros/manuales (L6)

| Señal positiva | Señal negativa |
|----------------|----------------|
| Editorial reputada (O'Reilly, Manning, No Starch, oficial) | Self-published sin revisión |
| Autor verificable, contributor del proyecto | Autor sin trayectoria |
| Edición reciente (≤3 años para tema rápido) | Edición obsoleta |
| Citado por la comunidad y la doc oficial | Nadie lo referencia |
| Errata pública y correcciones | Sin mecanismo de errata |
| Revisión por pares editorial | — |

## Criterios para blogs/posts (L7)

| Señal positiva | Señal negativa |
|----------------|----------------|
| Autor contributor del proyecto o experto reconocido | Autor anónimo |
| Fecha clara y reciente | Sin fecha |
| Cita fuentes primarias | Afirmaciones sin respaldo |
| Comentado/discutido por la comunidad | Aislado, sin engagement |
| Coincide con docs oficiales (no copia, corrobora) | Copia sin atribución |

## Execution Steps

1. Leer `research/candidate-pool.yml`.
2. Para cada candidata, evaluar las 6 dimensiones.
3. Confirmar `credibility_level` (puede subir o bajar del `level_estimate`).
4. Detectar sesgo/COI: si existe, penalty en la confianza final.
5. Comprobar accesibilidad (delega la verificación viva de URL/DOI a `reference-validator`).
6. Escribir `research/credibility/{source-id}.yml` (esquema en `assets/credibility.schema.yml`).
7. Marcar candidatas `rejected` (ruido) vs `admitted` para triangulación.

## Esquema de evaluación (resumen)

```yaml
credibility:
  source_id: blog-tutorial-bsn
  credibility_level: L7
  authority: 0.3
  bias: none                  # none|commercial|ideological|self-promo
  conflict_of_interest: none
  freshness: 0.9              # reciente
  accessibility: live         # live|paywall|archived|rot
  verifiability: medium       # alta si cita primarias
  verdict: admitted           # admitted|rejected
  notes: "Útil como contexto; corroboración, no fuente única."
```

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Link roto y sin archivar | `rejected` (o `reference-validator` busca archivo) |
| Sesgo comercial no declarado | Penalty; si domina, `rejected` |
| Libro L6 obsoleto para tema rápido | Bajar nivel o `rejected` |
| Blog L7 que coincide con oficial | `admitted` como corroboración, no como única |

## Output Contract

- `research/credibility/{source-id}.yml` por candidata.
- `research/credibility-summary.yml` con conteo admitidos/rechazados por nivel.
- `evidence-cross-validator` solo triangula con `admitted`.

## References

- `references/bias-signals.md` — señales de sesgo y COI.
- `assets/credibility.schema.yml` — esquema de la evaluación.
