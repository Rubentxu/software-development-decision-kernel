# Ejemplos reales de divergencia prosa↔código

Casos extraídos del piloto y de la auditoría del libro Bevy. Sirven de patrón para `code-prose-coherence-checker`.

## Caso 1: MANUAL_COPY + DIVERGENCE (el fallo del Cap. 12)
- **Prosa del libro**: mostraba `bsn! { Name("Casa") : { House, Mailbox : {...} } }` (sintaxis estilo JSON).
- **Código real del repo** (`chapter-12-scenes/src/lib.rs`): usa `world.spawn((..., ChildOf(village)))`.
- **Detección**: el snippet del HTML no aparece como región del repo (`MANUAL_COPY`) Y describe una sintaxis que el código no usa (`DIVERGENCE`).
- **Causa raíz**: el libro se escribió pegando código fabricado en lugar de incluir regiones del repo.

## Caso 2: BROKEN_INCLUDE
- El libro hace `include::chapters/chapter-10/src/lib.rs[tag=on_trigger]`.
- Al refactorizar el repo, la región se renombró a `trigger_handler`.
- **Detección**: el `tag::on_trigger` ya no existe → `BROKEN_INCLUDE`.
- **Fix**: actualizar el include o el code-map.

## Caso 3: CONTENT_DRIFT
- El libro dice "este sistema suma 1 al contador cada frame".
- El código de la región hizo `counter += dt.delta_seconds() * rate` (cambió a continuo).
- **Detección**: la región existe, pero la prosa ya no describe el comportamiento → `CONTENT_DRIFT`.
- **Fix**: re-revisar el capítulo (`release-maintainer`).

## Caso 4: ORPHAN_REGION_IN_BOOK
- El repo tiene una región `tag::debug_overlay` bien testeada.
- El libro la incluye pero la rodea de media frase sin explicarla.
- **Detección**: región referenciada sin prosa que la explique → código mudo.

## Regla de decisión
Ante la duda entre "la prosa es imprecisa" o "el código cambió": comparar contra la evidence card correspondiente. Si hay card verificada, la prosa debe alignarse a la card; el código a la card y al repo.
