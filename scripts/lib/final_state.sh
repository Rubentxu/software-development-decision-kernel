#!/usr/bin/env bash
# final_state.sh — las tres cifras del estado final que el release imprime
# al operador en el paso 15.
#
# POR QUE VIVE AQUI Y NO DENTRO DE release.sh
# El guard de esto tiene que EJECUTAR el codigo que vigila. Pegar el bloque
# en el test fue exactamente el defecto que INC-DEBT-074 acaba de cerrar en
# el merge del changelog: el unico guard de aquel bloque daba verde contra
# una copia del codigo viejo, luego no vigilaba el codigo. Una copia no
# vigila el codigo; una llamada si.
#
# QUE ESTA DE ROTA Y POR QUE (INC-DEBT-072)
# El paso 15 imprimia:
#
#   binary:        sddk 2.10.0
#   bundle:        None
#   current:       2.10.0
#
# MEDIDO sobre el recibo real instalado tras 2.10.0:
#
#   version        = '2.10.0'
#   bundle         = True
#   bundle_version = None      <- JSON null, la CLAVE existe
#   layout         = 'flat'
#
# El `null` es CORRECTO y no es el defecto: en layout flat no existe un
# directorio de bundle con version propia. El defecto es el respaldo:
# `dict.get("bundle_version", "?")` devuelve el valor guardado cuando la
# clave existe, y `null` EXISTE -- luego el default que el autor escribio
# para "no lo se" NUNCA dispara, y sale None impreso con formato de dato.
# Una ausencia con forma de dato es peor que un "?" feo: induce a abrir un
# ticket en una release que acaba de pasar quince gates.
#
# LA REGLA, en una linea: AUSENTE y NULL se tratan igual a proposito (los
# dos son "el recibo no lo dice"), y en layout flat la version del bundle
# ES la del binario, porque no hay bundle separado que la tenga.

# Sin declarar en el recibo, y distinguible de un dato.
FINAL_STATE_UNDECLARED="no-declarado-en-el-recibo"

# final_state_bundle_version <ruta_al_recibo>
#
# Imprime una de tres, y NUNCA la cadena "None":
#   <bundle_version>   layout versionado: el recibo declara cual es
#   <version>          layout flat: el bundle ES el binario
#   <sentinel>         ausente, null sin layout, o recibo ilegible
final_state_bundle_version() {
    local receipt_path="${1:-}"
    if [ -z "$receipt_path" ]; then
        echo "$FINAL_STATE_UNDECLARED"
        return 0
    fi
    python3 - "$receipt_path" <<'PY'
import json
import sys

UNDECLARED = "no-declarado-en-el-recibo"

try:
    with open(sys.argv[1]) as fh:
        receipt = json.load(fh)
except Exception:
    # Ilegible o ausente: el recibo no lo dice. No se inventa.
    print(UNDECLARED)
    raise SystemExit(0)

if not isinstance(receipt, dict):
    print(UNDECLARED)
    raise SystemExit(0)

# AUSENTE y NULL dan el mismo veredicto a proposito. La asimetria que
# rompe el fallback viejo no es "la clave existe" vs "no existe": es que
# el valor guardado puede ser null, y null tambien es "no lo dice".
declared = receipt.get("bundle_version")
if declared is not None:
    print(declared)
elif receipt.get("layout") == "flat":
    # Sin bundle con version propia, la del bundle es la del binario. Sin
    # esta rama la linea imprimiria la ausencia de un dato con forma de dato.
    print(receipt.get("version") or UNDECLARED)
else:
    print(UNDECLARED)
PY
}

# final_state_current_version <framework_dir>
#
# El mismo sentinel para el mismo motivo: un readlink que falla no es un
# dato, y "?" no dice de quien es la duda.
final_state_current_version() {
    local framework_dir="${1:-}"
    local link=""
    if [ -n "$framework_dir" ]; then
        link="$(readlink "$framework_dir/current" 2>/dev/null || true)"
    fi
    if [ -n "$link" ]; then
        basename "$link"
    else
        echo "$FINAL_STATE_UNDECLARED"
    fi
}

# final_state_figures <sddk_prefix> <framework_dir>
#
# Imprime las tres lineas del paso 15, en el orden en que las lee el
# operador. Cada cifra pasa por su propia comprobacion: una linea puede
# ser verdad y la otra no, y una linea que se calla tiene que decirlo.
final_state_figures() {
    local prefix="${1:-}"
    local framework_dir="${2:-}"

    local bin_ver=""
    if [ -n "$prefix" ] && [ -x "$prefix/sddk" ]; then
        bin_ver="$("$prefix/sddk" --version 2>&1 | head -1)"
    fi
    [ -n "$bin_ver" ] || bin_ver="$FINAL_STATE_UNDECLARED"

    local bundle_ver=""
    if [ -n "$prefix" ] && [ -x "$prefix/sddk" ]; then
        # El doctor se sigue ejecutando: es la comprobacion de que el
        # prefijo responde, y el gate 11 ya la uso como veredicto. Lo que
        # NO se hace es leer de el una cifra que no publica -- la version
        # del bundle la declara el recibo, no el doctor.
        "$prefix/sddk" dev doctor --prefix "$prefix" --format json >/dev/null 2>&1 || true
        bundle_ver="$(final_state_bundle_version "$prefix/sddk-install.json")"
    else
        bundle_ver="$FINAL_STATE_UNDECLARED"
    fi

    local current_ver=""
    current_ver="$(final_state_current_version "$framework_dir")"

    echo "  binary:        $bin_ver"
    echo "  bundle:        $bundle_ver"
    echo "  current:       $current_ver"
}
