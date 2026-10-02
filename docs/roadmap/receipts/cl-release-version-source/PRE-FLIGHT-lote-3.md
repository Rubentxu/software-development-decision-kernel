# CL-release-version-source — PRE-FLIGHT del lote 3

**Cycle:** `p-63676b11dc0ef88f/version-source`
**Date:** 2026-10-02T15:10:00Z
**Status:** READY
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto v2.5.2)
**HEAD:** `1c82e910`, `HEAD == origin/main`, árbol limpio, 0 commits sin publicar
**Authority:** ADR-0153 `status: proposed`; `SCOPE-CONTRACT-lote-3.md`

## Readiness: READY

| # | condición | estado |
|---|---|---|
| 1 | El defecto está **verificado vigente** | OK — §0 del SCOPE, con línea y con la asimetría medida |
| 2 | SCOPE con objetivo falsable, no-objetivos, STOP conditions y el arreglo que **no** se hace (el rename del gate) | OK |
| 3 | Superficie mapeada y **leída** | OK — abajo |
| 4 | Superficie acotada | 4 ficheros, 3 de ellos de una línea o dos |
| 5 | Riesgo de datos | **cero**: no escribe en storage, ni ledger, ni otro repo. El string `failed_precondition` queda **intacto** por diseño (§2 del SCOPE) |
| 6 | El tipo del engine es reutilizable sin dependencia nueva | OK — `sddk-gateway` ya depende de `sddk-engine` |

## Superficie (leída, con línea)

| qué | dónde | por qué importa |
|---|---|---|
| el literal que hay que eliminar | `release_cmd.rs:847-848` | el PRODUCTO del defecto: informa un lockstep que no ocurrió |
| campo reportado | `release.rs:75` | el que se sustituye por la autoridad |
| el que lo escribe | `release.rs:502` | dentro de `apply_release` |
| el que lo recibe | `release.rs:401` | el cuarto parámetro de `apply_release` |
| el doc que miente | `release.rs:395-396` | dice «the workspace Cargo.toml version»; ADR-0153 ya no es cierto |
| la puerta, **intocable** | `release.rs:144` y `:205` | su valor viaja al storage como cadena |
| la cadena durable | `release_failure_evidence.rs:85` | `failed_precondition`, comparada literalmente en 3 tests de `cli.rs` |
| el tipo canónico | `version_source.rs:282` (`VersionAuthority`), `:271` (`VersionCandidate`) | ya tiene `Debug + Clone + PartialEq + Eq`; le falta `Serialize` |
| el precedente de serializar en el engine | `version_source.rs` ya devuelve `PathBuf` y `Vec` | el derive es la única representación nueva, no una copia |
| el render que no lo decia | `release_cmd.rs:1215` (`release_outcome_text`) | imprime `converged` y `applied`; de la versión, nada |
| las 4 llamadas que pasan `false` | `release_flow.rs:109`, `:229`, `:235`, `:259` | el `false` era arbitrario porque nadie leía el campo |

## Lote de este apply

1. `Serialize` en `VersionAuthority` y `VersionCandidate`, con
   `rename_all = "snake_case"` — el mismo nombre de clave que la CLI ya emite
   en `release plan`, para que el concepto tenga **una** grafía en el JSON.
2. `ReleaseOutcome.version_lockstep_passed` → `version_authority: VersionAuthority`.
3. `apply_release` recibe la autoridad; doc de `:395-396` corregido.
4. La ruta forge deriva la autoridad de `ensure_version_lockstep_detailed`.
5. `release_outcome_text` la dice.
6. Las 4 llamadas de test pasan una autoridad real.

**Motivo de no tocar la puerta local:** §2 del SCOPE. Su nombre es ambiguo,
pero su valor llega al storage como cadena en `failed_precondition` y tres
tests de integración la comparan literalmente. Renombrarlo cambia un contrato
de datos durable por claridad en un nombre interno: precio unjustificado. El
criterio 4 mide que la puerta no cambia, sin reescribir esos tests.

## Lote de verificación (scoped, no el perfil completo)

```text
cargo test -p sddk-gateway --lib release
cargo test -p sddk-gateway --test release_flow
cargo test -p sddk-gateway --test release_blockers
cargo test -p sddk-cli --lib release
cargo test -p sddk-cli --test cli release
```

Más el falsificador end-to-end del lote 2
(`/var/home/rubentxu/ff-release-plan-authority.sh`), que monta proyecto Go y
Rust fuera del repositorio: si este lote rompiera la ruta de release, ese
guardián lo vería aunque sus propio casos no lo toquen.

## STOP conditions

1. Si la puerta local cambia de comportamiento o de texto ⇒ parada.
2. Si obliga a tocar `sddk-domain` ⇒ parada.
3. Si `ReleaseOutcome` **gana** un campo y conserva el bool ⇒ parada: dos
   fuentes de verdad para el mismo concepto.
4. Si aparece una copia de `VersionAuthority` en el gateway o en la CLI ⇒
   parada, aunque compile.

## Riesgos declarados

| riesgo | mitigación en este lote |
|---|---|
| romper un consumidor del JSON de `ReleaseOutcome` | el campo no lo lee nadie en el workspace, medido con `rg`, y el cambio va declarado en el changelog |
| que el doc corregido vuelva a mentir | criterio 2 con test **estructural**: un doc no se ejecuta, luego un test de comportamiento no lo vigila |
| que el JSON de la CLI y el del gateway digan `cross_checked` de dos maneras | mismo `rename_all` y mismo tipo, así que es imposible que difieran |

## Lo que este pre-flight NO autoriza

- No autoriza renombrar `LocalReleasePreconditions.version_lockstep_passed`
  ni tocar `failed_precondition` (§2 del SCOPE, con su coste medido).
- No autoriza que `apply_release` **valide** la autoridad: sería una puerta
  nueva y §3 del SCOPE lo prohíbe explícitamente.
- No autoriza publicar v2.5.3 (sigue bloqueada por la clave del KMS).
- No autoriza promover ADR-0153 a `accepted` en este lote: el cierre de D2 es
  condición necesaria, no suficiente, y la aceptación se mide criterio a
  criterio.
