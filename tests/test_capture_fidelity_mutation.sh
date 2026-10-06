#!/usr/bin/env bash
# Autofalsacion de la fidelidad del capturador de aristas.
#
# Tres properties del gate se apoyan en que la medicion vea lo que existe, y las
# tres se pueden romper sin que ningun test de resultado caiga, porque un
# capturador que ve de menos sigue produciendo un veredicto con forma correcta:
#
#   A. un `use` dentro de `#[cfg(test)]` es de PRODUCCION
#   B. un `pub use` no es una arista
#   C. el reparto composition root / fuera de ARCH010 es el declarado
#
# Un `PASS` aqui significa "la propiedad fue rota y ALGO la detecta". Una
# mutacion que no aplica se cuenta SKIP, que no es PASS.
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO" || exit 1
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/var/home/rubentxu/cargo-targets/release/sddk}"

BASE="crates/sddk-engine/src/rules/baseline.rs"
EVAL="crates/sddk-engine/src/rules/evaluators.rs"
WORK="$(mktemp -d)"
cp "$BASE" "$WORK/base.rs.orig"
cp "$EVAL" "$WORK/eval.rs.orig"
BASE_SHA="$(sha256sum "$WORK/base.rs.orig" | cut -d' ' -f1)"
EVAL_SHA="$(sha256sum "$WORK/eval.rs.orig" | cut -d' ' -f1)"

PASS=0
FAIL=0
SKIP=0
declare -a LINES=()

dispose() {
    if command -v mavis-trash >/dev/null 2>&1; then
        mavis-trash -- "$1" >/dev/null 2>&1 || rm -rf "$1"
    else
        rm -rf "$1"
    fi
}
trap 'dispose "$WORK"' EXIT

restore() {
    cp "$WORK/base.rs.orig" "$BASE"
    cp "$WORK/eval.rs.orig" "$EVAL"
    local a b
    a="$(sha256sum "$BASE" | cut -d' ' -f1)"
    b="$(sha256sum "$EVAL" | cut -d' ' -f1)"
    if [ "$a" != "$BASE_SHA" ] || [ "$b" != "$EVAL_SHA" ]; then
        echo "RESTAURACION NO BYTE-IDENTICA"; exit 1
    fi
}

# Un test cae si su nombre aparece entre los FAILED de la suite.
#
# El TARGET importa y no es cosmetico: `cargo test --lib <filtro>` sobre un
# filtro que no existe en la libreria corre CERO tests y sale con exito 0, luego
# "el test sigue verde" era la lectura de una suite que nunca se ejecuto. N5 y
# N6 salieron "no detectadas" por eso, no porque el codigo resistiera.
unit_cae() {
    local target="$1" filter="$2" out
    # Sin comillas a proposito: `$target` son VARIOS argumentos de cargo
    # (`--test rules_evaluator` es dos, no uno). Citado, cargo responde
    # "unexpected argument '--test rules_evaluator' found", no compila nada y
    # sale con codigo 0 en el grep: N4, N5 y N6 salian "no detectadas" porque
    # el harness no estaba ejecutando el test que decia ejecutar.
    # shellcheck disable=SC2086
    out="$(cargo test -q -p sddk-engine $target "$filter" 2>&1)"
    # Un filtro que no existe NUNCA puede contar como "sigue verde".
    # La unica comprobacion fiable es que la suite llegara a correr. Un test que
    # falla imprime `error: test failed`, asi que buscar `^error` confundia un
    # fallo con "cargo no arranco" — que es como se declararon N4, N5 y N6 no
    # medidas cuando si lo estaban.
    if ! printf '%s\n' "$out" | grep -qE 'running [1-9][0-9]* test'; then
        LINES+=("   (la suite no llego a correr; primera linea: $(printf '%s\n' "$out" | grep -m1 . || echo '<vacia>'))")
        return 2
    fi
    printf '%s\n' "$out" | grep -qE '^test result: FAILED|panicked at' && return 0
    return 1
}

verdict_for() {
    local name="$1" filter="$2" applied="$3" target="${4:---lib}"
    if [ "$applied" != "yes" ]; then
        restore
        SKIP=$((SKIP + 1))
        LINES+=("SKIP | $name | la mutacion no aplico")
        return
    fi
    unit_cae "$target" "$filter"
    local rc=$?
    if [ $rc -eq 2 ]; then
        restore
        FAIL=$((FAIL + 1))
        LINES+=("FAIL | $name | el filtro no existe en $target: la mutacion no se ha medido")
    elif [ $rc -eq 0 ]; then
        PASS=$((PASS + 1))
        LINES+=("PASS | $name | cae como debe: $filter")
    else
        FAIL=$((FAIL + 1))
        LINES+=("FAIL | $name | $filter sigue VERDE con el capturador roto")
    fi
    restore
}

applied() { if [ "$1" -eq 0 ]; then echo yes; else echo no; fi; }

# ── N1: `pub use` deja de ser arista ────────────────────────────────────────
python3 - "$BASE" <<'PY'
import sys
p=sys.argv[1]; s=open(p).read()
old='''        let rest = if let Some(r) = trimmed.strip_prefix("pub use ") {
            r
        } else if let Some(r) = trimmed.strip_prefix("use ") {'''
new='''        let rest = if let Some(r) = trimmed.strip_prefix("use ") {'''
assert s.count(old)==1, "N1 no aplica"
open(p,'w').write(s.replace(old,new))
PY
verdict_for "N1 un pub use deja de contar como arista" "a_pub_use_is_an_edge" "$(applied $?)"

# ── N2: nunca se marca el ambito de test ───────────────────────────────────
# Rompe la PROPIEDAD, no una linea: el `kind` deja de poder marcar test. La
# version anterior de esta mutacion (neutralizar el cierre del ambito) era un
# no-op — la logica de entrada al bloque repone `test_depth` en la misma
# iteracion, justo antes del `kind` — y salia "no detectada" sin detectar nada.
python3 - "$BASE" <<'PY'
import sys
p=sys.argv[1]; s=open(p).read()
old='''                kind: if test_depth.is_some() {
                    CrossCrateImportKind::TestScopedUse
                } else {
                    CrossCrateImportKind::Use
                },'''
new='''                kind: CrossCrateImportKind::Use,'''
assert s.count(old)==1, "N2 no aplica"
open(p,'w').write(s.replace(old,new))
PY
verdict_for "N2 ninguna arista se marca como de test" "a_use_in_a_test_module_is_not_a_production_edge" "$(applied $?)"

# ── N3: la arista posterior al bloque de test se marca como de test ─────────
# Esta es la version EXACTA del bug que los tests encontraron al construirse:
# registrar la profundidad en el atributo hacia que la linea `mod tests {`
# pareciera el cierre del bloque, y todo lo de abajo quedara como test.
python3 - "$BASE" <<'PY'
import sys
p=sys.argv[1]; s=open(p).read()
old='''        if pending_at.is_some_and(|at| depth_before > at) {
            test_depth = Some(depth_before);
            pending_at = None;
        }'''
new='''        if pending_at.is_some() {
            test_depth = Some(depth_before);
            pending_at = None;
        }'''
assert s.count(old)==1, "N3 no aplica"
open(p,'w').write(s.replace(old,new))
PY
verdict_for "N3 la arista tras el bloque de test se cuenta como de test" "a_use_after_a_test_module_closes_is_production_again" "$(applied $?)"

# ── N4: los evaluadores dejan de separar produccion de test ────────────────
# El helper y los bespoke. Se quita el filtro de produccion de ARCH001, que es
# el evaluador con cuerpo propio: sin esto, seis leyes separan test y
# produccion y tres no, y la misma arista cuenta distinto segun la forma del
# evaluador.
python3 - "$EVAL" <<'PY'
import sys
p=sys.argv[1]; s=open(p).read()
old='''fn is_production_edge(e: &CrossCrateImport) -> bool {
    e.kind != CrossCrateImportKind::TestScopedUse
}'''
new='''fn is_production_edge(_e: &CrossCrateImport) -> bool {
    true
}'''
assert s.count(old)==1, "N4 no aplica"
open(p,'w').write(s.replace(old,new))
PY
verdict_for "N4 el filtro de produccion desaparece" "arch010_split" "$(applied $?)" "--test rules_evaluator"

# ── N5: ARCH010 deja de separar el composition root ─────────────────────────
python3 - "$EVAL" <<'PY'
import sys, re
p=sys.argv[1]; s=open(p).read()
m = re.search(r'eval\.observed\["(composition_root_\w+)"\] = json!\(at_composition_root\);', s)
assert m, "N5 no aplica: no se encuentra la clave del composition root"
key = m.group(1)
old = m.group(0)
new = 'let at_composition_root = 0;\n    ' + old
open(p,'w').write(s.replace(old, new))
PY
verdict_for "N5 ARCH010 reporta 0 en el composition root" "arch010_split" "$(applied $?)" "--test rules_evaluator"

# ── N6: subject_crates vuelve a ser el recuento de declaraciones ────────────
# La cifra nueva, sin su distincion, seria un numero que no significa nada
# nuevo: 4 declaraciones de 2 crates leidas como "2" esconden que se repite.
python3 - "$EVAL" <<'PY2'
import sys
p=sys.argv[1]; s=open(p).read()
old='''    v.sort();
    v.dedup();
    v
}'''
new='''    v
}'''
assert s.count(old)==1, "N6 no aplica"
open(p,'w').write(s.replace(old,new))
PY2
verdict_for "N6 subject_crates no deduplica" "a_rule_reports_declarations_and_distinct_targets_separately" "$(applied $?)" "--test rules_evaluator"

# ── CONTROLES ──────────────────────────────────────────────────────────────
echo "== controles (codigo intacto) =="
for f in a_pub_use_is_an_edge \
         a_use_in_a_test_module_is_not_a_production_edge \
         a_use_after_a_test_module_closes_is_production_again \
         a_stray_brace_cannot_hide_a_production_edge
do
    if cargo test -q -p sddk-engine --lib "$f" >/dev/null 2>&1; then
        PASS=$((PASS+1)); LINES+=("PASS | control | $f sigue verde, el guard distingue")
    else
        FAIL=$((FAIL+1)); LINES+=("FAIL | control | $f cae sin mutacion: no discrimina")
    fi
done

echo
for l in "${LINES[@]}"; do echo "$l"; done
echo
echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
[ $FAIL -eq 0 ] && echo "RESULT: PASS" || echo "RESULT: FAIL"
exit $(( FAIL > 0 ))