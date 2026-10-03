#!/usr/bin/env bash
# Autofalsacion de tests/test_cycle_list_total_reconciliation.sh
#
# QUE HACE Y POR QUE ESTA SEPARADO
# --------------------------------
# El guard ya trae una autoprueba que exige que RECHACE seis modos de mentira.
# Eso demuestra que el guard distingue el caso bueno de los malos. NO demuestra
# que lo haga por los motivos que dice: un guard puede rejecting por un efecto
# colateral -- porque al declarar 187 en vez de 108 tambien se rompe el desglose,
# o porque la fila que se borro tambien era la unica ilegible -- y seguir
# pareciendo que vigila la propiedad.
#
# Este fichero exige lo contrario: que cada comprobacion del guard sea
# LOAD-BEARING una por una. Borra una comprobacion del guard en una COPIA de
# sandbox y exige que la mutacion correspondiente pase a ser ACEPTADA. Si al
# borrar una comprobacion la mutacion sigue rechazandose, esa comprobacion no
# hacia falta y es codigo muerto: se declara, no se deja pasar.
#
# Y el caso G0 va al reves, y es el que mas importa: si el guard acepta SIEMPRE,
# las seis mutaciones tienen que aceptarse. Un guard que no tiene dientes
# pasaria la autoprueba interna -- porque "rechazar" es lo que se le pide -- y
# solo este caso lo detecta.
#
# SANDBOX ONLY. Copias en temporal. No toca el guard real ni el repo.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GUARD="$SCRIPT_DIR/test_cycle_list_total_reconciliation.sh"
PASS=0
FAIL=0

ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

indent() {
  local line
  while IFS= read -r line; do printf '        %s\n' "$line"; done
}

if [[ ! -f "$GUARD" ]]; then
  echo "FAIL: $GUARD no existe"
  exit 1
fi

TMPROOT="$(mktemp -d)"
trap 'rm -rf "$TMPROOT"' EXIT
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Toda copia se ejecuta con ROOT apuntando al repo real. Sin esto la copia
# calcularia ROOT desde su propia ruta en `/tmp` -- o sea `/` -- y sus fixtures
# se caerian al vacio: el falsificador veria su propia rotura y declararia que
# el guard sigue rechazando. Fue exactamente lo que paso en la primera pasada.
run_copy() {
  local copy="$1"
  chmod +x "$copy"
  SDDK_GUARD_ROOT="$REPO_ROOT" "$copy" 2>&1
}

# ── G0: el control. Copia integra, y ademas el guard que acepta siempre ─────
echo "=== autofalsacion de la reconciliacion de cycle list ==="

# El guard real tiene que estar en verde antes de exigirle nada: si el producto
# esta roto, todo lo de abajo no distingue una teeth de otra.
if "$GUARD" >"$TMPROOT/base.out" 2>&1; then
  ok "el guard sin mutar esta en verde (control previo)"
else
  bad "el guard sin mutar ya esta en rojo; la autofalsacion no distinguiria nada"
  sed 's/^/        /' "$TMPROOT/base.out" | tail -12
  exit 1
fi

# `force_accept` replaces el `sys.exit(1)` de la autoprueba por un `pass`, con lo
# que el guard acepta cualquier declaracion. Es el guard sin dientes.
sed 's|^    sys.exit(1)$|    pass|' "$GUARD" >"$TMPROOT/force_accept.sh"
if grep -q 'sys.exit(1)' "$TMPROOT/force_accept.sh"; then
  bad "no se pudo construir G0: el guardia de salida no es el que esperaba"
else
  chmod +x "$TMPROOT/force_accept.sh"
  if run_copy "$TMPROOT/force_accept.sh" >"$TMPROOT/g0.out"; then
    if grep -q 'el guard la ACEPTO' "$TMPROOT/g0.out"; then
      bad "G0: un guard que acepta todo pasa su propia autoprueba"
    else
      ok "G0 un guard sin dientes NO pasa su autoprueba (el control tiene carga)"
    fi
  else
    # Que falle entero tambien es detectable: lo que no puede es pasar.
    ok "G0 un guard sin dientes no pasa su autoprueba (falla, que es detectable)"
  fi
fi

# ── G1..G6: cada comprobacion debe ser load-bearing ────────────────────────
# Cada caso borra UNA linea del bloque de comprobaciones y exige que la
# mutacion asociada deje de rechazarse. `borrar` es una linea literal unica.
expect_tooth_lost() {
  local name="$1" needle="$2" must_flip="$3" extra="${4:-}"
  local copy="$TMPROOT/mut_$must_flip.sh"
  if ! grep -qF "$needle" "$GUARD"; then
    bad "$name (la linea a borrar no existe: el guard cambio y el falsificador no lo vio)"
    return
  fi
  # El borrado es LITERAL, con el index() de awk. Con `sed /texto/d` los
  # corchetes de una expresion como truth["project_id"] abren un conjunto de
  # caracteres y el patron deja de casar: la primera version de G1 borro cero
  # lineas, lo vio como "la comprobacion no es borrable" y casi lo reporto como
  # defecto del guard en vez de defecto del falsificador.
  #
  # La segunda needle existe para el caso en que la comprobacion vive dentro de
  # un `for`: borrar solo el `if` deja un bucle con el cuerpo vacio, que es un
  # error de sintaxis y no una comprobacion menos -- y un error de sintaxis
  # tambien pone el guard en rojo, o sea un modo de fallo INDISTINGUIBLE del que
  # se quiere medir.
  awk -v n="$needle" -v x="${extra:-}" 'index($0,n)==0 && (x=="" || index($0,x)==0)' \
      "$GUARD" >"$copy"
  if cmp -s "$copy" "$GUARD"; then
    bad "$name (la mutacion no cambio el fichero: la comprobacion no es borrable)"
    return
  fi
  local out
  out="$(run_copy "$copy")"
  if grep -q "el guard la ACEPTO" <<<"$out"; then
    if grep -qF "$must_flip" <<<"$out"; then
      ok "$name (al borrar esa comprobacion, se pierde exactamente $must_flip)"
    else
      bad "$name (se perdio otra mutacion, no $must_flip: el efecto no es el que se atribuye)"
      grep 'el guard la ACEPTO' <<<"$out" | indent
    fi
  else
    bad "$name (el guard mutado sigue rechazando $must_flip, luego esa comprobacion no es lo unico que la sostiene: o es codigo muerto o el bucle que se borro dejo el python sin cuerpo)"
    indent <<<"$out" | grep -E 'Error|Traceback|PASS=' | head -4
  fi
}

expect_tooth_lost \
  "G1 sin el control de proyecto, M5 pasa" \
  'if declared_project != truth["project_id"]' \
  'M5 declarar otro proyecto'

expect_tooth_lost \
  "G2 sin el control del total, M1 pasa" \
  'elif int(declared_total) != exp_total' \
  'M1 declarar la suma de las dos poblaciones'

expect_tooth_lost \
  "G3 sin el control del resumen de ilegibles, M3a pasa" \
  'elif int(declared_unreadable) != exp_unreadable' \
  'M3a mentir solo en el resumen de ilegibles'

expect_tooth_lost \
  "G4 sin el conteo de filas, M2 pasa" \
  'if len(rows) != exp_total' \
  'M2 emitir una fila menos de las declaradas'

expect_tooth_lost \
  "G5 sin el recuento de marcas ilegibles, M3b pasa" \
  'if not_readable != exp_unreadable' \
  'M3b mentir solo en las marcas de ilegible'

expect_tooth_lost \
  "G6 sin el cierre aritmetico, M4a pasa" \
  'if sum(by_status.values()) != exp_total' \
  'M4a desglose que suma de mas'

expect_tooth_lost \
  "G7 sin el desglose por estado, M4b pasa" \
  'if by_status.get(status) != n' \
  'M4b desglose que reparte mal entre estados' \
  'for status, n in exp_by_status.items():'

expect_tooth_lost \
  "G8 sin la pertenencia al proyecto, M7 pasa" \
  'if any(not r.startswith' \
  'M7 filas del proyecto equivocado'

echo
echo "PASS=$PASS FAIL=$FAIL"
if [[ $FAIL -eq 0 ]]; then
  echo "RESULT: PASS -- cada comprobacion es load-bearing y el control tiene carga."
else
  echo "RESULT: FAIL -- o una comprobacion no vigila lo que dice, o el falsificador"
  echo "no borro lo que creia. Las dos son codigo muerto."
fi
[[ $FAIL -eq 0 ]]
