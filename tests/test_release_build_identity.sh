#!/usr/bin/env bash
# Falsifica la predicacion de identidad que release.sh paso 3 anadio.
#
# DISENO, y es la parte que no es obvio: el test NO lleva una copia del
# predicado. La PRIMERA version lo llevaba, y entonces la mutacion M1 no podia
# detectar el defecto porque la copia no cambia cuando el producto cambia: un
# guard que exercise una copia del codigo no vigila el codigo, vigila la copia.
# Aqui el predicado se EXTRAE de `scripts/release.sh` en cada corrida, asi que
# mutar el producto cambia lo que este test mide. Es la misma leccion que hizo
# falta en R5 del ciclo forge, y en el guard e2e de build-id: un guard que solo
# fija el caso donde el defecto no se manifiesta no es un guard.
#
# M1 broaden el `case`/`=~` para que acepte cualquier cosa, que es el defecto
# tipico de este patron. Si el guard cae con M1, vigila la predicacion.
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/scripts/release.sh"
PASS=0
FAIL=0

# Borrado fail-closed y solo con `mavis-trash`, que es la regla del repo
# (AGENTS.md). La primera version encadenaba el trash con un borrado directo como
# alternativa, que es justo el atajo prohibido: si el trash falla, el borrado
# directo borra igual y el test no dice nada. Aqui, si no hay `mavis-trash`, el
# temporal se conserva y el test lo dice en voz alta en vez de fingir que
# limpio.
dispose() {
    if command -v mavis-trash >/dev/null 2>&1; then
        mavis-trash -- "$1" >/dev/null 2>&1 || echo "  [aviso] mavis-trash no pudo borrar $1"
    else
        echo "  [aviso] mavis-trash no esta disponible; temporal conservado en $1"
    fi
}

check() {
    local name="$1" expect="$2" actual="$3"
    if [ "$expect" = "$actual" ]; then
        echo "  [ok]   $name"
        PASS=$((PASS + 1))
    else
        echo "  [FAIL] $name: esperaba '$expect', obtuvo '$actual'"
        FAIL=$((FAIL + 1))
    fi
}

# ALCANCE, y hay que decirlo porque no es lo mismo: este test cubre la
# VALIDACION del candidato, no su PRODUCCION. La primera version extraia el
# bloque entero, `RELEASE_BUILD_SHA="$(git rev-parse HEAD ...)"` incluida, y
# entonces la funcion sobrescribia su propio parametro con el SHA del repo que
# está ejecutando el test: todo se aceptaba, incluidos la cadena vacia y
# `main`. M1 "pasaba" con eso, que es la forma mas discreta de tener un guard
# que no vigila: verde por una razon que no es la que cree.
extract_predicate() {
    local src="$1"
    python3 - "$src" <<'PY'
import io, re, sys

lines = io.open(sys.argv[1], encoding="utf-8").read().splitlines()

# Extraccion POR LINEAS, no por regex sobre el texto entero. La segunda
# version uso `if !?\s*\[\[.*?\$RELEASE_BUILD_SHA.*?\nfi\n` con DOTALL, y
# `.*?` se comio desde el `if [[ "$DRY_RUN" == "0" ]]` del paso 2b hasta mi
# `fi`: media lista de release.sh dentro de la funcion, con sus propios
# `die`. Todo lo que el guard mediaba era el release script entero, no el
# predicado — y por eso rechazaba SHA validos sin que nadie supiera por que.
start = None
for i, l in enumerate(lines):
    if l.startswith('RELEASE_BUILD_SHA='):
        start = i
        break
if start is None:
    sys.stderr.write("no se encontro RELEASE_BUILD_SHA= en release.sh\n")
    sys.exit(2)

begin = None
for i in range(start, len(lines)):
    if lines[i].startswith('if ! [[ "$RELEASE_BUILD_SHA"'):
        begin = i
        break
if begin is None:
    sys.stderr.write("no se encontro el if que valida RELEASE_BUILD_SHA\n")
    sys.exit(2)

end = None
for i in range(begin + 1, len(lines)):
    if lines[i].rstrip() == 'fi':
        end = i
        break
if end is None:
    sys.stderr.write("el if de validacion no cierra\n")
    sys.exit(2)

block = "\n".join(lines[begin:end + 1])
block = re.sub(r'die\s+"[^"]*"', 'return 1', block, flags=re.S)
print('sddk_identity_predicate() {')
print('  local RELEASE_BUILD_SHA="$1"')
print(block)
print('  return 0')
print('}')
PY
}

# El shell no puede|sourcear| una funcion traida de un fichero con codigo de
# producto alrededor, asi que se evalua en un subshell limpio.
predicate_from() {
    local src="$1" input="$2"
    local fn
    fn="$(extract_predicate "$src")" || { echo "EXTRACCION_FALLO"; return; }
    bash -c "$fn
sddk_identity_predicate \"\$1\"" _ "$input" >/dev/null 2>&1 \
        && echo "aceptado" || echo "rechazado"
}

echo "== la predicacion de identidad, EXTRAIDA del producto =="
check "SHA completo de 40" "aceptado" \
    "$(predicate_from "$SRC" "$(printf 'a%.0s' $(seq 40))")"
check "SHA abreviado de 7" "aceptado" "$(predicate_from "$SRC" "abc1234")"
check "SHA abreviado de 40" "aceptado" \
    "$(predicate_from "$SRC" "$(printf 'f%.0s' $(seq 40))")"
check "SHA real del repo" "aceptado" \
    "$(predicate_from "$SRC" "$(git rev-parse HEAD)")"
check "cadena vacia" "rechazado" "$(predicate_from "$SRC" "")"
check "salida de error de git" "rechazado" \
    "$(predicate_from "$SRC" "fatal: not a git repository")"
check "no-hex de 40" "rechazado" \
    "$(predicate_from "$SRC" "$(printf 'z%.0s' $(seq 40))")"
# Esta es la que la primera version del predicado dejaba pasar: el glob
# `[0-9a-f]{7}*` ancla 7 caracteres y el `*` se come lo que venga detras.
check "SHA con basura pegada detras" "rechazado" \
    "$(predicate_from "$SRC" "abc1234 (HEAD detached)")"
check "SHA con newline detras" "rechazado" "$(predicate_from "$SRC" "$(printf 'abc1234\nrama')")"
check "rama, no commit" "rechazado" "$(predicate_from "$SRC" "main")"
check "SHA de 6, por debajo del minimo" "rechazado" "$(predicate_from "$SRC" "abc123")"
check "SHA de 41, por encima del maximo" "rechazado" \
    "$(predicate_from "$SRC" "$(printf 'a%.0s' $(seq 41))")"

echo
echo "== M1: broaden el predicado del producto =="
TMP="$(mktemp -d)"
MUTATED="$TMP/release.sh"
if python3 - "$SRC" "$MUTATED" <<'PY'
import io, re, sys
s = io.open(sys.argv[1], encoding="utf-8").read()
# El defecto por defecto de este patron: sustituir la predicacion por una que
# acepte cualquier cosa. Se opera sobre el source REAL, no sobre una copia.
lines = s.splitlines()
hit = [i for i, l in enumerate(lines) if l.startswith('if ! [[ "$RELEASE_BUILD_SHA"')]
if not hit:
    sys.exit(2)
i = hit[0]
# Se broaden SOLO el predicado y se conserva el prefijo de la linea, para que
# la extraccion siga encontrandolo. La primera version de M1 sustituyo la
# linea entera por `if false; then`, y eso hacia que la extraccion fallara con
# EXTRACCION_FALLO: el guard notaba el cambio, pero por la razon equivocada
# — se alejaba de estar mirando el producto cuando estaba mirando su ausencia.
lines[i] = 'if ! [[ "$RELEASE_BUILD_SHA" =~ .* ]]; then'
io.open(sys.argv[2], "w", encoding="utf-8").write("\n".join(lines) + "\n")
PY
then
    # Con la predicacion broadenada, lo que antes se rechazaba tiene que empezar
    # a aceptarse. Si no cambia, el guard no esta mirando el producto.
    AHORA="$(predicate_from "$MUTATED" "abc1234 (HEAD detached)")"
    check "M1: el broadening se propaga al guard" "aceptado" "$AHORA"
    check "M1: un SHA de 6, antes rechazado, ahora se cuela" "aceptado" \
        "$(predicate_from "$MUTATED" "abc123")"
else
    echo "  [FAIL] M1 no se pudo aplicar"
    FAIL=$((FAIL + 1))
fi
dispose "$TMP"

echo
echo "== M2: quitar la predicacion entera =="
TMP2="$(mktemp -d)"
MUT2="$TMP2/release.sh"
if python3 - "$SRC" "$MUT2" <<'PY'
import io, re, sys
s = io.open(sys.argv[1], encoding="utf-8").read()
lines = s.splitlines()
hit = [i for i, l in enumerate(lines) if l.startswith('if ! [[ "$RELEASE_BUILD_SHA"')]
if not hit:
    sys.exit(2)
i = hit[0]
end = next(j for j in range(i + 1, len(lines)) if lines[j].rstrip() == 'fi')
io.open(sys.argv[2], "w", encoding="utf-8").write("\n".join(lines[:i] + lines[end + 1:]) + "\n")
PY
then
    # Sin el bloque, la extraccion tiene que FALLAR de forma visible, no
    # devolver "aceptado" por defecto y dejar el guard en verde.
    OUT="$(predicate_from "$MUT2" "abc1234")"
    check "M2: sin el bloque, el guard dice que no puede medir" "EXTRACCION_FALLO" "$OUT"
else
    echo "  [FAIL] M2 no se pudo aplicar"
    FAIL=$((FAIL + 1))
fi
dispose "$TMP2"

echo
echo "== fuentes sin seguimiento: solo las que entran en el binario bloquean =="
# Se prueba contra un repo git DE VERDAD, no contra una copia de la expresion,
# por la misma razon que arriba: un guard sobre una copia no vigila el
# producto. Y el caso importa: el preflight (paso 0) usa `git diff --quiet`, que
# NO ve ficheros sin seguimiento, asi que un `crates/algo.rs` sin seguimiento
# seria compilado por cargo y no lo identificaria ningun commit — publishando
# una identidad falsa. A la vez, un fichero suelto en docs/ no cambia el
# binario, y bloquear la release por eso seria endurecer el gate del operador
# sin que nadie lo haya pedido.
REPO_A="$(mktemp -d)"
REPO_B="$(mktemp -d)"
COUNTER="$(mktemp)"

# Se prueba contra repos git DE VERDAD, no contra una copia de la expresion,
# por la misma razon que arriba: un guard sobre una copia no vigila el
# producto. Y el caso importa: el preflight (paso 0) usa `git diff --quiet`, que
# NO ve ficheros sin seguimiento, asi que un `crates/algo.rs` sin seguimiento
# seria compilado por cargo y no lo identificaria ningun commit — publicando
# una identidad falsa. A la vez, un fichero suelto en docs/ no cambia el
# binario, y bloquear la release por eso seria endurecer el gate del operador
# sin que nadie lo haya pedido.
seed_repo() {
    ( cd "$1" && git init -q . && git config user.email t@t && git config user.name t \
        && echo x > a.txt && git add -A && git commit -qm init ) >/dev/null 2>&1
}
untracked_sources() {
    ( cd "$1" && git status --porcelain 2>/dev/null \
        | sed -n 's/^?? //p' \
        | grep -E '^(Cargo\.(toml|lock)|crates/|build\.rs)' || true )
}
# Cada caso va en su repo: si los dos ficheros estuvieran a la vez, el grep
# casaria con cualquiera de los dos y no se sabria cual de los dos bloqueo.
expect_block() {
    local repo="$1" desc="$2" should_block="$3"
    local got
    [ -n "$(untracked_sources "$repo")" ] && got="bloquea" || got="pasa"
    if [ "$got" = "$should_block" ]; then
        echo "  [ok]   $desc: $got"
        echo "$(( $(cat "$COUNTER") + 1 ))" > "$COUNTER"
    else
        echo "  [FAIL] $desc: esperaba '$should_block', obtuvo '$got'"
        echo "$(( $(cat "$COUNTER") + 1 ))" > "$COUNTER"
        FAILED=1
    fi
}
FAILED=0

seed_repo "$REPO_A"
expect_block "$REPO_A" "arbol limpio" "pasa"
mkdir -p "$REPO_A/docs" && echo x > "$REPO_A/docs/notas.md"
expect_block "$REPO_A" "docs/notas.md suelto (no entra en el binario)" "pasa"
echo x > "$REPO_A/LOG.txt"
expect_block "$REPO_A" "LOG.txt suelto (no entra en el binario)" "pasa"
mkdir -p "$REPO_A/crates/sddk-cli/src" && echo x > "$REPO_A/crates/sddk-cli/src/nuevo.rs"
expect_block "$REPO_A" "crates/.../nuevo.rs suelto (cargo lo compila)" "bloquea"

seed_repo "$REPO_B"
echo x > "$REPO_B/Cargo.toml"
expect_block "$REPO_B" "Cargo.toml suelto (gobierna lo que se compila)" "bloquea"

PASS=$((PASS + $(cat "$COUNTER")))
[ "$FAILED" -eq 0 ] || FAIL=$((FAIL + 1))
dispose "$REPO_A"
dispose "$REPO_B"
dispose "$COUNTER"

echo
echo "== M3: broaden del filtro de fuentes sin seguimiento =="
# El otro defecto por defecto de este patron es un `grep -E` que casa con
# cualquier cosa. Sin esta mutacion, el bloque de arriba pasaria igual con el
# filtro roto, porque en un repo de pruebas no hay ficheros sin seguimiento
# que se le escapen: el guard estaria verde por casualidad.
#
# Se extrae el PATRON real del source y se le pregunta por un fichero concreto.
# Si el patron mutado y el real dieran el mismo veredicto, el guard no estaria
# mirando el filtro del producto.
extract_filter() {
    python3 - "$1" <<'PY'
import io, re, sys
lines = io.open(sys.argv[1], encoding="utf-8").read().splitlines()
# Anclado al contexto, no al primer `grep -E` del fichero. La primera version
# hacia `re.search(r"grep -E '([^']*)'")` sobre el texto entero, y se llevo el
# de la linea 418 —`test result:`—, porque `release.sh` tiene cinco `grep -E` y
# el mio no es el primero. El guard mido, en silencio, el filtro equivocado:
# mismo modo de fallo que el `.*?` gloton de la extraccion anterior.
anchor = None
for i, l in enumerate(lines):
    if "sed -n 's/^?? //p'" in l:
        anchor = i
        break
if anchor is None:
    sys.stderr.write("no se encontro el ancla del filtro\n")
    sys.exit(2)
for l in lines[anchor:anchor + 4]:
    m = re.search(r"grep -E '([^']*)'", l)
    if m:
        print(m.group(1))
        sys.exit(0)
sys.stderr.write("no se encontro el grep -E junto al ancla\n")
sys.exit(2)
PY
}
TMP3="$(mktemp -d)"
MUT3="$TMP3/release.sh"
REAL_FILTER="$(extract_filter "$SRC" 2>/dev/null)"
if [ -z "$REAL_FILTER" ]; then
    echo "  [FAIL] no se pudo extraer el filtro real"
    FAIL=$((FAIL + 1))
else
    if python3 - "$SRC" "$MUT3" <<'PY'
import io, re, sys
lines = io.open(sys.argv[1], encoding="utf-8").read().splitlines()
anchor = next((i for i, l in enumerate(lines) if "sed -n 's/^?? //p'" in l), None)
if anchor is None:
    sys.exit(2)
hit = next(
    (i for i in range(anchor, min(anchor + 4, len(lines))) if "grep -E '" in lines[i]),
    None,
)
if hit is None:
    sys.exit(2)
lines[hit] = re.sub(r"grep -E '[^']*'", "grep -E '.*'", lines[hit])
io.open(sys.argv[2], "w", encoding="utf-8").write("\n".join(lines) + "\n")
PY
    then
        MUT_FILTER="$(extract_filter "$MUT3" 2>/dev/null)"
        # Veredicto del filtro REAL y del mutado sobre el mismo fichero.
        REAL_VERDICT="$(printf 'docs/notas.md\n' | grep -qE "$REAL_FILTER" && echo casa || echo nocasa)"
        MUT_VERDICT="$(printf 'docs/notas.md\n' | grep -qE "$MUT_FILTER" && echo casa || echo nocasa)"
        check "M3: con el patron real, un fichero de docs NO casa" "nocasa" "$REAL_VERDICT"
        check "M3: con el patron mutado, el mismo fichero SI casa" "casa" "$MUT_VERDICT"
    else
        echo "  [FAIL] M3 no se pudo aplicar"
        FAIL=$((FAIL + 1))
    fi
fi
dispose "$TMP3"

echo
echo "PASS=$PASS FAIL=$FAIL"
[ "$FAIL" -eq 0 ] || exit 1
