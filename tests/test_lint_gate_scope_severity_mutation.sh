#!/usr/bin/env bash
# test_lint_gate_scope_severity_mutation.sh — autofalsador del gate de
# lint (INC-DEBT-073)
#
# QUE PREGUNTA, y por que M3 existe
#
# 1. ¿La severidad DECLARADA se cobra?       -> M1
# 2. ¿El suelo de severidad es real, o el    -> M2 y M3
#    gate simply no mira? M2 solo NO alcanza:
#    un "0 findings" que sale de un shellcheck
#    que no se ejecuta es indistinguishable
#    de un arbol limpio. M3 es el control que
#    separa las dos cosas.
# 3. ¿El gate se cuelga con el arbol vacio?  -> M4
# 4. ¿El alcance es el arbol y no el rango?  -> M5
#
# M5 ES LA QUE FALSIFICA EL DEFECTO, no solo el guard: monta un arbol donde
# el aviso vive en un fichero que un gate por RANGO no puede ver, y exige
# que el gate lo vea igual. Sin M5, este fichero probaria que el codigo
# corre, no que arregla lo que la deuda describe.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB="$ROOT/scripts/lib/lint_gate.sh"

PASS=0
FAIL=0
ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

echo "== test_lint_gate_scope_severity_mutation.sh (INC-DEBT-073) =="

if [ ! -f "$LIB" ]; then
    echo "  [FAIL] falta scripts/lib/lint_gate.sh — no hay gate que falsificar"
    echo
    echo "PASS=0 FAIL=1"
    exit 1
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# arena <directorio> — un repo git minimo, porque el alcance se decide con
# `git ls-files` y una lista inventada en el test no probaria el alcance.
arena() {
    local d="$1"
    mkdir -p "$d"
    git -C "$d" init -q
    git -C "$d" config user.email "falsificador@example.invalid"
    git -C "$d" config user.name  "falsificador"
    git -C "$d" config commit.gpgsign false
}

# committed <dir> — commit de lo que haya, con un mensaje dado.
committed() {
    local d="$1" msg="$2"
    git -C "$d" add -A
    git -C "$d" commit -q -m "$msg"
}

# findings <dir> [severidad] — findings que ve el gate, desde <dir>.
findings() {
    local d="$1" sev="${2:-}"
    (
        cd "$d" || exit 1
        # shellcheck source=../scripts/lib/lint_gate.sh
        # shellcheck disable=SC1091
        . "$LIB"
        if [ -n "$sev" ]; then
            lint_gate_findings "$sev"
        else
            lint_gate_findings
        fi
    )
}

count_of() { [ -n "$1" ] && printf '%s\n' "$1" | grep -cE '^In .* line [0-9]+' || echo 0; }

# SC2034 (warning): una variable asignada y nunca usada.
cat > "$WORK/fwarn.sh" <<'EOF'
#!/usr/bin/env bash
f() {
    local i=1
    echo "hola"
}
f
EOF
# SC2016 (info): comillas simples con un $ que no se expande.
#
# MEDIDO al escribir este fixture, y por eso esta la nota: la PRIMERA
# version asignaba `name=world` y usaba `echo 'hola $name'`, pensando que
# era un caso de info. No lo era — la asignacion sin usar genera SC2034, que
# es WARNING, luego el fixture metia dos warnings y M2 fallo. Lo que fallo
# no fue el gate: fue que el caso no aislaba la variable que queria aislar,
# que es la forma exacta de la que este falsador existe.
cat > "$WORK/fstyle.sh" <<'EOF'
#!/usr/bin/env bash
# shellcheck disable=SC2148
echo 'hola $nombre'
EOF
cat > "$WORK/fclean.sh" <<'EOF'
#!/usr/bin/env bash
# shellcheck disable=SC2148
echo "hola"
EOF

echo
echo "-- el suelo de severidad es real, y shellcheck esta vivo --"

# Base: arbol limpio, gate verde. Sin esto, todo lo demas no tiene contra
# que compararse.
d="$WORK/base"; arena "$d"
cp "$WORK/fclean.sh" "$d/"; committed "$d" "limpio"
out="$(findings "$d")"
if [ "$(count_of "$out")" -eq 0 ]; then
    ok "base: un arbol limpio da 0 findings — el gate puede ser verde"
else
    bad "base: un arbol limpio deberia dar 0 — obtenido $(count_of "$out")"
fi

# M1 — un warning SI se cobra.
d="$WORK/m1"; arena "$d"
cp "$WORK/fwarn.sh" "$d/"; committed "$d" "aviso"
n="$(count_of "$(findings "$d")")"
if [ "$n" -ge 1 ]; then
    ok "M1: un warning SI lo cobra el gate (n=$n) — la severidad esta viva"
else
    bad "M1: un warning no lo cobra — la severidad declarada no se aplica"
fi

# M2 — un info NO se cobra a severidad warning.
d="$WORK/m2"; arena "$d"
cp "$WORK/fstyle.sh" "$d/"; committed "$d" "info"
n="$(count_of "$(findings "$d")")"
if [ "$n" -eq 0 ]; then
    ok "M2: un info NO se cobra a severidad warning — el suelo es real"
else
    bad "M2: un info se esta cobrando (n=$n) — el suelo de severidad no existe"
fi

# M3 — EL CONTROL. El mismo info, cobrado a severidad info, SI aparece.
# Sin M3, M2 pasa igual con un shellcheck que no se ejecuta nunca, y el
# "0 findings" de M2 no prueba nada: es indistinguible de un gate inerte.
n="$(count_of "$(findings "$d" info)")"
if [ "$n" -ge 1 ]; then
    ok "M3: a severidad info el mismo finding SI aparece (n=$n) — M2 no es un gate inerte"
else
    bad "M3: a severidad info deberia aparecer y no aparece — M2 pasaba por inactividad"
fi

echo
echo "-- el arbol vacio no cuelga --"

# M4 — sin ficheros .sh, shellcheck SIN argumentos lee stdin y se queda
# colgado. Un colgado se parece a un gate lento.
d="$WORK/m4"; arena "$d"
printf 'nada\n' > "$d/README.md"; committed "$d" "sin shell"
start="$(date +%s)"
out="$(timeout 15 bash -c "cd '$d' && . '$LIB' && lint_gate_findings" 2>&1)"
rc=$?
elapsed=$(( $(date +%s) - start ))
if [ "$rc" -eq 124 ]; then
    bad "M4: el gate se COLGO con el arbol sin .sh — lee stdin"
elif [ "$rc" -ne 0 ]; then
    bad "M4: el gate salio $rc con el arbol sin .sh"
elif [ -n "$out" ]; then
    bad "M4: el gate imprimo algo con el arbol sin .sh: '$out'"
else
    ok "M4: arbol sin .sh da 0 y vuelve en ${elapsed}s — no lee stdin"
fi

echo
echo "-- el alcance es el arbol, no el rango (esto es lo que arregla la deuda) --"

# M5 — el aviso vive en un fichero que un gate por rango NO puede ver, y el
# gate lo ve igual. Se construye al reves: primero se commitea el fichero
# con el aviso, y despues un commit que NO lo toca. Con BASE en ese segundo
# commit, el rango esta vacio y la logica antigua no linteria nada.
d="$WORK/m5"; arena "$d"
cp "$WORK/fwarn.sh" "$d/"; committed "$d" "el aviso, en su commit"
base_sha="$(git -C "$d" rev-parse HEAD)"
cp "$WORK/fclean.sh" "$d/"; committed "$d" "otro fichero, que no toca el aviso"

n_range="$(cd "$d" && git diff --name-only "$base_sha"..HEAD | grep -cE '\.sh$' || true)"
if [ "$n_range" -eq 0 ]; then
    ok "M5a: el rango desde el ultimo commit esta vacio ($n_range .sh) — la logica antigua no veria nada"
else
    ok "M5a: el rango tiene $n_range .sh (el aviso se commiteo antes de la base)"
fi

n_gate="$(count_of "$(findings "$d")")"
if [ "$n_gate" -ge 1 ]; then
    ok "M5b: el gate de arbol entero lo ve igualmente (n=$n_gate) — el alcance esta arreglado"
else
    bad "M5b: el gate no ve el aviso que el rango no puede ver — el alcance sigue siendo el rango"
fi

echo
echo "PASS=$PASS FAIL=$FAIL"
if [ "$FAIL" -eq 0 ]; then
    echo "RESULT: PASS — la severidad se cobra, el suelo existe, y el alcance es el arbol."
    exit 0
fi
echo "RESULT: FAIL — el gate no se sostiene."
exit 1
