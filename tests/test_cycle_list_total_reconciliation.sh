#!/usr/bin/env bash
# Guard de reconciliacion de `sddk cycle list` contra la autoridad.
#
# QUE PROPIEDAD VIGILA, Y POR QUE ES LA UNICA QUE NO ES VACIA
# -----------------------------------------------------------
# `cycle list` imprime su total como `output.cycles.len()`
# (crates/sddk-cli/src/cycle.rs:2115), es decir, DESPUES de aplicar el filtro
# que sea, sobre el MISMO vector que emite. Por eso
#
#     total declarado == filas emitidas
#
# es estructuralmente insatisfacible: no puede fallar sin que el comando no
# corra, y un guard que solo mide eso es un guard que da verde siempre. Ese
# error se cometio aqui al principio y por eso esta escrito.
#
# La comparacion que SI tiene dientes es de FUENTE CRUZADA: el total que declara
# el producto contra las filas que la autoridad tiene para el proyecto que el
# propio producto declara. Las dos veng de sitios distintos -- una del proceso,
# otra de la tabla `cycles` -- luego pueden discrepar, y discrepan en cuanto
# `list_cycles` gana un filtro, un `LIMIT`, un alcance por `workspace_id`, o
# empieza a descartar en vez de contar los manifiestos ilegibles.
#
# Ese es exactamente el modo de fallo que produjo las cifras erroneas de
# INC-DEBT-060: dos poblaciones sharing fichero y un denominador que las suma.
# Ver tests/test_cycle_list_total_reconciliation_mutation.sh, que exige que este
# guard RECHACE cada modo de mentira, y un control que exige que acepte el caso
# bueno -- un guard que rechaza todo pasa la falsacion sin vigilar nada.
#
# HERMETICO. State home temporal por caso, fixture sintetico construido sobre un
# ledger que crea el propio producto (schema correcto por construccion, sin
# duplicar migraciones aqui). Sin red. No toca el ledger real ni el repo.
#
# Falla cerrado: sin binario no se da por bueno nada.
set -uo pipefail

# El guard corre sobre una COPIA en temporal cuando lo falsifica
# tests/test_cycle_list_total_reconciliation_mutation.sh, y la copia vive fuera
# del repo. Sin este override el guard calcularia ROOT a partir de su propia
# ruta -- es decir `/tmp/..` -- y sus fixtures se caerian al vacio, con lo que
# el falsificador mediria su propia rotura en vez de la del guard. Se declara
# aqui para que el motivo este escrito y no se lea como un knob de test.
ROOT="${SDDK_GUARD_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
PASS=0
FAIL=0

ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

# Indenta un bloque de texto con ocho espacios. Con un bucle y no con
# `sed 's/^/        /' "$var"` porque shellcheck marca lo segundo como SC2001 y
# el gate de este repo corre shellcheck sobre tests/test_*.sh.
indent() {
  local line
  while IFS= read -r line; do printf '        %s\n' "$line"; done
}

# ── binario ────────────────────────────────────────────────────────────────
# El guard necesita un binario CON `cycle list`. El del PATH puede no tenerlo
# (INC-DEBT-064), y medir con el equivocado es lo que vicio la revalidacion de
# R2, asi que se dice en pantalla de donde sale y se falla cerrado si no hay
# ninguno con el subcomando.
#
# La comprobacion de capacidad se aplica TAMBIEN al binario que se pasa por
# entorno. La primera version solo miraba los candidatos del bucle de
# descubrimiento, de modo que `SDDK_GUARD_BIN=<el binario viejo>` se saltaba la
# comprobacion y el guard arrancaba los casos contra un binario sin `cycle
# list`: fail-closed incumplido justo en el caso que este repo tiene vivo.
# Un chequeo que se puede desactivar con una variable de entorno no es un
# chequeo.
BIN="${SDDK_GUARD_BIN:-}"
if [[ -z "$BIN" ]]; then
  # Se mira tambien `$CARGO_TARGET_DIR`, no solo `target/`: en esta maquina el
  # directorio de compilacion esta fuera del repo, y un guard que solo conoce
  # `target/` falla cerrado sin motivo y acaba sin usarse.
  for cand in \
      "${CARGO_TARGET_DIR:+$CARGO_TARGET_DIR/debug/sddk}" \
      "${CARGO_TARGET_DIR:+$CARGO_TARGET_DIR/release/sddk}" \
      "$ROOT/target/release/sddk" "$ROOT/target/debug/sddk" \
      "$(command -v sddk 2>/dev/null || true)"; do
    [[ -n "$cand" && -x "$cand" ]] || continue
    if "$cand" cycle list --help >/dev/null 2>&1; then BIN="$cand"; break; fi
  done
fi
if [[ -z "$BIN" ]]; then
  echo "FAIL: no encuentro un binario sddk con 'cycle list'."
  echo "      Indicame uno con SDDK_GUARD_BIN=/ruta/a/sddk."
  exit 1
fi
if ! "$BIN" cycle list --help >/dev/null 2>&1; then
  echo "FAIL: el binario indicado no tiene 'cycle list': $BIN"
  # `sddk --version` escribe en STDERR, no en stdout: con `2>/dev/null` el guard
  # imprimia una version vacia en su propio mensaje de error, que es una manera
  # cara de no decir nada. Se captura con `2>&1`.
  echo "      '$("$BIN" --version 2>&1 || echo version desconocida)' es anterior"
  echo "      a 113f84ba, luego no se puede reconciliar nada con el."
  echo "      Es la condicion de INC-DEBT-064 en su forma mas literal: el"
  echo "      artefacto viejo no puede ejecutar el control."
  exit 1
fi
echo "=== reconciliacion de 'cycle list' contra la autoridad ==="
echo "  binario: $BIN ($("$BIN" --version 2>&1 || echo 'version desconocida'))"

WORKROOT="$(mktemp -d)"
cleanup() { rm -rf "$WORKROOT"; }
trap cleanup EXIT

# ── fixture: el producto crea el ledger, el guard le inserta filas ─────────
# Se deja que sea el producto quien cree el ledger porque asi el schema es el
# suyo y este fichero no lleva una segunda copia de las migraciones. El
# project_id se descubre del stdout del producto, no se supone.
#
# spec (JSON por stdin): {"real":[{...}], "spine":[{...}]}
# Cada item: {"status": str, "phase": str, "manifest": "good"|"empty"}
fixture() {
  local state_home="$1"
  # El spec entra por stdin y se lee ANTES de cualquier heredoc: un heredoc
  # sustituye stdin del proceso, luego leer el spec con `cat` desde dentro de
  # un `python3 - <<PY` lo deja vacio. Ese fallo salio primero, y el control de
  # no-vacuidad de mas abajo lo detecta -- sin el, los seis modos de mentira
  # habrian dado "rechazado" porque el truth ni existia.
  local spec; spec="$(cat)"
  mkdir -p "$state_home"
  printf '%s' "$spec" >"$state_home/spec.json"
  SDDK_STATE_HOME="$state_home" "$BIN" cycle list \
      --no-infer --root "$ROOT" --scope . >"$state_home/discover.out" 2>&1
  local pid
  pid="$(sed -n 's/^project: //p' "$state_home/discover.out" | head -1)"
  if [[ -z "$pid" ]]; then
    echo "  [FAIL] el producto no declaro project: al crear el ledger" >&2
    cat "$state_home/discover.out" >&2
    return 1
  fi
  local ledger
  ledger="$(find "$state_home" -name ledger.sqlite -print -quit)"
  if [[ -z "$ledger" ]]; then
    echo "  [FAIL] el producto no creo ledger.sqlite" >&2
    return 1
  fi
  python3 - "$ledger" "$pid" "$state_home/spec.json" <<'PY'
import json, sqlite3, sys
ledger, pid, spec_path = sys.argv[1], sys.argv[2], sys.argv[3]
spec = json.load(open(spec_path))
conn = sqlite3.connect(ledger)

# Plantilla de manifiesto VALIDO, con la forma que un manifiesto real tiene en
# este ledger. No se copia ninguno real: se escribe el conjunto de campos que
# `CycleManifest` exige, con los mismos valores de forma que se observan en los
# manifiestos que deserializan.
#
# Que la plantilla sea valida NO se supone: lo verifica el propio guard. El
# control de no-vacuidad espera que el producto declare ilegibles exactamente
# las filas con `{}`; si la plantilla no deserializara, el producto contaria
# tambien las "buenas" y el control saldria rojo. O sea que un manifiesto mal
# escrito aqui no se manifiesta como un guard roto silencioso.
GOOD = json.dumps({
    "schema_version": 1,
    "project_id": pid,
    "workspace_id": "w-fixture",
    "cycle_id": f"{pid}/fixture",
    "display_name": "fixture",
    "status": "CLOSED",
    "phase": "archive",
    "path": "a-min",
    "branch": "main",
    "base": "0000000",
    "head": None,
    "artifacts": {},
    "release": None,
})
EMPTY = "{}"

rows, truth = [], {"total": 0, "by_status": {}, "unreadable": 0,
                   "by_status_unreadable": {}, "project_id": pid}

def put(cycle_id, project_id, workspace_id, status, phase, manifest):
    rows.append((cycle_id, project_id, workspace_id, status, phase, manifest,
                 "2026-10-01T00:00:00Z", "2026-10-01T00:00:00Z"))

for i, item in enumerate(spec.get("real", [])):
    readable = item.get("manifest", "good") == "good"
    st = item.get("status", "OPEN")
    put(f"{pid}/real-{i:03d}", pid, "w-fixture", st,
        item.get("phase", "build"), GOOD if readable else EMPTY)
    truth["total"] += 1
    truth["by_status"][st] = truth["by_status"].get(st, 0) + 1
    if not readable:
        truth["unreadable"] += 1
        truth["by_status_unreadable"][st] = truth["by_status_unreadable"].get(st, 0) + 1

spine = 0
for i, item in enumerate(spec.get("spine", [])):
    put(f"SPINE-{i:03d}", "__spine_import__", "__spine_import_ws__",
        item.get("status", "OPEN"), item.get("phase", "build"),
        GOOD if item.get("manifest", "good") == "good" else EMPTY)
    spine += 1

conn.executemany(
    "INSERT INTO cycles (cycle_id, project_id, workspace_id, status, phase,"
    " manifest_json, created_at, updated_at) VALUES (?,?,?,?,?,?,?,?)", rows)
conn.commit()
truth["rows_all"] = truth["total"] + spine
with open(ledger + ".truth.json", "w") as fh:
    json.dump(truth, fh, sort_keys=True)
PY
  echo "$ledger"
}

# ── reconciliacion: producto vs autoridad ──────────────────────────────────
# Unico sitio donde se decide. Acepta si y solo si las tres declaraciones del
# producto cuadran con las filas que el fixture sabe que hay.
reconcile() {
  local out="$1" truth="$2" status="${3:-}"
  python3 - "$out" "$truth" "$status" <<'PY'
import json, re, sys
out, truth_path, status_filter = sys.argv[1], sys.argv[2], sys.argv[3]
truth = json.load(open(truth_path))
text = open(out, encoding="utf-8", errors="replace").read()

# La verdad esperada depende del filtro, y no es un detalle: `--status OPEN`
# estrecha la enumeracion a proposito, luego el total declarado tiene que ser el
# de ESE estado. Comparar el total filtrado contra la poblacion entera haria
# fallar el caso bueno y entrenaria a ignorar el rojo.
if status_filter:
    exp_total = truth["by_status"].get(status_filter, 0)
    exp_unreadable = truth["by_status_unreadable"].get(status_filter, 0)
    exp_by_status = {status_filter: exp_total} if exp_total else {}
else:
    exp_total = truth["total"]
    exp_unreadable = truth["unreadable"]
    exp_by_status = truth["by_status"]

def field(name):
    m = re.search(rf"^{re.escape(name)}: (\S+)$", text, re.M)
    return m.group(1) if m else None

problems = []
declared_project = field("project")
declared_total = field("cycles")
declared_unreadable = field("unreadable_manifests")
by_status = {k: int(v) for k, v in re.findall(r"^status\[([^\]]+)\]: (\d+)$", text, re.M)}
rows = re.findall(r"^cycle: (\S+)$", text, re.M)
# El campo va indentado dos espacios dentro de su fila; un regex anclado en
# `^manifest_readable` no lo encuentra nunca y cuenta cero ilegibles. asi que el
# rojo salio en la primera pasada por el regex, no por el producto.
not_readable = len(re.findall(r"^\s*manifest_readable: false\s*$", text, re.M))

# Cada comprobacion es UNA linea a proposito. Un bloque de varias lineas se
# puede quitar con `sed` solo por su valor de indentacion, y un mutador que no
# sabe borrar el bloque entero se lleva por delante el mutador equivocado -- con
# una linea, borrar la linea ES quitar la comprobacion, y el falsificador puede
# exigir que cada comprobacion sea load-bearing una por una.
if declared_project != truth["project_id"]: problems.append(f"proyecto declarado {declared_project!r} != autoridad {truth['project_id']!r}")
if declared_total is None: problems.append("el comando no declaro 'cycles:'")
elif int(declared_total) != exp_total: problems.append(f"total declarado {declared_total} != filas de la autoridad {exp_total}")
if declared_unreadable is None: problems.append("el comando no declaro 'unreadable_manifests:'")
elif int(declared_unreadable) != exp_unreadable: problems.append(f"ilegibles declarados {declared_unreadable} != los que hay {exp_unreadable}")
if len(rows) != exp_total: problems.append(f"emitio {len(rows)} filas por una autoridad de {exp_total}")
if not_readable != exp_unreadable: problems.append(f"marco {not_readable} filas ilegibles y la autoridad tiene {exp_unreadable}")
if sum(by_status.values()) != exp_total: problems.append(f"el desglose suma {sum(by_status.values())} y la autoridad tiene {exp_total}")
for status, n in exp_by_status.items():
    if by_status.get(status) != n: problems.append(f"desglose {status}={by_status.get(status)} != autoridad {n}")
# Un filtro que ensancha en vez de estrechar declararia estados que el producto
# no pidio, y eso tambien es mentir sobre la poblacion.
if status_filter and any(s != status_filter for s in by_status): problems.append(f"desglose con estados no pedidos bajo un filtro por {status_filter}")
# Toda fila emitida tiene que pertenecer al proyecto declarado. Sin esto, una
# fila de la otra poblacion colada en la salida pasaria el recuento.
if any(not r.startswith(f"{truth['project_id']}/") for r in rows): problems.append("emite filas de otro proyecto")

if problems:
    print("REJECT")
    for p in problems:
        print("  - " + p)
    sys.exit(1)
print("ACCEPT")
PY
}

# Corre el caso completo: fixture -> producto -> reconciliacion.
case_run() {
  local name="$1" spec="$2" status_filter=""
  shift 2
  if [[ "${1:-}" == "--status" ]]; then status_filter="$2"; fi
  local slug; slug="$(echo "$name" | tr -cd 'a-z0-9')"
  local dir="$WORKROOT/$slug"
  local ledger verdict
  ledger="$(fixture "$dir" <<<"$spec")" || { bad "$name (no se pudo construir el fixture)"; return; }
  SDDK_STATE_HOME="$dir" "$BIN" cycle list --no-infer --root "$ROOT" --scope . "$@" \
      >"$dir/out.txt" 2>"$dir/err.txt"
  local rc=$?
  if [[ $rc -ne 0 ]]; then
    bad "$name (el comando salio con $rc)"
    indent <"$dir/err.txt" | head -5
    return
  fi
  if verdict="$(reconcile "$dir/out.txt" "$ledger.truth.json" "$status_filter")"; then
    ok "$name"
  else
    bad "$name"
    indent <<<"$verdict"
  fi
}

# ═══════════════════════════════════════════════════════════════════════════
echo
echo "-- casos: el producto contra un fixture cuya verdad es conocida --"

# C1 -- una sola poblacion, todo legible. El control de que el guard no exige
# nada imposible: si esto no pasa, el guard esta roto y todo lo de abajo es
# ruido.
case_run "C1 una poblacion, 12 ciclos legibles" \
  '{"real":[{"status":"CLOSED"},{"status":"CLOSED"},{"status":"OPEN"},{"status":"OPEN"},{"status":"OPEN"},{"status":"OPEN"},{"status":"OPEN"},{"status":"PAUSED"},{"status":"RELEASED"},{"status":"RELEASE_PENDING"},{"status":"RELEASE_PENDING"},{"status":"RELEASE_PENDING"}]}'

# C2 -- DOS POBLACIONES en el mismo fichero. El caso que discrimina: el
# producto debe declarar 5, no 9. Un guard sin alcance por proyecto, o un
# producto que pierda el `WHERE project_id = ?1`, dan 9 aqui.
case_run "C2 dos poblaciones: declara 5, no 9" \
  '{"real":[{"status":"OPEN"},{"status":"OPEN"},{"status":"OPEN"},{"status":"CLOSED"},{"status":"CLOSED"}],"spine":[{"status":"OPEN"},{"status":"OPEN"},{"status":"OPEN"},{"status":"OPEN"}]}'

# C3 -- manifiestos ilegibles: se CUENTAN, no se tiran. El total sigue siendo
# el de la autoridad y los ilegibles salen marcados.
case_run "C3 dos manifiestos ilegibles: contados y marcados" \
  '{"real":[{"status":"OPEN"},{"status":"OPEN","manifest":"empty"},{"status":"CLOSED"},{"status":"CLOSED","manifest":"empty"},{"status":"PAUSED"},{"status":"RELEASED"}]}'

# C4 -- las dos poblaciones Y ilegibles a la vez, que es el estado real del
# ledger de este repo (79 importadas con `{}` + 2 reales ilegibles).
case_run "C4 spine con {} mas dos ilegibles reales" \
  '{"real":[{"status":"OPEN"},{"status":"CLOSED","manifest":"empty"},{"status":"CLOSED"},{"status":"RELEASED","manifest":"empty"}],"spine":[{"status":"OPEN","manifest":"empty"},{"status":"OPEN","manifest":"empty"}]}'

# C5 -- proyecto vacio. Cierra en 0 y declara 0, sin reventar.
case_run "C5 proyecto vacio" '{"real":[]}'

# C6 -- el camino del filtro por estado. El total declarado tiene que ser el de
# las filas de ESE estado, no el de todas, y el desglose tiene que cuadrar con
# lo que se ha estrecho.
case_run "C6 filtro por estado OPEN" \
  '{"real":[{"status":"OPEN"},{"status":"OPEN"},{"status":"OPEN"},{"status":"CLOSED"},{"status":"CLOSED"}]}' \
  --status OPEN

# ═══════════════════════════════════════════════════════════════════════════
echo
echo "-- autoprueba: el guard RECHAZA cada modo de mentira --"
# Sin esto, la seccion de arriba solo se ha observado en verde. Un guard cuyas
# comprobaciones nunca se han visto fallar es una afirmacion sin comprobar
# wearing un checkmark, y esta repo ya lo ha pagado siete veces.
#
# Y el control de no-vacuidad: el guard tiene que ACEPTAR la salida sin mutar.
# Un guard que rechazase todo pasaria esta seccion sin vigilar nada.
AUTO="$WORKROOT/auto"
LEDGER_AUTO="$(fixture "$AUTO" <<<'{"real":[{"status":"OPEN"},{"status":"OPEN","manifest":"empty"},{"status":"CLOSED"},{"status":"CLOSED"}],"spine":[{"status":"OPEN"},{"status":"OPEN"}]}')" \
  || { bad "autoprueba (no se pudo construir el fixture)"; }
SDDK_STATE_HOME="$AUTO" "$BIN" cycle list --no-infer --root "$ROOT" --scope . \
    >"$AUTO/good.txt" 2>/dev/null
TRUTH="$LEDGER_AUTO.truth.json"

# El control. Si el caso bueno no pasa, todo lo de abajo no significa nada.
if reconcile "$AUTO/good.txt" "$TRUTH" >/dev/null; then
  ok "control: la salida sin mutar se ACEPTA (el guard no es vacio)"
else
  bad "control: la salida sin mutar se RECHAZA -- el guard es vacio o el fixture no sirve"
fi

mutate() {
  local name="$1" rule="$2"
  # La verdad se pasa a la mutacion porque el numero que hay que sumar no esta
  # en la salida: las filas de la otra poblacion NO se emiten, luego un
  # mutador que las buscara en el texto sumaria cero. La primera version de M1
  # hacia exactamente eso y resulto ser una mutacion NULA -- hacia verde sin
  # cambiar nada, y la seccion de autoprueba la daba por buena. El defecto
  # estaba en el mutador, no en el guard, y se corrige en el mutador.
  python3 - "$AUTO/good.txt" "$AUTO/mut.txt" "$rule" "$TRUTH" <<'PY'
import json, re, sys
src, dst, rule, truth_path = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
truth = json.load(open(truth_path))
t = open(src, encoding="utf-8").read()
if rule == "inflate_total":
    # El error real de este repo: declarar la SUMA de las dos poblaciones. El
    # numero de filas de la otra poblacion sale de la autoridad, no del texto.
    total = int(re.search(r"^cycles: (\d+)$", t, re.M).group(1))
    spine = truth["rows_all"] - truth["total"]
    assert spine > 0, "el fixture necesita una segunda poblacion para que M1 signifique algo"
    t = re.sub(r"^cycles: \d+$", f"cycles: {total + spine}", t, count=1, flags=re.M)
elif rule == "drop_row":
    rows = re.findall(r"^cycle: .*\n(?:  .*\n)*", t, re.M)
    t = t.replace(rows[-1], "", 1)
elif rule == "hide_unreadable_count":
    # Solo el RESUMEN. Los marcas de fila se quedan como estaban, con lo que
    # la comprobacion de marcas sigue cumpliendo y la unica que puede=red
    # es la del resumen.
    t = re.sub(r"^unreadable_manifests: \d+$", "unreadable_manifests: 0", t, count=1, flags=re.M)
elif rule == "hide_unreadable_marks":
    # Solo las MARCAS de fila. El resumen se queda, con lo que la unica
    # comprobacion que puede red es la del recuento de marcas.
    t = re.sub(r"^\s*manifest_readable: false\s*$", "  manifest_readable: true", t, flags=re.M)
elif rule == "break_sum":
    # El desglose suma de mas, y sin que ningun estado real este mal: la
    # comprobacion aritmetica es la unica que puede red.
    t = re.sub(r"^(status\[OPEN\]: \d+)$", r"\1\nstatus[GHOST]: 1", t, count=1, flags=re.M)
elif rule == "break_status":
    # Un estado de mas y otro de menos: la suma sigue cuadrando, luego la
    # comprobacion aritmetica no puede red y la unica que puede es la del
    # desglose por estado. Es el caso que hace que las dos sean necesarias y
    # no una sola repetida dos veces.
    o = int(re.search(r"^status\[OPEN\]: (\d+)$", t, re.M).group(1))
    c = int(re.search(r"^status\[CLOSED\]: (\d+)$", t, re.M).group(1))
    t = re.sub(r"^status\[OPEN\]: \d+$", f"status[OPEN]: {o - 1}", t, count=1, flags=re.M)
    t = re.sub(r"^status\[CLOSED\]: \d+$", f"status[CLOSED]: {c + 1}", t, count=1, flags=re.M)
elif rule == "foreign_row":
    # El total y el numero de filas siguen cuadrando, y el proyecto declarado
    # es el correcto: lo unico que miente es QUE filas son. Una consulta que
    # devolviera las filas de otro proyecto pasaria el recuento entero, y sin
    # esta comprobacion pasaria el guard.
    pid = re.search(r"^project: (\S+)$", t, re.M).group(1)
    t = re.sub(rf"^cycle: {re.escape(pid)}/real-\d+$", "cycle: SPINE-999", t, count=1, flags=re.M)
elif rule == "claim_other_project":
    t = re.sub(r"^project: \S+$", "project: __spine_import__", t, count=1, flags=re.M)
elif rule == "silent_truncation":
    # La clase F63: declara menos de lo que emito y no lo dice.
    total = int(re.search(r"^cycles: (\d+)$", t, re.M).group(1))
    t = re.sub(r"^cycles: \d+$", f"cycles: {total - 1}", t, count=1, flags=re.M)
else:
    raise SystemExit(f"regla de mutacion desconocida: {rule}")
open(dst, "w", encoding="utf-8").write(t)
PY
  if reconcile "$AUTO/mut.txt" "$TRUTH" >/dev/null 2>&1; then
    bad "$name (el guard la ACEPTO -- no vigila)"
  else
    ok "$name"
  fi
}

# Cada mutacion corrompe UNA sola declaracion, no dos a la vez. La primera
# version hacia lo contrario -- M3 escondia el resumen Y las marcas, M4
# descuadraba el desglose Y un estado -- y el falsificador Thenlio que dos
# comprobaciones eran la misma repetida, cuando lo cierto es que cada una
# compara una DECLARACION DISTINTA del producto: el resumen, la marca por fila,
# el total del desglose y cada estado. Una mutacion compuesta no puede
# distinguir quien red, y por eso el falsificador de este guard vivio en
# `tests/test_cycle_list_total_reconciliation_mutation.sh` exigiendo que cada
# comprobacion sea load-bearing por separado.
mutate "M1 declarar la suma de las dos poblaciones"   inflate_total
mutate "M2 emitir una fila menos de las declaradas"    drop_row
mutate "M3a mentir solo en el resumen de ilegibles"    hide_unreadable_count
mutate "M3b mentir solo en las marcas de ilegible"     hide_unreadable_marks
mutate "M4a desglose que suma de mas"                  break_sum
mutate "M4b desglose que reparte mal entre estados"    break_status
mutate "M5 declarar otro proyecto"                     claim_other_project
mutate "M6 truncar en silencio y no decirlo"           silent_truncation
mutate "M7 filas del proyecto equivocado"              foreign_row

# ═══════════════════════════════════════════════════════════════════════════
echo
echo "PASS=$PASS FAIL=$FAIL"
if [[ $FAIL -eq 0 ]]; then
  echo "RESULT: PASS -- el total que declara el producto es el que tiene la autoridad."
else
  echo "RESULT: FAIL -- el producto declara una poblacion que la autoridad no tiene,"
  echo "o el guard dejo de distinguir el caso bueno de los seis modos de mentira."
fi
[[ $FAIL -eq 0 ]]
