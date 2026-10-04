---
id: INC-DEBT-069
title: "/tmp es un tmpfs con tope de 48 GB que los sandboxes de los tests llenan, y cuando se llena la suite y la release mueren con ENOSPC sin decir que el disco es la causa"
status: resolved
severity: medium
priority: P2
fingerprint: "tmpfs_full_breaks_suite_and_release_without_saying_why"
fingerprint_aliases: []
cluster_id: CL-ENV-PREMATURE-FAILURE
created: 2026-10-04
created_by: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
owner: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
detected_at: 2026-10-04
detected_in_session: session-79
component: tests
surface: crates/*/tests/*.rs (sandboxes) y scripts/release.sh (paso 1)
related: [INC-DEBT-068, INC-RELEASE-TAG-FIX]
references:
  - tests/test_release_state_pointer.sh
  - crates/sddk-cli/tests/cycle_attention_enumeration.rs
  - scripts/lib/release_diagnostics.sh
  - tests/test_release_diagnostics.sh
  - tests/test_release_diagnostics_mutation.sh
  - tests/test_release_diagnostics_wiring.sh
---

## Que es

Los sandboxes que crean los tests viven bajo `/tmp`, y **`/tmp` es un tmpfs con
tope de 48 GB, no el disco**. Varios de esos sandboxes **no se limpian al
terminar el test**, luego el uso crece con cada corrida hasta que el tmpfs se
satura.

Cuando se satura, la suite falla con:

```
error: Disk quota exceeded (os error 122)
```

y **los tests que caen no son los que estan rotos**: son los que escriben.

## MEDIDO, session-79

La suite completa dio **1** test rojo, y al relanzarla **6**, todos en el mismo
sitio y con el mismo motivo: `cli_full_runtime_pipeline_dogfood`,
`cli_dev_install_accepts_committed_manifest`,
`cli_dev_install_default_layout_is_executable_and_verify_passes`,
`cli_dev_install_suffix_check_is_exact_match`,
`cli_dev_install_with_bin_suffix_avoids_nesting_and_uses_executable_mode`,
`uninstall_removes_prefix_and_editor_symlinks`, `verify_detects_tampered_bundle`.
**Ninguno toca el codigo de session-79**, y el primero **pasaba en aislamiento**:
`cargo test --exact cli_full_runtime_pipeline_dogfood` -> `1 passed`, mientras el
mismo test dentro del binario de 195 caia. Eso es lo que hace el diagnostico
enganoso: **un test que falla en el suite y pasa solo no esta roto, se ha
quedado sin espacio**, y la conclusion opuesta —"ha roto el ultimo cambio"— es la que
tienta.

Causa, medida y no supuesta:

| Medida | Valor |
|---|---|
| `/tmp` | **tmpfs, tope 48 G, al 100 %** |
| `/var/home` (disco real) | 269 G **libres** |
| DIRECTORIOS `mktemp` huerfanos | **144**, **27 GB** |
| Lote liberado (>6 h, ninguno abierto por `lsof`) | 92 directorios, **20.3 GB** |
| `/tmp` despues | de 79 % a **37 %** |
| Suite tras liberar | **286 bloques, 0 FAILED, 5468 pasados, 24 ignorados** |

Que `/var/home` tuviera 269 G libres mientras el error decia "Disk quota
exceeded" es la parte que hace el diagnostico fallar: **el mensaje blames al
disco y el culpable es el tmpfs**, y no hay forma de saberlo sin medir las dos
cosas por separado.

## Por que es `medium` y no `high`

El fallo es **ruidoso y fail-closed**: la suite se pone roja y la release muere
antes de publicar, sin estado parcial. No corrompe nada ni deja una version
mentirosa instalada.

Tampoco es `low`, por dos razones medidas y no supuestas:

1. **El coste es de diagnostico, no de fallo.** Una sesion entera se fue en
   medir si el cambio habia roto algo. La regla del repo —"un test rojo no es
   motivo para parar: investiga"— es correcta y aqui se pago cara, porque la
   causa no aparece en ningun mensaje.
2. **El pipeline no lo detecta antes de gastarlo.** El paso 1 de `release.sh`
   corre la suite completa **antes** de ningun paso que pueda avisar, luego el
   fallo llega tras el build y la compilacion de todos los targets.

## Lo que esta medicion NO dice

**No todos los sandboxes fugan, y no se afirma que lo hagan.** De los ficheros
de test que declaran `struct Sandbox`, **9 tienen `impl Drop` con limpieza**
—entre ellos el que este bloque anadio. Los 144 huerfanos son de procesos y
helpers cuyo `Drop` no corre: un panic, un `std::process::exit`, o un test que
crea el directorio y nunca lo borra. **Cual es la poblacion exacta no esta
medida**, y por eso este documento no propone un arreglo con su lista de
ficheros.

**No es el OOM del backlog P1** (`bl-bl-01M42JGYG4000388551BF9NZ40`), que es
memoria. Es la misma **familia**: el release depende del estado de una maquina
que no es el repo, y por eso muere en el paso mas temprano y sin diagnostico
propio. Ese backlog ya tenia una segunda via de muerte medida —el
`CARGO_TARGET_DIR` compartido con otra maquina, confirmado en vivo otra vez en
session-79— y esta es una **tercera via, distinta**: no contends disco para
escribir, sino que el sitio donde se escribe tiene un tope que nadie vigila.

## Que habria que decidir (no se decide aqui)

Hay dos caminos y son de owners distintos, asi que **no se abre ninguno**:

1. **Que los sandboxes limpien solos.** Es un cambio en los tests, con su
   Instrumento y sus propios casos. Requiere saber antes cuales fugan, y esa
   medicion **no esta hecha**.
2. **Que el pipeline avise antes de gastar el build.** Un chequeo de espacio
   libre al principio del paso 1 convertiria un fallo de ENOSPC sin pistes en un
   fallo que nombra la causa. Es mas pequeno, y no depende de (1) para ser
   util: sigue avisando aunque algo else's sandbox fugue.

Se deja escrito el porque de no elegir: **los dos son trabajo con su propio
alcance**, y elegir aqui seria abrir un frente que este bloque no cierra.

## Cierre

Cierra cuando (1) los sandboxes que fugan limpien al terminar —con la poblacion
medida antes, no supuesta— **o** (2) el paso 1 nombre la causa antes de empezar
a compilar. **No cierra por antiguedad**: cierra con un comportamiento, y en ese
momento un `os error 122` en la suite habra dejado de ser un misterio.

## RESUELTA — session-80, por el criterio (2) que la propia deuda declaraba

Se implemento el camino 2. **No es una reinterpretacion del criterio**: el
documento decia "o (2) el paso 1 nombre la causa antes de empezar a compilar", y
eso es exactamente lo que ahora hace el paso 0.

`scripts/lib/release_diagnostics.sh` + el cableado en `release.sh`:

- **El margen se mide antes de gastar.** `release_check_resources` mira el disco
  del scratch y la memoria disponible, y si no dan **falla cerrado nombrando el
  recurso, el punto de montaje y las dos cifras** (la que hay y la que se
  exige). Fail-closed porque seguir sin margen solo convierte un fallo en un
  fallo distinto.
- **La causa tiene nombre.** `cause_of_exit_code` traduce el codigo a su causa:
  122 es ENOSPC, 137 es SIGKILL/OOM, 126/127 es "no encontrado", y un codigo
  **desconocido declara que no se conoce** en vez de inventar una — una causa
  inventada dirige la investigacion a un sitio falso.
- **El bloque sale aunque el fallo no pase por `die`.** Un unico manejador de
  salida (`release_on_exit`) diagnostica y limpia, con marcador de idempotencia
  en disco porque `die` y el manejador viven en shell distinto y con una
  variable el bloque salia dos veces.

**MEDIDO, no supuesto:**

| Medida | Valor |
|---|---|
| Casos del guard de la libreria | **42**, `PASS=42 FAIL=0` |
| Casos del guard de cableado (release real) | **24**, `PASS=24 FAIL=0` |
| Mutaciones que tienen que caer | **9**, `PASS=9 FAIL=0 SKIP=0` |
| `die` que ahora pasan por el diagnostico | los **73** (antes 32 sin causa) |

El caso E1 del guard de cableado **ejecuta `release.sh --dry-run` de verdad** con
un margen imposible y comprueba que el release para nombrando el recurso; E2 lo
ejecuta con margen y comprueba que el preflight se supera. E2 es lo que hace que
E1 no sea una puerta trasera.

### Lo que NO cierra esto, escrito para que no se lea al reves

1. **Los sandboxes siguen sin limpiarse solos.** El camino (1) no se ha tocado:
   sigue sin medirse que poblacion exacta fuga. Ya no bloquea, porque (2) se
   cumple, pero la causa de origen —el `Drop` que no corre tras un panic o un
   `exit`— sigue viva y volvera a llenar el tmpfs.
2. **La suite suelta tampoco lo dice.** Correr `cargo test --workspace` sin
   pasar por `release.sh` sigue sin preflight. Este arreglo vive en el release,
   que es donde estaba el hueco del release.
3. **Un SIGKILL del proceso padre sigue sin diagnosticar.** No hay manejador que
   se ejecute. Lo que se sabe es que el log **si** dice en que paso fue
   (MEDIDO: `step()` escribe con `printf` directo a fd 1 y bash no bufferiza, luego
   el marker esta en disco), y que (2) evita llegar a esa situacion por falta de
   espacio. Lo que no se puede es diagnosticar la muerte en si.
