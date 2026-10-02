# CL-release-version-source — PRE-FLIGHT del lote 2

**Cycle:** `p-63676b11dc0ef88f/version-source`
**Date:** 2026-10-02T14:10:00Z
**Status:** READY
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto v2.5.2)
**HEAD:** `9de63438` (`docs(receipt): el lote 1 del contrato de version, con la
mutacion que escapaba`), `HEAD == origin/main`, árbol limpio
**Authority:** ADR-0153 `status: proposed`; `SCOPE-CONTRACT-lote-2.md` en este
directorio

## Readiness: READY

| # | condición | estado |
|---|---|---|
| 1 | El defecto está **verificado vigente**, no heredado | OK — reproducido con el binario de este checkout, §0 del SCOPE del lote 2 |
| 2 | SCOPE con objetivo falsable, no-objetivos, STOP conditions y **el arreglo que no se hace aquí** | OK |
| 3 | Superficie mapeada y **leída**, con línea | OK — abajo |
| 4 | Superficie acotada al lote | 1 fichero de código (`release_cmd.rs`), 0 fuera |
| 5 | Riesgo de datos | **cero**: no escribe en storage, ni en ledger, ni en otro repo |
| 6 | El cambio es aditivo para el consumidor existente | OK — se declara un campo nuevo; no se renombra ni se quita ninguno |

## Superficie (leída, con línea)

| qué | dónde | por qué importa |
|---|---|---|
| `run_release_plan` | `release_cmd.rs:653` | donde nace la salida del plan |
| call site D1 | `release_cmd.rs:668` | `ensure_version_lockstep(...)` y punto; el resultado se tira en el `map(\|_\| ())` de `version.rs:63` |
| `ReleasePlanOutput` | `release_cmd.rs:644` | el tipo que se serializa; hoy no tiene ningún campo de versión |
| `release_plan_text` | `release_cmd.rs:1113` | el render de texto; hoy no dice nada del origen |
| call site D2 | `release_cmd.rs:847-848` | `?` y luego `let version_lockstep_passed = true;` |
| puerta local | `release_cmd.rs:947` → `release.rs:205` | **no** se toca; ver SCOPE §2 |
| campo reportado | `release.rs:75` y `release.rs:502` | llega serializado a la salida, nadie lo lee para decidir |
| doc que miente | `release.rs:395-396` | dice «the workspace Cargo.toml version», que ADR-0153 ya dejó de ser cierto; **no** se corrige aquí, es `sddk-gateway` (STOP 2) |
| forma del resultado | `version_source.rs:282` (`VersionAuthority`) y `:315` (`was_cross_checked`) | lo que hay que mapear a la salida |
| consumidor de la salida | `crates/sddk-cli/tests/cli.rs:4342` y siguientes | leen `branch`, `base`, `tag`, `steps` por clave: un campo nuevo no los rompe |

## Lote de este apply

`crates/sddk-cli/src/release_cmd.rs`, en tres puntos: el tipo de salida, el
mapeo de la autoridad, y el render de texto. Los call sites de D1 pasan a la
variante `detailed`. **D2 no se toca** (SCOPE §2).

**Motivo de acotar:** D1 y D2 son el mismo síntoma y contratos distintos. Si
se mezclaran, un fallo del render y un fallo del contrato del gateway serían
indistinguibles al leer el resultado — el error que el lote 2 de C3m ya pagó
una vez, y que este SCOPE vuelve a nombrar para no repetirlo.

## Lote de verificación (scoped, no el perfil completo)

```text
cargo test -p sddk-cli --lib release
cargo test -p sddk-cli --test cli release_plan
```

Más el falsificador end-to-end de §5 del SCOPE, que monta un proyecto Go y uno
Rust en un fixture temporal **fuera de este repositorio**: la ruta de release
tiene que poder verificarse contra un proyecto que no es Rust, y no se puede
hacer dentro de este repo sin falsear la prueba.

El perfil completo (`cargo test --workspace`, clippy, fmt) queda para
`verify`/release, según AGENTS.md §2.3.

## STOP conditions

1. Si el lockstep de un repo **Rust** cambia de resultado o de texto ⇒ parada.
2. Si el cambio obliga a tocar `crates/sddk-gateway/` ⇒ parada, y D2 se cierra
   en su propio ciclo.
3. Si `release plan` pasa a fallar en un ecosistema que antes pasaba ⇒ parada.
   Este lote informa; no bloquea.
4. Si un consumidor existente de la salida se rompe ⇒ parada; la salida tenía
   que ser aditiva.

## Riesgos declarados

| riesgo | mitigación en este lote |
|---|---|
| que el texto y el JSON digan cosas distintas | salen del **mismo** valor, no de dos renderizados separados |
| que se lea como que Go no puede publicar | el valor solo lo toman los ecosistemas cuya entrada del registro lo dice; criterio 2 del SCOPE |
| arreglar D2 a medias y dejar la puerta local en `was_cross_checked()` | prohibido por SCOPE §2 y STOP 2 |

## Lo que este pre-flight NO autoriza

- No autoriza tocar `crates/sddk-gateway/`, y por tanto **no cierra D2**.
- No autoriza publicar una release (sigue bloqueada por la clave del KMS).
- No autoriza tocar el storage real ni ningún ledger.
- No autoriza promover ADR-0153 a `accepted`: la aceptación llega cuando los
  criterios estén verdes medidas una a una, y D2 sigue abierto.
