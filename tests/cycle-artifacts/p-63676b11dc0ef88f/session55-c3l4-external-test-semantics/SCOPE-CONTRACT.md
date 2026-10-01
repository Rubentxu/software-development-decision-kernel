# SCOPE-CONTRACT — C3l.4 External test semantics: ausencia ≠ PASS

**Slice:** `session55-c3l4-external-test-semantics`
**Roadmap:** `docs/roadmap/ROADMAP.md` §C3l.4 · **Paquete:** `docs/sddk-roadmap-acceptance-truthfulness-2026-09-30/ROADMAP-ACCEPTANCE-TRUTHFULNESS.md:252`
**Predecesor:** C3l.3 (`7360c32e`) · **Baseline de partida:** `4657e8b0`
**UAT:** AT-UAT-009 (PASS) · AT-UAT-010 (**BLOCKED**, no PASS)

---

## 1. Defecto verificado, con su RED medido

`aiw_s5_chronos_real.rs` resolvía el provider así:

```rust
let bin = match mcp_bin() { Some(b) => b, None => return };
```

RED observado **en esta máquina, con `CHRONOS_MCP_BIN` y `COGNICODE_MCP_BIN` ambos ausentes**:

```
=== chronos HOY ===
test capture_produces_events_and_verified_claim ... ok
test events_below_threshold_is_unknown ... ok
test result: ok. 3 passed; 0 failed; 0 ignored   (finished in 0.00s)

=== cognicode, MISMO entorno ===
test result: ok. 2 passed; 0 failed; 3 ignored
```

Dos de los tres tests **no se ejecutaron** y el run lo reportaba como verde. `finished in 0.00s` es la prueba material: no se puede hacer spawn de un proceso y capturar eventos en cero tiempo.

**El contraste es la mitad del hallazgo:** `aiw_s1_cognicode_real.rs`, en el mismo entorno, ya reportaba `3 ignored` porque usaba la convención correcta (`#[ignore]` + `expect`). El defecto estaba **aislado a 2 sitios** y la solución correcta **ya existía en el repo**. Esto no es diseño nuevo: es generalizar un patrón propio.

## 2. La divergencia fundamental (por qué importa más allá de Chronos)

`ProviderKind::Null` (usado por el runtime evidence port) y `COGNICODE_MCP_BIN` ausente producían **exactamente el mismo falso verde por rutas distintas**: en un caso la ausencia está *codificada en el tipo*, en el otro en un `return`. Un type-system que modela la ausencia correctamente y un harness que la silencia son el mismo defecto a dos alturas.

## 3. Contrato congelado: los cinco estados

| Estado | Significado | En el run ordinario |
|---|---|---|
| `PassObserved` | provider resuelto **y** contrato sostenido | pass |
| `FailObserved` | provider resuelto **y** contrato roto | **fail** |
| `BlockedExternalDependency` | binario ausente o inservible | ignored |
| `NotRun` | el perfil EXT no se pidió | ignored |
| `NotApplicable` | la dependencia no aplica a este build | ignored |

**Invariante load-bearing:** `is_pass()` es `true` **solo** para `PassObserved`. Y **resolver un binario nunca devuelve un pass** — resolver es precondición, no observación. Sin esa distinción, "encontré el binario" se degrada en "el contrato se sostiene".

## 4. Entregable

| Path | Δ | Descripción |
|---|---|---|
| `crates/sddk-engine/src/ext_outcome.rs` | +~300 (nuevo) | Enum de 5 estados + `ExtProviderReceipt` + `resolve_provider` + `sha256_of` |
| `tests/ext_provider_gate.sh` | +~130 (nuevo) | Launcher: 5 estados, 4 exit codes, recibo JSON |
| `crates/sddk-engine/tests/aiw_s5_chronos_real.rs` | reescrito | `#[ignore]` + `expect`; test nuevo `absent_provider_is_blocked_not_pass` |
| `crates/sddk-engine/src/runtime_evidence_port_mcp.rs` | +8 | `clientInfo.version` `"0.1.0"` → `env!("CARGO_PKG_VERSION")` |
| `.gitignore` | +5 | el recibo del gate es artefacto de run, no evidencia |

## 5. Falsificador congelado

**M5** — restaurar el patrón defectuoso (`None => return`, sin `#[ignore]`) debe devolver el falso verde. Si no lo devuelve, el arreglo no es load-bearing.

## 6. Exit gate (del paquete)

> Eliminar todo patrón equivalente a `let Some(bin) = ... else { return; }` dentro de tests que certifican integración externa.

Verificación: `grep -rn "None => return" --include=*.rs crates/*/tests/` → **1 hit**, y es el comentario que *documenta* el defecto. Satisfecido.

## 7. Reglas de ejecución

- RED medido antes de tocar nada, con el contraste del patrón correcto como control.
- No cambiar `#[ignore]` por un skip silencioso alternativo: si el perfil EXT se pide sin provider, **falla**.
- `FailObserved` y `BlockedExternalDependency` no pueden colapsarse: es como una regresión real se reporta como problema de entorno.
- Shellcheck limpio en el launcher (gate del repo).

## 8. Límites declarados por adelantado

1. **AT-UAT-010 NO es un PASS.** `chronos-mcp` no está instalado en esta máquina. La semántica está implementada y falsificada; la **captura real sigue sin observarse**. Registrado como BLOCKED en la matriz, no como pendiente pequeño.
2. **El contrato enum↔launcher se pinea por test, no por convención**: `ext_outcome_states_match_the_launcher_contract` falla si el script deja de conocer un estado. Aun así, un test que lee el script no prueba que el script *emita* ese estado — eso quedaría para un UAT de contrato del launcher cuando haya un provider real.
3. **El receipt del gate no se commitea**: se reescribe en cada run. La observación durable va en `UAT-MATRIX.md`.
4. **Este slice no toca `ProviderKind::Null`.** El falso verde por *tipos* queda abierto; lo cierra la consolidación provider/capability (**C3m.3** + **C3m.5**), no este slice.

## 9. Corrección de un error propio

En el turno anterior escribí que el ADR de significado canónico de provider/capability era **C3m.0**. **Es incorrecto:** C3m.0 es el ADR de **KMT** (Knowledge Merkle Tree). La consolidación de provider/capability es **C3m.3** (runtime provider-neutral provenance) y **C3m.5** (R0 bounded-context decision). Anotado aquí para que el recibo no propague el error.
