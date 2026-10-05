#!/usr/bin/env bash
# uat_ctx_006_skill_runtime_alignment.sh — CTX-UAT-005 (C3i).
#
# Criterio de la matriz: "Skill resume contra runtime real (0/1/N) → la skill
# enseña inferencia y recoveries, no bloqueo por 'no discovery'; legacy callers
# intactos."
#
# Por qué este script existe y qué aporta que los anteriores no aportan:
# `uat_ctx_002` cubre el estado 0 y `uat_ctx_004` los estados 1 y N, pero cada
# uno abre su propio sandbox. Ninguno demuestra que la escalera completa
# 0 → 1 → N se sostenga en UNA instancia del runtime, y —esto es lo que
# estaba sin cubrir— ninguno comprueba que las ACCIONES DE RECUPERACIÓN que la
# skill nombra sean reales y no decorativas. La skill dice dos cosas accionables:
#
#   estado 0 → "run `sddk cycle start ...`": la acción legal es arrancar ciclo.
#   estado N → "ask the human to choose; never guess": la salida es la lista de
#              candidates, y la elección se hace pasando el id explícitamente.
#
# Aquí ambas acciones se ejecutan de verdad contra el runtime. Una skill que
# enseñara un recovery inexistente pasaría los tests de texto (que ya existen
# en tests/test_workflow_contract.py) y fallaría aquí.
#
# Uso:
#   bash tests/uat_ctx_006_skill_runtime_alignment.sh [--bin <ruta>] [--keep]
#
# Exit 0 = PASS; exit 1 = FAIL; exit 2 = error de invocación.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
TARGET_DIR="$(cargo metadata --format-version 1 --no-deps 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])' \
    || printf '%s/target' "$REPO_ROOT")"
BIN="${SDDK_BIN:-$TARGET_DIR/release/sddk}"
KEEP="false"

while [ $# -gt 0 ]; do
    case "$1" in
        --bin)
            BIN="${2:?--bin requires a value}"
            shift 2
            ;;
        --keep)
            KEEP="true"
            shift
            ;;
        *)
            echo "error: unknown flag $1" >&2
            exit 2
            ;;
    esac
done

if [ ! -x "$BIN" ]; then
    echo "error: sddk binary not found or not executable: $BIN" >&2
    echo "hint: cargo build --release --bin sddk" >&2
    exit 2
fi

echo "CTX-UAT-005 — binario: $BIN ($("$BIN" --version))"

SANDBOX="$(mktemp -d)"
# El trap NO puede alterar el código de salida del script: si la limpieza
# falla, el exit observado sería el de `rm`, no el del veredicto. Se guarda el
# status y se restaura, y se devuelve HOME a su valor real antes de borrar
# (este script lo sobreescribe con el sandbox).
# shellcheck disable=SC2329  # cleanup se invoca vía trap EXIT.
cleanup() {
    local rc=$?
    export HOME="${REAL_HOME:-$HOME}"
    if [ "$KEEP" = "true" ]; then
        echo "sandbox kept: $SANDBOX"
    else
        rm -rf "$SANDBOX" || true
    fi
    return "$rc"
}
trap cleanup EXIT
REAL_HOME="$HOME"

export SDDK_STATE_HOME="$SANDBOX/state"
export XDG_DATA_HOME="$SANDBOX/data"
export XDG_CACHE_HOME="$SANDBOX/cache"
export HOME="$SANDBOX/home"
export SDDK_ACTOR="uat-ctx-005"
mkdir -p "$SDDK_STATE_HOME" "$XDG_DATA_HOME" "$XDG_CACHE_HOME" "$HOME"

WORKTREE="$SANDBOX/worktree"
mkdir -p "$WORKTREE"
git -C "$REPO_ROOT" init -q "$WORKTREE"

SKILL="$REPO_ROOT/skills/sddk-cycle-resume/SKILL.md"
MCW="$REPO_ROOT/prompts/sddk/mcw.md"

FAILURES=0
step() { printf '\n=== %s ===\n' "$1"; }
fail() {
    printf 'FAIL: %s\n' "$1" >&2
    FAILURES=$((FAILURES + 1))
}
ok() { printf 'ok: %s\n' "$1"; }
assert_eq() {
    if [ "$2" = "$3" ]; then
        ok "$1: $3"
    else
        fail "$1: expected $2, got $3"
    fi
}
assert_contains() {
    if grep -qF "$2" "$3"; then
        ok "$1"
    else
        fail "$1: '$2' not found in $(basename "$3")"
    fi
}
assert_absent() {
    if grep -qF "$2" "$3"; then
        fail "$1: '$2' unexpectedly present in $(basename "$3")"
    else
        ok "$1"
    fi
}

jqf() { # jqf <json-file> <jq-filter>
    python3 -c '
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    data = json.load(handle)
print(eval("data" + sys.argv[2]))' "$1" "$2"
}

# Un bootstrap degradado responde 0 o 4 según compila o no capsule; 4 es
# `no_capsule_source` (INC-DEBT-042), una degradación honesta, no un fallo.
bootstrap() { # bootstrap <session> <out-json> [ciclo]
    local session="$1" out="$2" cycle="${3:-}"
    local rc
    set +e
    if [ -n "$cycle" ]; then
        "$BIN" context bootstrap --root "$WORKTREE" --session "$session" \
            --cycle "$cycle" --format json >"$out" 2>"$out.err"
    else
        "$BIN" context bootstrap --root "$WORKTREE" --session "$session" \
            --format json >"$out" 2>"$out.err"
    fi
    rc=$?
    set -e
    if [ "$rc" -ne 0 ] && [ "$rc" -ne 4 ]; then
        fail "bootstrap $session exited $rc (esperado 0 o 4): $(cat "$out.err")"
        exit 1
    fi
}

# ── 1. La superficie enseña, y no bloquea por "no discovery" ───────────────
# Los pines de texto viven en tests/test_workflow_contract.py; aquí se repite
# lo mínimo imprescindible porque esta UAT afirma la CONJUNCIÓN: lo que la
# skill enseña y lo que el runtime hace, en la misma corrida. Si un texto
# cambiara sin cambiar el runtime (o al revés), la conjunción se rompe y aquí
# se ve aunque el otro guard no se ejecutara.
step "la skill y el prompt enseñan la inferencia tipada, no un bloqueo"
for surface in "$SKILL" "$MCW"; do
    label="$(basename "$(dirname "$surface")")/$(basename "$surface")"
    assert_contains "$label: enseña NoActiveCycle" "NoActiveCycle" "$surface"
    assert_contains "$label: enseña AmbiguousCycle" "AmbiguousCycle" "$surface"
    assert_absent "$label: sin el token de descubrimiento obsoleto" \
        "runtime-active-cycle-discovery-unavailable" "$surface"
done
# El recovery de estado 0 tiene que estar escrito en la skill, porque es el
# que abajo se ejecuta de verdad contra el runtime.
assert_contains "la skill nombra cycle start como acción legal" "cycle start" "$SKILL"
assert_contains "la skill prohíbe adivinar la ambigüedad" "never guess" "$SKILL"

# ── 2. Estado 0: NoActiveCycle tipado, y su hint es ACCIONABLE ─────────────
step "estado 0: NoActiveCycle con hint de recuperación real"
CONVERGE="$SANDBOX/boot0.json"
bootstrap "uat-ctx005-0" "$CONVERGE"
PROJECT_ID="$(jqf "$CONVERGE" "['project_id']")"
assert_eq "adopción convergida" "complete" "$(jqf "$CONVERGE" "['adoption']")"
assert_eq "cero leases ⇒ no_active_cycle" "no_active_cycle" "$(jqf "$CONVERGE" "['cycle']['state']")"
ok "project_id=$PROJECT_ID"
# El hint debe ser ejecutable: la skill dice que la acción legal es arrancar
# ciclo, así que el hint tiene que decirlo con el comando real, no con una
# frase genérica.
assert_contains "el hint nombra el comando que lo recupera" "sddk cycle start" "$CONVERGE"
# Y el estado 0 no es un bloqueo: la sesión queda bindeada a nivel proyecto,
# que es lo que permite seguir trabajando.
assert_eq "binding a proyecto escrito (no es un bloqueo)" "True" "$(jqf "$CONVERGE" "['binding_written']")"

# ── 3. Recovery documentado del estado 0: arrancar ciclo ───────────────────
# Aquí está el corazón de la UAT: la acción que la skill enseña, ejecutada.
step "recovery del estado 0: la acción que la skill enseña realmente recupera"
CYCLE_ONE="ctx005-alpha"
if ! "$BIN" cycle start --root "$WORKTREE" --name "$CYCLE_ONE" --lease-owner owner-alpha \
    --format json >"$SANDBOX/start-alpha.json" 2>"$SANDBOX/start-alpha.err"; then
    fail "cycle start exited non-zero: $(cat "$SANDBOX/start-alpha.err")"
    exit 1
fi
PREFIXED_ONE="$(jqf "$SANDBOX/start-alpha.json" "['cycle_id']")"
ok "ciclo creado: $PREFIXED_ONE"

# ── 4. Estado 1: un lease resuelve sin pedir nada ───────────────────────────
step "estado 1: una lease resuelve el ciclo sin input humano"
BOOT1="$SANDBOX/boot1.json"
bootstrap "uat-ctx005-1" "$BOOT1"
assert_eq "una lease ⇒ resolved" "resolved" "$(jqf "$BOOT1" "['cycle']['state']")"
assert_eq "resuelve al ciclo con lease" "$PREFIXED_ONE" "$(jqf "$BOOT1" "['cycle']['cycle_id']")"
assert_eq "contexto compilado desde el ledger" "compiled" "$(jqf "$BOOT1" "['context_source']")"
assert_eq "status completo con capsule" "complete" "$(jqf "$BOOT1" "['status']")"
if [ "$(jqf "$BOOT1" "['capsule_id']")" = "None" ]; then
    fail "estado 1 sin capsule: el runtime no reconstruyó contexto"
else
    ok "capsule entregada en el estado 1"
fi

# ── 5. Estado N: ambigüedad tipada y accionable por el humano ──────────────
step "estado N: segunda lease ⇒ ambigüedad con candidates accionables"
CYCLE_TWO="ctx005-beta"
if ! "$BIN" cycle start --root "$WORKTREE" --name "$CYCLE_TWO" --lease-owner owner-beta \
    --format json >"$SANDBOX/start-beta.json" 2>"$SANDBOX/start-beta.err"; then
    fail "cycle start (segundo) exited non-zero: $(cat "$SANDBOX/start-beta.err")"
    exit 1
fi
PREFIXED_TWO="$(jqf "$SANDBOX/start-beta.json" "['cycle_id']")"
ok "segundo ciclo creado: $PREFIXED_TWO"

BOOTN="$SANDBOX/bootn.json"
bootstrap "uat-ctx005-n" "$BOOTN"
assert_eq "dos leases ⇒ ambiguous" "ambiguous" "$(jqf "$BOOTN" "['cycle']['state']")"
if python3 -c '
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    cycle = json.load(handle)["cycle"]
sys.exit(1 if "cycle_id" in cycle else 0)
' "$BOOTN"; then
    ok "ambiguous sin cycle_id (el agente no adivina)"
else
    fail "ambiguous state carries cycle_id — the runtime guessed"
fi
# La skill dice que los candidates se eligen por el humano: eso exige que cada
# candidate sea accionable, es decir, traiga identidad y quién la tiene viva.
# La lectura es tolerante a propósito (`get` con default): si un runtime
# mutado devolviera `resolved` en vez de `ambiguous`, la clave `candidates`
# no existiría y un `[]` directo mataría el script con `set -e` a media
# corrida, ocultando el resto de la evidencia. Un UAT que aborta no informa.
if python3 -c '
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    cycle = json.load(handle).get("cycle") or {}
candidates = cycle.get("candidates") or []
sys.exit(0 if len(candidates) == 2
         and all(c.get("cycle_id") and c.get("owner") and c.get("expires_at_ms")
                 for c in candidates)
         else 1)
' "$BOOTN"; then
    ok "2 candidates con cycle_id + owner + expires_at_ms"
else
    fail "candidates incompletos: la elección humana no sería posible"
fi

# ── 6. Recovery documentado del estado N: el humano elige y lo dice ────────
# La skill prohíbe adivinar y prescribe pasar el id. Se simula la elección
# humana TOMANDO UN candidate de la lista que el propio runtime devolvió (no
# uno compuesto a mano): si el candidate no fuera utilizable, esto fallaría.
step "recovery del estado N: elegir un candidate resuelve sin adivinar"
CHOSEN="$(python3 -c '
import json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    candidates = (json.load(handle).get("cycle") or {}).get("candidates") or []
print(candidates[0]["cycle_id"] if candidates else "")' "$BOOTN")"
if [ -z "$CHOSEN" ]; then
    fail "no hay ningún candidate que un humano pueda elegir: la salida de ambigüedad no es accionable"
    CHOSEN="$PREFIXED_ONE"  # sigue el recorrido para no abortar y dejar el resto del log
else
    ok "el humano elige: $CHOSEN"
fi
if [ "$CHOSEN" = "$PREFIXED_ONE" ] || [ "$CHOSEN" = "$PREFIXED_TWO" ]; then
    ok "la elección proviene de la lista de candidates, no de fuera"
else
    fail "el candidate elegido no pertenece a la lista de ambigüedad: $CHOSEN"
fi

BOOT_CHOSEN="$SANDBOX/boot-chosen.json"

# MEDIDO (session-84 bis 7, 6 ejecuciones: 1 verde, 5 con
# `expected compiled, got recovered`). La causa NO es un flake del runtime: es
# que `context_source` depende de SI el ciclo elegido tenia capsule, y el guion
# exigia `compiled` para un candidato que escoge a ciegas.
#
#   - `compiled`  = la capsule se reconstruyo ahora.
#   - `recovered` = ya habia capsule y se reuso por digest (read-reuse, que es
#                   una capacidad declarada del runtime).
#
# En este guion `PREFIXED_ONE` (alpha) compilo su capsule en el paso 4, asi que
# elegirlo da `recovered` SIEMPRE; `PREFIXED_TWO` (beta) nunca la compilo, asi
# que elegirlo da `compiled` SIEMPRE. Lo que varia entre corridas es el ORDEN de
# `candidates` (medido: `['beta','alpha']` una vez, `['alpha','beta']` dos), y
# por eso el mismo guion daba verde y rojo sin que cambiara nada.
#
# La version anterior ataba `compiled` al candidato que saliera primero, o sea
# a un valor que el runtime no controla: verde por azar cuando salia beta.
# Lo que se afirma ahora, y es mas fuerte, es la REGLA, en las dos ramas.
#
# ORDEN IMPORTA: la regla se mide ANTES de que el paso de la eleccion toque
# ningun ciclo. Si se midiera despues, elegir beta como `CHOSEN` le habria
# dado capsule y la rama "sin capsule" devolveria `recovered` — la propia
# comprobacion volveria a depender del orden de `candidates`.
if [ "$PREFIXED_TWO" != "$PREFIXED_ONE" ]; then
    BOOT_FRESH="$SANDBOX/boot-fresh.json"
    bootstrap "uat-ctx005-fresh" "$BOOT_FRESH" "$PREFIXED_TWO"
    assert_eq "elegir un ciclo sin capsule la compila" "compiled" \
        "$(jqf "$BOOT_FRESH" "['context_source']")"

    BOOT_REUSE="$SANDBOX/boot-reuse.json"
    bootstrap "uat-ctx005-reuse" "$BOOT_REUSE" "$PREFIXED_ONE"
    assert_eq "elegir un ciclo con capsule la reusa por digest" "recovered" \
        "$(jqf "$BOOT_REUSE" "['context_source']")"
else
    fail "los dos ciclos tienen el mismo id: la regla de las dos ramas no se puede ejercitar"
fi

bootstrap "uat-ctx005-chosen" "$BOOT_CHOSEN" "$CHOSEN"
assert_eq "el candidate elegido resuelve" "explicit" "$(jqf "$BOOT_CHOSEN" "['cycle']['state']")"
assert_eq "resuelve exactamente al elegido" "$CHOSEN" "$(jqf "$BOOT_CHOSEN" "['cycle']['cycle_id']")"
assert_eq "status completo tras la elección" "complete" "$(jqf "$BOOT_CHOSEN" "['status']")"
# Los dos candidatos ya tienen capsule tras la comprobacion de la regla, asi que
# aqui el valor correcto es `recovered` SIEMPRE. Fijarlo a `compiled` es
# justamente el defecto que se acababa de medir.
assert_eq "el candidato elegido reusa la capsule existente" "recovered" \
    "$(jqf "$BOOT_CHOSEN" "['context_source']")"

# ── 7. Legacy callers intactos: la escalera no rompió a quien ya resolvía ──
step "legacy callers: los comandos de la skill siguen resolviendo"
for cycle in "$PREFIXED_ONE" "$PREFIXED_TWO"; do
    set +e
    "$BIN" cycle status --root "$WORKTREE" --scope . --cycle "$cycle" --format json \
        >"$SANDBOX/status.json" 2>"$SANDBOX/status.err"
    rc=$?
    set -e
    assert_eq "cycle status --cycle $cycle" "0" "$rc"
    assert_eq "cycle status devuelve el ciclo pedido" "$cycle" "$(jqf "$SANDBOX/status.json" "['cycle_id']")"
    # El caller legacy que pasa id explícito recibe además la lease viva: es
    # el dato que la skill mete en `cli_context.lease`.
    if grep -qF '"owner"' "$SANDBOX/status.json"; then
        ok "la lease del ciclo explícito es observable ($cycle)"
    else
        fail "cycle status no expone la lease para $cycle"
    fi
done
# Y el camino de inferencia (sin --cycle) sigue siendo el de la skill: en
# estado N debe seguir degradando, no resolverse por la puerta de atrás.
BOOT_STILL_N="$SANDBOX/boot-still-n.json"
bootstrap "uat-ctx005-still-n" "$BOOT_STILL_N"
assert_eq "sin --cycle la ambigüedad persiste (el explícito no la contaminó)" \
    "ambiguous" "$(jqf "$BOOT_STILL_N" "['cycle']['state']")"

printf '\n'
if [ "$FAILURES" -eq 0 ]; then
    printf 'UAT CTX-UAT-005: PASS (todos los escenarios)\n'
    exit 0
fi
printf 'UAT CTX-UAT-005: FAIL (%s)\n' "$FAILURES" >&2
exit 1
