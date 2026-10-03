#!/usr/bin/env bash
# tests/test_install_signature_execution.sh
#
# Por que este guard existe, y por que no es el de assets:
#
# `tests/test_install_asset_contract.sh` comprueba el TEXTO de install.sh. Un
# `local bundle_file` sin asignacion, con todas sus lecturas presentes, es
# invisible para un guard de texto: el patron que busca sigue ahi, la
# declaracion sigue ahi, y el instalador sigue pareciendo completo. Ese
# defecto llego a produccion con 3b0dc9dc (la reescritura del anchor
# key-based), que dejo las TRES asignaciones de destino fuera y se llevo por
# delante la instalacion de cualquier release. Solo se vio al EJECUTAR la via
# de distribucion, que es lo que este guard hace.
#
# Es la misma clase que INC-DEBT-056c: se habia verificado el codigo y nunca
# el comportamiento. Un guard que compara codigo no puede observar una
# variable sin fuente; solo puede observarla algo que la ejecute con `set -u`.
#
# El contrato que ata:
#   (1) `verify_signature` alcanza una DECISION en todos los caminos y no
#       muere por una variable sin fuente. Con `set -u` activo, "morirse" y
#       "decidir no verificar" son salidas DISTINTAS, y el instalador las
#       confunde: la segunda es un refusal honesto, la primera es un fallo que
#       no se puede saltar ni con SDDK_ALLOW_UNSIGNED=1, porque la rama que
#       honra esa variable esta DESPUES de la lectura.
#   (2) El camino sin firma CON `SDDK_ALLOW_UNSIGNED=1` ACCEPTA y avisa. Sin
#       este control, un guard que rechazase todo pasaria la falsificacion sin
#       vigilar nada: un guard que solo rechaza no vigila, informa.
#   (3) Una firma presente pero invalida REFUSA, y nombra los DOS anclas.
#   (4) Una firma detached sin `.pem` REFUSA: sin certificado no hay nada que
#       emparejar con `--certificate-identity`, luego pasaria cualquier
#       firmante.
#   (5) El ancla en estado de transicion REFUSA y no emite clave.
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# La ruta es sobreescribible para que la pareja de autofalsacion pueda apuntar
# a una copia en sandbox. La copia va DENTRO de tests/ a proposito: una
# falsificacion en /tmp hace que las rutas relativas del producto resuelvan
# contra otro arbol, y los fallos que se ven entonces son de la copia y no de
# la mutacion -- que es como se leen mal las autofalsaciones.
INSTALL_SH="${SDDK_INSTALL_SH:-$REPO/scripts/install.sh}"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

PASS=0
FAIL=0
ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

[ -f "$INSTALL_SH" ] || { echo "FALLO: no existe $INSTALL_SH"; exit 1; }

# ── Extraccion de la funcion REAL del fichero REAL ────────────────────────
# Texto del script de distribucion, no una reescritura: si la funcion cambia,
# esto cambia con ella. Sin esto el guard mediria una copia, y una copia
# puede estar bien mientras el producto esta roto.
# Se extraen LAS TRES funciones de las que depende la decision. Si el arnes
# sustituyera `_signature_absent` o `_rebuild_verify_key` por stubs, una
# mutacion sobre ellas seria INOBSERVABLE: el guard mediria el stub, y una
# autofalsacion que no puede observar su propia diana no es autofalsacion.
# Solo se simula lo que de verdad necesita red o un binario externo.
extract_functions() {
    local src="$1" out="$2"
    awk '
        /^(verify_signature|_signature_absent|_rebuild_verify_key)\(\) \{/ { inblock = 1; grab = 1 }
        inblock { print }
        inblock && /^\}/ { inblock = 0; grab = 0; print "" }
    ' "$src" > "$out"
    for fn in verify_signature _signature_absent _rebuild_verify_key; do
        grep -q "^$fn() {" "$out" || return 1
    done
}

# ── Arnes ──────────────────────────────────────────────────────────────────
# Los stubs se declaran antes de invocar; bash resuelve al llamar, asi que el
# orden no altera lo medido. Lo que se mide es la funcion del producto.
#
# TRES cosas que este arnes tiene que modelar bien, y las tres importan mas
# que los casos:
#
#  1. El destino se escribe CON CONTENIDO. El producto decide con `-s`, que
#     es "existe Y tiene tamano"; un fichero de cero bytes no lo cumple. Con
#     `: > "$dest"` el stub miente sobre el formato de lo que descarga.
#  Solo quedan DOS stubs, y los dos son inevitables: `download_optional`
#  descarga de red y `cosign` es un binario externo. `_signature_absent` y
#  `_rebuild_verify_key` se EXTRAEN DEL PRODUCTO, porque un guard que sustituye
#  la logica que quiere medir no mide esa logica.
#
#  2. El stub de `cosign` DISTINGUE las dos anclas por sus argumentos, porque
#     la funcion las invoca distinto: con `--key` es la key-based, sin el es
#     la legacy. Un stub que devuelve lo mismo para las dos hace que "el
#     key-based falla y el legacy salva" sea indistinguible de "el key-based
#     acepto".
build_harness() {
    {
        cat <<'STUBS'
set -uo pipefail
download_optional() {
    local url="$1" dest="$2" base
    base="$(basename "$url")"
    case "$base" in
        *.bundle.json) [ "${SIG_BUNDLE:-0}"   = 1 ] && printf '{"stub":"bundle"}\n' > "$dest" && return 0 ;;
        *.sig)         [ "${SIG_DETACHED:-0}" = 1 ] && printf 'stub-signature\n'    > "$dest" && return 0 ;;
        *.pem)         [ "${SIG_CERT:-0}"     = 1 ] && printf 'stub-certificate\n' > "$dest" && return 0 ;;
    esac
    return 1
}
cosign() {
    local anchor="legacy"
    case " $* " in *" --key "*) anchor="key" ;; esac
    case "$anchor:${COSIGN_MODE:-absent}" in
        key:key_ok)       return 0 ;;
        key:legacy_ok)    return 1 ;;
        key:both_fail)    return 1 ;;
        legacy:legacy_ok) return 0 ;;
        legacy:both_fail) return 1 ;;
        *)                return 1 ;;
    esac
}
STUBS
        cat "$1"
    } > "$2"
}

# ── Aislamiento por caso ───────────────────────────────────────────────────
# `verify_signature` escribe sus temporales JUNTO al artefacto descargado
# (`$file.sig`, `$file.pem`, `$file.bundle.json`, `$file.sddk-anchor.pub`).
# Con los casos compartiendo directorio, uno hereda los del anterior: con S4
# dejando un `sddk.pem`, S6 --que no descarga certificado-- encontraba uno
# ajeno, `-s` era cierto, y el refusal por "detached sin .pem" no se
# disparaba. S6 PASABA POR LA RAZON CONTRARIA a la que mide. Ademas el guard
# escribia dentro del checkout y le ensuciaba el arbol a quien lo ejecutara.
#
# Cada caso corre en su directorio propio, y se comprueba que no le sobre
# ningun temporal: un temporal que sobrevive es un temporal que se ha colado
# en el caso siguiente, y por eso el chequeo va con `! -name sddk` en vez de
# contar ficheros sin pensarlo.
run_case() {
    local id="$1" expect="$2" needle="$3" setup="$4"
    local d="$WORK/case-$id"
    mkdir -p "$d"
    bash -c "cd '$d'; export SDDK_RELEASE_VERIFY_KEY_BODY=\"$ANCHOR_REAL\"; source '$WORK/harness.sh'; $setup; verify_signature \"\$PWD/sddk\" 'https://example.invalid/sddk' bin" \
        >"$WORK/$id.out" 2>&1
    local rc=$?
    printf '%s' "$rc" > "$WORK/$id.rc"
    # NO se exige que el caso no deje temporales: la funcion deja a proposito
    # los ficheros de firma junto al artefacto, y en el producto viven en el
    # TMP_DIR del instalador, que `cleanup()` posee entero. Exigir un
    # directorio vacio seria marcar como fallo el comportamiento correcto.
    # Lo que SI importa es que cada caso tenga SUYO y que no pueda heredar
    # nada: eso lo comprueba el caso S6, que no descarga certificado y por
    # tanto no puede tener un `.pem` propio.
    if [ "$rc" = "$expect" ]; then
        ok "$id rc=$rc (esperado $expect)"
    else
        bad "$id rc=$rc, esperado $expect"
    fi
    if [ -n "$needle" ] && grep -qi -- "$needle" "$WORK/$id.out"; then
        ok "$id dice «$needle»"
    elif [ -z "$needle" ]; then
        ok "$id sin texto exigido"
    else
        bad "$id no dice «$needle»: $(tail -2 "$WORK/$id.out" | tr '\n' ' ' | cut -c1-90)"
    fi
}

echo "== contrato: verify_signature alcanza una decision y no muere por una variable sin fuente =="

if ! extract_functions "$INSTALL_SH" "$WORK/fn.sh"; then
    bad "no se pudo extraer verify_signature del install.sh real"
else
    ok "verify_signature + _signature_absent + _rebuild_verify_key extraidas del install.sh real ($(wc -l < "$WORK/fn.sh") lineas)"
    build_harness "$WORK/fn.sh" "$WORK/harness.sh"
    # Ancla real para los casos que verifican, y su ausencia declarada para el
    # que debe rechazarla. Es la variable que lee la funcion DEL PRODUCTO.
    ANCHOR_REAL='ZmFrZS1hbmNob3Ita2V5LWJvZHk='

    # S1 sin firma y sin permiso: REFUSAL. Y no por variable sin fuente.
    run_case S1 1 'cannot verify' \
        'SIG_BUNDLE=0; SIG_DETACHED=0; SIG_CERT=0; SDDK_ALLOW_UNSIGNED=0; COSIGN_MODE=absent'
    if grep -qi 'unbound variable\|variable sin asignar' "$WORK/S1.out"; then
        bad "S1 murio por una VARIABLE SIN FUENTE: $(grep -i 'unbound\|sin asignar' "$WORK/S1.out" | head -1)"
    else
        ok "S1 no murio por variable sin fuente"
    fi

    # S2 CONTROL DE NO-VACUIDAD: sin firma pero CON permiso -> ACEPTA y avisa.
    run_case S2 0 'SDDK_ALLOW_UNSIGNED=1' \
        'SIG_BUNDLE=0; SIG_DETACHED=0; SIG_CERT=0; SDDK_ALLOW_UNSIGNED=1; COSIGN_MODE=absent'

    # S3 firma key-based valida: ACEPTA y lo dice.
    run_case S3 0 'signature verified' \
        'SIG_BUNDLE=1; SDDK_ALLOW_UNSIGNED=0; COSIGN_MODE=key_ok'

    # S4 el key-based falla, el legacy verifica: acepta por el SEGUNDO ancla.
    run_case S4 0 'keyless legacy anchor' \
        'SIG_BUNDLE=1; SIG_DETACHED=1; SIG_CERT=1; SDDK_ALLOW_UNSIGNED=0; COSIGN_MODE=legacy_ok'

    # S5 firma presente pero invalida contra los dos anclas: REFUSA y nombra ambos.
    run_case S5 1 'BOTH anchors' \
        'SIG_BUNDLE=1; SDDK_ALLOW_UNSIGNED=0; COSIGN_MODE=both_fail'

    # S6 detached SIN .pem: REFUSA. Aqui el certificado no puede heredarse.
    run_case S6 1 'no .pem certificate' \
        'SIG_BUNDLE=0; SIG_DETACHED=1; SIG_CERT=0; SDDK_ALLOW_UNSIGNED=0; COSIGN_MODE=legacy_ok'

    # La asercion que habria cazado el defecto original, ahora que los casos
    # van aislados: S6 no descarga .pem, luego en SU directorio no puede
    # haber ninguno. Con los casos compartiendo directorio, S4 dejaba uno y
    # S6 se lo encuentrava; esta comprobacion es la que hace imposible que
    # eso vuelva a pasar por el mismo camino.
    if [ -e "$WORK/case-S6/sddk.pem" ]; then
        bad "S6 tiene un .pem que no descargo: el caso heredo un temporal de otro"
    else
        ok "S6 sin .pem propio: los casos no comparten temporales"
    fi

    # S7 el ancla en transicion: REFUSA y no emite clave. El centinela va
    # LITERAL y no por variable del guard, y no por estilo: bash no reescanea
    # el resultado de `$setup`, luego un `$ANCHOR_PLACEHOLDER` dentro del
    # setup llegaria literal al `bash -c` interno, ya con `set -u` activo, y
    # el caso dying con 127 por una variable del arnes. Un fallo que parece
    # del producto y es de la instrumentacion es el peor sitio posible para
    # descubrirlo.
    run_case S7 1 'placeholder' \
        'export SDDK_RELEASE_VERIFY_KEY_BODY='@@SDDK_TRANSITION_ANCHOR_NOT_A_REAL_KEY@@'; SIG_BUNDLE=1; SDDK_ALLOW_UNSIGNED=0; COSIGN_MODE=key_ok'

    # DESPUES del ultimo caso, no antes. Puesto aqui el recuento daba 6
    # directorios para 7 casos -- porque S7 aun no habia corrido -- y el
    # fallo que reportaba era del sitio donde estaba escrita la comprobacion,
    # no del aislamiento que dice medir. Una asercion colocada antes de que
    # su precondicion se cumpla no vigila: diagnostica su propia colocacion.
    ndirs="$(find "$WORK" -maxdepth 1 -type d -name 'case-*' | wc -l)"
    if [ "$ndirs" -eq 7 ]; then
        ok "cada caso en su directorio ($ndirs distintos, aislamiento comprobable)"
    else
        bad "aislamiento ROTO: $ndirs directorios de caso para 7 casos — uno reutiliza el de otro"
    fi
fi

# ── El guard declara su propia pareja de autofalsacion ─────────────────────
MUT="tests/test_install_signature_execution_mutation.sh"
if [ -f "$REPO/$MUT" ]; then
    ok "pareja de autofalsacion presente: $MUT"
else
    bad "sin pareja de autofalsacion: $MUT — un guard que no se falsifica no demuestra que vigile"
fi

echo
echo "PASS=$PASS FAIL=$FAIL"
[ "$FAIL" -eq 0 ] || exit 1
echo "RESULT: PASS — la via de distribucion alcanza una decision en los 7 caminos y no muere por una variable sin fuente."
