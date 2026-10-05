# cl-version-inspection — PRE-FLIGHT

**Cycle:** `p-63676b11dc0ef88f/version-coherence-068` (VA7)
**Date:** 2026-10-05T18:42:00Z
**Status:** READY
**Workspace:** 2.11.2 (declarada; último tag remoto v2.11.2)
**HEAD:** `312205c4` (`feat(release): un release puede tener varios responsables…`), árbol limpio
**Branch:** `fix/version-coherence-068`, 14 commits por delante de su base, sin aterrizar
**Authority:** SCOPE-CONTRACT en
`docs/roadmap/receipts/cl-version-inspection/SCOPE-CONTRACT.md`; ROADMAP.md §C4.1;
ADR-0157 / 0158 / 0159 / 0160 ya `accepted`

## Readiness: READY

| # | condición | estado |
|---|---|---|
| 1 | Objetivo falsable escrito | OK — SCOPE-CONTRACT, con no-objetivos y STOP |
| 2 | Superficie mapeada y **leída**, no inferida | OK — tabla de abajo, con línea |
| 3 | Riesgo de datos acotado | **cero**: la inspección lee lo que el registry ya leyó |
| 4 | No duplica autoridad | OK — se construye **después** de `reduce()`, nunca en vez de él |
| 5 | No altera forma JSON consumida | MEDIDO en VA3: `version_authority` no lo deserializa nadie |
| 6 | Tests que fijan la propiedad | declarados abajo, cada uno con su falsificador |

## Superficie (leída, con línea)

| qué | dónde | por qué importa |
|---|---|---|
| `PRODUCT_VERSION_OBSERVATION` | `version_authority.rs:552` | la capability que se pide; el informe la declara, no la inventa |
| `reduce()` | `version_authority.rs:444` | el veredicto. **El informe se construye después**, nunca aparte |
| `VersionResolverRegistry::resolve` | `version_authority.rs:656` | filtra por capability y recoge. Hoy **no dice a quién descartó**, y eso es justo lo que el informe necesita |
| `VersionResolverPort` | `version_authority.rs:591` | `provider_id` es procedencia y `observe` es la única puerta |
| `VersionAuthority` | `version_authority.rs:316` | seis veredictos, todos con `observations`: el informe no necesita inventar estado |
| `version_authority_text` | `release_cmd.rs:1563` | el render existente. Se **extiende**, no se duplica: plan y inspect salen de la misma función |
| `ReleaseCommand` | `release_cmd.rs:34` | donde entra `Inspect`, como `Subcommand` del mismo grupo |
| `default_version_registry` | `version_provider.rs:584` | la composición de providers; el informe no añade ninguno |

## Superficie mínima de este lote

1. `crates/sddk-domain/src/version_inspection.rs` — nuevo, puro, sin I/O.
2. `crates/sddk-domain/src/version_authority.rs` — **un** método más en el
   registry, `resolve_inspecting`, que devuelve el informe junto al veredicto.
   No se toca `resolve`: quien ya lo llama sigue igual.
3. `crates/sddk-cli/src/release_cmd.rs` — `Inspect` como subcomando y el render.
4. `crates/sddk-cli/tests/cli.rs` — aceptación de extremo a extremo.
5. `crates/sddk-gateway/tests/version_provider_conformance.rs` — los diez puntos
   del contrato, en la suite que ya existe para providers.

## Lote de verificación (scoped, no el perfil completo)

- `cargo test -p sddk-domain --test version_inspection_*`
- `cargo test -p sddk-cli --test cli` (el subcomando nuevo)
- `cargo test -p sddk-gateway --test version_provider_conformance`
- `cargo clippy -p sddk-domain -p sddk-cli --all-targets -- -D warnings`

El perfil completo (`cargo test --workspace`) se reserva para el cierre del
bloque, igual que en VA5 y VA6.

## Riesgo asumido, escrito para que no sorprenda

El informe **puede mentir con naturalidad**: enumerar providers y sus razones es
fácil y verosímil mientras omite que no se cross-validó nada. Por eso
`NOT_CHECKED` es un campo **calculado a partir de lo que pasó**, y hay un test
que exige que una resolución de una sola fuente lo diga. Si mañana el reducer
empieza a cross-validar, ese texto tiene que cambiar, y el test lo va a notar.