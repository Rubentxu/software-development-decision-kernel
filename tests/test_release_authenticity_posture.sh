#!/usr/bin/env bash
# tests/test_release_authenticity_posture.sh
#
# Por que este guard existe, y por que mide la POSTURA y no el texto:
#
# El paso 9c exigia `.sig` y `.pem` sin condicion. Una release publicada sin
# firma —que el propio 8c admite con un aviso explicito, y que el 9b declara
# COMPLETA porque el contrato canonico de 9 assets no incluye firmas— se
# publicaba y luego moria en 9c con un 404. Medido en la release real de
# v2.5.3: `9/9 canonical assets reachable from public CDN (HTTP 200)` y, un
# paso despues, `could not fetch sddk.sig`, con la release YA publicada y un
# exit != 0 que hace creer que no salio.
#
# La incoherencia era interna: 9b decia "el conjunto esta completo" y 9c decia
# "falta la mitad" del mismo conjunto. Un gate que exige un fichero que otro
# gate del mismo pipeline declara opcional no es un gate, es una contradiccion
# que se resuelve por orden de aparicion.
#
# El contrato que ata:
#   (1) UNSIGNED exige DOS cosas: que el operador lo declarase
#       (SDDK_SKIP_SIGNING=1) Y que no haya ninguna firma sobre la mesa. Un
#       "no verifico" que no mira si hay algo que verificar es una excusa, no
#       una medicion. **Con firmas presentes se verifica igual**: una propiedad
#       de supply-chain que se puede comprobar y se deja sin comprobar por
#       obedecer una bandera es un downgrade silencioso, que es peor que no
#       tener el paso. Este es el caso que sostiene el guard.
#   (2) Sin SDDK_SKIP_SIGNING, 9c sigue INTENTANDO descargar y verificar. El
#       arreglo no puede haber convertido el paso en decoracion.
#   (3) SDDK_SKIP_AUTHENTICITY_CHECK=1 sigue siendo una via distinta, y por
#       debajo de UNSIGNED en precedence: cuando ambas estan puestas y no hay
#       firmas, la verdad que hay que decir es "no hay nada que verificar", no
#       "el operador pidio saltar".
#   (4) El cierre distingue: una release sin verificar NUNCA emite un PASS sin
#       calificar. El `ok "public-release gate PASS"` a pelo era la forma de
#       que un pipeline verde describiese mal lo que habia publicado.
#
# El guard EJECUTA el bloque del producto, no lo compara. Extracto por
# marcadores, no por numeros de linea, y la extraccion se verifica antes de
# correr: si el bloque no aparece exactamente una vez, el guard falla en vez
# de medir el arbol equivocado.
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Sobreescribible para que la pareja de autofalsacion apunte a una copia.
RELEASE_SH="${SDDK_RELEASE_SH:-$REPO/scripts/release.sh}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

PASS=0
FAIL=0
SKIP=0
ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }
skp() { echo "  [skip] $1"; SKIP=$((SKIP + 1)); }

[ -f "$RELEASE_SH" ] || { echo "FALLO: no existe $RELEASE_SH"; exit 1; }

# ── Extraccion ─────────────────────────────────────────────────────────────
# Dos extracciones distintas, y la razon de que sean dos importa: la FUNCION
# decide, el BLOQUE cablea. Medir solo la funcion deja pasar un arreglo que
# nadie llama; medir solo el bloque deja pasar una politica correcta que el
# 9c ignora. Se miden los dos.
EXTRACT_DIR="$WORK/extract"
mkdir -p "$EXTRACT_DIR"

# (1) la funcion de politica
awk '
    /^release_authenticity_posture\(\) \{/ { inblock = 1 }
    inblock { print }
    inblock && /^\}/ { inblock = 0; exit }
' "$RELEASE_SH" > "$EXTRACT_DIR/posture.fn"

if ! grep -q '^release_authenticity_posture() {' "$EXTRACT_DIR/posture.fn"; then
    bad "no se encontro release_authenticity_posture() en $RELEASE_SH"
    echo "  (el 9c tiene que DERIVAR la postura de la firma; sin la funcion no hay politica que medir)"
    echo
    echo "PASS=$PASS FAIL=$((FAIL + 1)) SKIP=$SKIP"
    exit 1
fi
ok "funcion release_authenticity_posture extraida del producto"

# (2) el bloque del 9c, entre su `step` y el cierre del gate REL-1
awk '
    /step "9c\/15 — verify supply-chain authenticity/ { grab = 1 }
    grab { print }
    grab && /# <<< REL-1 public-release gate end <<</ { exit }
' "$RELEASE_SH" > "$EXTRACT_DIR/block.raw"

# El bloque se cierra con el `fi` del REL-1, que NO es parte de lo que se mide.
sed '$d' "$EXTRACT_DIR/block.raw" | sed '$d' > "$EXTRACT_DIR/block.sh"

if ! grep -q 'AUTH_POSTURE' "$EXTRACT_DIR/block.sh"; then
    bad "el bloque del 9c no menciona AUTH_POSTURE — la politica existe pero el 9c no la consulta"
    echo
    echo "PASS=$PASS FAIL=$((FAIL + 1)) SKIP=$SKIP"
    exit 1
fi
ok "bloque del 9c extraido y cablea a la postura"

# ── Arnes ──────────────────────────────────────────────────────────────────
# Se ejecutan DOS cosas del producto, no una: la funcion (que decide) y el
# bloque (que usa lo que decidio). Los stubs son solo los que dependen de la
# red o de un binario externo; la decision y el cableado son los del producto.
build_harness() {
    local body="$1" dest="$2"
    {
        cat <<'STUBS'
set -uo pipefail
COSIGN_PRESENT="${COSIGN_PRESENT:-1}"
# SIGN_ARTIFACTS se declara AQUI, como literal, y no por entorno. Un array no
# cruza como asignacion de prefijo — `A=(x y) bash -c ...` deja a `A` con el
# escalar "(x y)", y `export` de un array no lo exporta — luego el bucle que
# cuenta las firmas it'd iteraria una vez sobre "(x y)" y contaria cero, sin
# ningun error: un guard que miente diciendo "no hay firmas" cuando si las hay.
SIGN_ARTIFACTS=(sddk sddk-unified.tar.gz sddk-bundle.tar.gz)
# Los stubs EMITEN a stdout. Acumular en una variable seria inutil: el arnes
# corre en un subshell, luego esa variable no sale de el y el guard leeria
# un log vacio sin ningun error visible. El log ES la unica salida que cruza
# el subshell, y los contadores se reconstruyen leyendolo.
step() { :; }
warn() { echo "warn: $*"; }
ok()   { echo "ok: $*"; }
die()  { echo "die: $*"; }
cosign() { [ "$COSIGN_PRESENT" = 1 ]; }
mktemp() { printf '%s' "$HARNESS_TMPDIR/auth.$$"; mkdir -p "$HARNESS_TMPDIR/auth.$$"; }
curl() {
    # El 9c descarga por URL; lo que se mide es SI descarga y QUE pide.
    echo "fetch: ${*: -1}"
    return 0
}
STUBS
        cat "$body"
    } > "$dest"
}

# El 9c invoca `bash tests/test_supply_chain_authenticity.sh`; para no tocar el
# arbol se corre el arnes desde una arena con ese path presente.
build_sandbox() {
    local sb="$1"
    mkdir -p "$sb/tests"
    cat > "$sb/tests/test_supply_chain_authenticity.sh" <<'SB'
printf 'PASS=1 FAIL=0\n'
SB
}

# ── Un caso ────────────────────────────────────────────────────────────────
# Escribe el arnes, lo corre desde la arena, y deja el resultado en variables
# legibles por el llamante.
#
# $1 nombre  $2 skip_signing  $3 skip_auth  $4 nº de firmas presentes
# $5 cosign presente (1/0)
run_case() {
    local name="$1" skip_signing="$2" skip_auth="$3" n_sigs="$4" cosign_p="$5"
    local sb="$WORK/case-$name"
    rm -rf "$sb" 2>/dev/null || true
    mkdir -p "$sb/tmpfiles" "$sb/tests"
    build_sandbox "$sb"

    # Firmas reales en $TMP: el bloque 9c las cuenta con `-f`, luego tienen
    # que EXISTIR de verdad. Un `: >` crearia ficheros de 0 bytes, que el
    # producto no mira para contar pero que un `[ -s ]` si miraria — aqui se
    # cuentan por existencia, como hace el producto, y por eso se escriben
    # con contenido.
    local i=0
    while [ "$i" -lt "$n_sigs" ]; do
        printf 'stub-signature-%s\n' "$i" > "$sb/tmpfiles/sddk.sig"
        printf 'stub-bundle-%s\n' "$i"   > "$sb/tmpfiles/sddk.bundle.json"
        i=$((i + 1))
    done

    cat "$EXTRACT_DIR/posture.fn" "$EXTRACT_DIR/block.sh" > "$sb/harness.sh"
    build_harness "$sb/harness.sh" "$sb/run.sh"

    # El arnes corre en un SUBSHELL, luego sus variables no salen de el. Los
    # contadores se inicializan aqui (el guard corre con `set -u`, y referenciar
    # uno sin asignar previo es un fallo, no un cero) y se reconstruyen leyendo
    # el log del propio arnes: el log es la unica salida que cruza el subshell.
    RC=0; DIES=0; FETCHES=0; OUT=""
    ( cd "$sb" \
      && TMP="$sb/tmpfiles" \
         HARNESS_TMPDIR="$sb" \
         VERSION="9.9.9" TAG="v9.9.9" REPO="example/repo" \
         SDDK_SKIP_SIGNING="$skip_signing" \
         SDDK_SKIP_AUTHENTICITY_CHECK="$skip_auth" \
         COSIGN_PRESENT="$cosign_p" \
         bash run.sh > "$sb/stdout" 2>"$sb/stderr" )
    RC=$?

    DIES=$(grep -c '^die: ' "$sb/stdout" 2>/dev/null); DIES=${DIES:-0}
    FETCHES=$(grep -c '^fetch: ' "$sb/stdout" 2>/dev/null); FETCHES=${FETCHES:-0}
    OUT=$(cat "$sb/stdout" 2>/dev/null)
}

has() { printf '%s' "$OUT" | grep -qF -- "$1"; }

# ══════════════════════════════════════════════════════════════════════════
# S1 — release declarada sin firma y SIN firmas: no hay nada que verificar.
# ══════════════════════════════════════════════════════════════════════════
run_case S1 1 0 0 1
if [ "$DIES" != "0" ]; then
    bad "S1 (sin firmar, sin firmas) el 9cmurio: la release publicada sin firma es insatisfacible"
elif [ "$FETCHES" != "0" ]; then
    bad "S1 (sin firmar, sin firmas) el 9c intento descargar $FETCHES assets: no hay firma que verificar"
elif ! has "9c NOT_RUN"; then
    bad "S1 (sin firmar, sin firmas) el 9c no declaro NOT_RUN: un skip sin motivo declarado es indistinguible de un pass"
elif ! has "posture: UNSIGNED"; then
    bad "S1 (sin firmar, sin firmas) la postura no se declaro UNSIGNED"
elif ! has "SIN verificar autenticidad"; then
    bad "S1 el cierre emitio un PASS sin calificar: una release sin verificar jamas se describe como verificada"
else
    ok "S1 sin firmar y sin firmas: NO_RUN declarado con motivo, sin descarga, sin PASS sin calificar"
fi

# ══════════════════════════════════════════════════════════════════════════
# S2 — release firmada: 9c sigue intentando descargar y verifica. El arreglo
# no puede haber convertido el paso en decoracion.
# ══════════════════════════════════════════════════════════════════════════
run_case S2 0 0 0 1
if [ "$DIES" != "0" ]; then
    bad "S2 (firmada) el 9c murio con cosign presente y release firmada"
elif [ "$FETCHES" -lt 9 ]; then
    bad "S2 (firmada) el 9c solo intento $FETCHES descargas: se esperaba el conjunto completo de 9"
elif ! has "posture: VERIFY"; then
    bad "S2 (firmada) la postura no fue VERIFY"
elif has "9c NOT_RUN"; then
    bad "S2 (firmada) el 9c se declaró NOT_RUN: el arreglo lo volvio decoracion"
else
    ok "S2 firmada: 9 descargas y verificacion, la postura sigue siendo VERIFY"
fi

# ══════════════════════════════════════════════════════════════════════════
# S3 — ESTE CASO SOSTIENE EL GUARD. SDDK_SKIP_SIGNING=1 pero hay firmas
# sobre la mesa: se verifican igual. Un "no verifico" que no mira si hay
# algo que verificar es una excusa, no una medicion.
# ══════════════════════════════════════════════════════════════════════════
run_case S3 1 0 1 1
if [ "$FETCHES" -lt 9 ]; then
    bad "S3 (bandera de no-firma PERO firmas presentes) el 9c solo intento $FETCHES descargas: una propiedad de supply-chain comprobable se dejo sin comprobar por obedecer una bandera"
elif ! has "posture: VERIFY"; then
    bad "S3 con firmas presentes la postura fue UNSIGNED: la bandera apago una verificacion que si se podia hacer"
else
    ok "S3 bandera de no-firma con firmas presentes: se verifica igual (downgrade silencioso prevenido)"
fi

# ══════════════════════════════════════════════════════════════════════════
# S4 — la via SDDK_SKIP_AUTHENTICITY_CHECK=1 sigue viva y es distinta.
# ══════════════════════════════════════════════════════════════════════════
run_case S4 0 1 0 1
if [ "$DIES" != "0" ]; then
    bad "S4 (skip de autenticidad declarado) el 9c murio"
elif [ "$FETCHES" != "0" ]; then
    bad "S4 (skip de autenticidad declarado) el 9cNonetheless descargo $FETCHES assets"
elif ! has "posture: DECLARED_SKIP"; then
    bad "S4 la postura no fue DECLARED_SKIP"
elif ! has "SIN verificar autenticidad"; then
    bad "S4 el cierre emitio un PASS sin calificar"
else
    ok "S4 skip de autenticidad declarado: via distinta, sin descarga, cierre calificado"
fi

# ══════════════════════════════════════════════════════════════════════════
# S5 — ambas banderas puestas y sin firmas: manda UNSIGNED, porque la verdad
# que hay que decir es "no hay nada que verificar", no "el operador pidio
# saltar". Son dos atributos distintos de la misma release.
# ══════════════════════════════════════════════════════════════════════════
run_case S5 1 1 0 1
if ! has "posture: UNSIGNED"; then
    bad "S5 (ambas banderas, sin firmas) la postura fue distinta de UNSIGNED: se antepuso el salto pedido al hecho de que no hay nada que verificar"
elif ! has "9c NOT_RUN"; then
    bad "S5 (ambas banderas, sin firmas) no se declaro NOT_RUN con el motivo"
elif [ "$FETCHES" != "0" ]; then
    bad "S5 (ambas banderas, sin firmas) el 9c descargo $FETCHES assets sin nada que verificar"
else
    ok "S5 ambas banderas sin firmas: UNSIGNED gana y declara el motivo real"
fi

# ── Control de no-vacuidad ─────────────────────────────────────────────────
# Un guard que dijera "9c nunca falla" pasaria S1 y S4 (que no descargan) y
# no mediria nada. Este control exige el caso contrario y CORRE SU PROPIO
# CASO: leer las variables que dejo S5 seria mirar UNSIGNED con cero
# descargas y concluir "ok", que es precisamente el falso verde. Aqui se
# corre una release firmada, y se exige que la descarga ocurra de verdad.
# Si alguien "arregla" esto volviendo el paso decoracion, S3 y este control
# caen a la vez, desde dos casos distintos.
run_case C1 0 0 0 1
# RC se usa aqui, y por una razon concreta: un arnes que no se ejecuta y un
# producto que falla dan los dos un log sin veredicto, y confundirlos es
# exactamente el fallo de "esta asercion cae por lo que no esta mal" que ya
# costo una sesion en este repo. Se comprueba que el arnes CORRIO antes de
# atribuirle al producto cualquier conclusion.
if [ "$RC" -ne 0 ] || [ -z "$OUT" ]; then
    bad "control de no-vacuidad: el arnes no se ejecuto (rc=$RC) — cualquier veredicto sobre el producto seria falso"
elif ! has "posture: VERIFY"; then
    bad "control de no-vacuidad: una release firmada no llego a la postura VERIFY"
elif [ "$FETCHES" -eq 0 ]; then
    bad "control de no-vacuidad: la postura dice VERIFY pero el bloque no descargo nada — el paso se ha vuelto decoracion"
else
    ok "control de no-vacuidad: una release firmada llega a VERIFY Y descarga ($FETCHES assets)"
fi

echo
echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
if [ "$FAIL" -eq 0 ]; then
    echo "RESULT: PASS — la postura de autenticidad se deriva de la postura de firma, y un NO_RUN siempre declara su motivo."
    exit 0
fi
echo "RESULT: FAIL — el 9c y la politica de firma siguen sin ser coherentes entre si."
exit 1
