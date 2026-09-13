# Bounded-context cutover plan

## Paso 1 — Module skeletons
Crear módulos vacíos y sus `mod.rs`/`lib.rs` exports.

## Paso 2 — Characterization
Capturar compile tests, CLI golden, event schemas y critical UAT.

## Paso 3 — Moves
Mover familias completas. Mantener legacy reexports:

```rust
#[deprecated(note = "use crate::knowledge::...")]
pub use crate::knowledge::...;
```

cuando consumidores externos lo necesiten.

## Paso 4 — Import ratchets
Prohibir nuevas imports a paths legacy.

## Paso 5 — Remove shims
Sólo después de que repo search + installed UAT no muestren consumo productivo.

## Paso 6 — Implement Alignment
Ningún nuevo fichero Alignment se crea en `engine/` root; nace directamente bajo `alignment/`.
