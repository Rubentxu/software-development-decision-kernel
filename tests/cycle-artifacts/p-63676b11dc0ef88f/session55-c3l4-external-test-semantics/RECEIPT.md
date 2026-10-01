# RECEIPT — C3l.4 External test semantics: ausencia ≠ PASS

**Slice:** `session55-c3l4-external-test-semantics`
**Fecha:** 2026-10-01 · **Baseline de partida:** `4657e8b0` (predecesor C3l.3 `7360c32e`)
**Commits:** `6d826044` (fix) · `fc5fea2f` (higiene del recibo)
**SCOPE-CONTRACT:** [`SCOPE-CONTRACT.md`](SCOPE-CONTRACT.md) (congelado antes de escribir código)
**UAT:** AT-UAT-009 **PASS** · AT-UAT-010 **BLOCKED** (no PASS) · `boundary_class = MCP_EXTERNAL`

---

## §1 El RED, medido con su control

`aiw_s5_chronos_real.rs` resolvía el provider con `None => return`. En esta máquina, con `CHRONOS_MCP_BIN` y `COGNICODE_MCP_BIN` **ambos ausentes**:

```
=== chronos (defectuoso) ===
test capture_produces_events_and_verified_claim ... ok
test events_below_threshold_is_unknown ... ok
test result: ok. 3 passed; 0 failed; 0 ignored    (finished in 0.00s)

=== cognicode, MISMO entorno (correcto) ===
test result: ok. 2 passed; 0 failed; 3 ignored
```

Dos de los tres tests **no se ejecutaron**. `finished in 0.00s` es la prueba material: no se puede hacer spawn de un proceso y capturar eventos en cero tiempo.

**El contraste es parte del hallazgo, no adorno:** el suite de CogniCode, en el mismo entorno, ya reportaba `3 ignored`. El defecto estaba aislado a **2 sitios** y la convención correcta **ya existía en el repo**. Esto no es diseño nuevo — es generalizar un patrón propio.

## §2 GREEN, en las dos direcciones

```
=== run ordinario (mismo entorno sin binario) ===
capture_produces_events_and_verified_claim ... ignored
events_below_threshold_is_unknown ... ignored
spawn_fails_closed_on_missing_binary ... ok
absent_provider_is_blocked_not_pass ... ok
test result: ok. 2 passed; 0 failed; 2 ignored

=== perfil EXT pedido explícitamente SIN binario ===
test result: FAILED. 0 passed; 2 failed
EXT profile requested without a usable chronos-mcp:
  BlockedExternalDependency { env_var: "CHRONOS_MCP_BIN",
    found: "not on PATH and env var unset or empty" }
```

El run ordinario es honesto (2 passed = 2 tests que corrieron de verdad) y el perfil EXT **falla en vez de fingir**.

## §3 Falsificador M5 — OBSERVED

Con el verde ya commiteado, se restauró el patrón defectuoso (`None => return`, sin `#[ignore]`):

```
test result: ok. 4 passed; 0 failed; 0 ignored   (finished in 0.00s)
```

El falso verde regresa. **El exit gate es load-bearing.** Restaurado; `git diff` limpio respecto al commit.

## §4 El contrato

`ext_outcome` fija cinco estados; `is_pass()` es `true` **solo** para `PassObserved`. Y la invariante que más importa: **resolver un binario nunca devuelve un pass** — `resolve_provider` devuelve `NotRun` aunque lo encuentre, porque resolver es precondición, no observación. Sin esa distinción, "encontré el binario" se degrada en "el contrato se sostiene".

`tests/ext_provider_gate.sh` es la otra mitad: 5 estados, 4 exit codes (0 pass / 1 fail / 2 blocked / 3 not_run) y recibo JSON con path/sha256/version/capabilities. **Un provider resuelto cuyo perfil falla es `fail_observed`, nunca `blocked`** — colapsar los dos es exactamente cómo una regresión real se reporta como problema de entorno. El script se verificó: sin binario devuelve exit 2 y escribe el recibo; shellcheck limpio.

Los dos lados del contrato se pinean mutuamente por test (`ext_outcome_states_match_the_launcher_contract`).

## §5 Exit gate

> Eliminar todo patrón equivalente a `let Some(bin) = ... else { return; }` en tests que certifican integración externa.

`grep -rn "None => return" --include=*.rs crates/*/tests/` → **1 hit**, y es el comentario que *documenta* el defecto. **Satisfecho.**

## §6 Gates observados

| Gate | Resultado |
|---|---|
| `ext_outcome` (unit) | **4 / 0** |
| `aiw_s5_chronos_real` ordinario | **2 passed / 2 ignored** (honesto) |
| `aiw_s5_chronos_real --ignored` sin binario | **0 passed / 2 FAILED** (falla fuerte) |
| `cargo fmt --check` | limpio |
| `cargo clippy -p sddk-engine --all-targets -- -D warnings` | exit 0 |
| `shellcheck tests/ext_provider_gate.sh` | limpio |
| `cargo test -p sddk-engine --lib` | **1362 / 0 / 1** (era 1358; +4) |
| `cargo test -p sddk-engine` (suite completa) | 0 fallos |

## §7 Límites declarados

1. **AT-UAT-010 NO es un PASS.** `chronos-mcp` no está instalado. La semántica está implementada y falsificada; la **captura real sigue sin observarse**. Registrado como BLOCKED.
2. **El pin enum↔launcher es parcial.** El test lee el script y falla si deja de conocer un estado; eso **no** prueba que el script lo *emita*. Queda para un UAT de contrato del launcher cuando exista un provider real.
3. **El recibo del gate no se commitea** — se reescribe en cada run y ensuciaría el árbol. La observación durable va en `UAT-MATRIX.md`. (Error propio: lo commiteé en `6d826044` y lo revertí en `fc5fea2f`.)
4. **`ProviderKind::Null` NO se toca.** El falso verde por *tipos* queda abierto; lo cierra la consolidación provider/capability (**C3m.3** + **C3m.5**), no este slice.
5. **Sin perfil completo del workspace ni release** — corresponden a `verify`/release.

## §8 Corrección de un error propio

En el turno anterior afirmé que el ADR de significado canónico de provider/capability era **C3m.0**. **Es incorrecto: C3m.0 es el ADR de KMT** (Knowledge Merkle Tree). La consolidación provider/capability es **C3m.3** (runtime provider-neutral provenance) y **C3m.5** (R0 bounded-context decision). Anotado en el SCOPE-CONTRACT §9 para que no se propague.

## §9 Siguiente paso

**C3l.5** — X04: dos CLI / concurrencia real multi-proceso (`SQLITE_MULTI_PROCESS`). Después C3l.6 (X07 segundo binario real) y C3l.7 (architecture gate, que cierra la vía C3l y desbloquea C3n).

**En paralelo y sin decisión previa**, la consolidación de la costura provider/capability: los `ProviderKind` duplicados, los tres `ObservationSet` y `ProviderKind::Null` necesitan **C3m.3 + C3m.5**, no tickets sueltos — abrir esos tres como trabajo independiente crearía una segunda autoridad para el mismo concepto (§2.7).
