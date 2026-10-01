#!/usr/bin/env bash
# test_dev_install_source_guard.sh
#
# Guard: el checkout NUNCA vuelve a llevar un BUNDLE.toml que rompa
# `sddk dev install --source`.
#
# Por que existe (session-36, evidencia observada):
#   El checkout llevaba un BUNDLE.toml fosil (schema v2 pero version
#   1.145.1, commit d572547b, 2026-09-08, sin tocar desde entonces).
#   `sddk dev install --source .` lee ESE archivo (install.rs: pre-write
#   compat check + manifest anchor check) y fallo con
#   "binary 2.2.27 is not compatible with bundle 1.145.1". Los releases
#   publicados nunca lo sufren porque scripts/release.sh genera el
#   BUNDLE.toml fresco DENTRO del tarball en staging (step 5): el archivo
#   del checkout es vestigial para la via canonica y solo existe para
#   romper la via --source.
#
# Contrato:
#   1. Si BUNDLE.toml esta commiteado en el checkout, su version y su
#      rango binario deben incluir la version real del workspace
#      ([workspace.package] version de Cargo.toml). El consumidor
#      (--source install) compila contra esa version.
#   2. Si BUNDLE.toml declara manifest_sha256 (schema v2), debe matchear
#      el sha256 del MANIFEST.sha256 commiteado. Es el mismo fail-closed
#      que install.rs aplica en runtime (INC-DEBT-025 parte 2); aqui se
#      replica en CI para el arbol.
#   3. AUTORIZADA la alternativa "no existe": dev install --source exige
#      BUNDLE.toml (install.rs falla sin el), asi que el checkout debe
#      llevar uno COHERENTE, no ninguno. Un BUNDLE.toml ausente es un
#      fallo de este guard (debe regenerarse con sddk dev manifest
#      --bundle o scripts/release-bump.sh).
#
# Uso:  bash tests/test_dev_install_source_guard.sh
# Exit: 0 = coherente | 1 = drift
set -uo pipefail

cd "$(git rev-parse --show-toplevel 2>/dev/null)" || {
    echo "FATAL: no git toplevel" >&2
    exit 1
}

fail() { echo "  [FAIL] $*" >&2; exit 1; }
ok()   { echo "  [OK] $*" >&2; }

# ── 1. Version real del workspace ──────────────────────────────────────────
WORKSPACE_VERSION="$(sed -n '/^\[workspace\.package\]/,/^\[/p' Cargo.toml \
    | sed -n 's/^version *= *"\([^"]*\)".*/\1/p' | head -1)"
[ -n "$WORKSPACE_VERSION" ] || fail "no pude leer [workspace.package] version de Cargo.toml"

# ── 2. BUNDLE.toml debe existir en el checkout (autorizado por contrato) ───
[ -f BUNDLE.toml ] || fail "BUNDLE.toml no existe en el checkout: dev install --source exige uno coherente (regenerar con sddk dev manifest --bundle)"

# ── 3. Campos de version y rango binario ──────────────────────────────────
BVERSION="$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' BUNDLE.toml | head -1)"
BMIN="$(sed -n 's/^binary_min_version *= *"\([^"]*\)".*/\1/p' BUNDLE.toml | head -1)"
BMAX="$(sed -n 's/^binary_max_version *= *"\([^"]*\)".*/\1/p' BUNDLE.toml | head -1)"
[ -n "$BVERSION" ] || fail "BUNDLE.toml sin campo version"
[ -n "$BMIN" ] || fail "BUNDLE.toml sin binary_min_version"
[ -n "$BMAX" ] || fail "BUNDLE.toml sin binary_max_version"

ver_key() { printf '%s' "${1#v}" | awk -F. '{printf "%06d%06d%06d", $1, $2, $3}'; }
WK="$(ver_key "$WORKSPACE_VERSION")"
BK="$(ver_key "$BVERSION")"
MINK="$(ver_key "$BMIN")"
MAXK="$(ver_key "$BMAX")"

# Regla 1: la version del bundle (y su rango) deben incluir la del workspace.
[ "$BK" = "$WK" ] || fail "BUNDLE.toml version $BVERSION != workspace $WORKSPACE_VERSION (fosil: regenerar; asi rompio dev install --source en session-36)"
[ "$MINK" -le "$WK" ] 2>/dev/null || fail "binary_min_version $BMIN excluye al workspace $WORKSPACE_VERSION"
[ "$MAXK" -ge "$WK" ] 2>/dev/null || fail "binary_max_version $BMAX excluye al workspace $WORKSPACE_VERSION"
ok "rango binario [$BMIN, $BMAX] incluye workspace $WORKSPACE_VERSION"

# ── 4. Ancla del manifest (schema v2), replica de verify_manifest_anchor ───
if grep -q '^manifest_sha256' BUNDLE.toml; then
    DECLARED="$(sed -n 's/^manifest_sha256 *= *"\([^"]*\)".*/\1/p' BUNDLE.toml | head -1)"
    DECLARED="${DECLARED#sha256:}"
    [ -f MANIFEST.sha256 ] || fail "BUNDLE.toml declara manifest_sha256 pero MANIFEST.sha256 no existe"
    ACTUAL="$(sha256sum MANIFEST.sha256 | awk '{print $1}')"
    [ "$DECLARED" = "$ACTUAL" ] || fail "manifest_sha256 declarado ($DECLARED) != sha256 real de MANIFEST.sha256 ($ACTUAL) (INC-DEBT-025)"
    ok "manifest_sha256 ancla verificado"
fi

# ── 5. Contadores de superficie, contra el recuento real del manifest ──────
# The table is written out, not derived, for the reason INC-DEBT-052 measured:
# the field names do NOT correspond to the surface paths (`prompts/sddk` ->
# `prompts_count`), and deriving one from the other is precisely what produced
# `prompts_count = 0` — the counting match looked for a literal the surface
# list never contained, the arm never fired, and `_ => {}` swallowed it. A
# silent zero is worse than a missing one: nothing can tell it from a
# legitimately empty surface.
#
# OBSERVED in session-65h, on the committed BUNDLE.toml: it declared
# `skills_count = 244` where the manifest has 245, and did not declare
# `impeccable_reference_count` at all. The other four checks of this guard
# (version, range, anchor, schema) were blind to it: the file can be
# coherent in its header and lie in its body. That is the same class as the
# `prompts_count = 0` above, and it survived two reviews.
#
# Both directions are checked. A declared counter that disagrees with the
# manifest is a bundle that lies about its own contents; a surface the
# manifest covers with no counter is a count that can only be wrong.
SURFACE_TO_FIELD="agents:agents skills:skills prompts/sddk:prompts assets:assets specs:specs docs/impeccable-reference:impeccable_reference"

for PAIR in $SURFACE_TO_FIELD; do
    SURFACE="${PAIR%%:*}"
    FIELD="${PAIR##*:}"

    DECLARED="$(sed -n "s/^${FIELD}_count *= *\([0-9]*\).*/\1/p" BUNDLE.toml | head -1)"
    if [ -z "$DECLARED" ]; then
        fail "BUNDLE.toml no declara ${FIELD}_count (superficie '$SURFACE' cubierta por el manifest). Regenerar con sddk dev manifest --bundle"
    fi

    ACTUAL="$(grep -c "  ${SURFACE}/" MANIFEST.sha256 || true)"
    if [ "$DECLARED" != "$ACTUAL" ]; then
        fail "${FIELD}_count declara $DECLARED pero el manifest tiene $ACTUAL entradas en '$SURFACE' (BUNDLE.toml fosil: regenerar con sddk dev manifest --bundle)"
    fi
done
ok "los 6 contadores de superficie coinciden con MANIFEST.sha256"

# A surface missing from the table is a hole in the CHECK, not a pass: the loop
# above would simply not ask about it. Falsified exactly that way (removing
# `docs/impeccable-reference` from the table left the guard green while the
# counter was never verified). So the table is pinned to the surfaces the
# manifest actually covers, in both directions.
# The count is derived from MANIFEST_SURFACES in common.rs rather than
# restated here, because restating is the duplication that produced (a) and
# (b) in INC-DEBT-056. If a surface is added there, this fails until the line
# is written below.
DECLARED_SURFACES="$(sed -n '/MANIFEST_SURFACES: \[&str;/,/^]/p' crates/sddk-cli/src/dev/common.rs \
    | grep -oE '"[^"]+"' | tr -d '"' | sort)"
# Split the table into one surface per line. `printf '%s\n' "$VAR"` emits the
# WHOLE string as a single line, which is what made this check pass on a
# mangled table. `tr ' ' '\n'` splits explicitly, so there is no unquoted
# expansion to warn about (SC2086) and no way to read it as one line.
TABLE_SURFACES="$(printf '%s' "$SURFACE_TO_FIELD" | tr ' ' '\n' | cut -d: -f1 | sort)"

if [ "$DECLARED_SURFACES" != "$TABLE_SURFACES" ]; then
    MISSING="$(comm -23 <(printf '%s\n' "$DECLARED_SURFACES") <(printf '%s\n' "$TABLE_SURFACES") | tr '\n' ' ')"
    EXTRA="$(comm -13 <(printf '%s\n' "$DECLARED_SURFACES") <(printf '%s\n' "$TABLE_SURFACES") | tr '\n' ' ')"
    fail "esta tabla no cubre exactamente las superficies de MANIFEST_SURFACES. Sin verificar: ${MISSING:-ninguna}; sin superficie: ${EXTRA:-ninguna} — un contador no verificado es un contador que solo puede mentir"
fi
ok "la tabla de contadores cubre todas las superficies declaradas"

# ── 6. schema_version soportado ────────────────────────────────────────────
SCHEMA="$(sed -n 's/^schema_version *= *\([0-9]*\).*/\1/p' BUNDLE.toml | head -1)"
[ -n "$SCHEMA" ] || fail "BUNDLE.toml sin schema_version"
[ "$SCHEMA" -le 2 ] 2>/dev/null || fail "schema_version $SCHEMA > 2 (no soportado por el instalador)"
ok "schema_version = $SCHEMA"

echo "guard dev-install-source: coherente (bundle $BVERSION, workspace $WORKSPACE_VERSION)" >&2
exit 0
