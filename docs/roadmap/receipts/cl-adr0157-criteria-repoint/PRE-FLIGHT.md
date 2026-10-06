# SDDK PRE-FLIGHT — `cl-adr0157-criteria-repoint`

> Emitido **antes** de tocar el fichero del gate, conforme a `/home/rubentxu/AGENTS.md` §2.

| Campo | Valor |
|---|---|
| **Project** | `p-63676b11dc0ef88f` |
| **Goal** | Que `REL-2.12.1` pueda publicarse. Hoy **no puede**: el release muere en el 1b por este gate. |
| **Milestone** | release 2.12.1 (M7, `agent-session start` → `authority=sddk`) |
| **WorkItem** | Defecto detectado durante la corrida de `REL-2.12.1` (`RELEASE_RC=1`, paso `1b/15`). Sin WorkItem propio: es un **bloqueo de la entrega**, y por eso se atiende antes que C5. |
| **HEAD** | `f985f119` |
| **Objective** | Repuntar los dos criterios de ADR-0157 que nombran tests que ya no existen, a los tests que `e60dae75` puso en su lugar. |
| **Definition of Done** | 1) `tests/test_adr_0157_criteria.sh` da `NOT_APPLIED=0` y sale 0. 2) Los dos tests destino existen **y se ejecutan** (no basta con existir: el gate distingue). 3) El cambio queda en un commit atómico. 4) `REL-2.12.1` vuelve a correr y pasa el 1b. |
| **Required inputs** | `tests/test_adr_0157_criteria.sh`, `crates/sddk-domain/tests/kernel_purity_fitness.rs`, `git log -S` del nombre desaparecido. |
| **Known decisions** | **No se toca el gate.** El gate hizo su trabajo: se negó a certificar porque dos criterios nombraban tests inexistentes. El defecto es de quien movió los tests y no repuntó el ADR. Un gate al que se le ablanda el umbral para que pase es peor que un release parado. |
| **Known blockers** | Ninguno. El árbol está libre: `REL-2.12.1` murió con `RELEASE_RC=1`. |
| **Missing information** | Si `el_nucleo_no_nombra_tecnologia_concreta` cubre **exactamente** la propiedad de C1 o es más estrecha. Se comprueba leyendo el test, no se supone. |
| **Next executable task** | Editar las dos líneas `criterion` de `sddk-domain` y re-ejecutar el gate. |
| **Readiness** | **READY** |

---

## Lo medido, y por qué el release murió

```
REL-2.12.1 · paso 1b/15 · RELEASE_RC=1
tests/test_adr_0157_criteria.sh → PASS=24 FAIL=0 NOT_APPLIED=2
  RESULT: FAIL — ADR-0157 no cumple todos sus criterios; NO se sostiene como accepted
```

**`FAIL=0` y aun así falla**, que es la parte importante: no hay ni una aserción
rota. Fallan dos criterios porque **no se aplicaron**, y el gate es explícito en
que un criterio no medido no está cumplido (no se sabe). Es la disciplina de
NOMBRAR ≠ EJECUTAR aplicada al propio gate de ADR-0157.

Los dos, y solo los dos, son del lado `sddk-domain`:

| Criterio | Test nombrado | Qué pasó |
|---|---|---|
| C1 | `the_decision_module_names_no_concrete_technology` | renombrado en `e60dae75` a `el_nucleo_no_nombra_tecnologia_concreta` |
| C2 | `the_fitness_scanner_can_actually_see_a_name` | ya no existe en `sddk-domain`; queda en `sddk-engine`, y el contrapeso de dominio pasó a ser `el_guard_cubre_todos_los_modulos_del_dominio` |

**Una sola causa detrás de los dos:** `e60dae75` (*"la ley de pureza del núcleo se
vigila en los cuarenta y seis módulos, no en uno"*) consolidó los dos tests de
pureza del dominio, y **nadie repuntó los criterios del ADR**. El criterio C2
sigue declarando el test de `sddk-domain` **y** el de `sddk-engine`; el segundo
existe y pasa, luego la mitad de motor de C2 nunca estuvo rota.

## El precedente que este bloque sigue

C4 (líneas 128-135 del propio gate) ya documenta el mismo problema y cómo se
resolvió: la fila `lockstep_passes_when_the_ref_has_no_v_prefix` «FE con
ADR-0159, y no por un descuido», porque fijaba una convención que ya no existe.
Y la conclusión escrita ahí es la que se aplica aquí:

> *Repuntar el criterio a un test que dijera lo contrario habría sido dejar el
> gate verde sobre una ley que ya no existe, que es peor que un N/A.*

Por eso este bloque repunta **a los tests que miden lo mismo o más** —el de
pureza ahora cubre 46 módulos, no uno— y **no** baja el listón del gate.

## Lo que este bloque NO hace

- **No toca `criterion()`**, ni el tratamiento de `NOT_APPLIED`, ni el umbral.
  El mecanismo que ha detectado esto se queda intacto a propósito.
- **No reconstruye los tests antiguos.** Si la consolidación hubiera dejado la
  propiedad sin cubrir, el arreglo sería repuntar a un test que no la cubre, que
  es exactamente lo que el precedente de C4 prohíbe.
- **No re-lanza el release sin verificar el gate antes.** Primero `NOT_APPLIED=0`,
  después `scripts/release.sh`.
