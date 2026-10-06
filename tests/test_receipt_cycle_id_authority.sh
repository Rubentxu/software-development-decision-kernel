#!/usr/bin/env bash
# Autoridad del `cycle_id` declarado en los recibos de ciclo.
#
# EL DEFECTO
# ----------
# Un recibo es un registro de lo que se hizo y con que evidencia. Si su
# encabezado declara un `cycle_id` que el ledger no tiene, el lector no puede
# comprobar NADA: `sddk cycle status <id>` responde que no hay ciclo, y no hay
# forma de distinguir "el ciclo se cerro y se archivo" de "el ciclo nunca
# existio". El documento afirma una cosa que la autoridad contradice.
#
# MEDIDO sobre los recibos de `docs/roadmap/receipts/` antes del arreglo:
# 13 declaraban un ciclo. Cinco declaraban un `cycle_id` que NO existe en el
# ledger (`cl-ledger-declaration`, `cl-vault-declaration`, `cl-vault-graph`,
# `cl-vault-html-replica`, `cl-vault-node-projection` — cinco, no los tres que
# decia INC-DEBT-063) y dos usaban el campo `**Cycle:**` para nombrar una
# SECCION del roadmap (`C3h`, `C4`), que no es un ciclo sino otra cosa.
#
# LAS DOS CLASES SON EL MISMO DEFECTO
# ----------------------------------
# Un campo que se llama `Cycle:` y no contiene un ciclo promete una cosa y
# entrega otra. Por eso el guard tambien falla cuando un campo `**Cycle:**` no
# trae ningun `cycle_id` de forma completa: la salida honesta es renombrar el
# campo a lo que nombra, no dejar que un campo llamado Cycle afirme un ciclo
# que nadie emitio.
#
# EL HUECO QUE ESTE GUARD SE ABRIO A SI MISMO
# --------------------------------------------
# La primera version de este guard solo leia `**Cycle:**`. Al corregir los
# recibos se renombraron esos campos a `**Bloque:**` / `**Seccion ROADMAP:**` y se
# anadio un `**Ciclo SDDK:**` que dice "ninguno" — y el guard, tal como estaba,
# se quedaba ciego ante ese campo nuevo. Es el defecto INC-AUDIT-S14
# (DEFAULT-GATE-DISCONNECTED) repetido por el guard que lo evita: por eso ahora
# vigila TODO campo cuyo nombre hable de un ciclo, y comprueba el `cycle_id` que
# declare exista donde este declarado.
#
# FAIL-CLOSED, Y POR QUE
# ----------------------
# Si el ledger no se puede leer, este guard NO pasa. Un guard que se pone verde
# cuando la autoridad falta esta mintiendo con la misma forma que el defecto que
# vigila: ausencia de dato presentada como afirmacion positiva.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

PASS=0
FAIL=0
ok()  { printf '  [ok]   %s\n' "$*"; PASS=$((PASS + 1)); }
ko()  { printf '  [FAIL] %s\n' "$*"; FAIL=$((FAIL + 1)); }

STATE_BASE="${SDDK_STATE_HOME:-${XDG_STATE_HOME:-$HOME/.local/state}}"
export STATE_BASE

# Forma de un cycle_id real: project_id hex de 16 + slug. Deliberadamente
# ESTRICTA: un token como `OPEN/build` o `B-direct` no es un cycle_id, y hacer
# que el guard adivine cual de los tokens de una linea pretendia serlo produce
# falsos positivos que taught al guard a callarse.
ID_RE='p-[0-9a-f]{16}/[A-Za-z0-9._-]+'
# El campo que AFIRMA un ciclo del ledger: si existe, tiene que existir.
CYCLE_FIELD_RE='^\*\*Cycle:\*\*'
# Cualquier OTRO campo que nombre un ciclo: si declara un id, ese id tiene que
# existir. Aqui "no hay ciclo" es una declaracion valida y no se mide.
#
# El campo tiene que EMPEZAR por "Ciclo"/"Cycle", no solo contener la palabra.
# Una version anterior uso `**[^*]*[Cc]iclo[^*]*:**` y mitado tres lineas de prosa
# en negrita queosion de ciclo:
#   **Lo que hay que decidir, y no es de este ciclo:** quien produce `HostEvent`.
# Esos tres no declaran un ciclo, y un guard que se ensancha hasta meterse con
# la prosa del documento acaba(callandose, que es la forma que un guard
# encuentra para seguir verde sin vigilar nada.
OTRO_CICLO_RE='^\*\*(Ciclo|Cycle)\b[^*]{0,24}:\*\*'

# ALCANCE, Y LO QUE DEJA FUERA A PROPOSITO
# ----------------------------------------
# Se vigilan los documentos que AFIRMAN RESULTADO: `RECEIPT*.md` (incluidos los
# `-lote-N`) y `CLOSURE.md`. Un recibo es "un registro de lo que se hizo y con
# que evidencia", y ahi un `cycle_id` falso deja al lector sin nada que
# comprobar.
#
# Quedan FUERA `PRE-FLIGHT.md` (16 ficheros) y `SCOPE-CONTRACT.md` (11), que
# declaran INTENCION, no resultado. Es una decision, no un limite de medicion:
# el numero esta medido (27 ficheros con un `cycle_id` inexistente, mas 31
# afirmaciones `**Cycle:** C3e` sin id de forma completa) y queda registrado
# como seguimiento en `docs/debt/INC-DEBT-063-FOLLOWUP-PLANNING-DOCUMENTS.md`.
# La razon de no reescribirlos es que un plan que nombra un ciclo que nunca llego
# a emitirse no afirma que el trabajo ocurrio bajo ese ciclo: afirma lo que el
# autor pretendia al escribirlo. Cambiarlo es reescribir historia, y este repo
# prohibe hacerlo sin mapa de enlaces. Si mañana el sweep se hace, este guard se
# amplia — y por eso el alcance esta escrito aqui y no escondido en un rango.
#
# Una version anterior de este guard miraba TODO `docs/roadmap/receipts/**`, y por
# eso aparecio con PASS=43 FAIL=64: no era un defecto del guard sino su alcance.
# Un guard cuyo alcance se estrecha para no tener Deal con lo que mide es el
# defecto INC-AUDIT-S14-DEFAULT-GATE-DISCONNECTED; por eso el recorte va
# declarado con numeros, no ejecutado en silencio.
mapfile -t RECEIPTS < <(
    find docs/roadmap/receipts -type f \( -name 'RECEIPT*.md' -o -name 'CLOSURE.md' \) 2>/dev/null | sort
)
if [ "${#RECEIPTS[@]}" -eq 0 ]; then
    printf 'ERROR: no hay recibos que vigilar. El guard no tiene nada que medir.\n'
    exit 1
fi

printf 'autoridad: %s\n' "$STATE_BASE/sddk/projects/<project_id>/ledger.sqlite"

# ── 1. Recoger las afirmaciones de ciclo ─────────────────────────────────────
declare -a AFIRMA_FICHERO=()   # fichero
declare -a AFIRMA_ETIQUETA=()  # texto de la asercion, para el veredicto
declare -a AFIRMA_IDS=()       # ids declarados (vacio = "no hay ciclo aqui")
declare -a SIN_AFIRMAR=()

for f in "${RECEIPTS[@]}"; do
    block="$(basename "$(dirname "$f")")"
    n=0
    while IFS= read -r line; do
        [ -z "$line" ] && continue
        n=$((n + 1))
        ids="$(printf '%s\n' "$line" | grep -oE "$ID_RE" | sort -u | tr '\n' ' ')"
        if printf '%s\n' "$line" | grep -qE "$CYCLE_FIELD_RE"; then
            AFIRMA_ETIQUETA+=("campo **Cycle:**")
        else
            AFIRMA_ETIQUETA+=("$(printf '%s\n' "$line" | grep -oE '^\*\*[^*]*:\*\*')")
        fi
        AFIRMA_FICHERO+=("$f")
        AFIRMA_IDS+=("${ids% }")
    done < <(grep -hE "$CYCLE_FIELD_RE|$OTRO_CICLO_RE" "$f" 2>/dev/null)
    [ "$n" -eq 0 ] && SIN_AFIRMAR+=("$block")
done

printf 'recibos sin afirmacion de ciclo (fuera de alcance): %d\n' "${#SIN_AFIRMAR[@]}"
printf 'afirmaciones de ciclo a comprobar: %d\n' "${#AFIRMA_FICHERO[@]}"
printf '\n'

# ── 2. Una sola consulta, y fail-closed si la autoridad no esta ──────────────
# Los ids viajan por ENTORNO, no por stdin: el heredoc del programa ya ocupa
# stdin, y leer de ahi haria que el guard se comprobara a si mismo. La primera
# version lo hizo, y fallo cerrado por su propia causa.
ALL_IDS="$(for i in "${AFIRMA_IDS[@]}"; do printf '%s\n' "$i"; done | tr ' ' '\n' | grep -E "$ID_RE" | sort -u)"
export ALL_IDS

PROBE="$(python3 - <<'PY' 2>/dev/null
import os, sqlite3, sys
state = os.environ["STATE_BASE"]
ids = [l.strip() for l in os.environ.get("ALL_IDS", "").splitlines() if l.strip()]
if not ids:
    sys.exit(0)
projects = sorted({i.split("/", 1)[0] for i in ids})
missing = []
cons = {}
for pid in projects:
    db = os.path.join(state, "sddk", "projects", pid, "ledger.sqlite")
    if not os.path.isfile(db):
        missing.append(db)
        continue
    try:
        cons[pid] = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    except Exception as e:  # noqa: BLE001
        print(f"{db}: {e}", file=sys.stderr)
        missing.append(db)
if missing:
    for m in missing:
        print(f"no se pudo abrir: {m}", file=sys.stderr)
    sys.exit(3)
for i in ids:
    pid, _slug = i.split("/", 1)
    try:
        n = cons[pid].execute("SELECT COUNT(*) FROM cycles WHERE cycle_id=?", (i,)).fetchone()[0]
    except Exception as e:  # noqa: BLE001
        print(f"{i}: {e}", file=sys.stderr)
        sys.exit(3)
    print(f"{i}\t{n}")
PY
)"
rc=$?

if [ "$rc" -ne 0 ] || { [ -n "$ALL_IDS" ] && [ -z "$PROBE" ]; }; then
    printf '  [FAIL] la autoridad no se pudo leer; el guard NO pasa sin ella.\n'
    printf '         ledger esperado bajo: %s\n' "$STATE_BASE"
    printf '         (export SDDK_STATE_HOME o XDG_STATE_HOME si vive en otro sitio)\n'
    printf '\nPASS=%d FAIL=1 SKIP=0\n' "$PASS"
    printf 'RESULT: FAIL — sin autoridad no hay veredicto, y "sin veredicto" no es verde.\n'
    exit 1
fi

# ── 3. Veredicto ─────────────────────────────────────────────────────────────
for idx in "${!AFIRMA_FICHERO[@]}"; do
    f="${AFIRMA_FICHERO[$idx]}"
    etiqueta="${AFIRMA_ETIQUETA[$idx]}"
    ids="${AFIRMA_IDS[$idx]}"
    block="$(basename "$(dirname "$f")")"

    if [ "$etiqueta" = "campo **Cycle:**" ] && [ -z "$ids" ]; then
        ko "$block: el campo **Cycle:** no declara ningun cycle_id. O se renombra a lo que nombra, o se declara el ciclo que existe."
        continue
    fi

    if [ -z "$ids" ]; then
        ok "$block: $etiqueta declara que no hay ciclo (no afirma nada que no se pueda comprobar)."
        continue
    fi

    limpio=1
    for i in $ids; do
        n="$(printf '%s\n' "$PROBE" | awk -F'\t' -v k="$i" '$1==k {print $2}')"
        if [ "${n:-0}" -eq 0 ]; then
            ko "$block: $etiqueta declara '$i' y el ledger NO tiene ese ciclo (0 filas). La documentacion afirma un ciclo que la autoridad nunca emitio."
            limpio=0
        fi
    done
    [ "$limpio" -eq 1 ] && ok "$block: $etiqueta declara $(printf '%s' "$ids" | tr '\n' ' ') y existe en el ledger."
done

printf '\n----------------------------------------\n'
printf 'PASS=%d FAIL=%d SKIP=0\n' "$PASS" "$FAIL"
if [ "$FAIL" -eq 0 ]; then
    printf 'RESULT: PASS — ningun recibo declara un ciclo que la autoridad no tiene.\n'
    exit 0
fi
printf 'RESULT: FAIL — hay recibos que declaran ciclos inexistentes.\n'
exit 1
