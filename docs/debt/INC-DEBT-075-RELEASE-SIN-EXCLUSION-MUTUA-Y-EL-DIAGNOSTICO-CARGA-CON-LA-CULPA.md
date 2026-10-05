---
id: INC-DEBT-075
title: "release.sh no tiene exclusion mutua, y el diagnostico de retencion atribuye a un cargo un target dir que no era suyo: dos releases del mismo tag podian coexistir, y el aviso que deberia haberlo detenido senalaba el directorio equivocado"
status: resolved
severity: high
priority: P1
fingerprint: "release_has_no_mutual_exclusion_and_retention_diagnostic_blames_the_product_for_real_contention"
fingerprint_aliases: []
cluster_id: CL-AUTHORITY-SPLIT
created: 2026-10-05
created_by: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
owner: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
detected_at: 2026-10-05
detected_in_session: session-82
resolved_at: 2026-10-05
component: release pipeline / diagnostico
surface: scripts/release.sh, scripts/lib/release_exclusion.sh, scripts/lib/release_diagnostics.sh, tests/test_release_diagnostics.sh
related: [INC-DEBT-073, INC-DEBT-074]
references:
  - scripts/release.sh
  - scripts/lib/release_exclusion.sh
  - scripts/lib/release_diagnostics.sh
  - tests/test_release_diagnostics.sh
  - tests/test_release_exclusion.sh
  - tests/test_release_exclusion_mutation.sh
  - tests/test_cargo_target_attribution.sh
---

# INC-DEBT-075 — RESUELTO — dos releases del mismo tag ya no pueden convivir, y el aviso de retencion se refiere al target dir que el cargo USA

## EL ENCUADRE INICIAL ERA PARCIALMENTE FALSO, y se corrige aqui

Este documento se abrio diciendo que el defecto era «el supuesto del test C9».
**No lo era, o no solo**: el diagnostico moria sobre QUE target dir. Las dos
sesiones que tocaron el fallo concluyeron, con razon, que el diagnostico no
mintio porque la contencion era real. Cierto. Lo que ninguna de las dos midio
es sobre que target decia la contencion.

## Que se midio

**MEDIDO, no hipotetico.** Convivieron dos ejecuciones del release sobre el
mismo checkout, el mismo HEAD y la misma version:

| pid | sesion | log | PPID |
|---|---|---|---|
| 38340 | session-82 | `release-2110-try4.log` | 26812 |
| 99022 | session-83 | `/tmp/rel-2.11.0.log` | 3001 (`systemd --user`) |

El `cargo test --workspace --offline` de la otra sesion (pid 100388) retuvo el
`CARGO_TARGET_DIR` compartido y las 4 aserciones de C9 cayeron.

**La DIRECCION, que dos sesiones pusieron al reves**: el C9 que cayo fue el de
session-82 —el pid aparece 5 veces en su log y **0** en el de la otra sesion—,
el cargo retenedor era de la OTRA sesion (`PPID 100388 = 99022`), y el release
de session-82 **no sobrevivio**: pid 38340 no existe y su log cierra
`RELEASE_RC=1` a las 09:31:22. El C9 de la otra sesion pasa (`PASS=20 FAIL=0`).

No era un huerfano de una sesion anterior: eran dos ejecuciones simultaneas e
independientes del mismo tag, y **nadie lo impidio**.

## El defecto del diagnostico, que es el que hacia que C9 cayera

`_cargo_uses_target` tenia un tercer criterio escrito en prosa: *«un cargo
lanzado aqui sin declarar target compila donde le digan, **que suele ser
este**»*. Ese «suele» es una suposicion, y se MIDIO que es falsa:

    ~/.cargo/config.toml -> [build] target-dir = "/var/home/rubentxu/cargo-targets"

O sea que **todo** proyecto Rust del host compila en un unico target
compartido. Un `cargo` con nuestro mismo cwd y sin `CARGO_TARGET_DIR` puede
estar compilando AHI, que no es el target que se le preguntaba — y el aviso
decia «el target dir compartido esta retenido: pid N» sobre un pid que no
esperaba ese target.

Un aviso que se refiere a otro directorio es peor que no avisar, porque se lee
como un dato sobre el release en curso.

## Que se resolvio, con las tres salidas

1. **El aviso se refiere al target que el cargo USA.** Se le pregunta a
   `cargo metadata` que target resolveria para ese cwd, y solo se cuenta si
   coincide. **No se reimplementa la resolucion de cargo**, porque
   `build.target-dir` puede cambiar y una copia de esa regla se quedaria vieja
   sin que nadie lo note. La severidad NO cambia: sigue siendo un aviso y no una
   muerte, porque dos proyectos distintos compartiendo target dir es el uso
   normal de una maquina de desarrollo.

2. **Exclusion mutua** en `release.sh`, por repo **y version**. El segundo
   intento ABORTA nombrando el pid del primero. Sin directorio de candados
   ABORTA, que es distinto de avisar. El `--dry-run` no toma candado, porque no
   publica y no puede pisar a nadie.

3. **Huerfanos**: se resuelve por la RECUPERACION del candado huerfano, que se
   DECLARA en vez de bloquear para siempre. Un candado cuyo dueno murio —con o
   sin su `trap`, por ejemplo al apagarse el host— se recupera y se dice que se
   hacia y con que pid.

## LO QUE NO SE TOCO, y por que la distincion es el Nucleo

`release.sh` ya avisaba de la retencion del target dir, y con buen criterio. Se
separa en dos casos que no son el mismo:

    dos proyectos distintos en un target dir compartido  -> AVISAR (correcto)
    dos releases del MISMO repo y la MISMA version      -> ABORTAR

El segundo tiene consecuencia destructiva: los dos podrian llegar al paso 9 y
escribir el mismo artefacto, o uno sobrescribir al otro con `--force`.

## GATES

    tests/test_cargo_target_attribution.sh     PASS=5  FAIL=0
    tests/test_release_exclusion.sh            PASS=17 FAIL=0
    tests/test_release_exclusion_mutation.sh   PASS=10 FAIL=0 SKIP=0
    tests/test_release_diagnostics.sh          PASS=43 FAIL=0
    tests/test_release_diagnostics_mutation.sh PASS=20 FAIL=0 SKIP=0
    shellcheck SIN filtro sobre los 5 tocados   0 avisos

Dientes verificados por medicion y no por argumento: contra la rama 3 sin el
arreglo, `test_cargo_target_attribution.sh` da `PASS=4 FAIL=1` y `RESULT: FAIL`,
con su control de deteccion real (E3) en verde — o sea cae por SU comprobacion
y no porque este roto.

## LO QUE NO SE HA MEDIDO, y se declara

- **NO se ha medido que dos releases lleguen ambos al paso 9.** La ventana
  existia; el incidente no. La 2.11.0 se publico bajo concurrencia y se
  verifico igual contra API y CDN —9 assets, `draft=false`,
  `prerelease=false`, sha256 declarado == real—, que es la comprobacion que
  toca cuando se publica bajo un estado no limpio.

- **La exclusion tiene una ventana declarada**: dos procesos que vean el MISMO
  candado huerfano a la vez pueden escribir encima los dos. Exige que el dueno
  previo muera en ese instante, y el caso que de verdad importa —dos releases
  VIVOS— no la comparte, porque ahi el pid esta vivo y nadie escribe encima.
  Esta escrito en la libreria, no escondido aqui.

- **Un flake preexistente de C9 NO se ha cerrado.** MEDIDO con dos campanas
  paralelas de 20 corridas cada una: con el arreglo **1/20**, con la libreria
  intacta **2/20**. O sea que es **preexistente** y el cambio no lo empeora, pero
  su causa **NO esta establecida**. `obtenido` es una ruta del propio root del
  test, lo que apunta a una salida que se filtra dentro del `$( )` del caso, y
  no se ha medido cual. No se toca el codigo de ese test sin reproducirlo.

- **La exclusion se entrego en un checkout COMPARTIDO con otra sesion**, que
  revirtio `scripts/lib/release_diagnostics.sh` mientras se escribia. MEDIDO: el
  arreglo se perdio una vez y hubo que reaplicarlo. Es la TERCERA vez que la
  misma clase aparece en este bloque —el guard que copiaba el codigo, el test
  que copiaba el script, y ahora el arbol que se comparte— y la leccion
  operativa es la misma: **lo que no esta commiteado no existe**, ni siquiera
  durante veinte minutos.
