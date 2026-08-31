# Fuentes para detección de drift

Dónde consultar la versión *real* actual de cada tecnología.

## Rust
- MSRV: `rust-lang/rust` RELEASES.md.
- Crates: `https://crates.io/api/v1/crates/{name}` (campo `max_version`).
- Docs: `https://docs.rs/{name}` (versión publicada más reciente).

## Bevy
- Releases: `bevyengine.org/news` y `bevyengine/bevy` GitHub releases.
- Breaking changes por versión: `bevyengine/bevy` migration guides.
- Crates del ecosistema: docs.rs (no asumir numeración de Bevy).

## General
- CHANGELOG.md del repo oficial.
- GitHub releases (con notas de breaking changes).
- Para estándares: el RFC/spec correspondiente.

## Cadencia recomendada
- Tras cada release mayor del framework principal → ejecutar detector.
- Mensual para crates del ecosistema.
- Antes de cualquier reedición del libro.

## Anti-patrón
Confiar en la versión que "recuerda" el modelo. Siempre consultar la fuente viva y registrar `retrieved_at`.
