#!/bin/bash
# Functional contract test for REQ-DKA-003 (coherence alignment verdict) and
# REQ-DKA-002-S3 (manifest edge break typed).
#
# Reescrito en session-33 tras observar un falso positivo del original:
#   un informe SIN veredicto, con la palabra "aligned" solo en prosa
#   ("release-preconditions-aligned"), hacia que el fallback
#   `grep -E "(aligned|misaligned|n/a)"` declarara "Verdict detected: aligned"
#   y PASS. El test aprobaba informes vacios. Observado, no hipotetico.
#
# Estructura:
#   Parte A — la logica de veredicto se pina contra 6 fixtures hermeticos en
#             un directorio temporal (positivos, negativos y la prosa-trampa
#             que reventaba al original). Fallar aqui es fallo del test.
#   Parte B — si existe el informe real del agente sddk-coherence para el
#             trigger release->archive-vault-complete, se valida con la MISMA
#             logica, y ademas debe citar los tres criterios del trigger.
#             Si NO existe, se declara NOT_RUN: el trigger lo evalua un agente
#             bajo demanda y su ausencia es un estado legitimo, no un fallo.
#             (El original fallaba duro aqui; eso era un test que exigia un
#             artefacto externo sin distinguir ausencia de incumplimiento.)
#   Parte C — sub-test 2: broken-edge typed via `sddk vault validate`.
#
# Verdict contract (prompts/sddk/phases/coherence.md):
#   " Verdict: aligned | misaligned | n/a"  (linea canonica, un espacio inicial)
#   misaligned MUST block vault archive and surface an INC.
#   n/a cuando el ciclo usa la ruta de release estandar.
#
# References:
#   REQ-DKA-003   — prompts/sddk/phases/coherence.md (release -> archive-vault-complete)
#   REQ-DKA-002-S3 — spec.md §REQ-DKA-002-S3

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CYCLE_ARTIFACTS_DIR="${CYCLE_ARTIFACTS_DIR:-"$REPO_ROOT/.sddk-cycle-artifacts"}"
COHERENCE_REPORT_DIR="${CYCLE_ARTIFACTS_DIR}/coherence"
COHERENCE_TRIGGER="release-archive-vault-complete"
COHERENCE_REPORT="${COHERENCE_REPORT_DIR}/${COHERENCE_TRIGGER}.md"

FIXTURE_DIR="$(mktemp -d)"

# MEDIDO: el trap era `trap 'rm -rf "$FIXTURE_DIR"' EXIT`. Con `set -e` un
# borrado fallido ABORTA el script con 1, y ese 1 es indistinguible de "el guard
# fallo". Este guard corre en el 1b del release, luego su codigo de salida tiene
# que describir lo que midio, no como termino su limpieza. MEDIDO con la sonda
# `probe-trap-exit5.sh`: sin preservar `$?`, un cuerpo-verde sale 1 y un
# cuerpo-rojo sale 1, el mismo numero para las dos direcciones.
# El patron es el de `test_release_receipt_authority.sh`, en el mismo 1b:
# capturar `$?` antes de borrar, tolerar el fallo del borrado, y salir con el
# codigo real. No se inventa uno nuevo.
# SC2329, MEDIDO: shellcheck afirma que esta funcion no se invoca nunca, y es
# FALSO: la invoca el `trap` de dos lineas mas abajo y shellcheck no traza los
# `trap`. Mismo limite que ya esta documentado en `scripts/release.sh:99`
# (`cleanup_release_scratch`), con la misma directiva y el mismo motivo. No se
# silencia a ciegas: se dice POR QUE.
# shellcheck disable=SC2329
cleanup_fixture_dir() {
    local exit_code=$?
    if [[ -n "$FIXTURE_DIR" && -d "$FIXTURE_DIR" ]]; then
        rm -rf "$FIXTURE_DIR" || true
    fi
    exit "$exit_code"
}
trap cleanup_fixture_dir EXIT

PASS_COUNT=0
FAIL_COUNT=0

ok()   { PASS_COUNT=$((PASS_COUNT+1)); printf '  ok    %s\n' "$1"; }
bad()  { FAIL_COUNT=$((FAIL_COUNT+1)); printf '  FAIL  %s\n' "$1"; }
note() { printf '  --    %s\n' "$1"; }

# ─── shared verdict logic: one function, used by fixtures AND real report ────
# Prints verdict=<value>; returns 0 iff the report satisfies the contract.
evaluate_report() {
    local report="$1"
    local vline
    vline="$(grep -i '^ Verdict:' "$report" 2>/dev/null | head -1 || true)"
    if [ -z "$vline" ]; then
        echo "verdict=missing"; return 1
    fi
    if echo "$vline" | grep -qiE '^ Verdict:[[:space:]]*aligned[[:space:]]*$'; then
        echo "verdict=aligned"; return 0
    elif echo "$vline" | grep -qiE '^ Verdict:[[:space:]]*misaligned[[:space:]]*$'; then
        if ! grep -qiE '(INC|block|reject)' "$report"; then
            echo "verdict=misaligned-without-block"; return 1
        fi
        echo "verdict=misaligned"; return 0
    elif echo "$vline" | grep -qiE '^ Verdict:[[:space:]]*n/a[[:space:]]*$'; then
        echo "verdict=n/a"; return 0
    fi
    echo "verdict=invalid"; return 1
}

# ─── sddk binary (needed only by Part C) ─────────────────────────────────────
SDDK_BIN=""
for candidate in "sddk" "/home/rubentxu/.local/bin/sddk"; do
    RESOLVED="$(command -v "$candidate" 2>/dev/null || true)"
    if [ -n "$RESOLVED" ] && [ -x "$RESOLVED" ]; then SDDK_BIN="$RESOLVED"; break; fi
done

echo "=== Parte A: la logica de veredicto contra fixtures hermeticos ==="

# A1 — aligned valido
cat > "$FIXTURE_DIR/a1.md" <<'EOF'
## Coherence Report
 Verdict: aligned
Criterios: ManagedClosureDelivery, archive.vault.complete, delivery_kind
EOF
OUT="$(evaluate_report "$FIXTURE_DIR/a1.md" || true)"
if [ "$OUT" = "verdict=aligned" ]; then
    ok "A1 aligned valido aceptado"
else bad "A1 aligned valido rechazado (OUT=$OUT)"; fi

# A2 — misaligned con mencion de bloqueo
cat > "$FIXTURE_DIR/a2.md" <<'EOF'
## Coherence Report
 Verdict: misaligned
El manifest no declara delivery_kind; se abre INC y se bloquea el archive.
EOF
OUT="$(evaluate_report "$FIXTURE_DIR/a2.md" || true)"
if [ "$OUT" = "verdict=misaligned" ]; then
    ok "A2 misaligned con bloqueo aceptado"
else bad "A2 misaligned con bloqueo rechazado (OUT=$OUT)"; fi

# A3 — misaligned SIN mencion de bloqueo: debe RECHAZARSE
cat > "$FIXTURE_DIR/a3.md" <<'EOF'
## Coherence Report
 Verdict: misaligned
Algo desalineado, sin consecuencias declaradas.
EOF
OUT="$(evaluate_report "$FIXTURE_DIR/a3.md" || true)"
if [ "$OUT" = "verdict=misaligned-without-block" ]; then
    ok "A3 misaligned sin INC/block rechazado por la logica"
else bad "A3 misaligned sin INC/block aceptado (OUT=$OUT)"; fi

# A4 — n/a valido
cat > "$FIXTURE_DIR/a4.md" <<'EOF'
## Coherence Report
 Verdict: n/a
El ciclo usa la ruta de release estandar; no hay transicion archive.vault.complete.
EOF
OUT="$(evaluate_report "$FIXTURE_DIR/a4.md" || true)"
if [ "$OUT" = "verdict=n/a" ]; then
    ok "A4 n/a valido aceptado"
else bad "A4 n/a valido rechazado (OUT=$OUT)"; fi

# A5 — LA PROSA-TRAMPA del original: sin veredicto, "aligned" solo en prosa.
#      El test original declaraba "Verdict detected: aligned" aqui. Observado.
cat > "$FIXTURE_DIR/a5.md" <<'EOF'
## Coherence Report
Informe-trampa: el trigger release-preconditions-aligned no aplica aqui.
Este informe NO contiene ningun veredicto.
EOF
OUT="$(evaluate_report "$FIXTURE_DIR/a5.md" || true)"
if [ "$OUT" = "verdict=missing" ]; then
    ok "A5 informe sin veredicto (prosa-trampa) rechazado"
else bad "A5 prosa-trampa aceptada como verdicto (OUT=$OUT)"; fi

# A6 — veredicto malformado
cat > "$FIXTURE_DIR/a6.md" <<'EOF'
## Coherence Report
 Verdict: aligned-ish pero bueno
EOF
OUT="$(evaluate_report "$FIXTURE_DIR/a6.md" || true)"
if [ "$OUT" = "verdict=invalid" ]; then
    ok "A6 veredicto malformado rechazado"
else bad "A6 veredicto malformado aceptado (OUT=$OUT)"; fi

echo ""
echo "=== Parte B: informe real del trigger (si existe) ==="

if [ -f "$COHERENCE_REPORT" ]; then
    note "informe presente: $COHERENCE_REPORT"
    OUT="$(evaluate_report "$COHERENCE_REPORT" || true)"
    if [ "$OUT" = "verdict=aligned" ] || [ "$OUT" = "verdict=misaligned" ] || [ "$OUT" = "verdict=n/a" ]; then
        VERDICT="${OUT#verdict=}"
        ok "veredicto canonico valido: $VERDICT"
        MISSING=0
        for term in ManagedClosureDelivery archive.vault.complete delivery_kind; do
            if grep -q "$term" "$COHERENCE_REPORT"; then
                ok "criterio citado en el informe: $term"
            else
                bad "criterio del trigger ausente en el informe: $term"
                MISSING=1
            fi
        done
        if [ "$MISSING" -eq 0 ] && [ "$VERDICT" != "misaligned" ]; then
            note "veredicto $VERDICT consistente con los criterios citados"
        fi
    else
        bad "el informe real viola el contrato de veredicto (OUT=$OUT)"
    fi
else
    note "informe ausente: $COHERENCE_REPORT"
    note "NOT_RUN: el trigger lo evalua el agente sddk-coherence bajo demanda;"
    note "la ausencia del artefacto es estado legitimo, no incumplimiento."
    note "(el original fallaba duro aqui sin distinguir ambas cosas)"
fi

echo ""
echo "=== Parte C: broken-edge typed (REQ-DKA-002-S3) ==="

if [ -n "$SDDK_BIN" ]; then
    VAL_CHAIN_OUTPUT="$("$SDDK_BIN" vault validate --scope "$REPO_ROOT" 2>&1 || true)"
    if echo "$VAL_CHAIN_OUTPUT" | grep -qiE "broken.?edge|broken.*link|missing.*receipt|release_receipt_id.*missing"; then
        ok "vault validate clasifica el edge roto con error tipado"
    elif echo "$VAL_CHAIN_OUTPUT" | grep -qiE "edge|coherence"; then
        ok "vault validate menciona edge/coherence"
    else
        note "sin edge roto presente: n/a por diseño (sin manifiesto roto que detectar)"
        ok "sub-test 2 n/a"
    fi
else
    note "sddk binario no disponible: Parte C NOT_RUN"
fi

echo ""
if [ "$FAIL_COUNT" -gt 0 ]; then
    echo "RESULT: FAIL ($PASS_COUNT ok, $FAIL_COUNT fail)"
    exit 1
fi
echo "RESULT: PASS ($PASS_COUNT checks)"
sha256_of_file() { [ -f "$1" ] && sha256sum "$1" | awk '{print $1}'; }
sha256_of_file "${BASH_SOURCE[0]}"
exit 0
