---
id: INC-DEBT-077
title: "el censo de guards depende del glifo test_ en el nombre: seis guards de integracion de tests/ que NADIE ejecutaba eran invisibles para el gate, y uno se colaba porque ci.yml lo nombra"
status: resolved
severity: medium
priority: P2
fingerprint: "guard_census_depends_on_test_prefix_in_filename"
fingerprint_aliases: []
cluster_id: CL-AUTHORITY-SPLIT
created: 2026-10-05
created_by: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
owner: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
detected_at: 2026-10-05
detected_in_session: session-84
resolved_at: 2026-10-05
component: release pipeline / cobertura de gates
surface: tests/test_gate_coverage.py, tests/test_gate_coverage_ci_mutation.py, tests/uat_ctx_*.sh
related: [INC-DEBT-076, INC-DEBT-055, INC-DEBT-064]
references:
  - tests/test_gate_coverage.py
  - tests/test_gate_coverage_ci_mutation.py
  - tests/uat_ctx_001_adoption_convergence.sh
  - tests/uat_ctx_002_context_bootstrap.sh
  - tests/uat_ctx_003_durable_deltas.sh
  - tests/uat_ctx_004_cycle_inference.sh
  - tests/uat_ctx_005_explicit_cycle_migration.sh
  - tests/uat_ctx_006_skill_runtime_alignment.sh
  - tests/uat_ctx_007_context_expand.sh
  - tests/lib_public_release_gate.sh
  - tests/ext_provider_gate.sh
  - scripts/release.sh
---

# INC-DEBT-077 — RESUELTO — el censo de guards ya no depende del nombre del fichero

## Que se midio

`tests/test_gate_coverage.py` —el guard que existe precisamente para que ningun
test quede sin runner— censaba `tests/test_*.sh` y `tests/test_*.py`. **El glifo
`test_` en el nombre era la unica condicion de entrada.**

Hay diez ejecutables en `tests/` que no casan. Seis de ellos son guards de verdad,
y NADIE los ejecutaba: ni el 1b, ni `ci.yml`, ni ningun otro script.

| guard | red | contenedores | estado real MEDIDO |
|---|---|---|---|
| `uat_ctx_001_adoption_convergence.sh` | 0 | 0 | rc=2, `sddk binary not found` |
| `uat_ctx_002_context_bootstrap.sh` | 0 | 0 | idem |
| `uat_ctx_003_durable_deltas.sh` | 0 | 0 | idem |
| `uat_ctx_004_cycle_inference.sh` | 0 | 0 | idem |
| `uat_ctx_005_explicit_cycle_migration.sh` | 0 | 0 | idem |
| `uat_ctx_006_skill_runtime_alignment.sh` | 0 | 0 | idem |
| `uat_ctx_007_context_expand.sh` | 0 | 0 | idem |

**No estan rotos.** Los seis aceptan `--bin <ruta>` y con un binario presente
ejercitan de verdad. Lo que no habia era quien los ejecutara.

## Por que no pueden estar en el 1b, y por que ese motivo NO es "no son hermeticos"

Lo natural es decir «son integracion, no hermeticos, se exceptan». **Eso seria
falso por la razon que importa.** Son tan hermeticos como cualquier otro: 0 red,
0 contenedores.

La razon real es **ORDEN**. Los seis exigen el binario release, y el binario lo
construye el release en el paso **3**, mientras el 1b corre en el **1b**: en ese
punto aun no existe. Es la misma condicion que ya arrastra
`test_h05_isolation.sh`, cuya excepcion dice «requiere `cargo build --release`
antes». Se exceptan con ese motivo, no con uno inventado.

Consecuencia que se declara: el sitio natural de estos seis es un paso
posterior al 3. **Cablearlos ahi es la decision pendiente, no un detalle de este
commit**, y no se hace aqui porque exigiria tocar el orden del release con un
bloque que ya esta publicado.

## El sexto que se habria colado

`uat_ctx_004_cycle_inference.sh` lo ejecuta `ci.yml`. Es decir, el gate lo
contenia como cubierto y el otro gate lo senalaba como «su unico runner es
`.github/workflows/`», que segun AGENTS.md 2.5 no bloquea. **Los dos gates
tenian razon y se contradecian**, porque uno mira mencion y el otro ejecucion.
Es exactamente el patron que INC-DEBT-076 cerro, agregado por un camino que el
mismo commit abria.

## Lo que no es un guard, y la distincion que hace falta

De los diez fuera del censo, cuatro no son guards, y llamarlos guard seria el
mismo error al reves:

- **`lib_public_release_gate.sh` es una LIBRERIA.** Define
  `run_public_release_gate` y la sourcean `scripts/release-assets-contract.sh` y
  `tests/test_release_public_gate.sh`. No tiene `main`: ejecutarla no hace nada.
- **`ext_provider_gate.sh` es un LAUNCHER de frontera MCP_EXTERNAL.** Sin
  argumentos sale 3 (`not_run`); con un provider ausente sale 2
  (`blocked_external_dependency`), que **por contrato nunca es pass** —cablearlo
  al 1b mataria toda release en una maquina sin `chronos-mcp`. Su otra mitad, el
  enum de estados, ya esta pineado por un test de produccion
  (`ext_outcome_states_match_the_launcher_contract`).
- **`falsify-ci-anchor-real.sh`** requiere tag publicado y red.
- **`clean_machine_uat.sh`** monta contenedores de verdad (39 llamadas a
  `docker`/`podman`), y lo ejecuta su propio workflow por tag. Segun AGENTS.md
  2.5 es evidencia asincrona **por diseno**, no un hueco.

## Que se cambia

- El censo pasa a ser **todo `.sh` y `.py` de `tests/`**, y lo que no es un guard
  se declara en `NOT_GUARDS` con su motivo. Un censo por prefijo de nombre es un
  censo que hay que actualizar cuando alguien nombra un guard `uat_*` en vez de
  `test_*`, y ese recordatorio es justo lo que falla.
- Los seis `uat_ctx_*` pasan a `EXCEPTIONS` con el motivo de orden medido.
- **Regla 5**, que mantiene `NOT_GUARDS` honesta con las tres mismas condiciones que `EXCEPTIONS`.

Resultado: el censo pasa de **83 a 91** tests, con **0 sin runner y sin motivo**,
frente a los 6 que el gate anterior no podia ni ver.

## Falsacion

`tests/test_gate_coverage_ci_mutation.py`, `PASS=33 FAIL=0 SKIP=0`.

- **M8** — volver al censo por prefijo de nombre: rojo, y el rojo es el de la
  Regla 1 acusando a excepciones de apuntar a ficheros que **si existen**.
- **M9** — volver a filtrar por `os.X_OK`: rojo, y acusa a
  `test_release_routes_parity.sh` de no existir.
- **M10a/M10b/M10c** — desactivar cada regla de `NOT_GUARDS`: **NO da rojo**, y la
  asercion afirma lo contrario del resto del fichero a proposito.

## Dos instrumentos que se encontraron fallando a si mismos

1. **La v1 del cambio filtro el censo por `os.X_OK`**, con lo que
   `test_release_routes_parity.sh` y `test_release_bundle_parity.sh` —que
   existen, estan en EXCEPTIONS y van en 644— desaparecian y la Regla 1 los
   acuso de «el fichero no existe». **Existir y tener bit son dos preguntas**, y
   la que importa —enumerado sin bit se salta en silencio— es la Regla 3, que ya
   existe. Un filtro que responde dos preguntas produce un rojo que parece de la
   propiedad y es del instrumento.
2. **La v1 de M10 sustituia la linea de la regla por `pass`**, y las tres
   mutaciones dieron `NameError` o `IndentationError`: el gate caia por un error
   de sintaxis y la asercion lo contaba como prueba. Un rojo de la herramienta no
   es un rojo de la propiedad. Y la v1 de M10a era peor —reasignar `NOT_GUARDS`
   dentro de `main()` la vuelve local y rompe el fichero entero con
   `UnboundLocalError`—, de modo que M10a apunta a la regla de obsolescencia, que
   es la que de verdad tiene que estar vigilada.

## Lo que se declara sin cerrar

- **El paso que ejecutaria los seis `uat_ctx_*` no existe.** Es la decision que
  queda, y depende de donde quepa un binario release sin que el release pague dos
  builds.
- **`NOT_GUARDS` no esta vigilada de hecho.** M10 lo demuestra: desactivar sus
  tres reglas deja el gate en verde. Se conservan porque una lista de «esto no es
  un guard» sin reglas que la examine es un cajon de sastre con mejor
  vocabulario, y queda dicho que hoy no vigila para que nadie lo lea como si
  vigila.
- **La superficie de gates del 1b sigue siendo una lista escrita a mano** (esto
  es INC-DEBT-076, y sigue igual). Este commit solo garantiza que lo que hay
  dentro de `tests/` se ve; no cambia quien lo ejecuta.
