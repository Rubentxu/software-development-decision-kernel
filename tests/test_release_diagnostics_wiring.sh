#!/usr/bin/env bash
# Que release.sh USE el diagnostico, y no solo que la libreria exista.
#
# POR QUE ESTE TEST ES SEPARADO Y NO UN CASO MAS DE test_release_diagnostics.sh
# ---------------------------------------------------------------------------
# Aquel ejerce las funciones con las manos. Este comprueba lo unico que puede
# fallar sin que ninguna asercion de aquel se entere: que `release.sh` sourcee
# la libreria, que `step` deje constancia del paso, que haya UN SOLO manejador
# de salida (dos trampas EXIT se pisan en bash), que los preflights esten
# EN EL PASO 0 y no mas adelante, y que el conjunto se comporte de verdad
# cuando el release muere.
#
# Un guard que vigila la libreria y otro que vigila el cableado miden cosas
# distintas: una puede estar impecable mientras la otra no la invoca, y eso no
# se ve en ninguno de los dos. El precedente de esta serie es la sexta vez que
# un componente bien construido no llegaba a ejecutarse nunca.
#
# DOS CASOS QUE SE CONTRADICEN, Y POR QUE LOS DOS IMPORTAN
# -------------------------------------------------------
#   E1 margen imposible  -> el release para ANTES de empezar, nombrando el
#                           recurso, el punto de montaje y las dos cifras.
#   E2 margen suficiente  -> el release SUPERA el preflight y muere mas
#                           adelante, en la admision, porque la version del
#                           workspace ya esta publicada.
#
# E2 es el que hace que E1 no sea una puerta trasera. Un preflight que solo
# sabe decir "no" no vigila nada; uno que se puede superar verifica que su
# fallo era real y no una regla que siempre se cumple.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RELEASE="$ROOT/scripts/release.sh"
RELEASE_WIRE="${BASH_SOURCE[0]}"
LIB="$ROOT/scripts/lib/release_diagnostics.sh"

PASS=0
FAIL=0

asert() {
    local label="$1" cond="$2" detail="${3:-}"
    if [ "$cond" = "1" ]; then
        printf '  [ok]   %s\n' "$label"
        PASS=$((PASS + 1))
    else
        printf '  [FAIL] %s%s\n' "$label" "${detail:+ -- $detail}"
        FAIL=$((FAIL + 1))
    fi
}

# El marcador de caso depende de TODAS sus aserciones, por el mismo motivo que
# en el test principal: un `printf` suelto imprime `[ok] E1` con el caso entero
# caido, y el falsador leeria verde un caso roto.
caso_empieza() { _DESDE_FAIL=$FAIL; }
caso_termina() {
    if [ "$FAIL" -eq "$_DESDE_FAIL" ]; then
        printf '  [ok] %s\n' "$1"
    else
        printf '  [FAIL-CASO] %s (%s asercion(es) caidas)\n' "$1" "$((FAIL - _DESDE_FAIL))"
    fi
}

# --- E0: el codigo dice lo que tiene que decir --------------------------------
caso_empieza

asert "E0: release.sh sourcea la libreria de diagnostico" \
    "$(grep -q 'source "\$ROOT/scripts/lib/release_diagnostics.sh"' "$RELEASE" && echo 1 || echo 0)"
asert "E0: y esa libreria existe de verdad (si no, el source falla en el release)" \
    "$([ -f "$LIB" ] && echo 1 || echo 0)"

# UN SOLO manejador de salida. MEDIDO el motivo: en bash un `trap ... EXIT`
# nuevo REEMPLAZA al anterior, y el repo tenia dos. El segundo (linea del paso
# 5) sustituyó al primero y dejo el scratch —y con el el marcador del
# diagnostico— en disco. Contarlos es barato; lo que importa es que sean UNO.
#
# Se cuentan solo lineas cuyo PRIMER token es `trap`: un grep laxo tambien
# cuenta los comentarios que hablan de trampas, y un guard que se cuenta a si
# mismo declara una propiedad que no existe. MEDIDO: asi-contando daba 2 con un
# solo trap real.
n_traps="$(grep -cE '^[[:space:]]*trap .* EXIT' "$RELEASE")"
asert "E0: hay UN SOLO trap EXIT, no varios que se pisen" \
    "$([ "$n_traps" -eq 1 ] && echo 1 || echo 0)" "traps EXIT reales: $n_traps"

asert "E0: ese trap es el que diagnostica y limpia" \
    "$(grep -q 'trap release_on_exit EXIT' "$RELEASE" && echo 1 || echo 0)"

asert "E0: release_on_exit llama al diagnostico ANTES de limpiar el scratch" \
    "$(awk '/^release_on_exit\(\)/,/^}/' "$RELEASE" \
        | awk '/release_diagnose_exit/{d=NR} /cleanup_release_scratch/{c=NR} END{print (d && c && d<c) ? 1 : 0}')"

asert "E0: la limpieza del scratch cubre tambien TMP, que se creo en el paso 5" \
    "$(grep -q 'cleanup_release_scratch() { rm -rf "\$RELEASE_SCRATCH" "\$TMP"; }' "$RELEASE" && echo 1 || echo 0)"

asert "E0: el marcador de 'ya diagnostique' se crea DENTRO del scratch" \
    "$(grep -q 'RELEASE_DIAGNOSED_FILE="\$RELEASE_SCRATCH/\.sddk-release-diagnosed"' "$RELEASE" && echo 1 || echo 0)"

# `step` tiene que dejar constancia, no solo imprimir.
asert "E0: step delega en release_step (anuncia Y recuerda)" \
    "$(grep -q '^step() { release_step "\$@"; }' "$RELEASE" && echo 1 || echo 0)"

# Los preflights tienen que estar ANTES del trabajo caro, no despues.
linea_recursos="$(grep -n 'release_check_resources "\$RELEASE_SCRATCH"' "$RELEASE" | head -1 | cut -d: -f1)"
linea_lock="$(grep -n 'release_check_cargo_lock' "$RELEASE" | head -1 | cut -d: -f1)"
linea_suite="$(grep -n 'cargo test --workspace --offline' "$RELEASE" | head -1 | cut -d: -f1)"
asert "E0: el preflight de recursos esta antes de gastar la suite" \
    "$([ -n "$linea_recursos" ] && [ -n "$linea_suite" ] && [ "$linea_recursos" -lt "$linea_suite" ] && echo 1 || echo 0)" \
    "recursos en $linea_recursos, suite en $linea_suite"
asert "E0: el aviso de lock esta antes de gastar la suite tambien" \
    "$([ -n "$linea_lock" ] && [ -n "$linea_suite" ] && [ "$linea_lock" -lt "$linea_suite" ] && echo 1 || echo 0)" \
    "lock en $linea_lock, suite en $linea_suite"
asert "E0: el preflight de recursos falla CERRADO (die), el de lock solo avisa" \
    "$(awk '/release_check_resources "\$RELEASE_SCRATCH"/{getline; print (/die /) ? 1 : 0; exit}' "$RELEASE")"

# Las dos invocaciones del release llevan `--skip-tests`. Se comprueba sobre el
# TEXTO y no sobre el comportamiento a proposito: quitarlo en caliente haria que
# el dry-run alcanzara el 1b, el 1b volveria a correr este fichero, y el
# falsador se quedaria colgado en vez de reportar un fallo. Un falsador que se
# cuelga no mide nada, asi que la garantia de no-recursion se mide aqui de forma
# segura, y los casos E1/E2 la confirman en su propio sitio con el dry-run real.
#
# El needle apunta a la INVOCACION (`bash "$RELEASE" ...`) y no a la cadena
# `release.sh --dry-run --skip-tests` tal cual: el needle tiene que existir en el
# fichero. MEDIDO — la primera version contaba 1 con las dos invocaciones
# correctas, porque buscaba un texto que no aparece en ninguna linea y solo
# hallaba una mencion dentro de un comentario. Un needle que no casa con lo que
# el codigo escribe no falla: falla de otra manera, mas tarde y peor.
n_skips="$(grep -cE '^[[:space:]]*bash "\$RELEASE" --dry-run --skip-tests' "$RELEASE_WIRE")"
asert "E0: las dos invocaciones del release llevan --skip-tests (si no, este guard se re-dispara)" \
    "$([ "$n_skips" -eq 2 ] && echo 1 || echo 0)" "invocaciones con --skip-tests: $n_skips"

# La salida de un test que falla se MUESTRA, no se tira. MEDIDO: el 1b era
# `bash "$t" >/dev/null || die "... (run manually for details)"`, que es la
# misma clase de defecto que los 32 `die` sin causa en la forma mas frecuente
# de todas, y ademas hacia que el bloque de diagnostico —que declara "para eso
# esta el log de arriba"— remitiera a un log que el propio release habia
# descartado. Un diagnostico que manda a un log que no existe no diagnostica.
#
# TODOS los needles de este bloque pasan por `codigo()` y por `presenta()`.
#
# `codigo()` filtra los comentarios: sin eso, un needle que busca el defecto
# encuentra tambien la frase que lo EXplica, y el caso queda en rojo con el
# defecto ya arreglado. MEDIDO, buscando `bash "$t" >/dev/null`.
#
# `presenta()` usa `grep -c` y no `grep -q`, y no es un detalle. MEDIDO, con
# `set -o pipefail` —que este test tiene—: `codigo | grep -q` falla cuando SI
# encuentra, porque `grep -q` sale en cuanto casa, el escritor recibe SIGPIPE
# y el pipeline devuelve el fallo del escritor. Un needle que acierta reporta
# que falla. `grep -c` lee el flujo entero y no sufre eso.
#
# Es la TERCERA forma de needle roto del bloque, y las tres son la misma: el
# needle no mide lo que dice. (1) buscabamos una cadena que no existe en
# ninguna linea y hallabamos una mencion dentro de un comentario. (2)
# buscabamos el defecto en el fichero entero y lo encontramos en el comentario
# que lo explica. (3) el needle acierta y el mecanismo que lo rodea lo declara
# fallido. En un repo que documenta sus propios errores, un needle tiene que
# mirar CODIGO y no depender de como se comporta lo que tiene delante.
#
# Y una CUARTA, anadida al construir lo de arriba, que es la mas silenciosa de
# las cuatro: escribir `coincide X && echo 0 || echo 1` donde `coincide`
# IMPRIME 1 y por tanto devuelve 0. El `&&` evalua el ESTADO DE SALIDA, no el
# valor impreso, luego ese `&&` se cumple siempre y el asert nunca puede caer.
# Un needle que no puede fallar es peor que uno sin needle, porque compra
# cobertura. Por eso `coincide` se consume SIEMPRE por `[ "$(coincide ...)" = N ]`
# y nunca por su codigo de retorno.
codigo() { grep -vE '^[[:space:]]*#' "$1"; }
coincide() { [ "$(codigo "$RELEASE" | grep -c "$1")" -gt 0 ] && echo 1 || echo 0; }
ausente() { [ "$(coincide "$1")" = 0 ] && echo 1 || echo 0; }
asert "E0: el bucle de shell NO descarta la salida del test (>/dev/null)" \
    "$(ausente 'bash "\$t" >/dev/null')"
asert "E0: el bucle de shell escribe la salida en un log del scratch" \
    "$(coincide 't_log="\$RELEASE_SCRATCH/shell-test-')"
asert "E0: y muestra las ultimas lineas ANTES de morir" \
    "$(coincide 'tail -10 "\$t_log" >&2')"
# Los FALLOS se muestran por NOMBRE, no solo la cola del log. MEDIDO: con
# `tail -30` los tres `[FAIL]` estaban entre C0 y C6 —al principio del log— y
# el informe enseñaba desde C7, o sea la mitad verde. Un informe que enseña la
# cola corta del log equivocado es peor que no enseñar nada: parece completo.
#
# NEEDLES ROTOS AL CONSTRUIR ESTOS CASOS. La lista va SIN NUMERO a proposito:
# un contador de needles que se desincroniza es el mismo defecto que el primer
# needle, y se desincronizo dos veces mientras se escribia este mismo
# comentario. Todos son la misma cosa: el needle no media lo que decia.
#
#   * una cadena que no existe en ninguna linea y aparecia hallada en un
#     comentario;
#   * un needle que busca el defecto en el fichero entero y encuentra el
#     comentario que lo EXplica, con el defecto ya arreglado;
#   * `codigo | grep -q` con `set -o pipefail`: `grep -q` sale en cuanto casa,
#     el escritor recibe SIGPIPE y el pipeline devuelve SU fallo, luego
#     acertar se reportaba como fallar. `grep -c` lee el flujo entero;
#   * `coincide X && echo 0 || echo 1` donde `coincide` imprime 1 y devuelve 0:
#     el `&&` evalua el ESTADO DE SALIDA y no el valor impreso, luego el asert
#     no podia caer nunca —un needle que no puede fallar compra cobertura—;
#   * un `.*` que cruza una alternancia: entre dos needle hay un `|`, que en
#     ERE no es un caracter;
#   * needle escrito contra lo que uno ESPERA en vez de contra lo que el
#     codigo ESCRIBE: se buscaba una corcheta escapada donde lo que hay es un
#     backslash literal, y `FALLOS en $t` donde lo que hay es `FALLOS en %s:`.
#
# De la lista sale la regla: un needle tiene que ser lo mas CORTO que
# distingue el caso, debe mirar CODIGO y no el fichero entero, y debe
# comprobarse contra el fichero real antes de darlo por bueno.
asert "E0: el 1b presenta una seccion de FALLOS, no solo la cola" \
    "$(coincide 'FALLOS en %s')"
asert "E0: y esa seccion esta acotada a las lineas de fallo" \
    "$(coincide 'tail -20 >&2 || true')"
asert "E0: y el die no promete un log que se va con el scratch" \
    "$(coincide 'se borra con el scratch al salir')"
asert "E0: el die nombra DONDE esta el log" \
    "$(coincide 'log hasta la salida en \$t_log')"
asert "E0: el bucle de python tampoco la descarta (mismo defecto, mismo paso)" \
    "$(ausente 'python3 "\$p" >/dev/null')"
asert "E0: y no queda ningun 'run manually for details' que no diga donde esta" \
    "$(ausente 'die ".*run manually for details')"
caso_termina E0

# --- E1: margen imposible -> el release para nombrando el recurso ------------
caso_empieza

# --- E1: margen imposible -> el release para nombrando el recurso ------------
#
# `--skip-tests` NO es una optimizacion: es la garantia de que este guard no
# puede dispararse a si mismo. MEDIDO, y el modo de fallo es el peor posible.
#
# La primera version lanzaba `release.sh --dry-run` a secas. Ese dry-run llega
# al paso 1b, y 1b corre este mismo fichero, que lanza OTRO dry-run, cuyo 1b
# corre este fichero otra vez. La cadena medida fue:
#
#   release.sh --dry-run -> test_release_diagnostics_wiring.sh
#                        -> release.sh --dry-run -> 1b -> test_release_admission.sh
#                        -> ... y de ahi otra vez al wiring
#
# No es un bucle visible en el log: el dry-run mas externo se queda esperando y
# el 1b del release que lo invoco se queda esperando tambien, luego el sintoma
# es "el release tarda mucho" y la causa esta a tres niveles de profundidad.
# Con `--skip-tests` el dry-run muere en el paso 0 —que es donde vive lo que
# este guard mide— y no puede alcanzar el 1b, luego no puede re-entrar. La
# garantia es estructural, no una bandera: no depende de que el preflight
# funcione.
export SDDK_SKIP_SIGNING=1
e1_log="$(mktemp)"
e2_log="$(mktemp)"
e1_rc=0
e2_rc=0

SDDK_RELEASE_MIN_FREE_MB=999999999 SDDK_RELEASE_MIN_AVAIL_MB=1 \
    bash "$RELEASE" --dry-run --skip-tests > "$e1_log" 2>&1 || e1_rc=$?

asert "E1: el release se detiene" "$([ "$e1_rc" -ne 0 ] && echo 1 || echo 0)" "rc=$e1_rc"
asert "E1: nombra el disco como el recurso que falta" \
    "$(grep -q 'disco insuficiente' "$e1_log" && echo 1 || echo 0)" "$(head -5 "$e1_log")"
# La cifra que se mide es la que hay, no la que se exige: exigir un numero
# concreto en el hueco "disco insuficiente" haria que el aserto pasara por el
# motivo equivocado si el script invirtiera los dos papeles.
asert "E1: el mensaje trae LAS DOS cifras, la que hay y la que se exige" \
    "$(grep -qE 'disco insuficiente en el scratch: [0-9]+ MiB libres .*se necesitan [0-9]+ MiB' "$e1_log" && echo 1 || echo 0)" \
    "$(grep 'disco insuficiente' "$e1_log" | head -1)"
asert "E1: nombra el punto de montaje, para saber DONDE falta" \
    "$(grep -qE 'disco insuficiente en el scratch: [0-9]+ MiB libres en /[^,]+,' "$e1_log" && echo 1 || echo 0)" \
    "$(grep 'disco insuficiente' "$e1_log" | head -1)"
asert "E1: no culpa a la memoria, que si tiene margen" \
    "$(grep -q 'memoria insuficiente' "$e1_log" && echo 0 || echo 1)" \
    "$(grep 'insuficiente' "$e1_log" | head -3)"
asert "E1: el bloque de diagnostico sale con el paso en curso" \
    "$(grep -q 'por que fallo el release' "$e1_log" && echo 1 || echo 0)"
asert "E1: el diagnostico nombra el paso '0/15'" \
    "$(grep -qE 'paso *: 0/15' "$e1_log" && echo 1 || echo 0)" \
    "$(grep -E 'paso *:' "$e1_log" | head -1)"
# No-vacuidad de la garantia estructural: si el dry-run llegara al 1b, este
# guard volveria a dispararse a si mismo. Afirmarlo es lo que convierte
# `--skip-tests` de un comentario en una garantia comprobada: quitarlo de las
# dos invocaciones hace caer ESTE caso, no solo un test lento.
asert "E1: el dry-run muere en el paso 0 y NO alcanza el 1b (si lo alcanzara, este guard se repetiria)" \
    "$(grep -q '1b/15' "$e1_log" && echo 0 || echo 1)" \
    "$(grep -E '^==>' "$e1_log" | tail -1)"
asert "E1: y no llego a compilar nada" \
    "$(grep -qE '1/15|Compiling|Checking' "$e1_log" && echo 0 || echo 1)" \
    "$(grep -E '^(==>|   Compiling)' "$e1_log" | tail -1)"
caso_termina E1

# --- E2: margen suficiente -> el preflight NO es una puerta trasera ----------
caso_empieza
#
# LO QUE E2 AFIRMA Y LO QUE NO, escrito para que nadie lo lea mas fuerte de lo
# que es. Afirma que con margen el release SUPERA el preflight. No afirma que
# muera por la admision, porque la causa siguiente depende del estado del
# arbol de trabajo, que un test no puede suponer: MEDIDO, con el arbol sucio el
# release muere en `working tree is dirty`, que esta en el MISMO paso 0 y por
# detras del preflight. Afirmar la causa exacta haria que este test pasara solo
# en un arbol limpio, y fallara de forma enganosa en cuanto se ejecute durante
# el desarrollo, que es cuando mas hace falta. La afirmacion util —"el gate se
# puede superar"— no depende de nada de eso.

SDDK_RELEASE_MIN_FREE_MB=1 SDDK_RELEASE_MIN_AVAIL_MB=1 \
    bash "$RELEASE" --dry-run --skip-tests > "$e2_log" 2>&1 || e2_rc=$?

asert "E2: con margen el preflight se supera y lo DICE" \
    "$(grep -q 'recursos comprobados' "$e2_log" && echo 1 || echo 0)" \
    "$(head -6 "$e2_log")"
asert "E2: y en efecto no se detuvo por recursos" \
    "$(grep -q 'disco insuficiente' "$e2_log" && echo 0 || echo 1)" \
    "$(grep 'insuficiente' "$e2_log" | head -1)"
asert "E2: el release sigue y muere por otra causa (rc no cero esta vez tambien)" \
    "$([ "$e2_rc" -ne 0 ] && echo 1 || echo 0)" "rc=$e2_rc -- un preflight que nunca dejara pasar nada daria rc=0 aqui"
asert "E2: el diagnostico de ese fallo tambien nombra su propio paso" \
    "$(grep -q 'por que fallo el release' "$e2_log" && echo 1 || echo 0)"
asert "E2: este dry-run tampoco llega al 1b" \
    "$(grep -q '1b/15' "$e2_log" && echo 0 || echo 1)"
caso_termina E2

# --- E3: el diagnostico no se repite entre die y trap ------------------------
caso_empieza

# El bloque tiene que salir UNA vez. Repetido se lee como dos fallos distintos y
# desplaza la mirada del operador, que es peor que no diagnosticar.
bloques="$(grep -c 'por que fallo el release' "$e1_log")"
asert "E3: el bloque de diagnostico aparece UNA sola vez" \
    "$([ "$bloques" -eq 1 ] && echo 1 || echo 0)" "apariciones: $bloques"
asert "E3: y el paso aparece una sola vez tambien" \
    "$([ "$(grep -cE '^\s+paso *:' "$e1_log")" -eq 1 ] && echo 1 || echo 0)" \
    "apariciones: $(grep -cE '^\s+paso *:' "$e1_log")"
caso_termina E3

echo
echo "PASS=$PASS FAIL=$FAIL"
if [ "$FAIL" -eq 0 ]; then
    echo "RESULT: PASS - release.sh usa el diagnostico, y se puede superar."
    exit 0
fi
echo "RESULT: FAIL - la libreria existe pero el release no la aprovecha."
exit 1
