# RECEIPT — C3l.7: el architecture gate no certifica conformidad con deuda abierta

**Slice:** `session60-c3l7-architecture-gate-verdict`
**Fecha:** 2026-10-01 · **Baseline:** `2bf3027e` (HEAD == origin/main al abrir)
**Perfil completo:** 5233 passed / 0 failed / 23 ignored (+9 vs session-59: 11 unitarios del veredicto + 2 tests nuevos del gate − 4 movidos de `src/` a `tests/`)
**Workflow:** `A-lite` · **UAT cubierto:** AT-UAT-015 · **R10** de la matriz

---

## §1 El paquete asumía un defecto; había tres

La especificación de C3l.7 (`ROADMAP-ACCEPTANCE-TRUTHFULNESS.md:370`) dice que
*"el test actual considera correcto que `ARCH001 FAIL → exit 1`"*. **Medido, ese
supuesto está obsoleto.** El estado real del repo antes de esta slice:

```text
$ sddk dev check-architecture --root .
ARCH001   PASS
ARCH003   WAIVED     waived: Two composition-root edges in sddk-cli survive the …
ARCH004   N/A        kernel repo, not a pack host (Phase 4 substrate not shipped…
…  (10 reglas N/A: evaluador no implementado)
EXIT=0
```

La edge `engine → storage` que motivaba el supuesto **ya no existe**:
`sddk-engine/Cargo.toml` no depende de `sddk-storage`. Y el gate salía **0**
— verde — con 2 waivers vivos y 10 reglas sin evaluar.

### D1 — El test no ejecutaba nada

`check_architecture_runs_against_repo` resolvía el binario en
`target/{release,debug}/sddk`. En esta máquina el target dir es compartido
(`/var/home/rubentxu/cargo-targets`), así que el fichero no estaba ahí, el test
hacía `return` y se reportaba **`ok`**:

```text
running 1 test
PROBE: sddk binary not found at ".../target/debug/sddk", skipping
test result: ok. 1 passed; 0 failed; 0 ignored; ... finished in 0.00s
```

Es el patrón que C3l.4 ya corrigió en otro sitio (auto-verde por `return`),
reintroducido aquí. **Un gate que no se ejecuta no puede certificar nada.**

### D2 — La afirmación era obsoleta

El test exigía `ARCH001` **FAIL** y exit 1. Con el binario presente habría
**cayado**: ARCH001 hoy es PASS. El test certificaba un estado que el repo ya
no tiene.

### D3 — El defecto estructural: el gate no distingue deuda de conformidad

`RuleStatus` = `{Pass, Fail, Waived, NotApplicable}`. **Cero ocurrencias de
`OPEN_DEBT`.** `Waived` y `NotApplicable` salían ambos por el mismo camino a
`exit 0`, y `Waived` no distingue expirado de vigente. La política del paquete
— *"una violación severity=error sólo puede estar en OPEN_DEBT, WAIVED_UNTIL(date)
o FIXED, y no puede contarse como architecture gate PASS"* — **no era
representable** en el modelo.

## §2 La solución: un veredicto tipado, no un exit code

Nuevo módulo `sddk_domain::rules::verdict` (11 tests unitarios). No se tocó el
enum `RuleStatus` — tiene **21 consumidores** en `rules_evaluator.rs` y
`check_arch.rs`; añadir una variante habría sido un cambio de contrato público
injustificado para lo que es una pregunta **agregada**.

```rust
pub enum Verdict { Conformant, OpenDebt, Waived, NotEvaluated }
```

`Verdict::is_conformant()` es `true` **sólo** para `Conformant`. La afirmación
"este repo es arquitectónicamente conforme" no está disponible por defecto: hay
que ganársela. Reglas de la clasificación:

| Outcome por regla | Veredicto | Racional |
|---|---|---|
| `Fail` + `Error` | **OpenDebt** | violación real, sin excepción |
| `Waived` + `Error` | **Waived** | conforme sólo mientras el waiver viva |
| `NotApplicable` | **NotEvaluated** | no se evaluó: el silencio no es evidencia |
| `Fail` + `Warning`/`WarningThenRatchet` | Conformant | advisory; `WarningThenRatchet` existe para grandfatherar violaciones |
| entrada vacía | **NotEvaluated** | un fichero de reglas vacío no puede acuñar conformidad |

`merge` es conservador: `OpenDebt` > `Waived` > `NotEvaluated` > `Conformant`.
Una violación entre varios `Pass` **domina**, no se promedia.

**Exit codes:** `0` sólo `Conformant`; `1` `OpenDebt` (contrato histórico
intacto); **`2` nuevo** para `Waived`/`NotEvaluated` — un consumidor que debe
fallar cerrado sobre *no demostrado* lo distingue de una violación probada.

`WarningThenRatchet` era una tercera severidad que no aparecía en la lectura
inicial del paquete; se clasifica como warning **a propósito**: bloquear sobre
una violación que esa severidad existe para congelar en el sitio impediría que
el código nuevo cumpla, que es su propósito declarado.

## §3 GREEN

`crates/sddk-cli/tests/check_architecture_gate.rs` — **4 passed / 0 failed en
0.67s** (antes: `4 passed` en **`0.00s`** sin ejecutar el gate).

```text
$ sddk dev check-architecture --root .
…
VERDICT: WAIVED (exit 2)
This gate does NOT certify architectural conformance. See VERDICT above.
```

El JSON `--out` lleva ahora `verdict` además de `exit_status`, con la invariante
pinnada: `exit == 0 ⟺ verdict == "CONFORMANT"`.

**Defectos colaterales encontrados al reparar el skip**, invisibles mientras el
gate no se ejecutaba:

1. **CWD equivocado**: los tests de integración corren en `crates/sddk-cli`, no
   en la raíz, así que `--root .` no encontraba el fichero de reglas y salía
   exit 1 con stdout vacío. Ahora el root se resuelve desde `CARGO_MANIFEST_DIR`.
2. **3 fixtures con `schema_version: "1.0.0"`** cuando el loader exige `1.2.0`.
   El skip los ocultaba; al dejar de ejecutarse, fallaron.

## §4 Falsificadores

| # | Falsificador | Resultado |
|---|---|---|
| **F15** | Volver al exit code antiguo (`if has_error_fail {1} else {0}`) | **OBSERVED** — 2 FAIL |
| **F16** | Reponer el `skip` silencioso + el path de binario inexistente | **OBSERVED, decisivo** — 2 FAIL con `finished in 0.00s` |

**F16 es el que importa**: reproduce exactamente el patrón `0.00s` que delató D1.
Los dos tests nuevos lo cazan; los antiguos, no.

## §5 Límites declarados

1. **AT-UAT-015 sigue sin ser PASS.** Su criterio es *"0 error no-waived o waiver
   vigente tipado"*. El repo tiene 2 waivers vivos (ARCH003, ARCH008) y 10
   evaluadores sin implementar, así que **hoy el veredicto es `WAIVED`, no
   `CONFORMANT`**. La slice hace que el gate sea honesto; **no** convierte al
   repo en conforme. Cerrar AT-UAT-015 requiere eliminar esos waivers o
   implementar los evaluadores, que es trabajo de C3l.7/C5.
2. **`Waived` no tipa el expiry en el estado.** El expiry por ancestry ya
   existe en el evaluador (`granted_until_sha` + `WAIVER_NO_EXPIRY_SENTINEL`),
   pero `Waived` como estado no transporta la fecha. El detalle en `N/A` de un
   waiver expirado sí la nombra. Tipar el expiry en el veredicto es trabajo
   posterior, no se afirma aquí.
3. **El exit code 2 es nuevo.** No hay consumidores externos de
   `check-architecture` (verificado: sólo ADRs, docs y el propio test), pero un
   llamador externo previo que esperase 0 para "sin violation" ahora recibe 2
   cuando hay waivers. Es el comportamiento pretendido: ambos casos significan
   "no demostrado".
4. **R10 no pasa a VERIFIED.** El claim "architecture conformant" sigue sin
   validity; lo que cambia es que **el gate deja de sustentarlo**.

## §6 Perfil de verificación

```text
cargo test -p sddk-domain --lib verdict            11 passed; 0 failed
cargo test -p sddk-cli --test check_architecture_gate   4 passed; 0 failed (0.67s)
cargo fmt --all -- --check                         limpio
cargo clippy -p sddk-domain -p sddk-cli --all-targets -- -D warnings   exit 0
cargo test -p sddk-cli --test context_fitness     7 passed (contrato cross-crate de modulos root)
cargo test --workspace                             5233 passed; 0 failed; 23 ignored
```
