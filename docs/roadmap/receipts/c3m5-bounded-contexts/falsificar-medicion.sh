#!/usr/bin/env bash
# Falsifica medir-contextos.py con mutaciones al SOURCE REAL.
#
# Un instrumento que solo se ejecuta una vez no esta verificado: sale verde
# porque el repo esta como esta. Aqui se rompe el repo de verdad, se comprueba
# que la cifra se mueve en la direccion enunciada ANTES de ejecutar, y se
# deja todo como estaba. `git checkout --` siempre, incluso al fallar.
#
# Que una mutacion NO se detecte no se disimula: se declara. Y una mutacion
# que no debe cambiar el numero tambien cuenta, porque una medida que se mueve
# con un comentario es una medida de prosa.
set -uo pipefail

RAIZ="$(git rev-parse --show-toplevel)"
cd "$RAIZ" || exit 2

MEDIR="docs/roadmap/receipts/c3m5-bounded-contexts/medir-contextos.py"
CLI="crates/sddk-cli/src/cycle.rs"
LIB="crates/sddk-engine/src/lib.rs"
TOCADOS=("$CLI" "$LIB")

snapshot() {
    python3 - "$MEDIR" <<'PY'
import importlib.util, sys
spec = importlib.util.spec_from_file_location("m", sys.argv[1])
m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
R = m.raices_de(m.SRC)
m.REEXPORTADOS = m.leer_reexportados(m.ENGINE / "src/lib.rs")
U = m.leer_grafo(R)
sin = sorted(n for n in R
             if not U.get(n, {"producto": set(), "test": set()})["producto"]
             and not U.get(n, {"producto": set(), "test": set()})["test"])
print("%d|%s" % (len(sin), ",".join(sin)))
PY
}

rev() { git checkout -- "${TOCADOS[@]}" 2>/dev/null; }
trap rev EXIT

BASE="$(snapshot)"
BASE_N="${BASE%%|*}"
BASE_LISTA="${BASE#*|}"
echo "baseline: ${BASE_N} modulos sin consumidor de nada"
echo

DET=0; SOBRE=0; NO_MED=0

# $1 nombre  $2 descripcion  $3 mutacion (codigo a anadir a $CLI)  $4 esperado python
caso() {
    local nombre="$1" desc="$2" mut="$3" cond="$4"
    echo "── ${nombre}: ${desc}"
    rev
    printf '\n%s\n' "$mut" >> "$CLI"
    local ahora; ahora="$(snapshot)"
    rev
    if python3 -c "
base_n = $BASE_N
n, lista = '''$ahora'''.split('|')
n = int(n); lista = set(lista.split(',')) if lista else set()
base_lista = set('''$BASE_LISTA'''.split(',')) if '''$BASE_LISTA''' else set()
ok = ($cond)
print(('  DETECTADA' if ok else '  SOBREVIVIDA'), '  ${BASE_N} ->', n)
raise SystemExit(0 if ok else 1)
"; then DET=$((DET+1)); else SOBRE=$((SOBRE+1)); fi
    echo
}

# 1. Consumo real por ruta `use`. Debe ENCOGER el conjunto en 1.
caso M1 "consumo real por ruta \`use\` en un modulo muerto" \
     '// MUTACION M1
use sddk_engine::structured_work::Submit; fn _m1() { let _ = Submit; }' \
     'n == base_n - 1 and ("structured_work" not in lista)'

# 2. Consumo por la FACHADA, que no lleva ruta. Debe ENCOGER el conjunto en 1.
#    La primera version de esta mutacion se limitaba a MENCIONAR `Engine`, y eso
#    no ejercita la via de la fachada en absoluto: daba "30 -> 30" igual que una
#    mutacion de ruido, y por eso era un test que no probaba lo que decia. Esta
#    llama a un metodo cuyo nombre ES el de un modulo hoy sin consumidor: si la
#    via `.<mod>(` no esta, ese modulo no sale del conjunto y el caso falla.
caso M2 "consumo por la fachada \`.<mod>()\`, sin ninguna ruta \`use\`" \
     '// MUTACION M2
fn _m2(e: &mut dyn AnyEngine) { e.lab_promotion(); }' \
     'n == base_n - 1 and ("lab_promotion" not in lista)'

# 3. Una CITA en doc-comment no es consumo. El numero NO debe moverse.
caso M3 "cita en doc-comment de un modulo muerto" \
     '// MUTACION M3veModules::paths en un comentario: citar no es consumir' \
     'n == base_n and lista == base_lista'

# 4. Una SUBCADENA no es el modulo. El numero NO debe moverse.
caso M4 "identificador que CONTIENE el nombre (\`is_up_to_date\`)" \
     '// MUTACION M4
fn is_up_to_date() -> bool { true }' \
     'n == base_n and lista == base_lista'

# 5. `pub use X::*` no prueba consumo: el asterisco no dice quien lo usa.
#    Anadir el reexport no debe ENCOGER el conjunto.
rev
printf '\npub use gate_signing::*;\n' >> "$LIB"
AHORA="$(snapshot)"
rev
if python3 -c "
base_n = $BASE_N
n = int('''$AHORA'''.split('|')[0])
ok = (n == base_n)
print(('  DETECTADA' if ok else '  SOBREVIVIDA'), '  ${BASE_N} ->', n, '  (::* no prueba consumo)')
raise SystemExit(0 if ok else 1)
"; then DET=$((DET+1)); else SOBRE=$((SOBRE+1)); fi

echo
echo "═══════════════════════════════════════════════════════"
echo "  DETECTADAS=$DET  SOBREVIVIDAS=$SOBRE  NO_MEDIBLES=$NO_MED"
echo "═══════════════════════════════════════════════════════"

rev
if [ "$(snapshot)" != "$BASE" ]; then
    echo "ATENCION: el repo no quedo como estaba. Revisa a mano antes de commitear."
    exit 1
fi
echo "source restaurado y verificado: la medicion vuelve al baseline"
[ "$SOBRE" -eq 0 ] || exit 1
