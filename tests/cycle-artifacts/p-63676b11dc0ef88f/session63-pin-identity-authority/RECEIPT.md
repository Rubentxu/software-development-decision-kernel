# RECEIPT — el pin de identidad era la autoridad declarada y no lo era

**Slice:** `session63-pin-identity-authority`
**Fecha:** 2026-10-01 · **Baseline:** `b44d36f9` (HEAD == origin/main al abrir)
**Workflow:** `A-lite` · **Deuda:** INC-DEBT-049 (parte del pin) · **UAT:** sin UAT propio; cierra el mecanismo de remediación de INC-DEBT-049

---

## §1 Cómo empezó: parecía deuda de entorno y era un defecto de producto

Session-62 cerró C3m.2 y, al intentar registrar la sesión en el CLI de cycle,
encontró que `sddk cycle status` no veía 65 ciclos ni 3.911.680 B de ledger.
Quedó registrado como **INC-DEBT-049**: dos identidades de proyecto vivas, el
CLI resolviendo la vacía, y un remedio local propuesto
(`sddk project pin --project-id p-63676b11dc0ef88f`) que se dejó **sin aplicar**
porque cambiar la identidad autoritativa del repo es gobernanza.

El operador lo autorizó. **Aplicado.** Y al aplicarlo salió esto:

```text
$ sddk project pin …             -> project pinned: p-63676b11dc0ef88f     OK
$ sddk project resolve …         -> project_id: p-63676b11dc0ef88f
                                    identity_source: pinned                 OK
$ sddk adopt status              -> project_id: p-995939af668a53d8        NO
$ sddk cycle status              -> no active cycle found for
                                    project p-995939af668a53d8            NO
```

**El pin se escribía y no surtía efecto.** Eso convierte la deuda en **defecto
de producto**, y es justo el tipo de cosa que la regla 4 del operador manda
mirar antes de plantear cambios: responsabilidades duplicadas con
implementaciones divergentes.

## §2 La causa raíz: cinco resolvers, dos honraban el pin

Medido sobre `crates/sddk-cli/src/`:

| # | Resolver | Leía el pin | Superficie expuesta |
|---|---|---|---|
| 1 | `RuntimeContext::open` (`cycle.rs:448`) | ✅ | `cycle` con `--cycle` explícito |
| 2 | `run_project_resolve` (`lib.rs:1650`) | ✅ | `project resolve` |
| 3 | `resolve_project_ids` (`lib.rs:1498`) | ❌ | `config set` |
| 4 | inferencia de ciclo (`cycle.rs:255-291`) | ❌ | `cycle status`, `cycle next` |
| 5 | `plan_adoption` (`sddk-engine/adoption.rs`) | ❌ | `adopt status/plan/apply/…` |

**La afirmación falsa estaba en el propio código**, no en un documento externo:

```rust
/// When present, `project resolve` and every runtime context honor it over
/// remote/seed derivation, so a renamed or case-drifted remote cannot fork
/// the ledger.
```

Dos de cinco contexts no lo honraban. Y un segundo comentario, en la inferencia
de ciclo, enumeraba la lógica como *"remote OR fallback_seed OR generate"* —
omitiendo el pin, que era exactamente la diferencia que rompía.

**Por qué el contrato declarado no estaba probado:** los dos e2e del pin
(`pin_overrides_remote_drift`, `unpin_restores_remote_derivation`) invocan
`project resolve`. El único resolver que funcionaba era el único que se
probaba. `adopt status`, `cycle status` y `config set` no tenían ni un caso con
pin en toda la suite.

## §3 La corrección

Una función canónica decide la identidad de todo el CLI:

```rust
pub(crate) fn resolve_identity_honoring_pin(
    root: &Path, scope: &str,
    remote: Option<String>, fallback_seed: Option<String>,
) -> anyhow::Result<ResolvedProjectIdentity>
```

El pin gana sobre remote y sobre seed. La usan `resolve_project_ids`,
`RuntimeContext::open` y la inferencia de ciclo.

`plan_adoption` es una **función pura sin acceso a disco** (contrato deliberado:
*"builds an adoption plan without reading or writing process or filesystem
state"*), así que no puede leer el fichero. Recibe el pin como **dato**:

```rust
pub pinned_project_id: Option<String>,
```

El CLI lo lee, el engine lo razona. `validate_plan_input` **falla cerrado** ante
un pin malformado: no puede caerse al remote en silencio, porque esa caída
silenciosa es lo que produjo el `status: complete` sobre un storage vacío.

**`.sddk/project-pin.json` a `.gitignore`.** Es configuración de identidad
**por máquina**; versionarlo forzaría a todo otro checkout del repo al
`project_id` de quien commitea — la bifurcación misma que el pin previene.
`.sddk/followups/` sigue trackeado a propósito: es evidencia redactada.

## §4 Falsificadores OBSERVED

| # | Mutación | Resultado |
|---|---|---|
| **F49** | `prepare_adoption_plan` deja de pasar el pin | **OBSERVED** — e2e FAILED: `adopt status debe honourar el pin (esperaba p-pinnedauthoritative01, derivado p-c3d5cbc69b93a9bb)` |
| **F50** | la inferencia de ciclo deja de leer el pin | **OBSERVED** — e2e FAILED en la aserción de `cycle status` |
| **F51** | `resolve_identity_honoring_pin` ignora el pin | **OBSERVED** — unit test FAILED: `resolve_project_ids debe honourar el pin` |
| **F52** | `plan_adoption` ignora el pin | **OBSERVED** — `pinned_project_id_wins_over_remote_derivation` FAILED |

Un valor sobre el segundo plano: con la mutación de **F52**, el test de pin
malformado **sigue verde**, porque la validación vive en `validate_plan_input`,
independiente del `match` de derivación. Son dos comportamientos distintos y
están cubiertos por separado; un solo test no habría detectado la separación.

### Un falsificador que era una falsación y se descartó

El primer intento de F49 se aplicó con 8 espacios de indentación sobre una línea
que tenía 4: **el fichero no cambió** y el e2e pasó "sin romper". Tomado como
OBSERVED habría sido una mentira. Sólo cuenta después de verificar el fichero
mutado. Se registra porque es exactamente el modo de fallo que este repo ya
pagó antes (INC-DEBT-045: un caso que nunca exertitaba lo que declaraba).

## §5 Dos errores propios durante los tests

1. **`adopt status` sin receipt sale con código 1** (`status: absent`, que es la
   respuesta correcta para un checkout recién hecho). afirmé `code == 0`.
   El test medía el código cuando lo que debía medir es el `project_id`.
2. **El harness e2e no aislaba XDG.** Los dos casos compartían remote, luego
   el mismo `project_id`, y **`adopt status` abrió el ledger real del
   desarrollador**: `database is locked` y escritura en
   `~/.local/share/sddk/`. Ahora `run()` fija `XDG_DATA_HOME`/`XDG_STATE_HOME`/
   `XDG_CACHE_HOME` dentro del sandbox. Los e2e previos sólo llamaban a
   `project resolve`, que no abre nada, así que el defecto llevaba tiempo
   latente sin molestar a nadie.

## §6 Perfil de verificación

```text
cargo test -p sddk-cli --lib                        859 passed; 0 failed; 1 ignored
cargo test -p sddk-cli --test project_pin_e2e       4 passed;  0 failed   (2 nuevos)
cargo test -p sddk-engine --test adoption_identity  3 passed;  0 failed   (2 nuevos)
cargo test -p sddk-engine -p sddk-cli               todo verde
cargo fmt --all -- --check                          limpio
cargo clippy -p sddk-engine -p sddk-cli --all-targets -- -D warnings   exit 0
cargo test --workspace                              5242 passed; 0 failed (272 targets)
```

**El perfil completo cuadra con aritmética, no con impresión:** session-62 midió
5237 y esta slice añade **exactamente 5** tests → 5242. Si el número no
cuadrase, habría que desconfiar de la métrica antes que del resultado.

**Tests nuevos, cinco**, y cada uno cubre un resolver que antes no tenía
ninguna prueba con pin:

| Test | Resolver que fija |
|---|---|
| `pinned_project_id_wins_over_remote_derivation` | `plan_adoption` |
| `malformed_pin_fails_closed_instead_of_falling_back_to_the_remote` | `validate_plan_input` |
| `pinned_identity_is_authoritative_for_adopt_config_and_cycle` | `prepare_adoption_plan` + inferencia de ciclo |
| `unpinned_checkout_still_derives_from_the_remote` | no-regresión del caso normal |
| `resolve_project_ids_honors_the_pin` | `resolve_identity_honoring_pin` |

El cuarto es el que protege el 99% de los checkouts: un fix que alterase el
camino sin pin pasaría el tercero y rompería el cuarto.

## §7 Lo que NO se arregla aquí

La otra mitad de INC-DEBT-049 sigue **abierta**: `adopt status` y `cycle status`
**no declaran** que existe historial bajo otra identidad con el mismo
`vault_path`. Ahora reportan el proyecto correcto, pero un checkout **sin pin**
que se re-adopte seguirá reportando `complete` sobre un storage vacío sin
avisar.

Eso decide un **contrato de estado** — ¿estado nuevo, `complete` pasa a warning,
o la advertencia sólo en `cycle status`? — y cambia lo que `sddk-cycle-resume` y
`sddk-debt-verify` pueden asumir. **SCOPE + ADR**, no una slice.

## §8 Estado del release y nota de trazabilidad

v2.5.0 sigue **BLOQUEADO** por `x86_64-linux-musl-gcc` ausente. Nada publicado,
ningún tag nuevo.

El binario instalado (`sddk 2.4.2`) **no contiene este fix**, así que el pin
sigue sin surtir efecto en la CLI del PATH hasta que se reconstruya e instale.
Eso es lo que hace el release bloqueado con consecuencias reales: **la autoridad
no ve su propia historia porque el binario que la implementa no se ha
republicado.**

Evidencia de esta slice medida sobre el árbol de trabajo de esta sesión;
comprobable con `git status --short` y el recibo.
