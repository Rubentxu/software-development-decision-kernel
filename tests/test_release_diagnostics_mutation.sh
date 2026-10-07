#!/usr/bin/env bash
# shellcheck disable=SC2016
# SC2016, MEDIDO y deliberado: cada mutacion es un payload de python que se
# pasa entre comillas simples precisamente para que el shell NO lo expanda —
# el codigo python tiene que llegar a `python3` con sus `$` intactos, porque
# forma parte de los strings que se comparan con el fichero—. Pasarlos a
# comillas dobles los expandiria aqui y la mutacion buscaria otra cosa, luego
# caeria por el motivo equivocado y el falsador contaria una deteccion que no
# ha medido la propiedad. Mismo motivo, misma directiva de fichero entero.


# Autofalsacion del diagnostico de release, y de su cableado.
#
# QUE FALSA ESTE SCRIPT
# ---------------------
# Nueve mutaciones, cada una corrompiendo UN punto de enforcement y cada una
# exigiendo que caiga el caso nominado que lo vigila. Una mutacion compuesta
# no puede decir cual de las comprobaciones cayo por efecto colateral, que es
# como se declara un guard que vigila menos de lo que cree.
#
# UNA MUTACION QUE NO SE APLICA ES `SKIP`, NUNCA `PASS`: cada una comprueba por
# sha que el fichero cambio de verdad, y la restauracion se comprueba byte a
# byte. Sin esa comprobacion, un parche que no encuentra su texto —porque el
# fichero cambio de forma— se contaria como una deteccion y el falsador seria
# verde sobre una base que no vigila nada.
#
# EL NEEDLE ES EL NOMBRE DEL CASO, NO EL TEXTO DE UNA ASERCION
# -----------------------------------------------------------
# Los casos se marcan con `caso_termina`, que imprime `[ok] C1` solo si TODAS
# las aserciones del caso pasaron. Buscar una asercion suelta leeria al reves
# un caso a medias caido. Ese defecto estuvo en este repo durante el
# desarrollo de estas pruebas y lo encontro el propio falsador al mirar el
# numero de casos que caian: cero, cuando la mutacion habia roto tres
# aserciones. Los marcadores ya no son decorativos.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB="$ROOT/scripts/lib/release_diagnostics.sh"
RELEASE="$ROOT/scripts/release.sh"
TEST="$ROOT/tests/test_release_diagnostics.sh"
WIRE="$ROOT/tests/test_release_diagnostics_wiring.sh"

PASS=0
FAIL=0
SKIP=0
OUT="$(mktemp)"
BACKUP="$(mktemp)"
# La trampa RESTAURA, y no solo limpia. MEDIDO (session-83): 9 de las 20
# mutaciones de este fichero apuntan a $LIB, que es el fichero REAL del repo,
# y la trampa anterior se limitaba a borrar el backup. Cualquier salida
# anormal entre `mutar` y su restauracion —un `exit`, una senal, un gate que
# mata el proceso— dejaba el repo mutado Y habia borrado el unico testigo, con
# lo que no habia ni forma de recuperar el contenido original.
#
# MEDIDO el efecto: una release posterior murio en el paso 3 con "el arbol
# tiene cambios RASTREADOS sin commitear", que habla de un arbol sucio y no
# de que un test dejara el repo a medias. El mensaje era cierto y la causa no
# estaba en el.
MUTANDO_AHORA=""
restaurar_si_queda() {
    if [ -n "$MUTANDO_AHORA" ] && [ -f "$BACKUP" ]; then
        cp "$BACKUP" "$MUTANDO_AHORA" 2>/dev/null || true
    fi
    rm -f "$OUT" "$BACKUP" 2>/dev/null || true
}
trap restaurar_si_queda EXIT
trap 'restaurar_si_queda; exit 130' INT
trap 'restaurar_si_queda; exit 143' TERM

banner() { printf '\n== %s\n' "$*"; }

# --- por que se rompe la base ---------------------------------------------------
#
# MEDIDO: esto era `tail -20 "$OUT"`, y la ventana se comia justo lo que
# importaba. El log del release mostraba
#
#     [FATAL] base: test_release_diagnostics_wiring.sh ya esta ROJO
#     ... (20 lineas de aserciones verdes, E7/E8/E9) ...
#     PASS=64 FAIL=2
#     RESULT: FAIL - la libreria existe pero el release no la aprovecha.
#
# es decir: el agregado y la cola del log, y NINGUN `[FAIL]`, porque los dos
# fallos estaban mas arriba. Un informe que enseña la cola de un log largo
# parece completo y no dice nada. Peor: el texto `RESULT: FAIL - ...` que se
# leia como de ESTE falsador era del GUARD INVOCADO, no de este script, luego
# la causa seguia sin estar a la vista ni con la correccion.
#
# Ahora se imprimen los `[FAIL]` POR NOMBRE —que es donde vive la causa— y, si
# no hubiera ninguno, la cola completa: un log sin `[FAIL]` que sale con codigo
# distinto de cero es un caso mas, y hay que verlo entero.
base_rota() {
    local n
    n="$(grep -c '\[FAIL' "$OUT" 2>/dev/null || true)"
    printf '  --- por que esta roja la base: %s linea(s) [FAIL] ---\n' "${n:-0}"
    grep -E '\[FAIL' "$OUT" 2>/dev/null || true
    if [ "${n:-0}" -eq 0 ]; then
        printf '  --- no hay [FAIL]: el log entero, que sale con un codigo distinto de cero ---\n'
        cat "$OUT"
    fi
}

# --- base: una autofalsacion sobre una base roja no demuestra nada ------------

banner "BASE — los dos guards tienen que estar verdes antes de quitarles dientes"
if bash "$TEST" > "$OUT" 2>&1; then
    printf '  [ok]   base: %s verde\n' "$(basename "$TEST")"
else
    printf '  [FATAL] base: %s ya esta ROJO; no se puede falsar sobre una base rota\n' "$(basename "$TEST")"
    base_rota
    exit 1
fi
if bash "$WIRE" > "$OUT" 2>&1; then
    printf '  [ok]   base: %s verde\n' "$(basename "$WIRE")"
else
    printf '  [FATAL] base: %s ya esta ROJO\n' "$(basename "$WIRE")"
    base_rota
    exit 1
fi

# --- la maquina de mutaciones ------------------------------------------------

# mutar <etiqueta> <fichero> <test> <caso> <que-debe-caer> <codigo python>
#
# Devuelve: PASS si el caso esperado CAE, FAIL si sigue verde, SKIP si el parche
# no se aplico. En los tres casos restaura el fichero byte-identico.
mutar() {
    local etiqueta="$1" fichero="$2" test="$3" caso="$4" que="$5" pycode="$6"
    local sha_antes sha_mut sha_restore

    sha_antes="$(sha256sum "$fichero" | cut -d' ' -f1)"
    cp "$fichero" "$BACKUP"
    MUTANDO_AHORA="$fichero"

    if ! MUT_FILE="$fichero" python3 -c "$pycode" 2>/dev/null; then
        printf '  [SKIP] %s — el parche lanzo error\n' "$etiqueta"
        SKIP=$((SKIP + 1))
        cp "$BACKUP" "$fichero"
        MUTANDO_AHORA=""
        return
    fi
    sha_mut="$(sha256sum "$fichero" | cut -d' ' -f1)"
    if [ "$sha_antes" = "$sha_mut" ]; then
        printf '  [SKIP] %s — el parche NO cambio el fichero: no esta midiendo nada\n' "$etiqueta"
        SKIP=$((SKIP + 1))
        cp "$BACKUP" "$fichero"
        MUTANDO_AHORA=""
        return
    fi

    # INTEGRIDAD DEL INSTRUMENTO: un parche que deja el fichero sin sintaxis
    # valida no mide la propiedad que dice medir —mide que el script no
    # arranca, que hace caer TODOS los casos—, y se contaria como deteccion.
    # MEDIDO, y no fue hipotetico: la primera version de M1 se carried el `;;`
    # al reescribir la rama 137, el `source` del test fallaba y caian los diez
    # casos. El falsador lo-annunciaba como "M1 detectada" con una honestidad
    # que no tenia. Un parche degenerado es `SKIP`, con su motivo, nunca `PASS`.
    if ! bash -n "$fichero" 2>/dev/null; then
        printf '  [SKIP] %s — el parche dejo el fichero SIN SINTAXIS VALIDA: mide que no arranca, no la propiedad\n' \
            "$etiqueta"
        SKIP=$((SKIP + 1))
        cp "$BACKUP" "$fichero"
        MUTANDO_AHORA=""
        return
    fi

    bash "$test" > "$OUT" 2>&1
    local caidas
    caidas="$(grep -cE '^\s+\[FAIL-CASO\]' "$OUT")"

    # restaurar ANTES de juzgar el resultado, para que un fallo del falsador no
    # deje el arbol mutado.
    cp "$BACKUP" "$fichero"
    MUTANDO_AHORA=""
    sha_restore="$(sha256sum "$fichero" | cut -d' ' -f1)"
    if [ "$sha_restore" != "$sha_antes" ]; then
        printf '  [FATAL] %s — la restauracion no fue byte-identica\n' "$etiqueta"
        exit 1
    fi

    if grep -qE "^[[:space:]]*\[ok\] ${caso}\$" "$OUT"; then
        printf '  [FAIL] %s — %s SIGUE VERDE. %s\n' "$etiqueta" "$caso" "$que"
        FAIL=$((FAIL + 1))
    else
        printf '  [ok]   %s — cae %s (%s caso(s) abajo). %s\n' "$etiqueta" "$caso" "$caidas" "$que"
        PASS=$((PASS + 1))
    fi
}

# --- M1..M8: la libreria ----------------------------------------------------

banner "M1..M8 — la libreria de diagnostico"

mutar "M1 tabla de causas sin la rama 137" "$LIB" "$TEST" C1 \
    "sin 137, un OOM deja de nombrarse y cae al 'sin causa con nombre'. Solo C1: 137 aparece en un solo caso." \
'
import os, re
p = os.environ["MUT_FILE"]; s = open(p).read()
s = re.sub(r"^        137\) echo .*$",
           "        137) echo \"sin causa con nombre para el codigo 137\" ;;",
           s, flags=re.M)
open(p, "w").write(s)
'

mutar "M2 el diagnostico no nombra el paso" "$LIB" "$TEST" C4 \
    "sin el nombre del paso, un log cortado solo dice que algo fallo." \
'
import os, re
p = os.environ["MUT_FILE"]; s = open(p).read()
s = re.sub(r"^        printf .    paso     : %s.*$", "        :", s, flags=re.M)
open(p, "w").write(s)
'

mutar "M3 la guardia de idempotencia vuelve a ser una variable" "$LIB" "$TEST" C6 \
    "una variable no cruza subshells, y el bloque sale dos veces: se lee como dos fallos." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace("if [ -n \"$marker\" ]; then", "if [ -z \"$marker\" ]; then")
open(p, "w").write(s)
'

mutar "M4 el preflight de recursos nunca falla" "$LIB" "$TEST" C7 \
    "un gate que nunca dice que no informa de nada, y aqui informa de ENOSPC." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace("    return \"$failed\"", "    return 0")
open(p, "w").write(s)
'

mutar "M5 _is_own_descendant vuelve a preguntar al reves" "$LIB" "$TEST" C9 \
    "subir desde \$\$ buscando el PID no ve lo que esta por debajo, y el cargo propio se cuela." \
'
import os, re
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace("_is_own_descendant() {\n    local cur=\"$1\"",
              "_is_own_descendant() {\n    local objetivo=\"$1\"\n    local cur=\"$$\"")
s = s.replace("        [ \"$cur\" = \"$$\" ] && return 0",
              "        [ \"$cur\" = \"$objetivo\" ] && return 0")
open(p, "w").write(s)
'

mutar "M6 el criterio de retencion vuelve a ser una cadena en los argumentos" "$LIB" "$TEST" C9 \
    "MEDIDO: con un grep, todo lo que lleve cargo en la ruta —los asv-* de esta maquina, el propio shell— se cuenta." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace("        [ \"${comm:-}\" = \"cargo\" ] || continue",
              "        case \"$comm\" in *cargo*) ;; *) continue ;; esac")
s = s.replace("ps -eo pid=,etimes=,comm= 2>/dev/null",
              "ps -eo pid=,etimes=,args= 2>/dev/null")
open(p, "w").write(s)
'

mutar "M7 un codigo sin causa con nombre recibe una causa inventada" "$LIB" "$TEST" C3 \
    "una causa inventada dirige la investigacion a un sitio falso: peor que declararla desconocida." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace("echo \"sin causa con nombre para el codigo $code\"",
              "echo \"problema del sistema\"")
open(p, "w").write(s)
'

mutar "M8 el diagnostico imprime la causa pero no la medicion" "$LIB" "$TEST" C5 \
    "una causa afirmada sin cifras obliga a creerla en vez de comprobarla." \
'
import os, re
p = os.environ["MUT_FILE"]; s = open(p).read()
s = re.sub(r"^        _diag_measure .*$", "        :", s, flags=re.M)
open(p, "w").write(s)
'

# --- M9: el cableado en release.sh -------------------------------------------

banner "M9 — el cableado"

mutar "M9 el trap vuelve a limpiar sin diagnosticar" "$RELEASE" "$WIRE" E3 \
    "sin el manejador, el release sigue siendo el de antes: limpio y mudo." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace("trap release_on_exit EXIT", "trap cleanup_release_scratch EXIT")
open(p, "w").write(s)
'

mutar "M10 el guard de wiring se quita --skip-tests y vuelve a dispararse" "$WIRE" "$WIRE" E0 \
    "sin --skip-tests el dry-run alcanza el 1b, el 1b corre este fichero, y el guard se llama a si mismo." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace("\"$RELEASE\" --dry-run --skip-tests", "\"$RELEASE\" --dry-run")
open(p, "w").write(s)
'

mutar "M11 el 1b vuelve a descartar la salida del test que falla" "$RELEASE" "$WIRE" E0 \
    "con >/dev/null, el release vuelve a no decir por que fallo y su unica consigna es mirar a mano sin decir donde." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace(
    "        if ! bash \"$t\" >\"$t_log\" 2>&1; then",
    "        if ! bash \"$t\" >/dev/null; then")
s = s.replace(
    "            if ! python3 \"$p\" >\"$p_log\" 2>&1; then",
    "            if ! python3 \"$p\" >/dev/null; then")
open(p, "w").write(s)
'

mutar "M12 el 1b vuelve a enseñar solo la cola del log" "$RELEASE" "$WIRE" E0 \
    "con tail, los fallos del principio del log desaparecen y el informe parece completo sin estarlo." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
s = s.replace("            tail -10 \"$t_log\" >&2", "            true")
s = s.replace("            tail -10 \"$p_log\" >&2", "            true")
open(p, "w").write(s)
'

mutar "M13 un caso hereda el marcador que dejo el anterior" "$TEST" "$TEST" C4 \
    "sin limpiar el marcador, la idempotencia hace que el segundo caso no imprima: caia SOLO dentro del release." \
'
import os
p = os.environ["MUT_FILE"]; lines = open(p).read().split("\n")
i = next(i for i, l in enumerate(lines) if l.strip() == "limpiar_marcador")
del lines[i]
open(p, "w").write("\n".join(lines))
'

# M14 y M15: la clase de gates, no la bandera.
#
# M10 quito `--skip-tests` de las invocaciones y cae E0. Eso es cierto y no
# basta: el defecto real era que el `if` de SKIP_TESTS cerraba antes de los
# CATORCE gates, luego la bandera no cubria el camino de vuelta. E0 seguiria
# verde con los trece sites sin cubrir. M14 quita UN `test_gate` de un gate
# real, y es lo que tiene que caer: si E4 sobrevive a esto, E4 no vigila la
# propiedad que su nombre dice.
mutar "M14 un gate se queda sin test_gate y queda alcanzable bajo --skip-tests" "$RELEASE" "$WIRE" E4 \
    "sin el envoltorio, ese test se ejecuta con --skip-tests y vuelve a entrar por el 3m." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
# quita el envoltorio de UN site real: el del falsador de diagnostico (3m), que
# es el que cerraba el ciclo. Se dejan los demas intactos a proposito: E4 tiene
# que caer por ESTE, no por un efecto colateral de habermelo borrado todos.
# El needle se ancla en la linea que abre el gate y en el `elif` de la
# invocacion, no en el texto del comentario de al lado: un needle escrito
# contra lo que uno espera, y no contra lo que el codigo escribe, es un needle
# roto. Ya paso, y el falsador lo declaro SKIP en vez de contarlo como
# deteccion.
lineas = s.split("\n")
salida, quitado = [], False
for l in lineas:
    if not quitado and l.startswith("if test_gate ") and "test_release_diagnostics_mutation.sh" in l:
        # se salta el `if`, la rama `:` y el `elif`, y la invocacion pasa a `if`
        estado = 0
        quitado = True
        continue
    if quitado and estado == 0:
        estado = 1
        continue          # la linea `:` con su comentario
    if quitado and estado == 1:
        assert l.startswith("elif bash tests/test_release_diagnostics_mutation.sh"), \
            "el site de 3m no tiene la forma que esta mutacion supone: " + l
        salida.append(l.replace("elif bash", "if bash", 1))
        estado = 2
        continue
    salida.append(l)
assert estado == 2, "no se encontró el site de 3m"
open(p, "w").write("\n".join(salida))
'

# M15: el helper se EJERCE en E5, luego la mutacion tiene que cambiar lo que
# DEVUELVE, no lo que parece. Cambiar `return 0` por `return 1` haria que un
# gate declarado NOT_RUN se ejecutara, que es el falso verde en su forma pura.
mutar "M15 test_gate devuelve 1 al saltarse, y un NO_EJECUTADO se vuelve PASS" "$RELEASE" "$WIRE" E5 \
    "con return 1 el sitio que llama toma la rama del gate y lo da por bueno sin haberlo ejecutado." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "        return 0\n    fi\n    return 1\n}"
nuevo = "        return 1\n    fi\n    return 1\n}"
assert viejo in s, "el cuerpo de test_gate no tiene la forma que esta mutacion supone"
s = s.replace(viejo, nuevo, 1)
open(p, "w").write(s)
'

# M16: laDisk y la memoria son reglas DISTINTAS, y C8 mide que la que falla es
# la que el mensaje senala. Se quita la rama de disco de la libreria: con ella
# fuera, el "debe fallar por disco" de C8 dejaria de fallar y las dos aserciones
# caeran. Es la unica mutacion que demuestra que C8 mira la FUNCION y no el
# entorno -- que era exactamente lo que no hacia: hasta el quinto intento de
# 2.9.0, C8 usaba el `df` real y se rompia en cuanto el disco se vaciaba.
mutar "M16 la rama de disco del preflight desaparece y C8 deja de senalar el disco" "$LIB" "$TEST" C8 \
    "sin la rama de disco, el caso que debe fallar por disco pasa, y con el pasan tambien las dos aserciones que comprueban el mensaje." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "if [ -n \"$free_mb\" ] && [ \"$free_mb\" -lt \"$SDDK_RELEASE_MIN_FREE_MB\" ]; then"
nuevo = "if [ -n \"$free_mb\" ] && false; then"
assert viejo in s, "la rama de disco no tiene la forma que esta mutacion supone"
s = s.replace(viejo, nuevo, 1)
open(p, "w").write(s)
'

# M18: `die` deja de PUBLICAR el codigo del comando, que es el defecto MEDIDO
# del noveno intento de 2.9.0 -- con `exit 1` fijo, el bloque de "por que fallo
# el release" decia SIEMPRE `codigo : 1` y las ramas 137/139/134/135 de
# `cause_of_exit_code` eran inalcanzables por construccion. Se quita la
# publicacion: E7 debe caer en la asercion que exige el 139 y en la que exige la
# causa nombrada, y en las dos de "dos muertes distintas".
mutar "M18 die deja de publicar el codigo real y el diagnostico vuelve a la constante" "$RELEASE" "$WIRE" E7 \
    "sin la publicacion, el bloque solo puede decir 'fallo declarado por release.sh', que es el defecto medido: un instrumento que solo reporta una constante no informa." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "    local cause=$?\n    if [ \"$cause\" != \"0\" ]; then\n        RELEASE_DIE_CODE=\"$cause\"\n    fi\n"
nuevo = "    local cause=$?\n"
assert viejo in s, "la publicacion del codigo en die no tiene la forma que esta mutacion supone"
s = s.replace(viejo, nuevo, 1)
open(p, "w").write(s)
'

# M19: el BORDE que el arreglo de M18 introduce. `die` desatado tras un comando
# que SI funciono no tiene causa; si se publica ese 0, `release_diagnose_exit 0`
# returns calladamente y el release se para SIN decir por que -- el fallo mas
# caro de todos, y el que el arreglo podia crear sin querer. Se quita la
# normalizacion a "solo si no es cero" y la ultima asercion de E7 debe caer.
mutar "M19 die publica tambien el 0 de un comando que funciono, y el bloque desaparece" "$RELEASE" "$WIRE" E7 \
    "publicar un 0 hace que release_diagnose_exit salga sin imprimir nada: el release se para sin diagnostico." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "    local cause=$?\n    if [ \"$cause\" != \"0\" ]; then\n        RELEASE_DIE_CODE=\"$cause\"\n    fi\n"
nuevo = "    local cause=$?\n    RELEASE_DIE_CODE=\"$cause\"\n"
assert viejo in s, "la normalizacion de die no tiene la forma que esta mutacion supone"
s = s.replace(viejo, nuevo, 1)
open(p, "w").write(s)
'

# M20: la limpieza vuelve a decidir el resultado del release. Se devuelve la
# forma MEDIDA del decimo intento —el padre antes que el hijo y sin `|| true`—,
# que con `set -euo pipefail` hace que un release que termina bien salga con 1 y
# sin bloque de diagnostico. E8 debe caer en la asercion del rc 0.
mutar "M20 la limpieza vuelve a abortar el manejador y un release verde sale con 1" "$RELEASE" "$WIRE" E8 \
    "sin el orden hijo-antes-de-padre ni el || true, el rm falla, set -e aborta el manejador antes de su return, y el codigo del release pasa a ser el de la limpieza." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "cleanup_release_scratch() { rm -rf \"$TMP\" \"$RELEASE_SCRATCH\" || true; }"
nuevo = "cleanup_release_scratch() { rm -rf \"$RELEASE_SCRATCH\" \"$TMP\"; }"
assert viejo in s, "la limpieza del paso 5 no tiene la forma que esta mutacion supone"
s = s.replace(viejo, nuevo, 1)
open(p, "w").write(s)
'

# M21: el gate de recursos se convierte en puerta trasera. E1 y E2, juntos, son
# los que dicen que es un UMBRAL y no una puerta: con margen insuficiente se
# para (E1), con margen suficiente no se para por recursos (E2). Si el gate
# parase SIEMPRE, E1 seguiria verde —para por la razon que el caso espera— y
# E2 caeria, que es justo la asercion que distingue un umbral de una puerta.
# Se cambia la comparacion de disco por `false` solo cuando hay margen de sobra.
mutar "M21 el preflight de recursos se para siempre y el gate deja de ser un umbral" "$LIB" "$WIRE" E2 \
    "si el gate para tambien con margen de sobra, E1 no lo nota —para por la razon que espera— y E2 cae: un gate que nunca deja pasar nada no es un gate." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "if [ -n \"$free_mb\" ] && [ \"$free_mb\" -lt \"$SDDK_RELEASE_MIN_FREE_MB\" ]; then"
nuevo = "if [ -n \"$free_mb\" ]; then"
assert viejo in s, "la rama de disco del preflight no tiene la forma que esta mutacion supone"
s = s.replace(viejo, nuevo, 1)
open(p, "w").write(s)
'

# --- M21..M24: E9, los recursos en el punto donde se consumen ----------------
# E9 es el caso que cierra el P1 bl-bl-01M42JGYG4000388551BF9NZ40, y tiene tres
# dientes que se pueden quitar por separado: que se mida, que se mida ANTES de
# gastar, y que el fallo lo nombre. Cada mutacion quita UNO y exige que caiga el
# caso.

mutar "M25 el paso 1 deja de medir los recursos (la del paso 0 no basta)" \
    "$RELEASE" "$WIRE" E9 \
    'la comprobacion desaparece del paso 1: queda solo la del paso 0, que es exactamente el defecto que el P1 describe.' \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
v = "    if ! release_check_resources \"$RELEASE_SCRATCH\"; then"
n = "    if ! false; then  # mutacion M25: la comprobacion ya no esta"
assert v in s, "el paso 1 no tiene la forma que esta mutacion supone"
open(p, "w").write(s.replace(v, n, 1))
'

mutar "M26 un die del paso 1 vuelve a no nombrar los recursos" \
    "$RELEASE" "$WIRE" E9 \
    'es el defecto original del log de session-76 reintroducido en uno solo de los cuatro fallos: la cobertura, no la redaccion.' \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
v = "die \"cargo fmt failed. recursos en el momento del fallo: $(release_resources_now)\""
n = "die \"cargo fmt failed\""
assert v in s, "el die de cargo fmt no tiene la forma que esta mutacion supone"
open(p, "w").write(s.replace(v, n, 1))
'

mutar "M27 release_resources_now deja de devolver el hecho" \
    "$LIB" "$WIRE" E9 \
    'una funcion que existe y no imprime nada deja el mensaje tan mudo como antes del arreglo: es el caso degenerado del que el guard tiene que acordarse.' \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
v = "release_resources_now() {\n    local avail_mb=\"?\""
n = "release_resources_now() {\n    return 0  # mutacion M27: el hecho se calla\n    local avail_mb=\"?\""
assert v in s, "la funcion no tiene la forma que esta mutacion supone"
open(p, "w").write(s.replace(v, n, 1))
'

mutar "M28 la comprobacion se mueve DETRAS del primer cargo" \
    "$RELEASE" "$WIRE" E9 \
    'medir despues de gastar no impide nada: llega tarde. Es la comprobacion como decoracion, que es la forma que tiene un gate de no hacer nada.' \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
bloque = """    if ! release_check_resources "$RELEASE_SCRATCH"; then
        die "recursos insuficientes justo antes de gastar la suite:"""
i = s.index(bloque)
j = s.index("    fi\n", i) + len("    fi\n")
s = s[:i] + s[j:]
# El ancla es la sentencia COMPLETA del cargo fmt, con su continuacion. Insertar
# entre la primera linea y su `|| die` partia el `\` y dejaba el fichero sin
# sintaxis. MEDIDO: el falsador lo conto como SKIP --"mide que no arranca, no la
# propiedad"—, que es lo que tiene que hacer un parche degenerado, y no como una
# deteccion.
sentencia = """    cargo fmt --all -- --check \\
        || die "cargo fmt failed. recursos en el momento del fallo: $(release_resources_now)"
"""
assert sentencia in s, "la sentencia de cargo fmt no tiene la forma que esta mutacion supone"
s = s.replace(sentencia, sentencia + "    release_check_resources \"$RELEASE_SCRATCH\" || true  # mutacion M28: medir tarde\n", 1)
open(p, "w").write(s)
'

# --- M29..M30: E6, que hasta aqui no tenia ninguna --------------------------
#
# MEDIDO: E6 cubria E0, E2, E3, E4, E5, E7 y E8, y E6 no. O lo que es peor: E6
# es el caso que existe para que un fallo YA MEDIDO no vuelva —el marcador que
# el test hereda del release—, y era el unico sin falsador. Un caso que nadie
# puede tumbar compra cobertura. Estas dos lo tumban.
#
# Ademas se falsan POR SEPARADO, una asercion cada una, porque son dos
# propiedades distintas: E6a vigila que el test sea dueno de SU marcador y E6b
# que ese marcador cuelgue de un temporal SUYO. Una mutacion compuesta no
# podria decir cual de las dos cayo.
#
# El needle se ancla en la linea que REALMENTE empieza por el texto, no en la
# que lo contiene dentro de una asercion: la palabra buscada aparece tambien
# dentro del propio `asert` de E6, y un needle que casa con esa segunda vez
# mutaria una linea del needle en vez de la del codigo.
mutar "M29 el test vuelve a heredar el marcador del release en vez de ser dueno del suyo" "$WIRE" "$WIRE" E6 \
    "vuelve la clase de fallo que E6 existe para cerrar: RELEASE_DIAGNOSED_FILE pasa a preferir el del release." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "export RELEASE_DIAGNOSED_FILE=\"$WIRE_DIAG_MARKER\""
nuevo = "export RELEASE_DIAGNOSED_FILE=\"${RELEASE_DIAGNOSED_FILE:-$WIRE_DIAG_MARKER}\""
lineas = s.split("\n")
cambiados = 0
salida = []
for l in lineas:
    if l == viejo:                      # la linea REAL de codigo, no la del needle
        l = nuevo
        cambiados += 1
    salida.append(l)
assert cambiados == 1, "se esperaba 1 linea de codigo real, se encontraron %d" % cambiados
open(p, "w").write("\n".join(salida))
'

mutar "M30 el marcador del test cuelga de un temporal heredado en vez de uno suyo" "$WIRE" "$WIRE" E6 \
    "WIRE_DIAG_MARKER pasa a colgar de TMPDIR, que es el estado que el release exporta." \
'
import os
p = os.environ["MUT_FILE"]; s = open(p).read()
viejo = "WIRE_DIAG_MARKER=\"$WIRE_TMPDIR/.sddk-diagnosed-wiring\""
nuevo = "WIRE_DIAG_MARKER=\"${TMPDIR:-/tmp}/.sddk-diagnosed-wiring\""
lineas = s.split("\n")
cambiados = 0
salida = []
for l in lineas:
    if l == viejo:
        l = nuevo
        cambiados += 1
    salida.append(l)
assert cambiados == 1, "se esperaba 1 linea, se encontraron %d" % cambiados
open(p, "w").write("\n".join(salida))
'

# --- resumen -----------------------------------------------------------------

printf '\n----------------------------------------\n'
printf 'PASS=%d FAIL=%d SKIP=%d\n' "$PASS" "$FAIL" "$SKIP"
if [ "$FAIL" -eq 0 ] && [ "$SKIP" -eq 0 ]; then
    printf 'RESULT: PASS — las %d comprobaciones tienen dientes, y el arbol quedo intacto.\n' "$PASS"
    exit 0
fi
if [ "$FAIL" -ne 0 ]; then
    printf 'RESULT: FAIL — hay comprobaciones que no vigilan lo que declaran.\n'
    exit 1
fi
printf 'RESULT: DECLARADO — %s mutacion(es) no se aplicaron y no cuentan como deteccion.\n' "$SKIP"
exit 1
