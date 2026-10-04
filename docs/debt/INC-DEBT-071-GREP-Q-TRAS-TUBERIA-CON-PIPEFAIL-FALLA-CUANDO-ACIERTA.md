---
id: INC-DEBT-071-GREP-Q-TRAS-TUBERIA-CON-PIPEFAIL-FALLA-CUANDO-ACIERTA
status: resolved
severity: high
priority: P1
cluster_id: CL-VERIFY
detected_at: 2026-10-04
detected_by: release 2.9.0, murine en el 1b
detected_in_session: session-80
resolved_at: 2026-10-05
resolved_in_session: session-81
resolved_by: 21 sitios arreglados + tests/test_grep_q_after_pipe.py + su autofalsador
fingerprint: "grep_q_after_pipe_with_pipefail_reports_writer_failure_on_a_hit"
fingerprint_aliases: []
related: [INC-DEBT-054, INC-DEBT-053]
---

# `grep -q` tras una tubería, con `pipefail`, falla **cuando acierta**

## El defecto

Con `set -o pipefail`, un pipeline cuyo **lector** sale pronto hace que el
pipeline devuelva el fallo del **escritor**:

```sh
set -o pipefail
sed -n '527,706p' release.sh | grep -q 'git merge-base --is-ancestor'
```

`grep -q` sale en cuanto casa. `sed` todavía tiene ~180 líneas por escribir,
recibe SIGPIPE y muere con 141. `pipefail` propaga ese fallo, luego el `if !`
entra por la rama de "no lo encontré" **aunque lo haya encontrado**.

El resultado no es un falso negativo siempre: depende de si el escritor ha
terminado cuando el lector cierra. Con entradas pequeñas `sed` suele acabar
antes y no pasa nada; con una slice grande gana la carrera y el needle falla.
**Por eso el mismo aserto puede dar verde y rojo sin que cambie nada.**

## MEDIDO, no supuesto

El release 2.9.0 murio en el paso 1b:

```
FAIL (e): step 1c does not use merge-base ancestor check
        Without that check, concurrent advances are not detected
        and the script may silently push over a faster remote.
```

`tests/test_release_tag_anchoring.sh` calculaba **los mismos limites** en los
dos sitios: 1c en la linea 527, siguiente paso en la 707, y pasaba (a), (b), (c)
y (d). Solo (e) caia, y solo dentro del release. Al correr el fichero aislado
daba **5 de 5**. El log del release lo deja escrito:

```
step 1c at line: 527
next step after 1c at line: 707 (semantic step 2)
...
PASS (d): step 1c delegates the predicate to the pre-push hook
FAIL (e): step 1c does not use merge-base ancestor check
```

El defecto **ya estaba escrito** en el `CHANGELOG` de la 2.8.0, con su
arreglo, porque se escribio al colisionar contra un needle del guard de diagnostico:
"`codigo | grep -q` con `set -o pipefail` falla **cuando acierta** [...] `grep -c`
lee el flujo entero y no sufre eso". Lo que faltaba era aplicarlo al resto del
arbol, y el release lo pago.

## Alcance medido

`grep -rlE '\|[[:space:]]*(grep|rg)[[:space:]][^|]*-q'` sobre `tests/`,
`scripts/` y `githooks/`, cruzando con los ficheros que activan `pipefail`:
**35 ficheros**, con esta cuenta de sitios:

| Ficheros | Sitios |
|---|---|
| `tests/test_release_diagnostics_wiring.sh` | 6 |
| `tests/test_release_sign_artifacts.sh` | 6 |
| `tests/test_release_ci_manifest_anchor.sh` | 5 |
| `tests/test_release_public_gate.sh` | 5 |
| `tests/test_vault_coherence_alignment.sh` | 5 |
| `tests/test_vault_mirror_auto.sh` | 5 |
| `tests/test_adr_promotion_format.sh` | 4 |
| `tests/test_release_admission.sh` | 4 |
| `tests/test_release_bundle_layout.sh` | 4 |
| `tests/test_release_pipeline_consistency.sh` | 4 |
| `scripts/release-bump.sh` | 4 |
| `scripts/release.sh` | 6 |
| otros 23 ficheros | 1-3 cada uno |

**No todos estan rottenos.** El fallo necesita que el escritor no haya
terminado cuando el lector cierra, luego el riesgo escala con el tamano de lo
piped. Los de entrada grande son los peligrosa; los de un `echo | grep -q` de
una linea son inocuos. Por eso el alcance de arriba es **candidatos**, no
confirmados, y hay que medirse uno a uno antes de tocarlos.

## Lo que ya esta hecho

`tests/test_release_tag_anchoring.sh` esta **arreglado y verificado**: la slice
del paso 1c se lee una sola vez a una variable y se busca DENTRO, sin tuberia.
MEDIDO: 12 ejecuciones seguidas, 5 de 5 cada una. Los seis sitios Affected
quedan en `casan_1c` / `casan_1c_sin`, que imprimen **cuantas** lineas casan y
nunca su codigo de salida, porque el codigo dice si grep fallo, no si el needle
accerto, y no es lo mismo.

## Por que se registra en vez de arreglarse entero

El release estaba en curso y este defecto **solo se manifiesta bajo carga** —no
en un green suite, sino cuando algo mas compite por la CPU, que es justo lo que
hace un release—. Arreglar 35 ficheros a la vez sin poder reproducir la carrera
en condiciones controladas es cambiar mucho para no medir nada. El bloque que
lo pago lo arreglo en el fichero que fallo; el resto se registra con su alcance
para atacarlo con su propia instrumentacion.

## Que habria que hacer para cerrarla

1. Un guard que encuentre `| grep -q` en ficheros con `pipefail` y **falle
   cerrado**, con su lista de excepciones. La lista de excepciones es lo que
   hace que la cuenta no vuelva a mentir: 35 no es 35 defectos, es 35
   candidatos.
2. Para cada candidato, decidir por tamano de la entrada, no por costumbre.
3. Sustitucion mecanica: `| grep -q 'X'` -> `[ "$(printf '%s' "$VAR" | grep -c -- 'X')" -gt 0 ]`,
   con la slice leida a una variable cuando el escritor se repita.

## Lo que NO se afirma

- No se ha reproducido la carrera de forma controlada, solo se ha medido su
  efecto en un release real. Los 35 candidatos estan **medidos como
  presencia**, no como defecto confirmado.
- No se ha tocado ninguno de los otros 34 ficheros. El alcance es real; el
  trabajo no esta hecho.
- El arreglo de `test_release_tag_anchoring.sh` esta verificado por ejecucion
  repetida, que es la unica evidencia disponible para una carrera: 12 de 12 no
  es una prueba de que no vuelva, es una prueba de que ya no se reproduce con
  esta carga.

---

# RESOLUCION (session-81) — la causa era correcta, y el criterio de triaje no

Lo de arriba se conserva como estaba. Es el registro de lo que se creia al
detectar, y **dos de sus afirmaciones estan medidas como falsas**. Se corrigen
aqui y no se borran, porque un documento que solo guarda sus aciertos no
sirve para que el proximo no repita el salto.

## 1. Se intento FALSAR la causa, y no se pudo

Lo primero que hizo este bloque fue intentar tumbarla. **No se pudo, y el
intento fallo tres veces antes de dar con el numero bueno** — que es justo lo
que hace que el numero bueno valga:

| Intento de falsacion | Resultado | Por que estaba mal |
|---|---|---|
| "el caso real tiene 9 625 B, muy por debajo del umbral" | FALSO | medido contra `v2.8.1`, que **no es el arbol que fallo** |
| "el needle no esta en la slice" | FALSO | por lo mismo: con el arbol real (`1c=527`, `next=707`) esta en la linea **535** |
| "el umbral son ~256 KiB" | FALSO | **n=3 por punto**, y la carrera es del orden del 1 %: 3 tiradas no ven casi nada |

La repeticion del metodo es el punto: **el umbral no existe**. Lo que hay es
una probabilidad, y se mide con la muestra que la probabilidad merece.

## 2. MEDIDO, con muestra suficiente

Arbol que fallo: commits `05d4eeb6`..`4e4c0e90`. Slice `527,706` de
`scripts/release.sh`: **8 471 B, 180 lineas**, needle en la linea 8.

| Forma | sin carga | con 32 procesos compitiendo |
|---|---|---|
| la slice real | **2,10 %** (42 de 2 000) | **9,17 %** (55 de 600) |
| el GUARD ENTERO | — | **PASS=37 FAIL=3** en 40 corridas |
| needle al FINAL de la slice | 1,50 % | **8,33 %** |
| 1 linea, escritor externo (`cat`) | 0,00 % | 0,00 % |
| 1 linea, escritor BUILTIN (`printf`) | 0,00 % | — |
| `grep -c` en vez de `grep -q` | **0,00 %** | **0,00 %** |
| el arreglo aplicado (variable + `grep -c`) | 0,00 % | — |

**La causa de este documento era correcta.** El release compila con `cargo`,
o sea con carga, y ahi la tasa por sitio es del **9,17 %**. Con los ~6 sitios
de la clase que hay en un fichero, la probabilidad de que el 1b pasara limpio
era de orden `0,91^6` ~ 57 %. **Esto no era un defecto que "pudiera" tocar la
release: era una cuenta pendiente que solo faltaba que saliera el dia
adecuado.** El release 2.9.0 fue ese dia.

## 3. FALSO: "el riesgo escala con el tamano de lo piped"

El criterio de triaje que proponia el punto 2 de "Que habria que hacer" —
*decidir por tamano de la entrada, no por costumbre* — **no se sostiene**.

La version fuerte de la afirmacion ("una linea es inocua") tiene respaldo: un
escritor externo con una sola linea dio 0 de 2 000. Pero la version que
justificaba tocar solo los sitios grandes —**"con needle al final el escritor
ya no tiene trabajo, luego es seguro"** — es **mediblemente falsa**: 1,50 %
sin carga y **8,33 % con carga**.

Lo que decide no es el tamano de la entrada sino **si al escritor le queda
algo por escribir cuando el lector cierra**, y eso no se lee del tamano. Por
eso el guard que cierra esta deuda **no tiene lista de excepciones escrita a
mano**: la excepcion es estructural (escritor builtin, o rc descartado) y se
IMPRIME. Una lista a mano es la forma mas rapida de que la cuenta vuelva a
mentir sin que nadie lo note.

## 4. FALSO: el alcance eran 35 ficheros

MEDIDO con dos instrumentos escritos por separado que coinciden:

- **32** ficheros con `pipefail` y la clase (no 35)
- **91** sitios de la clase, con comentarios y `${V#*|}` ya excluidos
- de esos 91, **21 son PELIGROSO** (escritor EXTERNO + rc consumido), en **13**
  ficheros. Los otros **70 son estructuralmente inmunes** y quedan exentos.

Que sean 21 y no 35 no es buena noticia: los 35 eran **candidatos**, y el
documento lo decia. Lo que faltaba era medirlos, y al medirlos la lista de
"peligrosos" es mas corta y mas afilada que la de candidatos.

**Reparto de los 21:**

| Fichero | Sitios |
|---|---|
| `tests/test_vault_mirror_auto.sh` | 5 |
| `tests/test_release_public_gate.sh` | 2 |
| `tests/test_h05_isolation.sh` | 2 |
| `tests/clean_machine_uat.sh` | 2 |
| `scripts/release.sh` | 2 |
| `tests/test_release_routes_parity.sh` | 1 |
| `tests/test_release_pipeline_consistency.sh` | 1 |
| `tests/test_install_asset_contract.sh` | 1 |
| `tests/test_changelog_coverage.sh` | 1 |
| `tests/test_changelog_coverage_baseline_mutation.sh` | 1 |
| `tests/test_build_identity_policy.sh` | 1 |
| `scripts/release-bump.sh` | 1 |
| `scripts/apply_banner.sh` | 1 |

## 5. La mina que nadie habia visto: `tar tzf ... | grep -qx`

`scripts/release.sh:1342` era el peor de los 21 y no lo delata el tamano:

    tar tzf "$BUNDLE_TARBALL" | grep -qx ".../BUNDLE.toml" || { echo "FATAL: ..."; exit 1; }

`tar tzf` lista **~400 miembros** del bundle y `grep -qx` cerraba en cuanto
encontraba `BUNDLE.toml` —que no es el ultimo—, dejando a `tar` con casi todo
el listing por escribir. Un 141 espurio hacia que el `||` imprimiera
**"FATAL: bundle tarball is missing BUNDLE.toml"** sobre un bundle que si lo
tenia, y saliera con 1 en un release perfectamente bueno.

## 6. Lo que se hizo

**Los 21 sitios** convertidos de `| grep -q` a `| grep -c`, con el consumidor
pasando a `[ ... -gt 0 ]` o `[ ... = 0 ]`. En `test_vault_mirror_auto.sh` y en
`test_release_tag_anchoring.sh` se lee la slice **una vez a una variable** y se
busca DENTRO, sin tuberia, que ademas abre el fichero una vez en vez de cinco.

**`tests/test_grep_q_after_pipe.py`** — el guard que este documento declaraba
que faltaba. Falla cerrado sobre la clase, con las excepciones estructurales.
MEDIDO: contra el arbol original (**pristine**) reporta `PELIGROSOS = 21` y
sale con 1; contra el arbol arreglado reporta `0` y sale con 0.

**`tests/test_grep_q_after_pipe_mutation.py`** — su autofalsador.
`PASS=9 FAIL=0 SKIP=0`, cada mutacion cayendo por su propia comprobacion.

Los dos cableados en el 1b de `release.sh`, con `test_gate_coverage.py` en
`SIN runner y SIN motivo: 0`.

## 7. Lo que esta medicion COSTO, que es la parte que nadie pregunta

Cinco de los errores de abajo son **mios, de este bloque**, y cada uno
produjo un numero plausible y falso. Se dejan escritos porque son el
argumento de por que el guard tiene la forma que tiene:

1. **`head -c 8` leia `#!/bin/b`**, que no contiene la aguja -> dio "100 % de
   fallo" y era un fixture sin needle. **Tercera vez** que un fixture sin la
   aguja produce una conclusion firme.
2. **n=3 por punto** en el umbral dio "~256 KiB" para una carrera del 1 %.
3. **Medir contra `v2.8.1`** en vez de contra el arbol que fallo.
4. **El clasificador tomo el ULTIMO token como nombre del escritor** -> "0
   writers externos sobre 98 sitios" y un "riesgo 0" escrito desde ahi.
5. **Enmascara `${V#*|}` sin saber que el `|` no era tuberia**, y **cuenta
   comentarios** que describen el defecto. Un guard que cuenta su propia
   documentacion no esta midiendo el producto.

Y tres mas, en el guard ya escrito:

6. **La rama `$((` estaba debajo de la de `$(`**, o sea codigo muerto: la
   aritmetica se tomaba por sustitucion de comandos y **quedaba exculpada**.
   MEDIDO: el guard reporting 17 en vez de 21.
7. **El regex casaba el segundo `|` de un `||`** como si fuera tuberia: un
   sitio que no existe. **MEDIDO: por eso el falsador dio `PASS=8 FAIL=1`**,
   y el fallo no era del guard sino del clasificador.
8. **`with_sandbox` solo restauraba los atributos en MAYUSCULAS**, luego un
   parche a `strip_comment` se **filtraba** a las mutaciones siguientes y M5
   media el efecto de M4. Un falsador que arrastra el defecto de la mutacion
   anterior entre casos no mide la mutacion que dice medir.

**Los numeros 6 y 7 los encontro el contraste entre DOS instrumentos
independientes, no la revision.** Un parser que solo devuelve cuentas no
informa: hay que obligarlo a ensenar las lineas que clasifica, y a que la
cuenta se pueda falsar a ojo. Por eso el guard imprime el reparto y el
falsador exige, para cada mutacion "espero verde", un **segundo pase contra
una variante sin la excepcion** — si esa variante tampoco cae, la fixture no
contenia lo que dice contener y el veredicto es `SKIP`, nunca `PASS`.

## 8. Que NO se afirma

- **No se afirma que esto no vuelva.** Un guard que busca la clase es una
  garantia de que la clase no esta; no es una garantia de que ningun test sea
  inmune a una carrera. `tests/test_gate_coverage.py` sigue sin cubrir tests
  en `.rs` que compartan singleton, que es otra clase.
- **Los 70 sitios exentos siguen ahi.** Son inmunes **por su estructura**
  (escritor builtin, o rc que nadie consume), no por permiso. Si alguien
  cambia un `printf` por `/usr/bin/printf`, el guard lo ve.
- **La medicion es de ESTA maquina** (Linux, pipe buffer por defecto). El
  mecanismo es el de POSIX, pero las tasas no son transferibles.
