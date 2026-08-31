# Estrategias de include por formato

Principio universal: **el código mostrado = el código probado**. Nunca copiar a mano.

## AsciiDoc
```asciidoc
include::../../chapters/chapter-12-scenes/src/lib.rs[tag=village-hierarchy]
```
Regiones en el `.rs`:
```rust
// tag::village-hierarchy
let village = world.spawn(...);
// end::village-hierarchy
```

## Hugo / Markdown (blog)
Hugo con `unsafe = true` permite HTML crudo, pero para includes de archivo usa shortcodes:
```md
{{< code-include file="chapters/chapter-12-scenes/src/lib.rs" tag="village-hierarchy" lang="rust" >}}
```
Define el shortcode `layouts/shortcodes/code-include.html` que lee el archivo del repo y extrae el tag. Así el libro y el repo están acoplados sin copiar.

## HTML estático (caso actual del libro Bevy)
Si el libro es HTML generado, un script de build extrae las regiones `tag::`/`end::` del repo y las inserta en el HTML. Esto es lo que ya hace el libro: el código del HTML debe venir del repo, no escrito a mano.

## Reglas comunes
- Un `tag::` estable: si renombras una región, actualiza el mapa y todos los includes.
- Regiones pequeñas y enfocadas (1 concepto = 1 región).
- Nunca incluir un archivo entero si solo importa una función: usar tags.
- Si una región crece mucho, dividirla en sub-regiones.

## Anti-patrón (lo que falló en el libro Bevy)
Pegar el código a mano en el HTML con sintaxis inventada. El `code-prose-coherence-checker` debe detectarlo: si el snippet del HTML no aparece como región del repo, es copia manual (o fabricación).
