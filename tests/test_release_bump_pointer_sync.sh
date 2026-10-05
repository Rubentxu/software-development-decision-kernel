#!/usr/bin/env bash
# test_release_bump_pointer_sync.sh
#
# QUE VIGILA
# Que `release-bump.sh` reconcilie el puntero de estado por su cuenta, de modo
# que el orden correcto exista POR CONSTRUCCION y no dependa de que alguien
# recuerde bumpear despues de reconciliar.
#
# POR QUE ESTE GUARD EXISTE, en una frase
# Porque el orden invertido ya ha matado una release por el motivo equivocado.
# MEDIDO en session-83 publicando 2.11.1: se reconcilio con `behind`=3, que
# esta DENTRO de la tolerancia, luego el reconciliador dijo "se conserva" y no
# movio el puntero; el commit del bump lo llevo a 4 y `tests/
# test_release_state_pointer.sh` (1b) mato la release a los cuatro minutos con
# un fallo que no hablaba del codigo que se iba a publicar. Y ya habia pasado
# en session-18, -22 y -23, que es lo que motivo que el reconciliador existiera.
#
# QUE NO HACE
# No reimplementa el puntero ni el bump: usa el producto de verdad
# (`scripts/release-bump.sh` y `scripts/reconcile_state_pointer.sh`) sobre un
# repo miniatura, igual que los demas guards de este repo. El unico fichero
# que este test ESCRIBE en el sandbox es el STATE.yaml del sandbox.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUMP="$ROOT/scripts/release-bump.sh"
FIXER="$ROOT/scripts/reconcile_state_pointer.sh"
TOLERANCE=3

PASS=0
FAIL=0
ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

echo "== test_release_bump_pointer_sync.sh =="

for f in "$BUMP" "$FIXER"; do
    if [ ! -f "$f" ]; then
        echo "  [FAIL] falta $f"
        echo
        echo "PASS=0 FAIL=1"
        exit 1
    fi
done

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# arena <dir> <con_state: si|no> <con_fixer: si|no>
#
# Repo miniatura con lo que release-bump.sh necesita de verdad. Los conmutador
# existen para poder afirmar el caso NEGATIVO (sin reconciliador el bump tiene
# que morir diciendo por que) y el caso de los sandboxes existentes (sin
# STATE.yaml el bump no debe intentar reconciliar nada).
arena() {
    local d="$1" con_state="$2" con_fixer="$3"
    mkdir -p "$d/scripts" "$d/crates/fake/src" "$d/docs/roadmap"
    cp "$BUMP" "$d/scripts/release-bump.sh"
    [ "$con_fixer" = "si" ] && cp "$FIXER" "$d/scripts/reconcile_state_pointer.sh"
    # El bump ya exige la libreria del merge del changelog desde 4b839836. Una
    # arena que no la traiga muere por ESE motivo y el fallo se lee como un
    # defecto del bump en vez de como una arena incompleta. MEDIDO al escribir
    # esta arena: sin esta linea los tres casos fallaban con "falta la
    # libreria del merge del changelog" y ninguno llegaba a probar el puntero.
    mkdir -p "$d/scripts/lib"
    cp "$ROOT/scripts/lib/changelog_merge.sh" "$d/scripts/lib/changelog_merge.sh"
    cat > "$d/Cargo.toml" <<'EOF'
[workspace]
members = ["crates/fake"]
[workspace.package]
version = "9.9.9"
EOF
    cat > "$d/crates/fake/Cargo.toml" <<'EOF'
[package]
name = "fake"
version = "9.9.9"
edition = "2021"
EOF
    # Un target de verdad: el bump regenera Cargo.lock con `cargo check`, y sin
    # un target el workspace no compila y el bump muere con rc=101. MEDIDO al
    # escribir la arena; sin esto el fallo se lee como un defecto del bump.
    printf 'pub fn nada() {}\n' > "$d/crates/fake/src/lib.rs"
    printf 'id = "sddk-framework"\nversion = "9.9.9"\n' > "$d/manifest.toml"
    cat > "$d/BUNDLE.toml" <<'EOF'
schema_version = 2
[bundle]
version = "9.9.9"
binary_min_version = "9.9.9"
binary_max_version = "9.9.9"
EOF
    printf '# Changelog\n\n## [9.9.9]\n\n### Other\n  - algo\n' > "$d/CHANGELOG.md"
    if [ "$con_state" = "si" ]; then
        cat > "$d/docs/roadmap/STATE.yaml" <<'EOF'
schema_version: 1
source:
  baseline_branch: main
  baseline_sha: "0000000"
  current_sha: "0000000"
  head_at_state_sync: "0000000"
  workspace_version_at_current: "9.9.9"
EOF
    fi
    git -C "$d" init -q
    git -C "$d" config user.email "guard@example.invalid"
    git -C "$d" config user.name  "guard"
    git -C "$d" config commit.gpgsign false
    git -C "$d" add -A
    git -C "$d" commit -q -m "base"
    # Un tag publicado, porque `release-bump.sh` exige una linea base ANTES de
    # honorar `--force-version` (scripts/release-bump.sh:133 lo rechaza, y el
    # `--force-version` se lee en la 208). MEDIDO al escribir esta arena: sin
    # el tag el bump muere con "no published semver tag found" y uno podria
    # leer ese fallo como un defecto del bump y no como una arena incompleta.
    git -C "$d" tag v9.9.9
    # Un par de commits mas, para que el puntero quede por detras de la
    # tolerancia y el reconciliador tenga trabajo REAL que hacer.
    for n in 1 2 3 4 5; do
        printf 'linea %s\n' "$n" > "$d/notas.txt"
        git -C "$d" add -A
        git -C "$d" commit -q -m "trabajo $n"
    done
    # Y el puntero se queda en el commit base, que ya va 5 por detras.
    if [ "$con_state" = "si" ]; then
        local base_sha
        base_sha="$(git -C "$d" rev-parse --short HEAD~5)"
        sed -i "s/current_sha: \"0000000\"/current_sha: \"$base_sha\"/" \
            "$d/docs/roadmap/STATE.yaml"
        sed -i "s/head_at_state_sync: \"0000000\"/head_at_state_sync: \"$base_sha\"/" \
            "$d/docs/roadmap/STATE.yaml"
        git -C "$d" add -A
        git -C "$d" commit -q -m "el puntero se queda atras a proposito"
    fi
}

pointer_sha() { sed -n 's/^  current_sha: "\([^"]*\)".*/\1/p' "$1/docs/roadmap/STATE.yaml" | head -1; }
behind() { git -C "$1" rev-list --count "$2..HEAD" 2>/dev/null; }

echo
echo "-- el bump se reconcilia a si mismo --"

d="$WORK/selfheal"; arena "$d" si si
pre_head="$(git -C "$d" rev-parse --short HEAD)"
before="$(pointer_sha "$d")"
out="$( cd "$d" && bash scripts/release-bump.sh --force-version 9.9.10 2>&1 )"
rc=$?
after="$(pointer_sha "$d")"

if [ "$rc" -ne 0 ]; then
    bad "el bump fallo (rc=$rc) con el reconciliador presente"
    printf '%s\n' "$out" | tail -12 | sed 's/^/         /'
elif [ "$before" = "$after" ]; then
    bad "el bump no movio el puntero (sigue en $after) — el orden sigue dependiendo de la memoria"
elif [ "$after" = "$pre_head" ]; then
    ok "el bump movio el puntero al HEAD previo ($before -> $after)"
else
    bad "el puntero quedo en $after y se esperaba $pre_head"
fi

# El commit del bump es lo que el operador hace despues. Ese commit es el UNICO
# que puede quedar por detras, porque el puntero no puede contenerse a si mismo.
git -C "$d" add -A
git -C "$d" commit -q -m "chore(release): bump version"
b_post="$(behind "$d" "$after")"
if [ "$b_post" -le 1 ]; then
    ok "tras el commit del bump el puntero queda a $b_post commit(s) — el minimo fisico"
else
    bad "tras el commit del bump el puntero queda a $b_post commit(s), sobre tolerancia $TOLERANCE"
fi

# Las DOS condiciones mas que el guard exige, comprobadas sobre el sandbox.
# No se ejecuta `tests/test_release_state_pointer.sh` aqui porque ese guard se
# mide sobre el REPO real: ejecutarlo en un sandbox no concluiria nada sobre
# el sandbox, y una asercion que no concluye es peor que no tenerla.
if git -C "$d" merge-base --is-ancestor "$after" HEAD 2>/dev/null; then
    ok "el puntero que deja el bump es ancestro de HEAD (no quedo en una rama lateral)"
else
    bad "el puntero $after no es ancestro de HEAD — el bump lo pondria en una rama lateral"
fi
b_tol="$(behind "$d" "$after")"
if [ "$b_tol" -le "$TOLERANCE" ]; then
    ok "el retraso del puntero ($b_tol) esta dentro de la tolerancia $TOLERANCE del guard real"
else
    bad "el retraso del puntero ($b_tol) supera la tolerancia $TOLERANCE — el 1b mataria la release"
fi

echo
echo "-- sin el reconciliador, el bump muere DICIENDO POR QUE --"

# Este es el caso que la deuda de 4b83986 pago por el otro extremo: sin esta
# comprobacion, un `bash` sobre un fichero ausente bajo `set -e` muere con
# "linea N", que no es un diagnostico. Y ese fallo ya mato la 2.11.0 en su
# primer intento, cuando el sandbox de un test sourceo una libreria que su
# copia no traia.
d="$WORK/sinfixer"; arena "$d" si no
out="$( cd "$d" && bash scripts/release-bump.sh --force-version 9.9.10 2>&1 )"
rc=$?
if [ "$rc" -eq 0 ]; then
    bad "el bump salio 0 sin reconciliador — dejaria el puntero a la deriva sin avisar"
else
    if printf '%s' "$out" | grep -q 'falta el reconciliador del puntero'; then
        ok "el bump muere nombrando la causa, no con un numero de linea"
    else
        bad "el bump murio pero sin decir la causa: $(printf '%s' "$out" | tail -3 | tr '\n' ' ')"
    fi
    if printf '%s' "$out" | grep -q 'scripts/reconcile_state_pointer.sh'; then
        ok "el mensaje nombra la RUTA del fichero que falta"
    else
        bad "el mensaje no nombra la ruta del fichero que falta"
    fi
fi

echo
echo "-- sin STATE.yaml, el bump NO intenta reconciliar nada --"

# Es el caso de los sandboxes que ya existen en tests/test_release_bump_*: no
# tienen puntero, luego no tienen nada que reconciliar y no deben exigir la
# dependencia. Un guard que rompe esos casos seria un guard que obliga a
# actualizar tests que no estan probando esto.
d="$WORK/sinstate"; arena "$d" no no
out="$( cd "$d" && bash scripts/release-bump.sh --force-version 9.9.10 2>&1 )"
rc=$?
if [ "$rc" -eq 0 ]; then
    ok "sin STATE.yaml el bump sale 0 y no exige el reconciliador"
else
    bad "sin STATE.yaml el bump fallo (rc=$rc) — no deberia intentar reconciliar"
    printf '%s\n' "$out" | tail -8 | sed 's/^/         /'
fi

echo
echo "PASS=$PASS FAIL=$FAIL"
if [ "$FAIL" -eq 0 ]; then
    echo "RESULT: PASS — el orden correcto existe por construccion."
    exit 0
fi
echo "RESULT: FAIL — el orden sigue dependiendo de la memoria."
exit 1
