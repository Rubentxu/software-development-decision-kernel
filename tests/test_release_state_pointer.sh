#!/usr/bin/env bash
# test_release_state_pointer.sh
#
# Guard: docs/roadmap/STATE.yaml no puede afirmar un HEAD que no es el HEAD.
#
# Por que existe (sesion-18, evidencia):
#   Sessions 16 y 17 movieron main siete commits (2a750b0..971e0ea) y
#   actualizaron CURRENT.md y SESSION-JOURNAL.md, pero NO tocaron
#   STATE.yaml. El puntero de autoridad se quedo en 5ce4bca / 2.0.1
#   mientras el repo ya iba en 971e0ea / 2.0.4. Tres sesiones de deriva,
#   y nadie lo noto porque nada lo contrastaba contra git.
#
#   Este test hace el contraste. Es el guard de mi propia deuda de proceso.
#
# Reglas de comparacion (deliberadamente estrictas):
#   - Compara el COMMIT COMPLETO, no el abreviado. Un prefijo de 7 hex
#    ambiguaaria en el universo; uno de 40 no.
#   - current_sha es un puntero a un commit YA PUBLICADO. Por eso se
#     compara contra el ultimo commit que contiene ese contenido, no
#     contra el HEAD de la rama de trabajo: si hay commits sin pushear
#     en el arbol, STATE.yaml describe correctamente el estado
#     publicado y el test debe pasar igual.
#   - Si el puntero no es alcanzable desde ningun ref, es drift puro y
#     falla.
#
# Uso:  bash tests/test_release_state_pointer.sh
# Exit: 0 = puntero coherente | 1 = drift
set -uo pipefail

cd "$(git rev-parse --show-toplevel 2>/dev/null)" || {
  echo "FATAL: no estoy dentro de un repo git" >&2
  exit 1
}

STATE="docs/roadmap/STATE.yaml"
CARGO="Cargo.toml"
rc=0

fail() { echo "  [FAIL] $*"; rc=1; }
ok()   { echo "  [ok]   $*"; }

echo "== guard: puntero de estado vs git =="

[ -f "$STATE" ] || { echo "FATAL: no existe $STATE" >&2; exit 1; }

# --- 1. current_sha es un commit real y alcanzable ------------------------
declare_sha=$(sed -n 's/^  current_sha: *"\([0-9a-f]\{7,40\}\)".*/\1/p' "$STATE" | head -1)

if [ -z "$declare_sha" ]; then
  fail "no se pudo extraer current_sha de $STATE"
elif ! git rev-parse --verify --quiet "$declare_sha^{commit}" >/dev/null; then
  fail "current_sha=$declare_sha no resuelve a ningun commit de este repo"
else
  full_sha=$(git rev-parse "$declare_sha^{commit}")
  ok "current_sha resuelve: ${full_sha:0:12} ($(git log -1 --format=%s "$full_sha" | cut -c1-50))"

  # --- 2. el commit existe en el historico PUBLADO (main u origin/main) ---
  if git merge-base --is-ancestor "$full_sha" origin/main 2>/dev/null; then
    ok "el puntero esta contenido en origin/main (alcanzable desde el trunk publicado)"
  else
    fail "current_sha=$full_sha NO esta en origin/main: el puntero afirma algo publicado que no lo esta"
  fi

  # --- 3. el puntero no apunta a un commit MAS ANTIGUO que el trunk -------
  # Si alguien bumpea el workspace y no reconcilia, el puntero queda
  # rezagado. Esto es exactamente lo que paso en sessions 16-17.
  trunk=$(git rev-parse origin/main)
  behind=$(git rev-list --count "$full_sha..$trunk")
  if [ "$behind" -eq 0 ]; then
    ok "el puntero es el ultimo commit publicado (0 commits de retraso)"
  else
    fail "el puntero va $behind commit(s) por DETRAS de origin/main: STATE.yaml esta desfasado"
  fi
fi

# --- 4. workspace_version_at_current == version real de Cargo.toml ---------
declared_ver=$(sed -n 's/^  workspace_version_at_current: *"\([^"]*\)".*/\1/p' "$STATE" | head -1)
real_ver=$(sed -n '/^\[workspace\.package\]/,/^\[/ s/^version *= *"\([^"]*\)".*/\1/p' "$CARGO" | head -1)

if [ -z "$declared_ver" ] || [ -z "$real_ver" ]; then
  fail "no pude comparar versiones (declared='$declared_ver' real='$real_ver')"
elif [ "$declared_ver" = "$real_ver" ]; then
  ok "workspace_version_at_current ($declared_ver) == Cargo.toml ($real_ver)"
else
  fail "workspace_version_at_current dice '$declared_ver' pero Cargo.toml dice '$real_ver'"
fi

# --- 5. manifest.toml no puede ir por detras de Cargo.toml -----------------
# Los tres digitos de version tienen que coincidir. Si divergen, el bundle
# publicado lleva una version distinta a la que cree el operator.
man_ver=$(sed -n 's/^version *= *"\([^"]*\)".*/\1/p' manifest.toml | head -1)
if [ "$man_ver" = "$real_ver" ]; then
  ok "manifest.toml ($man_ver) alineado con Cargo.toml"
else
  fail "manifest.toml dice '$man_ver', Cargo.toml dice '$real_ver'"
fi

echo
if [ "$rc" -eq 0 ]; then
  echo "RESULT: PASS — el puntero de estado describe el estado real."
else
  echo "RESULT: FAIL — STATE.yaml miente sobre el estado del repo."
  echo "         Reconciliar segun AGENTS.md §10.3: actualizar el puntero con el"
  echo "         SHA real y CONSERVAR la evidencia anterior sin reescribirla."
fi
exit "$rc"
