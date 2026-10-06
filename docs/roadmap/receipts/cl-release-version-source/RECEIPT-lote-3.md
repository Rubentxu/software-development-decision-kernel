# CL-release-version-source — RECEIPT del lote 3

**Bloque:** `cl-release-version-source` — bloque de roadmap, **no** es un ciclo del ledger
**Ciclo SDDK:** ninguno. El ledger de `p-63676b11dc0ef88f` da **0 filas** para
`version-source` (medido 2026-10-06). El encabezado declaraba antes un `cycle_id`
completo que la autoridad nunca emitió; corregido por INC-DEBT-063.
**Date:** 2026-10-02T15:50:00Z
**Baseline:** `main@1c82e910`
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto v2.5.2)
**Scope:** `SCOPE-CONTRACT-lote-3.md` — **cerrado en 6 de 6 criterios**
**Status:** **D2 cerrado.** Es lo que el lote 2 dejó abierto y escrito (§2 de su recibo)

## 1. Qué cambió

| fichero | cambio |
|---|---|
| `sddk-engine/src/version_source.rs` | `Serialize` en `VersionAuthority` y `VersionCandidate`, con `tag = "kind"` |
| `sddk-gateway/src/release.rs` | `ReleaseOutcome.version_lockstep_passed: bool` → `version_authority: VersionAuthority`; `apply_release` recibe la autoridad; doc de `:395-396` corregido |
| `sddk-cli/src/release_cmd.rs` | dos helpers con nombre; la ruta forge deriva la autoridad; el render de texto es **uno solo** para plan y apply; **la copia de la CLI desaparece** |
| `sddk-gateway/tests/release_flow.rs` | las 4 llamadas pasan una autoridad real, no un `false` arbitrario |

**Una sola representación del concepto.** `VersionAuthority` es del engine; el
gateway lo reutiliza y la CLI lo muestra. Ninguna capa define la suya.

## 2. Por qué los dos campos dejan de llamarse igual

Es la decisión central del lote, y es lo que el SCOPE §2 mide:

| campo | antes | después |
|---|---|---|
| `ReleaseOutcome` | `version_lockstep_passed: bool`, escrito a mano | `version_authority: VersionAuthority` |
| `LocalReleasePreconditions` | `version_lockstep_passed: bool`, **puerta** | igual, con su semántica escrita |

El segundo **no se renombra**, y el motivo está medido: su valor llega al
storage como la cadena de `ReleaseFailureEvidence::failed_precondition`
(`release_failure_evidence.rs:85`), comparada literalmente en tres tests de
`cli.rs` (`:6644`, `:6988`, `:7425`). Renombrarlo cambia un contrato de datos
durable por claridad en un nombre interno.

Con el resultado tipado, los dos campos **dejan de compartir nombre**, y la
homonimia que impedía el arreglo desaparece por construcción y no por una nota
que lo explique.

## 3. Criterios del SCOPE, medidos uno a uno

| # | criterio | resultado | cómo se midió |
|---|---|---|---|
| 1 | `ReleaseOutcome` declara la autoridad y ya no afirma un lockstep no comprobado | **PASS** | `the_outcome_has_no_boolean_named_after_the_lockstep` + mutación 1 |
| 2 | el doc de `apply_release` ya no nombra `Cargo.toml` | **PASS** | test **estructural** + mutación 2 |
| 3 | la ruta forge deriva la autoridad, no la fija | **PASS** | mutación 1 |
| 4 | la puerta local no cambia de semántica | **PASS** | los 3 tests de `failed_precondition` pasaron **sin reescribirlos** |
| 5 | `release apply` dice la autoridad en su salida | **PASS** | `the_apply_outcome_text_declares_the_authority` + mutación 5 |
| 6 | un proyecto Go sigue publicando por la ruta local | **PASS** | `the_local_gate_lets_a_go_project_through` + mutación 4 |

## 4. Los tres huecos que encontró el falsificador, y por qué existen

El falsificador dio **`PASS=6 FAIL=3`** en su primera versión. Los tres fallos
eran **huecos de test reales**, de la misma clase: **`release apply` no tiene
ninguna cobertura**, porque la ruta forge necesita red y los tests e2e no la
alcanzan.

| hueco | qué hacía el código | por qué nadie lo veía |
|---|---|---|
| la autoridad registrada | podía informar un `CrossChecked` inventado | el call site estaba dentro de un closure, sin seam |
| la puerta local | podía pasar a `was_cross_checked()` y **bloquear a Go y Bazel para siempre** | **todos** los fixtures de la ruta local son de Rust, luego la mutación es indistinguible del comportamiento correcto |
| el render del resultado | se podía borrar entero | `release apply` no es alcanzable sin red |

El segundo es el grave: la puerta local convertida en «hubo cross-check» es
exactamente el arreglo **equivocado** que la tentación sugiere, y habría
dejado a dos ecosistemas sin poder publicar jamás **con la suite en verde**.

Arreglo: dos funciones con nombre —`version_authority_or_fail` y
`version_lockstep_satisfied`—, que son cosas distintas y por eso no caben en un
mismo tipo, y cinco tests nuevos sobre fixtures reales de Go y Rust.

## 5. El falsificador falló contra sí mismo **cuatro** veces

Todas por su propia construcción, ninguna por el código:

1. **Anclaje literal + `cargo fmt`**: la mutación no aterrizaba, el build pasaba
   porque el fichero estaba intacto, los tests verdes, y el arnés declaraba
   `FAIL` sobre algo que nunca se probó. Un `FAIL` de este tipo es peor que
   nada: dice que el código resiste una presión que no se le aplicó.
2. **`\x27` en el reemplazo**: `re.sub` procesa la cadena de reemplazo como
   plantilla, y un escape escrito para Rust se convierte en error de sintaxis
   de regex. La mutación no llegaba a aplicarse.
3. **`[^)]*` con paréntesis anidados**: la expresión es la *cola* de la
   función, sin `;` final, luego el patrón se comía el `}` que la cerraba y
   1.177 caracteres más.
4. **El detector miraba el binario equivocado**: las mutaciones 4 y 5 se
   buscaban en el binario de **integración**, y sus tests viven en `--lib`. El
   arnés declaró «nadie lo detecta» sobre cobertura que sí existía. Se
   comprobó antes de tocar nada: los dos tests fallan de verdad bajo la
   mutación.

El arreglo que permanece: **una mutación que no aterriza, no compila, o que se
busca donde no vive su test, se marca `SKIP` — nunca `FAIL`.** Un `FAIL` que no
midió nada es la peor salida posible de un falsificador, porque se lee igual
que un hallazgo.

## 6. Verificación ejecutada (scoped, no el perfil completo)

```text
cargo test -p sddk-gateway --test release_flow        → 12 passed; 0 failed
cargo test -p sddk-gateway --test release_blockers    → 3 passed; 0 failed
cargo test -p sddk-engine --lib version              → 50 passed; 0 failed
cargo test -p sddk-cli --lib release                 → 22 passed; 0 failed
cargo test -p sddk-cli --test cli release            → 33 passed; 0 failed
cargo fmt --check                                    → limpio
cargo clippy -p sddk-engine -p sddk-gateway -p sddk-cli --all-targets -D warnings → limpio
bash /var/home/rubentxu/ff-release-outcome-authority.sh → PASS=9 FAIL=0 SKIP=0
```

Falsificador, 5 mutaciones, **las 5 detectadas**:

| mutación | detectada por |
|---|---|
| la autoridad registrada se inventa un cross-check | `a_go_project_reports_the_tag_as_the_only_authority` |
| el doc de `apply_release` vuelve a nombrar `Cargo.toml` | test estructural del doc |
| la forma serializada vuelve a ser la externa | `the_outcome_has_no_boolean_named_after_the_lockstep` |
| la puerta local pasa a exigir cross-check | `the_local_gate_lets_a_go_project_through` |
| `release apply` deja de decir la autoridad | `the_apply_outcome_text_declares_the_authority` |

Más dos comprobaciones de estado: el código sin mutar está verde, y tras
restaurar vuelve a estarlo.

## 7. Lo que este lote NO hace

- **No renombra** `LocalReleasePreconditions.version_lockstep_passed` ni toca
  `failed_precondition` (§2, con su coste medido).
- **No hace que `apply_release` valide** la autoridad. Hoy no valida el bool y
  validar sería una puerta nueva con su propio SCOPE; el doc lo dice.
- **No bloquea** releases por falta de versión declarada. Este lote informa.
- **No toca** `sddk-domain`.

## 8. Contexto real / fake

- **Real:** el código, los tests, el gateway y el engine compilados, los
  fixtures de Go y Rust de `tempfile` —directorios de verdad con un `go.mod` y
  un `Cargo.toml` reales—, y los 9 puntos del falsificador.
- **Fake:** nada.
- **No ejecutado:** `release apply --route forge` contra GitHub de verdad. Este
  lote no lo necesita para cumplir sus criterios —que son sobre el tipo, el
  render y la puerta—, pero **no** se afirma que un release por forge funcione
  de extremo a extremo. Esa medición es de otra clase y no se ha hecho.

## 9. Primer paso de la sesión siguiente

Con D2 cerrado, los siete criterios de ADR-0153 son medibles uno a uno. El
siguiente bloque es **promover ADR-0153 y ADR-0152 a `accepted`**, que exige
recorrer sus criterios individualmente y no declararlos verdes por suma. Y en
paralelo, el único bloqueo que queda para publicar v2.5.3 es del operador: la
clave del KMS.
