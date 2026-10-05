#!/usr/bin/env bash
# lint_gate.sh — el gate de shellcheck del repo, con su severidad DECLARADA
# y su alcance DECLARADO.
#
# QUE ESTA ROTO Y COMO SE MIDIO (INC-DEBT-073)
#
# El gate vivia entero dentro de tests/test_build_identity_policy.sh:
#
#   SH="$(git diff --name-only "$BASE"..HEAD | grep -E '\.sh$' | tr '\n' ' ')"
#   if shellcheck $SH >/dev/null 2>&1; then ... fi
#
# MEDIDO al escribir este fichero, y por eso la segunda linea esta redactada
# como `if shellcheck ...` y no suelta: una linea de comentario que empieza
# por la palabra `shellcheck` la lee shellcheck como una DIRECTIVA suya, y
# `shellcheck $SH` no es una directiva parseable -> SC1073/SC1072, que son
# error. O sea que el fichero que arregla el gate de lint no pasaba el gate
# de lint, por citar la linea que arregla.
#
# Dos propiedades, y las dos medidas sobre ESTE repo:
#
# 1. SIN FILTRO DE SEVERIDAD. Un `info` — o un `style` — cuenta igual que
#    un `error`. MEDIDO: el arbol entero tiene 0 errores y 1 warning, luego
#    la diferencia entre "cobra todo" y "cobra warning" son 12 findings de
#    info que hoy salen como fallo de release.
#
# 2. SOLO EL RANGO. Y el rango no es "el cambio": `BASE` vale dc69e6f2 por
#    defecto, un commit historico. MEDIDO con ese BASE: el gate vigilaba
#    51 de los 100 .sh, y los 49 restantes NO LOS VE NUNCA. El UNICO aviso
#    de warning del repo (SC2034, tests/lib_public_release_gate.sh:84) estaba
#    en uno de esos 49. O sea que el gate estaba VERDE con deuda real dentro,
#    y no por un descuido: por su alcance.
#
# LAS DOS PROPIEDADES JUNTAS
# El conjunto de ficheros que el gate vigila depende de que trabajo haya
# hecho la gente y de que commit historico se eligiera como base, no de que
# trabajo haya que hacer. Eso no lo arregla severidad: lo arregla el ALCANCE.
#
# LO QUE ESTE FICHERO HACE, y por que no es solo un `if`
# - Declara la severidad en una variable, para que se lea y se cite.
# - Cambia el alcance a arbol entero, con `git ls-files` y no con un rango.
# - Pasa la lista como ARRAY. La version anterior hacia
#   `shellcheck $SH` sin comillas, luego un nombre de fichero con espacio
#   se partia en dos.
# - Devuelve SIN LLAMAR a shellcheck cuando la lista esta vacia. Sin
#   argumentos, shellcheck lee stdin y se queda colgado: un colgado se
#   parece a un gate lento, y ahi un runner defectuoso se convierte en una
#   conclusion sobre el producto.
#
# LO QUE NO HACE
# No arregla los 12 findings de info. No los arregla porque la severidad que
# este gate cobra es la que el 1b de scripts/release.sh ya cobra, y bajar la
# severidad es una DECISION que se escribe, no un efecto colateral. Los 12
# quedan como censo medido, no como sorpresa por fichero.

# La severidad minima que este gate cobra. Se escribe aqui y no se supone.
# MEDIDO: es la misma que ya cobra el 1b de scripts/release.sh, y dos gates
# del mismo pipeline que cobran distinto son dos respuestas a la misma
# pregunta sobre el mismo repositorio.
LINT_GATE_SEVERITY="warning"

# El alcance, declarado por el mismo motivo.
#
# SC2034, y la ironia es el punto: esta variable la lee el gate que sourcea
# la libreria, que la imprime en su linea de resumen. No es una variable
# muerta, luego la supresion lleva su motivo — y el motivo es precisamente
# que sin ella este fichero no pasaria SU PROPIO gate, que es el aviso que
# el gate existia para cobrar. Un gate que no puede pasar su propio gate no
# vigila nada.
# shellcheck disable=SC2034
LINT_GATE_SCOPE="whole-tree"

# lint_gate_files — un .sh versionado por linea.
lint_gate_files() {
    git ls-files '*.sh'
}

# lint_gate_findings [severidad] — imprime los findings, uno por bloque, y
# devuelve 0 si no hay ninguno. Imprimirlos y no solo contar es lo que
# permite que quien recibe el fallo sepa de que linea se trata.
lint_gate_findings() {
    local severity="${1:-$LINT_GATE_SEVERITY}"
    local files=()
    local f
    while IFS= read -r f; do
        [ -n "$f" ] && files+=("$f")
    done < <(lint_gate_files)

    # Sin ficheros no se invoca shellcheck: sin argumentos lee stdin.
    [ "${#files[@]}" -gt 0 ] || return 0

    shellcheck --severity="$severity" "${files[@]}" 2>/dev/null
}

# lint_gate_count [severidad] — cuantos findings hay. Es lo que consume el
# gate, que solo necesita un si/no.
lint_gate_count() {
    local out
    out="$(lint_gate_findings "${1:-$LINT_GATE_SEVERITY}")"
    [ -n "$out" ] || return 0
    printf '%s\n' "$out" | grep -cE '^In .* line [0-9]+'
}
