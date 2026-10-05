#!/usr/bin/env bash
# test_reconcile_pointer_yaml_safety.sh
#
# QUE VIGILA, y por que son tres cosas y no una
#
# `scripts/reconcile_state_pointer.sh` es el unico que puede escribir
# STATE.yaml, y lo que escribe lo lee el gate del 1b. Tres defectos se
# comproaron MEDIDOS en session-83, todos en la misma superficie y todos con
# la misma raiz: el script afirmaba cosas que no habia hecho.
#
#   D1 — el `superseded_pointer` recien insertado se emitia como escalar YAML
#        de comillas dobles PARTIDO EN VARIAS LINEAS (f-strings concatenados
#        en lineas fisicas). YAML invalido. Y el repo real no lo notaba
#        porque su STATE.yaml ya tenia `superseded_pointer`: el defecto es
#        LATENTE y solo se dispara en un puntero que aun no lo tenga, o sea
#        en un BOOTSTRAP.
#   D2 — con la version de workspace ilegible escribia
#        `workspace_version_at_current: ""`: un puntero que DECLARA una
#        version vacia, que parece un dato y no lo es.
#   D3 — el bloque de validacion salia con 0 sin haber movido nada, y decia
#        "no hay PyYAML" cuando la causa real era el YAML invalido que D1
#        acababa de producir. Un exit 0 sin efecto: el llamante se lleva
#        una promesa en vez de una medida.
#
# El mas grave de los tres es D3, porque es el que convierte a D1 y D2 en
# silenciosos. Un defecto que se anuncia es un defecto acotado; uno que se
# declara correcto es un defecto que se propaga.
#
# LO QUE NO HACE: no reimplementa el puntero. Usa el script real sobre repos
# miniatura y lee el resultado con un lector DISTINTO del que escribe (PyYAML
# en vez del sed del propio script), porque un guard que valida con el mismo
# instrumento con el que se escribio no puede ver un error de escritura.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXER="$ROOT/scripts/reconcile_state_pointer.sh"

PASS=0
FAIL=0
ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

echo "== test_reconcile_pointer_yaml_safety.sh =="

if [ ! -f "$FIXER" ]; then
    echo "  [FAIL] falta $FIXER"
    echo
    echo "PASS=0 FAIL=1"
    exit 1
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# arena <dir> <con_superseded: si|no> <con_cargo: si|no>
#
# El conmutador `con_superseded` es el que reproduce el caso de bootstrap, que
# es donde D1 vivía sin que nadie lo viera: con el campo presente, la rama
# defectuosa ni se ejecuta.
arena() {
    local d="$1" con_sup="$2" con_cargo="$3"
    mkdir -p "$d/scripts" "$d/docs/roadmap"
    cp "$FIXER" "$d/scripts/reconcile_state_pointer.sh"
    if [ "$con_cargo" = "si" ]; then
        printf '[workspace.package]\nversion = "9.9.9"\n' > "$d/Cargo.toml"
    fi
    {
        printf 'schema_version: 1\nsource:\n'
        printf '  baseline_branch: main\n'
        printf '  baseline_sha: "0000000"\n'
        printf '  current_sha: "0000000"\n'
        printf '  head_at_state_sync: "0000000"\n'
        [ "$con_sup" = "si" ] && printf '  superseded_pointer: "viejo/1.0 nota"\n'
        printf '  workspace_version_at_current: "0.0.1"\n'
    } > "$d/docs/roadmap/STATE.yaml"

    git -C "$d" init -q
    git -C "$d" config user.email "guard@example.invalid"
    git -C "$d" config user.name  "guard"
    git -C "$d" config commit.gpgsign false
    git -C "$d" add -A
    git -C "$d" commit -q -m "base"
    local n
    for n in 1 2 3 4 5; do
        printf 'linea %s\n' "$n" > "$d/notas.txt"
        git -C "$d" add -A
        git -C "$d" commit -q -m "trabajo $n"
    done
    local base
    base="$(git -C "$d" rev-parse --short HEAD~5)"
    sed -i "s/current_sha: \"0000000\"/current_sha: \"$base\"/" "$d/docs/roadmap/STATE.yaml"
    sed -i "s/head_at_state_sync: \"0000000\"/head_at_state_sync: \"$base\"/" "$d/docs/roadmap/STATE.yaml"
    git -C "$d" add -A
    git -C "$d" commit -q -m "el puntero se queda atras"
}

# yaml_ok <fichero> — LECTOR INDEPENDIENTE. Devuelve 0 si PyYAML lo parsea.
yaml_ok() { python3 -c "import yaml,sys; yaml.safe_load(open(sys.argv[1],encoding='utf-8'))" "$1" 2>/dev/null; }

echo
echo "-- D1: el puntero recien insertado produce YAML VALIDO (caso bootstrap) --"

d="$WORK/bootstrap"; arena "$d" no si
out="$( cd "$d" && bash scripts/reconcile_state_pointer.sh 2>&1 )"
rc=$?
state="$d/docs/roadmap/STATE.yaml"
if [ "$rc" -ne 0 ]; then
    bad "el reconciliador salio $rc en un caso legitimo (deberia reconciliar)"
    printf '%s\n' "$out" | tail -8 | sed 's/^/         /'
elif ! yaml_ok "$state"; then
    bad "el STATE.yaml resultante NO es YAML valido — el superseded_pointer se emitsio partido"
elif [ -f "$state.reconcile-candidate" ]; then
    bad "dejo un .reconcile-candidate y aun asi salio 0"
else
    ok "el puntero con superseded_pointer NUEVO es YAML valido y se sustituyo en el sitio"
fi

# Y el valor insertado tiene que SER el punterior, no una cadena vacia.
prev="$(sed -n 's/^  superseded_pointer: *"\([^/]*\)\/.*/\1/p' "$state" | head -1)"
if [ -n "$prev" ] && [ "$prev" != "0000000" ] && [ "$prev" != "<none>" ]; then
    ok "el superseded_pointer registra de donde venia el puntero ($prev/...)"
else
    bad "el superseded_pointer no registra el puntero anterior (obtenido: '$prev')"
fi

echo
echo "-- D2: sin version de workspace, FALLA CERRADO en vez de declarar \"\" --"

d="$WORK/sinversion"; arena "$d" no no
out="$( cd "$d" && bash scripts/reconcile_state_pointer.sh 2>&1 )"
rc=$?
if [ "$rc" -eq 0 ]; then
    bad "salio 0 sin poder leer la version — declararia una version vacia como si fuera un dato"
else
    ok "salio $rc (no 0) sin version legible: no se escribe un puntero que no sabe"
fi
if printf '%s' "$out" | grep -q 'workspace.package'; then
    ok "el mensaje nombra el campo que no pudo leer"
else
    bad "el mensaje no dice que campo falta: $(printf '%s' "$out" | tail -2 | tr '\n' ' ')"
fi

echo
echo "-- D3: un YAML generado invalido se NIEGA, no se disfraza de falta de PyYAML --"

# Aqui se reproduce D1 a proposito: se inyecta el escalar partido en una copia
# del script, y se exige que el script lo DETECTE y muera. Es la prueba de que
# el fallo se anuncia, que es la diferencia entre un defecto acotado y uno
# que se propaga.
d="$WORK/invalido"; arena "$d" no si
python3 - "$d/scripts/reconcile_state_pointer.sh" <<'PY'
import sys
from pathlib import Path
p = Path(sys.argv[1])
t = p.read_text()
# Se rompe la serializacion a proposito: el texto pasa a contener un salto de
# linea dentro del escalar, que es exactamente D1.
marcador = '    superseded = "  superseded_pointer: " + json.dumps(superseded_note, ensure_ascii=False)'
if marcador in t:
    t = t.replace(marcador,
                  '    superseded = "  superseded_pointer: \\"" + superseded_note[:40] + "\\n" + superseded_note[40:]')
    p.write_text(t)
    print("inyectado")
else:
    print("NO INYECTADO: el ancla no esta", file=sys.stderr)
    raise SystemExit(1)
PY
out="$( cd "$d" && bash scripts/reconcile_state_pointer.sh 2>&1 )"
rc=$?
if [ "$rc" -eq 0 ]; then
    bad "con un escalar partido salio 0 — el defecto D1 seguiria siendo silencioso"
elif printf '%s' "$out" | grep -qi 'no hay PyYAML'; then
    bad "atribuyo el fallo a PyYAML, que SI esta presente: diagnostico equivocado"
elif printf '%s' "$out" | grep -qi 'NO es valido'; then
    ok "detecto el YAML invalido, lo nombro como lo que es y salio $rc"
else
    bad "murio pero sin nombrar la causa real: $(printf '%s' "$out" | tail -3 | tr '\n' ' ')"
fi
if [ -f "$d/docs/roadmap/STATE.yaml.reconcile-candidate" ]; then
    ok "el temporal queda conservado para inspeccionarlo"
else
    bad "el temporal se perdio: sin el no se puede depurar la causa"
fi

echo
echo "-- el caso bueno, que sin el resto de este fichero no concluiria nada --"
d="$WORK/bueno"; arena "$d" si si
out="$( cd "$d" && bash scripts/reconcile_state_pointer.sh 2>&1 )"
rc=$?
if [ "$rc" -eq 0 ] && yaml_ok "$d/docs/roadmap/STATE.yaml" && ! [ -f "$d/docs/roadmap/STATE.yaml.reconcile-candidate" ]; then
    ok "un puntero que ya tiene superseded_pointer se reconcilia y sigue siendo YAML valido"
else
    bad "el caso bueno no pasa (rc=$rc): todo lo de arriba seria ruido sobre un script roto"
    printf '%s\n' "$out" | tail -8 | sed 's/^/         /'
fi

echo
echo "PASS=$PASS FAIL=$FAIL"
if [ "$FAIL" -eq 0 ]; then
    echo "RESULT: PASS — el unico que escribe el puntero no puede salir 0 sin haberlo escrito."
    exit 0
fi
echo "RESULT: FAIL — el puntero vuelve a poder mentir."
exit 1
