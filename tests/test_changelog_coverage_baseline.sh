#!/usr/bin/env bash
# La linea base de "que version esta publicada" tiene que tener UNA respuesta
# (INC-DEBT-070).
#
# EL DEFECTO
# ----------
# `tests/test_changelog_coverage.sh` sacaba su linea base de
# `git -C "$ROOT" tag`, que es el clon LOCAL. `gh release create` publica en el
# remoto y nada del pipeline actualiza el clon, luego la lista local se queda
# vieja version tras version. MEDIDO al cerrar v2.8.0: clon local en v2.7.0,
# remoto en v2.8.0, luego el gate habria comparado `v2.7.0..HEAD` con **4 commits
# feat/fix/test ya publicados en 2.8.0**, y la forma facil de ponerlo verde era
# duplicarlos en la seccion siguiente — un changelog describiendo trabajo ya
# salido, en un artefacto que se publica.
#
# POR QUE ESTE TEST USA FIXTURES AISLADOS Y NO EL REPO
# ---------------------------------------------------
# En el repo real el clon y el remoto coinciden siempre en el momento de correr
# el gate, luego alli el defecto **no se puede ver**: pasaria y fallaria igual.
# La propiedad necesita un clon deliberadamente viejo, y un clon viejo se
# fabrica: se publica un tag en el remoto y se borra el local.
#
# La fixture se construye para que las DOS respuestas **digan cosas distintas**,
# que es lo unico que hace el caso falsable. Sin esa divergencia, un gate roto
# y uno arreglado darian el mismo veredicto y el test no mediria nada.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GATE="$ROOT/tests/test_changelog_coverage.sh"
ADMISSION="$ROOT/scripts/lib/release_admission.sh"

PASS=0
FAIL=0
ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

export GIT_CONFIG_GLOBAL="$TMP/gitconfig"
export GIT_CONFIG_SYSTEM=/dev/null
cat > "$GIT_CONFIG_GLOBAL" <<'CFG'
[user]
    name = fixture
    email = fixture@example.test
[init]
    defaultBranch = main
[commit]
    gpgsign = false
[tag]
    gpgsign = false
CFG
export GIT_TERMINAL_PROMPT=0
# El `git-wrapper` de esta maquina se niega a firmar commits en un repo sin
# identidad clasificada, y sin identidad no hay fixture: `git commit` falla, el
# gate lee un repo vacio y C0 "pasa" por el motivo equivocado. Un fixture
# roto que produce un verde es PEOR que no tener fixture, porque ocupa el
# lugar del que si mediria. Se usa la excepcion puntual que el propio wrapper
# documenta, y ademas identidad a nivel de repo.
export RANDOM_GIT_COMMITTER_DISABLED=1

# ── helpers de fixture ──────────────────────────────────────────────────────

# Escribe un fixture con la version de workspace pedida y una seccion de
# changelog que contiene EXACTAMENTE esas lineas de commit.
make_fixture() {  # make_fixture <dir> <version> <linea1> [linea2 ...]
    local dir="$1" version="$2"; shift 2
    local seccion=""
    local i=0
    for linea in "$@"; do
        i=$((i + 1))
        seccion+="  - ${linea}"$'\n'
    done
    mkdir -p "$dir/tests" "$dir/scripts/lib"
    cp "$GATE" "$dir/tests/test_changelog_coverage.sh"
    cp "$ADMISSION" "$dir/scripts/lib/release_admission.sh"
    printf '[workspace.package]\nversion = "%s"\n' "$version" > "$dir/Cargo.toml"
    {
        printf '# Changelog\n\n## [%s] - 2026-10-04\n\n### Other\n' "$version"
        printf '%s' "$seccion"
        printf '\n## [0.0.1] - 2026-01-01\n\n### Other\n  - feat(x): historia vieja\n'
    } > "$dir/CHANGELOG.md"
}

# Crea el repo git y su remoto bare. $1=dir  $2=url del remoto
init_repo() {  # init_repo <dir> <remote-url>
    local dir="$1" url="$2"
    git -C "$dir" config user.email "fixture@example.test"
    git -C "$dir" config user.name "fixture"
    git -C "$dir" init -q -b main
    git -C "$dir" add -A
    git -C "$dir" commit -qm "chore: fixture inicial"
    git -C "$dir" remote add origin "$url"
    git -C "$dir" push -q origin main
}

run_gate() {  # run_gate <dir> -> imprime la salida, devuelve el exit del gate
    ( cd "$1" && bash tests/test_changelog_coverage.sh 2>&1 )
}

# Un fixture tiene que ser lo que dice ser ANTES de medir nada sobre el. Sin
# esto, un fixture roto produce veredictos sin base y el test los cuenta.
assert_fixture() {  # assert_fixture <dir> <n-commits-HEAD> <etiqueta>
    local dir="$1" commits="$2" etiqueta="$3" n
    if ! git -C "$dir" rev-parse -q --verify HEAD >/dev/null 2>&1; then
        bad "$etiqueta: el fixture no tiene HEAD; el gate leeria un repo vacio y todo lo de abajo seria ruido"
        return 1
    fi
    n="$(git -C "$dir" rev-list --count HEAD 2>/dev/null || echo 0)"
    if [[ "$n" -lt "$commits" ]]; then
        bad "$etiqueta: el fixture tiene $n commits y deberia tener al menos $commits; los commits no se crearon"
        return 1
    fi
    return 0
}

echo "== La linea base de la version publicada tiene una sola autoridad =="
echo

# ── C1: clon viejo, remoto al dia. LA DIFERENCIA QUE HACE FALSABLE ─────────
# Se construye para que las dos respuestas NO coincidan:
#   - remota v1.0.0 -> el rango tiene 1 commit (feat posterior) y la seccion lo
#     cubre, luego PASS.
#   - local  v0.9.0 -> el rango tiene 3 commits, dos de ellos ya publicados en
#     v1.0.0 y ausentes de la seccion, luego FAIL.
# Un gate que lee el clon local falla aqui; uno que lee el remoto pasa. Y el
# fallo va en la direccion peligrosa: el defecto hace pedir trabajo ya
# publicado, que es como un changelog acaba describiendo una release que no es
# la suya.
echo "-- C1: el remoto va por delante del clon y el gate debe seguir al remoto"
# Tres commits tras v0.9.0; v1.0.0 corta el segundo, luego el rango remoto tiene
# UN commit y el rango local tiene TRES. La seccion cubre solo el primero.
F1="$TMP/f1"
git init -q --bare "$TMP/remote1.git"
# OJO: la seccion lleva SOLO la linea del commit posterior a v1.0.0. Si llevara
# las tres, el rango contra v0.9.0 (tres commits) tambien estaria cubierto y
# las DOS respuestas darian PASS — un caso que no discrimina no mide, por mucho
# que el gate este roto. Este fixture lo enseño el falsador, no la lectura.
make_fixture "$F1" "2.0.0" \
    "fix(engine): el arreglo que ya salio en 1.0.0"
init_repo "$F1" "$TMP/remote1.git"
# v0.9.0: el unico tag que el clon conoce al empezar.
git -C "$F1" tag v0.9.0
git -C "$F1" push -q origin v0.9.0
# Se publica v1.0.0 en el remoto y SE BORRA el local, que es el estado real
# medido al cerrar 2.8.0.
git -C "$F1" commit -q --allow-empty -m "feat(cli): el commit que el gate debe pedir"
git -C "$F1" commit -q --allow-empty -m "test(cli): el test que ya salio en 1.0.0"
git -C "$F1" commit -q --allow-empty -m "fix(engine): el arreglo que ya salio en 1.0.0"
git -C "$F1" tag v1.0.0 HEAD~1
git -C "$F1" push -q origin v1.0.0
git -C "$F1" tag -d v1.0.0 >/dev/null
# El clon queda con v0.9.0 solamente.
LOCAL_MAX="$(git -C "$F1" tag --sort=-version:refname | head -1)"
REMOTE_MAX="$(git -C "$F1" ls-remote --tags origin | sed 's#.*refs/tags/##; s/\^{}//' | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -1)"
if [[ "$LOCAL_MAX" == "$REMOTE_MAX" ]]; then
    bad "C1 no se puede medir: el clon ($LOCAL_MAX) y el remoto ($REMOTE_MAX) coinciden"
else
    ok "fixture C1 valida: clon en $LOCAL_MAX, remoto en $REMOTE_MAX (las dos respuestas difieren)"
fi

if assert_fixture "$F1" 4 "C1"; then
    ok "C1: el fixture tiene los 4 commits que el caso afirma"
fi
OUT1="$(run_gate "$F1")"
if grep -qE '^RESULT: PASS' <<<"$OUT1"; then
    ok "C1: con el clon viejo, el gate PASA — leyo el remoto y pide solo el commit no publicado"
else
    bad "C1: con el clon viejo el gate falla; la respuesta local le pidio trabajo ya publicado"
    sed -n '1,14p' <<<"$OUT1" | sed 's/^/        /'
fi
if grep -qE "published-version authority: remote \(origin\)" <<<"$OUT1"; then
    ok "C1: el gate DECLARA de donde salio la linea base"
else
    bad "C1: el gate no dice que autoridad respondio; un lector no puede distinguir un bootstrap de un clon viejo"
fi
if grep -qE "comparing against last published tag: v1\.0\.0" <<<"$OUT1"; then
    ok "C1: y compara contra el tag REMOTO (v1.0.0), no contra el local"
else
    bad "C1: el gate comparo contra un tag que no es el publicado"
fi

# ── C2: el remoto responde y no hay ningun tag. Bootstrap legitimo ─────────
# Distinguir "no hay nada publicado" de "no lo he mirado" es la razon de los
# tres resultados. Aqui el remoto responde de verdad, luego el skip con PASS es
# correcto; y lo que NO puede hacer es inventar un rango contra un tag local.
echo "-- C2: el remoto responde y no hay ningun tag (bootstrap)"
F2="$TMP/f2"
git init -q --bare "$TMP/remote2.git"
make_fixture "$F2" "1.0.0" "feat(cli): lo primero que se publica"
init_repo "$F2" "$TMP/remote2.git"
# Un tag local que el remoto NO ve: si el gate leyera el clon compararia
# contra el, y en un bootstrap asi no hay nada que comparar.
git -C "$F2" tag v0.0.1-only-local
if assert_fixture "$F2" 1 "C2"; then
    ok "C2: el fixture tiene el commit que el caso afirma"
fi
OUT2="$(run_gate "$F2")"; RC2=$?
if [[ "$RC2" -eq 0 ]] && grep -qE 'RESULT: PASS' <<<"$OUT2"; then
    ok "C2: bootstrap sale 0 y no inventa un rango contra un tag que el remoto no ve"
else
    bad "C2: un bootstrap limpio debe salir 0; el gate hizo otra cosa (exit $RC2)"
    sed -n '1,12p' <<<"$OUT2" | sed 's/^/        /'
fi
if grep -qE 'no v\* tags yet' <<<"$OUT2"; then
    ok "C2: y nombra el bootstrap como lo que es, en vez de callarse"
else
    bad "C2: el gate no distingue 'bootstrap' de 'no miré'"
fi

# ── C3: hay remoto configurado y NO RESPONDE. Fallo cerrado, con nombre ─────
# El caso que mas cuesta: el clon local tiene tags de sobra, luego degradar a
# la lista local "funciona" y produce un veredicto. Eso es exactamente lo que no
# se puede hacer, porque un veredicto derivado de una lista que puede estar vieja
# es un veredicto que no se sabe de donde sale.
echo "-- C3: el remoto esta configurado y no responde — fallo cerrado"
F3="$TMP/f3"
make_fixture "$F3" "2.0.0" "feat(cli): lo unico"
git -C "$F3" init -q -b main
git -C "$F3" config user.email "fixture@example.test"
git -C "$F3" config user.name "fixture"
git -C "$F3" add -A
git -C "$F3" commit -qm "chore: fixture inicial"
# Remoto configurado pero inalcanzable: la lista local esta llena y es tentadora.
git -C "$F3" remote add origin "$TMP/no-existe-este-remoto.git"
git -C "$F3" tag v0.9.0
if assert_fixture "$F3" 1 "C3"; then
    ok "C3: el fixture tiene el commit que el caso afirma"
fi
OUT3="$(run_gate "$F3")"; RC3=$?
if [[ "$RC3" -ne 0 ]]; then
    ok "C3: el gate sale distinto de cero cuando el remoto no responde"
else
    bad "C3: el gate salio 0 con el remoto sin responder; ha degradado a la lista local"
    sed -n '1,12p' <<<"$OUT3" | sed 's/^/        /'
fi
if grep -qE '^RESULT: FAIL' <<<"$OUT3"; then
    ok "C3: y declara FAIL en vez de un PASS sin base"
else
    bad "C3: el gate no declara FAIL; un resultado sin base no puede ser PASS"
fi
if grep -qiE "refusing to compare|remote did not answer|stale" <<<"$OUT3"; then
    ok "C3: y NOMBRA la causa (lista local potencialmente vieja)"
else
    bad "C3: el fallo no dice por que; un fallo mudo obliga a re-diagnosticar desde cero"
fi
if grep -qE "comparing against last published tag" <<<"$OUT3"; then
    bad "C3: el gate llego a comparar contra un tag — no debio llegar a comparar nada"
else
    ok "C3: y no llego a comparar contra ningun tag, que es lo unico correcto aqui"
fi

# ── C0: control de no-vacuidad. Sin esto, un gate que solo sabe decir FAIL ───
# pasaria los tres casos. El control exige que la fixture con todo en orden
# produzca un PASS, luego el test puede distinguir "certeza" de "pesimismo".
echo "-- C0 (control): con el clon al dia el gate PASA — el fixture sabe ser verde"
F0="$TMP/f0"
git init -q --bare "$TMP/remote0.git"
make_fixture "$F0" "2.0.0" "feat(cli): lo unico por publicar"
init_repo "$F0" "$TMP/remote0.git"
git -C "$F0" tag v1.0.0
git -C "$F0" push -q origin v1.0.0
git -C "$F0" commit -q --allow-empty -m "feat(cli): lo unico por publicar"
if assert_fixture "$F0" 2 "C0"; then
    ok "C0: el fixture tiene los 2 commits que el caso afirma"
fi
OUT0="$(run_gate "$F0")"; RC0=$?
if [[ "$RC0" -eq 0 ]] && grep -qE '^RESULT: PASS' <<<"$OUT0"; then
    ok "C0: el gate pasa cuando la linea base local y remota coinciden (el fixture no esta roto)"
else
    bad "C0: el fixture en verde falla; los tres casos anteriores no distinguirian nada (exit $RC0)"
    sed -n '1,12p' <<<"$OUT0" | sed 's/^/        /'
fi

echo
echo "PASS=$PASS FAIL=$FAIL"
[[ "$FAIL" -eq 0 ]]
