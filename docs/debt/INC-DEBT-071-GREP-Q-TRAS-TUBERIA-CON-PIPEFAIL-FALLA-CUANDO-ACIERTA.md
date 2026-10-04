---
id: INC-DEBT-071-GREP-Q-TRAS-TUBERIA-CON-PIPEFAIL-FALLA-CUANDO-ACIERTA
status: open
severity: high
priority: P1
cluster_id: CL-VERIFY
detected_at: 2026-10-04
detected_by: release 2.9.0, murine en el 1b
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
