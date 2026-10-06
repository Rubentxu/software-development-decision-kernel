#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# C3m.3 — el BUILD del provider se OBSERVA; la FAMILIA se nombra.
#
# ## QUE INTENTA REPRODUCIR
#
# `AnalysisBasis::provider_build` entra en el digest de la evidencia
# (`code_intelligence_port_mcp.rs`, `digest_result` lo hashea primero). Con un
# literal ahi, dos proveedores distintos que sirven el mismo claim producen
# evidencia indistinguible, y el receipt afirma una identidad de build sin
# haberla medido. MEDIDO: el CLI construia el `AnalysisBasis` a mano con
# `provider_build: "cognicode-mcp/verify-cmd".to_string()` mientras acababa de
# arrancar el binario que le Passingara `--provider-bin`.
#
# ## POR QUE ESTO ES RESTITUCION Y NO DECISION
#
# ADR-0155 (accepted, 2026-10-03, MISMO ciclo) afirma en §Consecuencias que
# `provider_build` "viene de un string declarado por el adaptador". MEDIDO: eso
# era FALSO en el CLI hasta este corte. El adapter si lo derivaba del
# `serverInfo.version` anunciado, pero `verify_kernel_cmd.rs` se construia su
# propio basis. Este guard vigila que la afirmacion del ADR sea cierta en el
# codigo, no que se cumpla una regla nueva.
#
# ## LO QUE ESTE GUARD EXIME, Y POR QUE
#
# El locator (`cognicode-mcp://find_usages/...`) y el `producer`
# (`"cognicode-mcp"`) SE QUEDAN, y este guard no los prohibe. ADR-0155 §218 lo
# dice del adaptador: "No declara que nombrar al proveedor en un *adaptador*
# sea un defecto: es donde el nombre vive". El nombre de FAMILIA es lo que este
# adapter es por construccion; el BUILD es lo que hay que medir.
#
# Se intento derivarlos de `provider_build` y era incorrecto por tres motivos,
# todos medidos y no evidentes:
#   1. `provider_build` es `nombre/version`, luego como esquema URI produce
#      `cognicode-mcp/0.4.1://...`, que esta malformado (un esquema no lleva `/`).
#   2. C2a fijo que cambiar ese URI exige ADR (`c2a-msgfix/SCOPE-CONTRACT.md` F5).
#   3. No hacia falta: el digest ya lleva la identidad al `basis`.
#
# Un guard que prohibe el nombre sin afirmar que la identidad siga siendo
# MEDIBLE empuja al primero que llega a "neutralizar" borrando la atribucion.
# Por eso la capa D afirma que la identidad observada sigue llegando a la
# evidencia, y no solo que el literal desaparecio.
#
# ## LAS CUATRO CAPAS, Y CUAL ES LA UNICA QUE DISTINGUE "OBSERVADO" DE "ESCRITO"
#
#   A. El runtime no fabrica el build: no construye `AnalysisBasis` a mano y el
#      literal no medido no existe.
#   B. El adapter deriva del `serverInfo` NEGOCIADO, no de una constante propia.
#   C. El valor se concatena de forma reconocible: cambiar la identidad observada
#      cambia la cadena. A y B pueden cumplirse y C fallar.
#   D. POSITIVA. El build observado SIGUE siendo load-bearing: entra en el digest
#      y ese digest es lo que el CLI convierte en base de la observacion. Esta
#      capa es la que impide que el defecto se "arregle" borrando la identidad.
#
# ## AISLAMIENTO
#
# Lee el arbol. No compila, no invoca el binario, no toca red. Un guard que
# compila el mundo entero para decir "esta escrito asi" paga el target entero
# para dar una informacion que un `grep` bien dirigido ya da.
# ---------------------------------------------------------------------------
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || exit 1

PASS=0; FAIL=0
ok()  { printf '  [ok]   %s\n' "$*"; PASS=$((PASS+1)); }
ko()  { printf '  [FAIL] %s\n' "$*"; FAIL=$((FAIL+1)); }

CMD=crates/sddk-cli/src/verify_kernel_cmd.rs
ADAPTER=crates/sddk-engine/src/code_intelligence_port_mcp.rs

# Codigo sin comentarios a pie de linea ni comentarios de bloque completos.
# Se cuentan solo lineas cuyo primer token NO es `//`, porque un comentario que
# NOMBRA lo retirado es documentacion correcta (ADR-0154, ADR-0155 §4) y un guard
# que obliga a borrarla esta pidiendo una mentira util.
sin_comentarios() {
    grep -vE '^[[:space:]]*(//|/\*|\*|\*/)' "$1" || true
}

printf '== C3m.3 — el build del provider se observa; la familia se nombra ==\n'

# --- A. el runtime no fabrica el build ---------------------------------------
if [ ! -f "$CMD" ]; then
    printf '  [FAIL] no existe %s\n' "$CMD"; FAIL=$((FAIL+1))
else
    # La forma concreta del defecto: construir el AnalysisBasis a mano.
    if grep -q 'AnalysisBasis {' "$CMD"; then
        ko "A el runtime construye un AnalysisBasis a mano; la identidad vuelve a ser suya"
    elif grep -qE 'let basis = provider\.basis\(' "$CMD"; then
        ok "A el basis sale de provider.basis(), no de una construccion local"
    else
        ko "A no se reconoce de donde sale el basis; el guard no mide lo que cree medir"
    fi

    # Y el literal concreto, que es un claim de build nunca medido. Solo en
    # codigo: el comentario que lo cita al explicar el defecto es correcto.
    codigo="$(sin_comentarios "$CMD")"
    if printf '%s' "$codigo" | grep -q 'cognicode-mcp/verify-cmd'; then
        ko "A el literal de build no medido sigue en codigo ejecutable"
    else
        ok "A el literal de build no medido no esta en codigo ejecutable"
    fi

    # El literal no puede haber sido movido de aqui a otro fichero de produccion.
    #
    # MEDIDO: esta comprobacion dio un FALSO POSITIVO en su primera forma.
    # Filtraba el resultado de `grep -rn` con `sin_comentarios`, que asume
    # lineas de fichero; pero `grep -rn` antepone `ruta:linea:`, luego ninguna
    # linea empezaba por `//`, el filtro no descartaba nada y la propia linea de
    # comentario del CLI —que cita el literal al explicar el defecto— contaba
    # como reaparicion. Un guard que no distingue su propio instrumento del
    # codigo que mide esta midiendo sobre algo que no es el codigo.
    #
    # El `awk` de abajo separa el contenido del prefijo antes de mirar si es
    # comentario, que es lo que la comprobacion queria decir.
    if grep -rn --include='*.rs' 'cognicode-mcp/verify-cmd' crates/*/src/ 2>/dev/null \
        | awk '{ c = $0; sub(/^[^:]*:[0-9]+:/, "", c); if (c !~ /^[[:space:]]*(\/\/|\/\*|\*)/) print }' \
        | grep -q .; then
        ko "A el literal de build no medido reaparece en otro modulo de src/"
    else
        ok "A el literal de build no medido no reaparece en ningun modulo de src/"
    fi
fi

# --- B. el adapter deriva del announce negociado -----------------------------
if [ ! -f "$ADAPTER" ]; then
    printf '  [FAIL] no existe %s\n' "$ADAPTER"; FAIL=$((FAIL+1))
else
    if grep -q 'provider_build: format!.*server_version' "$ADAPTER"; then
        ok "B provider_build se deriva de server_version"
    else
        ko "B provider_build no se deriva de server_version"
    fi

    # Y que server_version venga del handshake, no de una constante.
    #
    # MEDIDO: la primera version buscaba `serverInfo` en crudo. Eso lo
    # satisfacia el DOC de `basis()`, que lo cita al explicar que de donde sale
    # la identidad: el check daba verde con el handshake desconectado. Un check
    # que lee su propia documentacion como si fuera codigo medira la prosa.
    if sin_comentarios "$ADAPTER" | grep -q 'serverInfo'; then
        ok "B server_version se lee del announce del servidor (serverInfo)"
    else
        ko "B server_version no viene del handshake: la identidad no esta observada"
    fi

    # basis() tiene que ser alcanzable desde el comando, o el arreglo seria
    # codigo muerto y el comando seguiria sin identidad.
    if grep -q 'pub fn basis' "$ADAPTER"; then
        ok "B basis() es publico: el runtime puede preguntar al adapter"
    else
        ko "B basis() sigue privado: el comando no podria obtener la identidad observada"
    fi
fi

# --- C. el contenido depende del valor observado ------------------------------
#
# Si el guard se conforma con que `provider_build` exista, un
# `format!("cognicode-mcp/{}", version)` con la version ignorada pasaria. La
# forma exigida es la que de verdad concatena el valor.
if grep -qE 'provider_build: format!\("[^"]*/\{\}", self\.lock_state\(\)\.server_version\.clone\(\)\)' "$ADAPTER"; then
    ok "C provider_build concatena el server_version: cambiar la identidad cambia la evidencia"
else
    ko "C provider_build no concatena el server_version de forma reconocible; \
la identidad podria seguir siendo una constante"
fi

# --- D. POSITIVA: la identidad observada sigue siendo load-bearing -----------
#
# Sin esta capa, A y B se cumplen igual borrando el campo: el defecto se
# "arregla" destruyendo la trazabilidad en vez de medilandola.
if [ ! -f "$ADAPTER" ] || [ ! -f "$CMD" ]; then
    printf '  [FAIL] D no se puede evaluar sin los dos ficheros\n'; FAIL=$((FAIL+1))
else
    # D1: el build entra en el material del digest. Se exige que sea el PRIMER
    # elemento, que es lo que hace que el digest cambie con la identidad.
    digest_fn="$(sed -n '/fn digest_result/,/^    }/p' "$ADAPTER" | sin_comentarios /dev/stdin)"
    if printf '%s' "$digest_fn" | grep -qE 'let mut material = basis\.provider_build\.clone\(\)'; then
        ok "D1 el build observado es el primer material del digest de evidencia"
    elif printf '%s' "$digest_fn" | grep -q 'provider_build'; then
        ko "D1 provider_build aparece en digest_result, pero no como primer material; \
el digest podria no depender de la identidad"
    else
        ko "D1 el digest de evidencia ya no depende de provider_build: \
arreglar el literal borrando el campo es un fallo, no una solucion"
    fi

    # D2: y el CLI convierte ESE digest en la base de la observacion. Sin esta
    # union, el digest seria correcto y la evidencia no lo usaria.
    if grep -qE 'let input_digest = result\.digest\.0\.clone\(\)' "$CMD" \
        && grep -q 'for_provider_result("verify-cmd", &input_digest)' "$CMD"; then
        ok "D2 el digest del provider es la base de la observacion: la identidad llega a la evidencia"
    else
        ko "D2 el CLI no convierte result.digest en la base de la observacion; \
la identidad medida no llega al id"
    fi
fi

printf '\n----------------------------------------\n'
printf 'PASS=%d FAIL=%d\n' "$PASS" "$FAIL"
if [ "$FAIL" -eq 0 ]; then
    printf 'RESULT: PASS — el build del provider se observa (%d comprobaciones con dientes).\n' "$PASS"
    exit 0
fi
printf 'RESULT: FAIL — hay comprobaciones que no vigilan lo que declaran.\n'
exit 1