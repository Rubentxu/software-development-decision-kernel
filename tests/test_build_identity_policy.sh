#!/usr/bin/env bash
# Mide el cumplimiento de politicas del cambio de cl-build-identity.
#
# Existe para que el gate `policy-compliant` no lo afirme un agente: cada
# comprobacion ejecuta y devuelve codigo de salida, y el gate se apoya en eso.
# Lo que no se puede medir de forma fiable se DECLARA como no medido en vez de
# pasar por comprobado.
#
# Uso: tests/test_build_identity_policy.sh
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || exit 2
BASE="${BASE:-dc69e6f2}"
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/var/home/rubentxu/cargo-targets}"
export CARGO_TARGET_DIR
PASS=0
FAIL=0
NOT_MEASURED=0

TMP="$(mktemp -d)"
dispose() {
    if command -v mavis-trash >/dev/null 2>&1; then
        mavis-trash -- "$1" >/dev/null 2>&1 || echo "  [aviso] mavis-trash no pudo borrar $1"
    else
        echo "  [aviso] mavis-trash no disponible; temporal conservado en $1"
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

# `cargo test ... | grep -q PATTERN` NO FUNCIONA aqui, y falló dos veces antes
# de entenderse: `grep -q` sale en cuanto encuentra la coincidencia, `cargo
# test` recibe SIGPIPE escribiendo el resto y sale con error, y como el script
# usa `pipefail` el pipeline devuelve ese error. El guard se ponia verde por
# el motivo equivocado en cuanto la salida crecia lo suficiente — con un solo
# test filtrado pasaba, con los once de `dev::build_id` fallaba. Se captura a
# fichero y se busca despues: sin tuberia, sin SIGPIPE, sin `pipefail` que
# confunda "no aparecio" con "se rompio el pipe".
test_named() {
    local filter="$1" name="$2"
    local out
    out="$TMP/$(echo "${filter}${name}" | tr -c 'a-zA-Z0-9' '_').log"
    # `$filter` va SIN comillas a proposito: son varios argumentos de cargo
    # (`--lib dev::build_id`), no uno. Con `"$filter"` entero, cargo recibe un
    # unico argumento `--lib dev::build_id` que no entiende, falla, y el
    # helper contesta `no-ok` — un guard que miente porque el comando nunca
    # llego a correr. La segunda version de este helper hacia eso, y por eso
    # los tres guards de STOP salian FAIL con el producto en verde.
    # shellcheck disable=SC2086
    cargo test -p sddk-cli $filter > "$out" 2>&1
    grep -q "test .*\b$name\b \.\.\. ok" "$out" && echo ok || echo no-ok
}

echo "== STOP 2: --version no se toca =="
# `install.sh:416` resuelve la version con `awk '{print $NF}'`, luego el ultimo
# campo de la salida tiene que seguir siendo un semver desnudo. La primera
# version de esta comprobacion buscaba un golden `sddk-version.txt` que NO
# existe en `cli_golden/1.168.8/` — la lista real son nueve helps y no hay
# golden de `--version`—, luego se habria reportado «no medido» para siempre
# mientras el gate lo daba por comprobado. El guard de STOP 2 es el e2e
# `r1_version_last_field_is_a_bare_semver`, y a ese hay que preguntarle.
check "el ultimo campo de --version sigue siendo un semver desnudo" "ok" \
    "$(test_named "--test cli" r1_version_last_field_is_a_bare_semver)"

echo
echo "== STOP 4: el bundle no se toca =="
# Si el cambio hubiera alterado la superficie del bundle, MANIFEST.sha256 habria
# cambiado en el rango. No tiene que haber cambiado.
if git diff --name-only "$BASE"..HEAD | grep -qx 'MANIFEST.sha256'; then
    echo "  [FAIL] MANIFEST.sha256 cambio en el rango: la superficie del bundle se toco"
    FAIL=$((FAIL + 1))
else
    echo "  [ok]   MANIFEST.sha256 no cambio en $BASE..HEAD"
    PASS=$((PASS + 1))
fi

echo
echo "== STOP 6: el fallback a .git no decide =="
check "r6_the_git_fallback_never_concludes" "ok" \
    "$(test_named "--lib dev::build_id" r6_the_git_fallback_never_concludes)"
check "r6_unknown_is_never_reported_as_coming_from_git" "ok" \
    "$(test_named "--lib dev::build_id" r6_unknown_is_never_reported_as_coming_from_git)"

echo
echo "== redaccion: SIN contaminacion EN LO QUE ESTE CAMBIO ANADE =="
# Se escanean SOLO las lineas anadidas por el rango, no el fichero entero. La
# primera version pasaba los ficheros al scanner, y `SESSION-JOURNAL.md` esta
# en el rango porque este cambio le anade una entrada — con lo que el guard
# reportaba los 27 caracteres no latinos HISTORICOS que el propio repo declara
# y no reescribe, y fallaba por deuda que no es de este cambio. Es la confusion
# entre medir el cambio y medir la historia, que es el mismo error que el del
# `case` gloton: mirar el fichero entero cuando lo que se pregunta es por el
# delta.
HITS=0
SCANNED=0
for f in $(git diff --name-only "$BASE"..HEAD | grep -E '\.(md|rs|sh|yaml|yml|txt)$'); do
    [ -f "$f" ] || continue
    ADDED="$TMP/added-$(echo "$f" | tr -c 'a-zA-Z0-9' '_').txt"
    git diff -U0 "$BASE"..HEAD -- "$f" \
        | grep '^+' | grep -v '^+++' | sed 's/^+//' > "$ADDED" || true
    # Ficheros anadidos enteros: `git diff` los marca como `+++ /dev/null`.
    if grep -q '^+++ /dev/null' <(git diff -U0 "$BASE"..HEAD -- "$f" | grep '+++' || true); then
        cp "$f" "$ADDED"
    fi
    [ -s "$ADDED" ] || continue
    SCANNED=$((SCANNED + 1))
    # Session-75: esto llamaba a `/var/home/rubentxu/ce/06-scan.py` -- un scanner
    # en una ruta ABSOLUTA, FUERA del repo, en la maquina de quien lo escribio.
    # La autoridad de la regla de contaminacion no estaba en el repo, y ese
    # scanner tenia reglas y excepciones PROPIAS que no coincidian con
    # `test_docs_script_contamination.py`: senalaba `CHANGELOG.md` (declarado ahi
    # como cita intencional) y `SESSION-JOURNAL.md` (excluido ahi por ser
    # append-only, "se miden no se corrigen"). Ademas, en cualquier maquina sin
    # ese fichero, `python3` sale con != 0 y este guard.reportaria
    # contaminacion en todo lo que escanea: rojo falso, no rojo verdadero.
    # Ahora la autoridad es la misma que la del guard que barre el repo entero,
    # y por eso los dos NO pueden discrepar.
    if ! python3 "$ROOT/tests/test_docs_script_contamination.py" \
            --delta "$ADDED" --repo-path "$f" >/dev/null 2>&1; then
        echo "  [FAIL] contaminacion en las lineas anadidas de $f:"
        python3 "$ROOT/tests/test_docs_script_contamination.py" \
            --delta "$ADDED" --repo-path "$f" 2>&1 | grep '^HIT' | head -3
        HITS=$((HITS + 1))
    fi
done
if [ "$HITS" -eq 0 ]; then
    echo "  [ok]   scanner CLEAN en las lineas anadidas ($SCANNED ficheros con delta)"
    PASS=$((PASS + 1))
else
    FAIL=$((FAIL + 1))
fi

echo
echo "== shellcheck en el shell tocado =="
SH="$(git diff --name-only "$BASE"..HEAD | grep -E '\.sh$' | tr '\n' ' ')"
if [ -n "$SH" ]; then
    # shellcheck disable=SC2086
    if shellcheck $SH >/dev/null 2>&1; then
        echo "  [ok]   shellcheck sin avisos"
        PASS=$((PASS + 1))
    else
        echo "  [FAIL] shellcheck reporta avisos:"
        # shellcheck disable=SC2086
        shellcheck $SH 2>&1 | grep -E '^In |SC[0-9]' | head -5
        FAIL=$((FAIL + 1))
    fi
else
    echo "  [no medido] el rango no toca shell"
    NOT_MEASURED=$((NOT_MEASURED + 1))
fi

echo
echo "== gates documentales del repo =="
for t in tests/test_changelog_coverage.sh tests/test_debt_index_coherence.sh; do
    printf '  %-42s ' "$(basename "$t")"
    if bash "$t" >/dev/null 2>&1; then
        echo "PASS"
        PASS=$((PASS + 1))
    else
        echo "FAIL"
        FAIL=$((FAIL + 1))
    fi
done

echo
echo "== lo que NO se puede medir aqui, declarado en vez de dado por bueno =="
echo "  - La regla de cero intrusio (AGENTS.md 1): que este cambio no escriba"
echo "    dentro de otro repo. Es cierto por construccion — solo se han tocado"
echo "    ficheros bajo el CWD — pero este script no lo demuestra, y no se"
echo "    presenta como si lo hiciera."
echo "  - Que el operador este de acuerdo con el umbral de ficheros sin"
echo "    seguimiento que release.sh aplica (mas estricto que el preflight solo"
echo "    para lo que entra en el binario). Decision de contrato del gate de"
echo "    release, suya."

dispose "$TMP"
echo
echo "PASS=$PASS FAIL=$FAIL NO_MEDIDO=$NOT_MEASURED"
[ "$FAIL" -eq 0 ] || exit 1
