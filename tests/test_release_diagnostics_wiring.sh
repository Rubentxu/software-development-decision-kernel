#!/usr/bin/env bash
# shellcheck disable=SC2016
# SC2016, MEDIDO y deliberado: los needles de estos casos llevan `\$` DENTRO de
# comillas simples a proposito —lo que se busca en el fichero es la barra
# literal followed de dollar, porque asi esta escrito en el codigo que se
# comprueba—. Pasarlos a comillas dobles haria que el shell expandiera el
# dollar y el needle buscara otra cosa, luego el caso pasaria por vacuidad
# comparando contra un patron que no es el del codigo. Los payloads de
# python y los `bash -c` de composicion van en comillas simples por la misma
# razon. La directiva es de fichero entero porque el patron se repite en
# decenas de needles y es la misma decision en todos.


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

# Se comprueba la PROPIEDAD --que la limpieza del paso 5 cubra `$TMP` y que sea
# no fatal-- y no la forma literal. MEDIDO: este aserto tenia escrita la cadena
# exacta `rm -rf "$RELEASE_SCRATCH" "$TMP"; }` y cayo al arreglar el orden de los
# dos operandos (el hijo antes que el padre, por el motivo que esta escrito en
# E8), sin que la cobertura hubiera cambiado en nada. Un aserto que mide la
# puntuacion literal de una linea mide lo que uno escribio, que es la clase de
# fallo que E4 vino a cerrar aplicada a otro sitio.
E0_LIMPIA_PASO5="$(awk '
    /^cleanup_release_scratch\(\)/ {
        collecting = 1; buf = $0 "\n"
        if ($0 ~ /\}/) { last = buf; collecting = 0 }
        next
    }
    collecting {
        buf = buf $0 "\n"
        if ($0 ~ /^\}/) { last = buf; collecting = 0 }
    }
    END { printf "%s", last }
' "$RELEASE")"
asert "E0: la limpieza del scratch cubre tambien TMP, que se creo en el paso 5" \
    "$(printf '%s' "$E0_LIMPIA_PASO5" | grep -q 'TMP' && echo 1 || echo 0)" \
    "definicion: $(printf '%s' "$E0_LIMPIA_PASO5" | tr -d '\n' | cut -c1-60)"
asert "E0: y es no fatal, o el release sale con el codigo de su propia limpieza" \
    "$(printf '%s' "$E0_LIMPIA_PASO5" | grep -q '|| true' && echo 1 || echo 0)" \
    "definicion: $(printf '%s' "$E0_LIMPIA_PASO5" | tr -d '\n' | cut -c1-60)"

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

# MEDIDO: dentro del release, E2 caia con UNA asercion —"el diagnostico de ese
# fallo tambien nombra su propio paso"— y fuera pasaba entero. La causa es la
# misma que la del defecto C4/C5 de la libreria, y la clase es "un caso de test
# no puede heredar el estado que le impone el entorno": `release.sh` exporta
# `RELEASE_DIAGNOSED_FILE` apuntando a SU scratch, y ese marcador ya existe
# cuando corre el 1b. El dry-run anidado lo hereda, su `release_on_exit` ve el
# marcador y **no imprime el bloque** —porque la idempotencia hace exactamente
# lo que debe hacer—, luego el asert que busca el paso no lo encuentra. El
# defecto era invisible fuera del entorno que lo produce, que es la forma mas
# dificil de encontrar.
#
# El arreglo es que el test sea dueno de su marcador, no que la idempotencia
# afloje: la idempotencia esta bien en produccion —es lo que hace que el bloque
# salga una vez cuando `die` y el manejador corren los dos—, y lo que estaba mal
# era que el test lo compartia. Cada invocacion borra el marcador antes de
# arrancar, que es el estado fresco real de un proceso nuevo, y por eso el
# diagnostico se tiene que imprimir.
WIRE_TMPDIR="$(mktemp -d)"
WIRE_DIAG_MARKER="$WIRE_TMPDIR/.sddk-diagnosed-wiring"
export RELEASE_DIAGNOSED_FILE="$WIRE_DIAG_MARKER"
limpiar_marcador() { rm -f "$WIRE_DIAG_MARKER"; }
limpiar_marcador

e1_log="$(mktemp)"
e2_log="$(mktemp)"
e1_rc=0
e2_rc=0

limpiar_marcador
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
# que es. Afirma que con margen el release **SUPERA el preflight** y llega a su
# paso siguiente. No afirma que muera por la admision, porque la causa
# siguiente depende del estado del arbol de trabajo, que un test no puede
# suponer: MEDIDO, con el arbol sucio el release muere en `working tree is
# dirty`, que esta en el MISMO paso 0 y por detras del preflight.
#
# MEDIDO, y por que estas dos lineas se reescribieron: el caso decia aqui que
# no afirmaba que el dry-run se muriera, y dos aserciones mas abajo afirmaban
# exactamente eso —"el release sigue y muere por otra causa (rc no cero)" y "el
# diagnostico de ese fallo tambien nombra su propio paso"—. El caso se
# contradicia a si mismo, y sus dos aserciones se apoyaban en que el dry-run
# SIEMPRE se muriera despues del preflight. MEDIDO, y aqui esta el motivo por el
# que eso era una bomba: cuando se arreglo la limpieza (commit 2297de3e) el
# dry-run **completo y salio con 0**, y las dos aserciones cayeron **sin que el
# producto hubiera cambiado en nada**. Un caso que exige que el sistema falle
# para poder pasar no mide una propiedad del sistema: mide el defecto que mas
# tarde se arreglo, y el dia que se arregla cae —que es la forma mas cara de
# descubrir que un guard estaba pegado a un sintoma—. Por eso la afirmacion es
# ahora la que el caso siempre dijo: el gate deja seguir. Y la segunda es un
# **implicado** —"si se para, dice por que y en que paso"—, que es verdadero
# con o sin muerte y por eso no presupone ninguna.
#
# Y se ha medido por que NO hay una tercera asercion: se intento, buscando
# `release admission` como marca de "llegó al paso siguiente", y cayo al
# ejecutarse con el arbol sucio —esa linea va DESPUES del `working tree is
# dirty`—. Un aserto que depende del estado del arbol no es un aserto, es una
# bomba que solo se ve cuando alguien desarrolla.

limpiar_marcador
SDDK_RELEASE_MIN_FREE_MB=1 SDDK_RELEASE_MIN_AVAIL_MB=1 \
    bash "$RELEASE" --dry-run --skip-tests > "$e2_log" 2>&1 || e2_rc=$?

asert "E2: con margen el preflight se supera y lo DICE" \
    "$(grep -q 'recursos comprobados' "$e2_log" && echo 1 || echo 0)" \
    "$(head -6 "$e2_log")"
asert "E2: y en efecto no se detuvo por recursos" \
    "$(grep -q 'disco insuficiente' "$e2_log" && echo 0 || echo 1)" \
    "$(grep 'insuficiente' "$e2_log" | head -1)"
# Lo que demuestra que el gate NO es una puerta trasera son las dos
# aserciones de arriba juntas, y no hace falta una tercera: E1 con margen
# INSUFICIENTE muere nombrando el disco, y E2 con margen SUFICIENTE lo supera
# y no se para por recursos. Umbral y no puerta. MEDIDO: se intento anadir una
# tercera asercion —"se alcanza el paso siguiente del preflight", buscando
# `release admission`— y cayo al ejecutarse con el arbol sucio, porque esa
# linea se imprime DESPUES del `working tree is dirty` y un test no puede
# suponer el estado del arbol. Un aserto que depende del arbol no es un
# aserto: es una bomba que solo se ve cuando alguien desarrolla.
#
# Lo que si se exige es el implicado, que es verdadero con y sin muerte y por
# eso no presupone ninguna: si el dry-run se para, tiene que decir por que y en
# que paso.
# SC2015, MEDIDO: estaba escrito como `[ ... ] && echo 1 || { ...; }`, que no
# es un if-else -- el `{...}` corre tambien si el `echo 1` de en medio falla--.
# Aqui no puede fallar, luego el aviso es tecnicamente correcto y la forma es
# correcta tambien; se reescribe igual porque un `A && B || C` que solo es
# seguro por casualidad se lee como seguro, y ese es el tipo de cosa que
# somebody copia a un sitio donde B si puede fallar.
if [ "$e2_rc" = "0" ]; then
    E2_IMPLICA=1
elif grep -q 'por que fallo el release' "$e2_log"; then
    E2_IMPLICA=1
else
    E2_IMPLICA=0
fi
asert "E2: y si aun asi se para, el bloque nombra su propio paso" \
    "$E2_IMPLICA" "rc=$e2_rc"
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

# --- E4: la PROPIEDAD, no la bandera ---------------------------------------
#
# MEDIDO en el quinto intento de 2.9.0: el arreglo de la sesion-80 ponia
# `--skip-tests` en las dos invocaciones del guard, lo cual impedia volver a
# entrar por el 1b. Pero el `if` de SKIP_TESTS cierra antes del paso 2, y los
# CATORCE gates que invocan un test estan todos despues. El 3m corre el
# falsador, que invoca este mismo fichero, que lanza `release.sh --dry-run
# --skip-tests`, que se salta 1 y 1b pero LLEGA al 3m. Se observaron tres
# niveles vivos creciendo y una release parada mas de 30 minutos en un gate
# que jamas iba a pasar.
#
# Por eso E0 comprobaba que la bandera ESTUVIERA, y esto comprueba lo otro: que
# bajo `--skip-tests` no quede NINGUN test alcanzable. Un needle sobre la
# bandera volveria a dar verde en cuanto alguien anadiera un gate nuevo, que es
# exactamente como se colaron los trece que faltaban.
caso_empieza E4
SIN_CUBRIR="$(codigo "$RELEASE" | awk '
    /(^|[^_[:alnum:]])(bash|python3)[[:space:]]+tests\// {
        cubierto = 0
        for (i = NR - 4; i < NR; i++) {
            if (i > 0 && (buf[i] ~ /test_gate/ || buf[i] ~ /SKIP_TESTS/)) cubierto = 1
        }
        if (!cubierto) print "linea " NR ": " $0
    }
    { buf[NR] = $0 }
')"
asert "E4: ningun test que release.sh invoca queda fuera de test_gate o de SKIP_TESTS" \
    "$([ -z "$SIN_CUBRIR" ] && echo 1 || echo 0)" \
    "sin cubrir -> $(printf '%s' "$SIN_CUBRIR" | head -2 | tr '\n' ' ')"
asert "E4: y hay gates de verdad que comprobar (si la lista fuera vacia el caso no mediria nada)" \
    "$([ "$(codigo "$RELEASE" | grep -cE '(bash|python3)[[:space:]]+tests/')" -ge 10 ] && echo 1 || echo 0)"
caso_termina E4

# --- E5: el helper se EJERCE, no se lee ------------------------------------
#
# Un needle sobre el texto de `test_gate` pasaria aunque la funcion devolviera
# lo contrario. Se extrae la definicion real del release.sh y se ejecuta.
caso_empieza E5
TEST_GATE_FN="$(sed -n '/^test_gate() {/,/^}/p' "$RELEASE")"
R_SKIP="$(printf '%s\n' "$TEST_GATE_FN" | SKIP_TESTS=1 bash -c 'source /dev/stdin; test_gate x >/dev/null 2>&1; echo $?' 2>/dev/null)"
R_RUN="$(printf '%s\n' "$TEST_GATE_FN" | SKIP_TESTS=0 bash -c 'source /dev/stdin; test_gate x >/dev/null 2>&1; echo $?' 2>/dev/null)"
asert "E5: con SKIP_TESTS=1 declara el NO_EJECUTADO y devuelve 0" \
    "$([ "$R_SKIP" = "0" ] && echo 1 || echo 0)" "devolvio '$R_SKIP'"
asert "E5: y con SKIP_TESTS=0 devuelve 1, que es lo que deja pasar al gate" \
    "$([ "$R_RUN" = "1" ] && echo 1 || echo 0)" "devolvio '$R_RUN'"
DECLARA="$(printf '%s\n' "$TEST_GATE_FN" | SKIP_TESTS=1 bash -c 'source /dev/stdin; test_gate NOMBRE_DEL_GATE' 2>&1)"
asert "E5: y lo que declara nombra el flag y el gate, no dice solo 'skipping'" \
    "$(printf '%s' "$DECLARA" | grep -q 'NO_EJECUTADO (--skip-tests)' && printf '%s' "$DECLARA" | grep -q 'NOMBRE_DEL_GATE' && echo 1 || echo 0)" \
    "declaracion: $(printf '%s' "$DECLARA" | tr -d '\n' | cut -c1-60)"
caso_termina E5

# --- E6: el guard no depende del entorno que lo invoca ------------------------
#
# MEDIDO: dentro del release, E2 caia con UNA asercion —"el diagnostico de ese
# fallo tambien nombra su propio paso"— y fuera pasaba entero. La clase es
# "un caso de test no puede heredar el estado que le impone el entorno": el
# release exporta `TMPDIR="$RELEASE_SCRATCH"` y `RELEASE_DIAGNOSED_FILE`, luego
# el test hereda estado del proceso que lo invoca, y un asert que depende de
# ese estado tiene un fallo que SOLO aparece cuando lo hay. Un guard que solo
# se puede ver fallar dentro de un release es un guard cuyo fallo se descubre
# cuando el release se para, que es la forma mas cara de descubrirlo.
#
# Este caso comprueba la PROPIEDAD que hace al guard independiente del entorno:
# es dueno de su marcador y lo limpia antes de CADA invocacion, luego E1 no le
# deja a E2 un marcador ya escrito. Se comprueba sobre el codigo del propio test
# y NO ejecutandose a si mismo con un entorno falso: la primera version de este
# caso hacia justo eso, se lanzaba a si mismo, y eso es **la misma clase de
# regresion** que el arreglo de la sesion-80 —una prueba que se re-dispara a si
# misma—, y se produjo en el primer intento con decenas de procesos vivos. Un guard
# que se prueba asi necesita un guard que lo vigile, y el unico que puede
# vigilarlo es el mismo mecanismo que evita que la prueba se repita.
#
# LO QUE NO SE AFIRMA, escrito para que nadie lo lea mas fuerte: esto demuestra
# que el marcador es del test, no que el fallo original este causado por el
# marcador. Quitar el arreglo **no reproduce** el fallo fuera del release, luego
# la causa exacta sigue sin estar aislada y se dice aqui para que nadie la de
# por cerrada.
caso_empieza E6
CODIGO_WIRE="$(codigo "$RELEASE_WIRE")"
asert "E6: el test exporta SU marcador, y no hereda el del release" \
    "$(printf '%s' "$CODIGO_WIRE" | grep -q 'export RELEASE_DIAGNOSED_FILE="\$WIRE_DIAG_MARKER"' && echo 1 || echo 0)"
asert "E6: y el marcador cuelga de un temporal suyo, no de un caminho heredado" \
    "$(printf '%s' "$CODIGO_WIRE" | grep -q 'WIRE_DIAG_MARKER="\$WIRE_TMPDIR/' && echo 1 || echo 0)"
n_limpia="$(printf '%s' "$CODIGO_WIRE" | grep -c '^limpiar_marcador$')"
asert "E6: limpia el marcador ANTES de cada dry-run, para que E1 no se lo deje a E2" \
    "$([ "$n_limpia" -ge 3 ] && echo 1 || echo 0)" "llamadas: $n_limpia"
asert "E6: la propiedad se comprueba por codigo y no relanzando el test (una autorrecursion es la regresion que se arranco)" \
    "$(printf '%s' "$CODIGO_WIRE" | grep -qE 'bash "\$\{BASH_SOURCE\[0\]\}"' && echo 0 || echo 1)"
caso_termina E6

# --- E7: el codigo que se diagnostica es el del comando, no el de `die` -------
#
# MEDIDO en el noveno intento de 2.9.0: el `cargo test --workspace` salio
# no-cero SIN imprimir un solo `FAILED` y el bloque de "por que fallo el
# release" dijo `codigo : 1`. Ese 1 era el `exit 1` de `die`, no el de cargo.
# `die` es la UNICA via de salida no-cero de release.sh, luego con un `exit 1`
# fijo las ramas 137/139/134/135 de `cause_of_exit_code` -- las unicas que
# separan "el comando fallo" de "el proceso murio" -- eran INALCANZABLES POR
# CONSTRUCCION. La tabla estaba escrita y probada unitariamente, y no podia
# activarse nunca: un asert sobre una rama inalcanzable no mide que la rama
# exista, mide que la tabla tiene texto.
#
# Este caso compone el camino REAL -- `die`, el trap y la libreria, extraidas
# del release.sh de verdad -- y exige la PROPIEDAD, no la presencia de un
# bloque: que la causa nombrada sea la del comando que fallo, y que dos muertes
# por senal DISTINTAS se distingan. Un caso que solo comprobara "sale algo"
# pasaria con el defecto puesto, que es el fallo de instrumento mas caro.
#
# El codigo de PROCESO se comprueba aparte a proposito: el arreglo no lo cambia,
# porque `die` sigue saliendo con 1 y ese es el contrato que el resto espera.
# Fijarlo aqui es lo que impide que un arreglo futuro "propague el codigo" y
# rompa en silencio a quien dependa del 1.
caso_empieza E7
E7_BLOQUE=""
E7_RC=""
E7_N=0
componer_camino_real() {
    local comando="$1" salida="$WIRE_TMPDIR/e7-bloque.txt" marcador
    E7_N=$((E7_N + 1))
    marcador="$WIRE_TMPDIR/e7-marcador-$E7_N"
    # MEDIDO, y es la MISMA clase que E6 un caso mas abajo: la primera version
    # de este helper heredaba el `RELEASE_DIAGNOSED_FILE` que el propio test
    # exporta para sus dry-runs, luego la idempotencia de `release_diagnose_exit`
    # -- que es CORRECTA -- suprimia el bloque en la segunda, tercera y cuarta
    # invocacion. Dos casos de E7 caian y la causa no era el producto sino que el
    # sujeto heredaba estado del entorno: leer eso como "el arreglo no funciona"
    # habria hecho tirar el arreglo bueno. El caso es dueno de su marcador, y lo
    # limpia ANTES de cada invocacion.
    rm -f "$marcador" 2>/dev/null
    {
        printf '%s\n' ". \"$LIB\""
        sed -n '/^die()/,/^}/p' "$RELEASE"
        sed -n '/^release_on_exit()/,/^}/p' "$RELEASE"
    } | RELEASE_DIAGNOSED_FILE="$marcador" RELEASE_CURRENT_STEP="paso de prueba" bash -c '
        cleanup_release_scratch() { :; }
        trap release_on_exit EXIT
        source /dev/stdin
        eval "$1"
        die "el comando fallo"
    ' _ "$comando" >/dev/null 2>"$salida"
    E7_RC=$?
    E7_BLOQUE="$(cat "$salida" 2>/dev/null)"
}

# El sujeto tiene que EXISTIR: si el comando de ejemplo no hubiera muerto por
# senal, el caso pasaria por vacuidad y estaria probando un `die` sin causa.
componer_camino_real 'sh -c "kill -SEGV \$\$"'
E7_SEGV="$E7_BLOQUE"
componer_camino_real 'sh -c "kill -KILL \$\$"'
E7_KILL="$E7_BLOQUE"

# El antidispositivo de vacuidad va primero, y es la asercion que mas dice: si
# el comando no muere por la senal que se le pidio, todo lo de abajo es teatro.
case "$E7_SEGV" in
    *"codigo   : 139"*) E7_SUJETO_VIVO=1 ;;
    *)                  E7_SUJETO_VIVO=0 ;;
esac
asert "E7: el sujeto EXISTE - el comando muere por SIGSEGV y el bloque lo registra como 139" \
    "$E7_SUJETO_VIVO" "bloque: $(printf '%s' "$E7_SEGV" | tr -d '\n' | cut -c1-70)"

# La propiedad: la causa nombrada es la del comando, no la constante de `die`.
case "$E7_SEGV" in
    *"causa    : SIGSEGV"*) E7_NOMBRA_SEGV=1 ;;
    *)                      E7_NOMBRA_SEGV=0 ;;
esac
asert "E7: nombra SIGSEGV, la causa real, y no el 'fallo declarado por release.sh'" \
    "$E7_NOMBRA_SEGV" "causa: $(printf '%s' "$E7_SEGV" | grep 'causa' | cut -c1-70)"
case "$E7_SEGV" in
    *"fallo declarado por release.sh"*) E7_CONSTANTE=1 ;;
    *)                                 E7_CONSTANTE=0 ;;
esac
asert "E7: y el texto constante de 'un die' ya no aparece cuando hay causa real" \
    "$([ "$E7_CONSTANTE" = "0" ] && echo 1 || echo 0)"

# Dos muertes distintas NO pueden dar el mismo texto: si la causa fuera una
# constante elegida al azar, esto pasaria con un `echo SIGSEGV` en `die`.
case "$E7_KILL" in
    *"causa    : SIGKILL"*) E7_DISTINTA=1 ;;
    *)                      E7_DISTINTA=0 ;;
esac
asert "E7: una muerte distinta (SIGKILL) produce una causa distinta: lee el codigo, no lo inventa" \
    "$E7_DISTINTA" "causa: $(printf '%s' "$E7_KILL" | grep 'causa' | cut -c1-70)"

# El arreglo cambia lo que se DIAGNOSTICA, no lo que el proceso devuelve.
asert "E7: el proceso sigue exiting 1, que es el contrato que el resto espera" \
    "$([ "$E7_RC" = "1" ] && echo 1 || echo 0)" "rc real: '$E7_RC'"

# Y ambos codigos se dicen, porque un 139 sin el 1 al lado hace pensar que el
# release se estrello en vez de que el COMANDO se estrello.
case "$E7_SEGV" in
    *"proceso  : 1"*) E7_AMBOS=1 ;;
    *)                E7_AMBOS=0 ;;
esac
asert "E7: el bloque dice el codigo de la causa Y el de proceso, que no son lo mismo" \
    "$E7_AMBOS"

# El borde que el arreglo introduce: `die` desatado tras un comando que SI
# funciono no tiene causa que publicar. Si publicara ese 0, el bloque entero
# desapareceria y el release se pararia sin decir por que: el fallo mas caro de
# todos, y el que este arreglo podia crear sin querer.
componer_camino_real 'true'
case "$E7_BLOQUE" in
    *"causa    : fallo declarado por release.sh"*) E7_SIN_CAUSA=1 ;;
    *)                                             E7_SIN_CAUSA=0 ;;
esac
asert "E7: un die sin comando fallido sigue dando diagnostico (publicar un 0 lo borraria)" \
    "$E7_SIN_CAUSA" "bloque: $(printf '%s' "$E7_BLOQUE" | tr -d '\n' | cut -c1-70)"
caso_termina E7

# --- E8: una limpieza no puede decidir el resultado del release ---------------
#
# MEDIDO en el decimo intento de 2.9.0: el dry-run anidado de E2 llegaba a su
# `exit 0` y salia con **1**, sin imprimir bloque. `bash -x` lo deja claro: el
# manejador entra con `local code=0`, `release_diagnose_exit 0` sale sin
# imprimir —bien, un release verde no se diagnostica— y ahi mismo revienta. La
# causa es que `$TMP` es un SUBDIRECTORIO del scratch (`TMPDIR="$RELEASE_SCRATCH"`
# en la linea 86 y `mktemp -d` en la 1217), luego `rm -rf "$RELEASE_SCRATCH"
# "$TMP"` borra el padre PRIMERO y el hijo ya no existe cuando le llega el
# turno; con `set -euo pipefail` ese fallo aborta el manejador antes de su
# `return "$code"`, y el codigo que sale es el de la limpieza.
#
# Es la clase mas cara que hay: **un release que termina bien se declara
# fallido, y el bloque de diagnostico no dice nada**, porque el codigo que se
# capturo era 0 y 0 no se diagnostica. Y el efecto de rebote es peor: E2
# afirmaba "el release sigue y muere por otra causa (rc no cero)", y pasaba
# **por el motivo equivocado** -- no habia muerto por ninguna causa, se habia
# muerto en su limpieza--. Una asercion cuyo nombre dice una cosa y cuyo motivo
# es otra es peor que no tenerla, porque ocupa el sitio de la que diria la
# verdad. Ese fue el aviso, y por eso este caso existe.
#
# POR QUE SE COMPONE Y NO SE EJECUTA UN DRY-RUN ENTERO, que es la pregunta
# obvia y la que casi se cometio: un dry-run completo solo pasa de preflight con
# el ARBOL LIMPIO, y el preflight dice `working tree is dirty` si no lo esta.
# MEDIDO: al escribir este caso con el arbol sucio, las tres primeras
# aserciones cayeron y E2 "pasaba" por la causa que su propio comentario
# advertia. Un caso que solo puede correr en un arbol limpio es un caso que no
# se puede correr mientras se desarrolla, que es justo cuando hace falta, y
# tampoco tendria dientes en el falsador --que muta el fichero en el sitio y
# por tanto SIEMPRE working tree dirty--. Se compone entonces el camino real
# —las DOS definiciones de `cleanup_release_scratch`, el manejador y la
# libreria, extraidas del release.sh de verdad— con el mismo `set -euo pipefail`
# que tiene el release, y se le pide lo que se le pide al release: terminar.
caso_empieza E8
# Las dos definiciones se extraen enteras; la de la 1223 es la que reintroduce
# `$TMP`, y es la que falla. `awk` en vez de `sed` porque hay dos bloques con la
# misma firma y hay que quedarse con la ULTIMA completa, no con una linea suelta
# de cualquiera de las dos —medido: `tail -1` de las dos daba solo el `}` y la
# composicion no arrancaba, o sea un fallo de instrumentacion disfrazado de
# fallo del producto—.
E8_LIMPIA_FN="$(awk '
    /^cleanup_release_scratch\(\)/ {
        collecting = 1; buf = $0 "\n"
        if ($0 ~ /\}/) { last = buf; collecting = 0 }
        next
    }
    collecting {
        buf = buf $0 "\n"
        if ($0 ~ /^\}/) { last = buf; collecting = 0 }
    }
    END { printf "%s", last }
' "$RELEASE")"
E8_LIMPIA_PILA="$({ printf '%s\n' ". \"$LIB\""; sed -n '/^die()/,/^}/p' "$RELEASE"; sed -n '/^release_on_exit()/,/^}/p' "$RELEASE"; printf '%s\n' "$E8_LIMPIA_FN"; } | bash -c 'source /dev/stdin; echo OK' 2>&1)"
asert "E8: el sujeto EXISTE - la composicion arranca y trae las dos piezas reales" \
    "$(printf '%s' "$E8_LIMPIA_PILA" | grep -c 'OK' >/dev/null && printf '%s' "$E8_LIMPIA_PILA" | grep -q 'OK' && echo 1 || echo 0)" \
    "salida: $(printf '%s' "$E8_LIMPIA_PILA" | tr -d '\n' | cut -c1-60)"
asert "E8: y la limpieza que se compone es la del paso 5, la que reintroduce TMP" \
    "$(printf '%s' "$E8_LIMPIA_FN" | grep -q 'TMP' && echo 1 || echo 0)" \
    "definicion: $(printf '%s' "$E8_LIMPIA_FN" | tr -d '\n' | cut -c1-70)"

# El escenario que se midio, montado de verdad: un scratch con un HIJO dentro,
# que es la forma que tiene en el release (`$TMP` cuelga del scratch), y el
# padre se borra antes que el hijo.
escenario_release() {
    local raiz="$WIRE_TMPDIR/e8-scratch"
    { printf '%s\n' ". \"$LIB\""
      sed -n '/^die()/,/^}/p' "$RELEASE"
      sed -n '/^release_on_exit()/,/^}/p' "$RELEASE"
      printf '%s\n' "$E8_LIMPIA_FN"
    } | bash -c '
        set -euo pipefail
        cleanup_tmp() { :; }
        RELEASE_SCRATCH="$1"
        TMP="$RELEASE_SCRATCH/tmp.hijo"
        mkdir -p "$TMP"
        source /dev/stdin
        trap release_on_exit EXIT
        cleanup_tmp
        exit "$2"
    ' _ "$raiz" "$1" >/dev/null 2>"$WIRE_TMPDIR/e8-salida.txt"
    echo $?
}

# La primera mitad, y es la que estaba rota: termina bien -> sale 0.
E8_RC_OK="$(escenario_release 0)"
E8_SALIDA_OK="$(sed 's/\x1b\[[0-9;]*m//g' "$WIRE_TMPDIR/e8-salida.txt")"
E8_BLOQUE=0
case "$E8_SALIDA_OK" in
    *"por que fallo el release"*) E8_BLOQUE=1 ;;
esac
asert "E8: un release que TERMINA BIEN sale con 0, no con el codigo de su limpieza" \
    "$([ "$E8_RC_OK" = "0" ] && echo 1 || echo 0)" "rc real: '$E8_RC_OK'"
asert "E8: y no imprime bloque de fallo, porque no fallo" \
    "$([ "$E8_BLOQUE" = "0" ] && echo 1 || echo 0)"

# La segunda mitad, que es la que evita que E2 pase por el motivo equivocado y
# la que impide "arreglar" esto tapando la senal en vez de arreglar la causa: un
# release que se para SALE CON BLOQUE y nombra su causa.
#
# El codigo que se espera aqui es 137 y NO 1, y se declara por que: este
# escenario sale con `exit 137` DIRECTO, sin pasar por `die`, luego es el
# camino de una muerte por senal y ahi el release propaga su codigo real. El 1
# pertenece al camino de `die`, que es el unico que lo aplana a proposito. Los
# dos caminos se nombran porque confundirlos seria justo el defecto que E7 vino
# a cerrar.
E8_RC_KO="$(escenario_release 137)"
E8_SALIDA_KO="$(sed 's/\x1b\[[0-9;]*m//g' "$WIRE_TMPDIR/e8-salida.txt")"
E8_BLOQUE2=0
case "$E8_SALIDA_KO" in
    *"por que fallo el release"*) E8_BLOQUE2=1 ;;
esac
E8_NOMBRA2=0
case "$E8_SALIDA_KO" in
    *"SIGKILL"*) E8_NOMBRA2=1 ;;
esac
asert "E8: y uno que se PARA sale con bloque, luego el 0 de antes no es una senal tapada" \
    "$([ "$E8_BLOQUE2" = "1" ] && [ "$E8_NOMBRA2" = "1" ] && echo 1 || echo 0)" \
    "rc=$E8_RC_KO, bloque=$E8_BLOQUE2, nombra=$E8_NOMBRA2"
asert "E8: una muerte por senal que NO pasa por die conserva su codigo; el 1 es solo de die" \
    "$([ "$E8_RC_KO" = "137" ] && echo 1 || echo 0)" "rc real: '$E8_RC_KO'"
caso_termina E8

echo
echo "PASS=$PASS FAIL=$FAIL"
if [ "$FAIL" -eq 0 ]; then
    echo "RESULT: PASS - release.sh usa el diagnostico, y se puede superar."
    exit 0
fi
echo "RESULT: FAIL - la libreria existe pero el release no la aprovecha."
exit 1
