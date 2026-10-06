#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Falsifica tests/test_provider_neutral_provenance.sh con mutaciones al SOURCE REAL.
#
# Un guard que nunca ha sido contradicho no es un guard: es prosa con codigo de
# salida. Cada mutacion reintroduce el defecto de una forma distinta y el guard
# tiene que CAER.
#
# TRES categorias, y son distintas entre si:
#
#   DETECTADA  el guard cae, y cae POR LA COMPROBACION QUE SE ESPERABA. Importa
#              el "por": un guard que cae por el motivo equivocado esta tan
#              ciego como uno que no cae, solo que de otra forma.
#   MAL MOTIVO el guard cae, pero por otra comprobacion. No se cuenta como
#              deteccion: cuenta como el defecto que es, que es una comprobacion
#              que se solapa con otra y por eso no se puede atribuir.
#   SOBREVIVIDA el guard da verde con el defecto presente.
#
# Y la cuarta categoria, que no es resultado sino instrumento:
#
#   SKIP       la mutacion NO SE APLICO. Nunca se cuenta como PASS ni como
#              DETECTADA. Una falsacion que no llega a mutar no ha falsado nada,
#              y contarla como exito es el modo mas comodo de publicar un guard
#              que no mide lo que dice.
#
# El guard lee texto y no compila, luego el sandbox es minimo: solo los ficheros
# que el guard mira. No se copia el repo entero ni se invoca cargo.
# ---------------------------------------------------------------------------
set -uo pipefail

RAIZ="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
AQUI="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TRAB="$(mktemp -d)"
trap '"$HOME/.minimax/bin/mavis-trash" -- "$TRAB"' EXIT

CMD_REL=crates/sddk-cli/src/verify_kernel_cmd.rs
ADP_REL=crates/sddk-engine/src/code_intelligence_port_mcp.rs
PUERTO_REL=crates/sddk-engine/src/code_intelligence_port.rs
GUARD_REL=tests/test_provider_neutral_provenance.sh

DETECTADAS=0; MAL_MOTIVO=0; SOBREVIVIDAS=0; SKIPS=0; TOTAL=0

# Crea un sandbox minimo con exactamente la forma que el guard espera.
sandbox() { # $1 = nombre del sandbox
    local d="$TRAB/$1"
    mkdir -p "$d/tests" "$d/$(dirname "$CMD_REL")" "$d/$(dirname "$ADP_REL")" \
             "$d/$(dirname "$PUERTO_REL")"
    cp "$RAIZ/$GUARD_REL"       "$d/$GUARD_REL"
    cp "$RAIZ/$CMD_REL"         "$d/$CMD_REL"
    cp "$RAIZ/$ADP_REL"         "$d/$ADP_REL"
    cp "$RAIZ/$PUERTO_REL"      "$d/$PUERTO_REL"
    printf '%s' "$d"
}

# $1 sandbox  $2 fichero relativo  $3 viejo  $4 nuevo  $5 check esperado
mutar() {
    local sb="$1" rel="$2" old="$3" new="$4" esperado="$5"
    TOTAL=$((TOTAL + 1))
    local rc out
    python3 "$AQUI/aplicar.py" "$sb/$rel" "$old" "$new"
    rc=$?
    if [ "$rc" -ne 0 ]; then
        printf '  SKIP       %-52s -> la mutacion no se aplico (aplicar.py rc=%d)\n' "$esperado" "$rc"
        SKIPS=$((SKIPS + 1))
        return
    fi
    out="$(cd "$sb" && bash "$GUARD_REL" 2>&1)"
    rc=$?
    if [ "$rc" -eq 0 ]; then
        printf '  SOBREVIVIDA %-52s -> el guard dio verde con el defecto presente\n' "$esperado"
        SOBREVIVIDAS=$((SOBREVIVIDAS + 1))
    elif printf '%s' "$out" | grep -qF "$esperado"; then
        printf '  DETECTADA  %-52s -> cayo por su propia comprobacion\n' "$esperado"
        DETECTADAS=$((DETECTADAS + 1))
    else
        printf '  MAL MOTIVO %-52s -> cayo, pero por otra comprobacion:\n' "$esperado"
        printf '%s\n' "$out" | grep '\[FAIL\]' | sed 's/^/                 /'
        MAL_MOTIVO=$((MAL_MOTIVO + 1))
    fi
}

printf '== Falsificacion del guard de C3m.3 ==\n\n'

printf 'Control: sin mutaciones el guard DEBE pasar\n'
CTRL="$(sandbox ctrl)"
if out="$(cd "$CTRL" && bash "$GUARD_REL" 2>&1)" && printf '%s' "$out" | grep -q 'PASS='; then
    printf '  control OK\n\n'
else
    printf '  FALLO: el guard ya falla en el estado base; la falsificacion no significa nada\n'
    printf '%s\n' "$out"
    exit 1
fi

# --- A: el runtime fabrica el build ------------------------------------------
mutar "$(sandbox m1)" "$CMD_REL" \
    '    let basis = provider.basis(&subject_tag);' \
    '    let basis = sddk_engine::code_intelligence_port::AnalysisBasis { provider_build: "x".to_string(), protocol_major: 2025, protocol_minor: 3, capability_snapshot: provider.capabilities(), analyzer_set_digest: provider.capabilities().analyzer_set_digest, source_revision: "m".to_string(), request_scope: subject_tag.clone() };' \
    'A el runtime construye un AnalysisBasis a mano'

mutar "$(sandbox m2)" "$PUERTO_REL" \
    'pub enum ProviderKind {' \
    'pub enum ProviderKind {
    const _LEGACY_BUILD_CLAIM: &str = "cognicode-mcp/verify-cmd";' \
    'A el literal de build no medido reaparece en otro modulo de src/'

# --- B: el adapter deja de derivar de lo negociado ---------------------------
mutar "$(sandbox m3)" "$ADP_REL" \
    '            provider_build: format!("cognicode-mcp/{}", self.lock_state().server_version.clone()),' \
    '            provider_build: "cognicode-mcp".to_string(),' \
    'B provider_build no se deriva de server_version'

mutar "$(sandbox m4)" "$ADP_REL" \
    '        self.lock_state().server_version = reply["result"]["serverInfo"]["version"]' \
    '        self.lock_state().server_version = "0.0.0".to_string(); let _ = reply;' \
    'B server_version no viene del handshake'

mutar "$(sandbox m5)" "$ADP_REL" \
    '    pub fn basis(&self, scope: &str) -> AnalysisBasis {' \
    '    fn basis(&self, scope: &str) -> AnalysisBasis {' \
    'B basis() sigue privado'

# --- C: el valor se concatena, o solo parece concatenarse --------------------
mutar "$(sandbox m6)" "$ADP_REL" \
    'format!("cognicode-mcp/{}", self.lock_state().server_version.clone())' \
    'format!("cognicode-mcp/{}", self.lock_state().server_version.clone().len())' \
    'C provider_build no concatena el server_version de forma reconocible'

# --- D: la identidad observada deja de ser load-bearing ----------------------
mutar "$(sandbox m7)" "$ADP_REL" \
    '    let mut material = basis.provider_build.clone();' \
    '    let mut material = String::new();' \
    'D1 el digest de evidencia ya no depende de provider_build'

mutar "$(sandbox m8)" "$CMD_REL" \
    '        let input_digest = result.digest.0.clone();' \
    '        let input_digest = String::new();' \
    'D2 el CLI no convierte result.digest en la base de la observacion'

printf '\n----------------------------------------\n'
printf 'DETECTADAS=%d  MAL_MOTIVO=%d  SOBREVIVIDAS=%d  SKIP=%d  (de %d)\n' \
    "$DETECTADAS" "$MAL_MOTIVO" "$SOBREVIVIDAS" "$SKIPS" "$TOTAL"

if [ "$SKIPS" -ne 0 ]; then
    printf 'RESULT: FAIL — %d mutaciones no llegaron a aplicarse; una falsacion muerta no es una falsacion.\n' "$SKIPS"
    exit 1
fi
if [ "$MAL_MOTIVO" -ne 0 ] || [ "$SOBREVIVIDAS" -ne 0 ]; then
    printf 'RESULT: FAIL — el guard tiene comprobaciones que no pueden atribuir su caida.\n'
    exit 1
fi
printf 'RESULT: PASS — %d de %d mutaciones detectadas, cada una por su propia comprobacion.\n' \
    "$DETECTADAS" "$TOTAL"