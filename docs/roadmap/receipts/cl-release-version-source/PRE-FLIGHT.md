# CL-release-version-source — PRE-FLIGHT

**Cycle:** `p-63676b11dc0ef88f/version-source`
**Date:** 2026-10-02T10:35:00Z
**Status:** READY
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto v2.5.2)
**HEAD:** `512fbf3f` (`docs(debt): la clase ASCII de corrupcion no es
automatizable, y esa es la conclusion`), `HEAD == origin/main`, árbol limpio
**Authority:** ADR-0153 `status: proposed`; SCOPE-CONTRACT en este directorio

## Readiness: READY

| # | condición | estado |
|---|---|---|
| 1 | La deuda está **verificada vigente**, no heredada | OK — reproducida con el binario 2.5.3 de este checkout, ver §0 del SCOPE |
| 2 | ADR con decisión, dos clases de fuente y costes admitidos | OK — ADR-0153 |
| 3 | SCOPE con objetivo falsable, no-objetivos y STOP conditions | OK |
| 4 | Superficie mapeada y **leída**, no inferida | OK — abajo, con línea |
| 5 | Superficie acotada al lote | 2 ficheros del engine + el ADR, ninguno fuera |
| 6 | Riesgo de datos | **cero**: este lote no escribe ni en el storage ni en un ledger |

## Superficie (leída, con línea)

| qué | dónde | por qué importa |
|---|---|---|
| `ensure_version_lockstep` | `crates/sddk-engine/src/version.rs:96` | el sitio **único** que abre `Cargo.toml`; abre con `root.join("Cargo.toml")` en la línea 100 |
| doc que descarta la lista codificada | `version.rs:87-95` | **ya dice** que el arreglo es un contrato y no más formatos; el diseño tiene que honorarlo |
| `extract_project_version` | `version.rs` (tras 119) | la lógica de `[workspace.package]`/`[workspace]`/`[package]` se reutiliza, no se reimplementa |
| call site en release | `crates/sddk-cli/src/release_cmd.rs:847` | por dónde llega el predicado a `plan` y `apply` |
| precedente declarativo | `crates/sddk-domain/src/test_adapters.rs:27` (`EcosystemProfileV1`) | la forma «datos, no código de kernel» que este contrato imita |
| **ese precedente no está poblado** | `test_adapters.rs` | `EcosystemProfileV1 { … }` no se construye fuera de los tests: **no hay registro que enchufar**, y por eso el contrato es nuevo |
| el proyecto que reproduce | `/var/home/rubentxu/Proyectos/kotlin/pipeline-kotlin` | Gradle, sin `Cargo.toml`; el caso del principio |

## Lote de este apply

`crates/sddk-engine/src/version_source.rs` (nuevo) +
`crates/sddk-engine/src/version.rs` (un solo call site cambia).

**Motivo de acotar:** el contrato es puro (registro de datos + lector
genérico) y no toca nada que ya funcione para Rust. Implementarlo junto con el
cableado de `release_cmd.rs` haría que un fallo del contrato y uno del
cableado fueran indistinguibles al leer el resultado, que es exactamente el
error que el lote 2 de C3m ya pagó una vez.

## Lote de verificación (scoped, no el perfil completo)

```text
cargo test -p sddk-engine --lib version
```

`--lib version` incluye `version` y `version_source`. El perfil completo
queda para `verify`/release, según AGENTS.md §2.3.

## STOP conditions

1. Si el lockstep de un repo **Rust** cambia de comportamiento, incluido el
   texto del error ⇒ **parada**. Este trabajo no puede comprar cobertura
   a costa del predicado que ya funciona.
2. Si hace falta un `if` por ecosistema dentro de `version.rs` ⇒ parada. Hay un
   test que cuenta las entradas del registro y otro que añade un ecosistema
   sin tocar código; los dos lo morderían.
3. Si un manifiesto ilegible hace que la resolución pase a otro candidato ⇒
   parada. Es la clase «autoridad equivocada en silencio».
4. Si el ADR no queda `proposed` con la decisión y los costes escritos, no se
   implementa.
5. Si tocar el engine obliga a tocar `sddk-domain` ⇒ parar y escribir por qué;
   el tipo público de dominio tiene consumidores que este lote no puede ver.

## Riesgos declarados

| riesgo | mitigación en este lote |
|---|---|
| El registro se convierte de facto en lista codificada | es **datos puros**; los tests 3 y 7 de la verificación lo delatan |
| Un reader genérico se convierte en un `match` gigante por formato | el conjunto de formatos es **cerrado** y pequeño; añadir formato es añadir un brazo justificado, no un ecosistema |
| Relajar el lockstep en Go/Bazel y que parezca lo mismo que en Rust | dos resultados distintos (`CrossChecked` / `TagIsTheOnlyAuthority`) y test que fija que Rust nunca toma el segundo |

## Lo que este pre-flight NO autoriza

- No autoriza publicar una release (sigue bloqueada por la clave del KMS).
- No autoriza tocar el storage real ni ningún ledger.
- No autoriza promover ADR-0153 a `accepted`: se propone con la decisión
  escrita, y la aceptación llega cuando los siete criterios estén verdes
  medidas una a una.
