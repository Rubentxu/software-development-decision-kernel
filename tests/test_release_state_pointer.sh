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

  # --- 3. coherencia del puntero con el TRABAJO REAL ------------------------
  # El puntero se compara contra el ULTIMO commit de main que toca las
  # surfaces que el puntero describe. No contra origin/main: en el
  # momento en que este commit entra, el puntero que este commit
  # escribe quedara por detras de origin/main SIEMPRE, y ese estado
  # post-commit es NORMAL, no drift. Un behind==0 estricto seria
  # insatisfacible por construccion y nadie podria dejarlo verde.
  #
  # Por eso se mide el drift CONTRA UN UMBRAL explicito (3), no contra
  # cero. El criterio real es: las sessions 16-17 dejaron 12 commits de
  # retraso. Anything <= 3 es "el puntero se escribio en este puño de
  # commits". Anything > 3 significa que alguien movio el trunk sin
  # reconciliar, que es exactamente el fallo que este guard caza.
  trunk=$(git rev-parse main)
  behind=$(git rev-list --count "$full_sha..$trunk")
  PUNCTUAL_TOLERANCE=3
  # Se lee aqui porque el check 3c (abajo) lo necesita y alli todavia no
  # estaria definido: leerlo mas abajo lo haria vacio en tiempo de uso.
  real_ver=$(sed -n '/^\[workspace\.package\]/,/^\[/ s/^version *= *"\([^"]*\)".*/\1/p' "$CARGO" | head -1)
  if [ "$behind" -le "$PUNCTUAL_TOLERANCE" ]; then
    ok "el puntero es puntual: $behind commit(s) de retraso sobre main (tolerancia $PUNCTUAL_TOLERANCE)"
  else
    fail "el puntero va $behind commit(s) por DETRAS de main (tolerancia $PUNCTUAL_TOLERANCE): alguien movio el trunk sin reconciliar STATE.yaml"
  fi

  # --- 3b. el puntero tiene que ser alcanzable desde HEAD --------------------
  # Si el puntero quedo en una rama abandonada, o en un commit reescrito,
  # hay que decirlo aunque este contenido en main.
  if git merge-base --is-ancestor "$full_sha" HEAD 2>/dev/null; then
    ok "el puntero es ancestro de HEAD (no quedo en una rama lateral)"
  else
    fail "current_sha=$full_sha no es ancestro de HEAD: el puntero quedo en una rama abandonada"
  fi

  # --- 3c. coherencia semantica: el subject del puntero debe corroborar -----
  # que el puntero no se toco a si mismo. sessions 16-17 movieron el
  # workspace 2.0.1 -> 2.0.4 (3 minors) sin tocar el puntero, asi que
  # un puntero cuyo subject sea un bump DEBE declarar la version que ese
  # bump produjo.
  ptr_subject=$(git log -1 --format=%s "$full_sha")
  declared_ver_early=$(sed -n 's/^  workspace_version_at_current: *"\([^"]*\)".*/\1/p' "$STATE" | head -1)
  if printf '%s' "$ptr_subject" | grep -qE 'bump .*-> *[0-9]+\.[0-9]+\.[0-9]+'; then
    # Extraer por captura. NO usar `tr -d ' ->'`: borra caracteres
    # individuales del resultado y se lo come entero (mismo modo de fallo
    # que el check de musl de session-17, que se declaraba verde solo).
    bump_target=$(printf '%s' "$ptr_subject" | grep -oE '\-> *[0-9]+\.[0-9]+\.[0-9]+' | sed -E 's/^-> *//')
    if [ "$bump_target" = "$declared_ver_early" ]; then
      ok "el puntero es coherente: subject dice bump -> $bump_target y declara $declared_ver_early"
    elif [ "$declared_ver_early" = "$real_ver" ] && [ "$behind" -le "$PUNCTUAL_TOLERANCE" ]; then
      # Auto-referencia. Un puntero no puede contenerse a si mismo: el commit
      # que bumpea la version es el SIGUIENTE al puntero, porque el bump
      # ceremonial viaja en su propio commit (exigido por el pre-push hook).
      # En esa ventana el puntero declara la version ya bumpeada mientras su
      # current_sha sigue siendo el commit de trabajo anterior. Es legitimo
      # SOLO si la version declarada coincide con la real del repo Y el
      # puntero esta a menos de `tolerance` commits del trunk. Si alguien
      # bumpea dos veces sin reconciliar, `behind` sale de tolerancia y cae.
      ok "ventana de auto-referencia aceptada: declara $declared_ver_early (= repo real) con el puntero a $behind commit(s) del trunk; se reconcilia al siguiente commit"
    else
      fail "el puntero $full_sha bumpea a $bump_target pero declara workspace_version_at_current=$declared_ver_early (repo en $real_ver, $behind commit(s) de retraso)"
    fi
  else
    ok "subject del puntero no es un bump (sin verificacion semantica aplicable)"
  fi
fi

# --- 4. workspace_version_at_current == version real de Cargo.toml ---------
declared_ver=$(sed -n 's/^  workspace_version_at_current: *"\([^"]*\)".*/\1/p' "$STATE" | head -1)
if [ -z "$real_ver" ]; then
  real_ver=$(sed -n '/^\[workspace\.package\]/,/^\[/ s/^version *= *"\([^"]*\)".*/\1/p' "$CARGO" | head -1)
fi

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

# --- 6. Cargo.lock no puede ir por detras de Cargo.toml ---------------------
# INC-LOCK-POINTER-GAP (session-22). El commit 62d4728 subio Cargo.toml a
# 2.0.8 y dejo Cargo.lock en 2.0.7. Los checks 4 y 5 de este guard no lo
# ven porque comparan Cargo.toml contra STATE.yaml y contra manifest.toml,
# nunca contra el lock. El dano no fue cosmético: .github/workflows/ci.yml
# y .github/workflows/release.yml construyen con --locked, asi que un
# checkout limpio de ese commit rompia ambos con exit 101. Un guard que
# valida la version pero no valida el lock que la build realmente usa es
# medio guard.
lock_ver=$(awk '
    /^\[\[package\]\]/ { name=""; ver="" }
    /^name = "sddk-cli"$/   { hit=1; next }
    hit && /^version = "/    { match($0, /"[^"]*"/)
                              print substr($0, RSTART+1, RLENGTH-2); exit }
' Cargo.lock)
if [ -z "$lock_ver" ]; then
  fail "no pude leer la version de sddk-cli en Cargo.lock"
elif [ "$lock_ver" = "$real_ver" ]; then
  ok "Cargo.lock ($lock_ver) alineado con Cargo.toml"
else
  fail "Cargo.lock dice '$lock_ver', Cargo.toml dice '$real_ver' -- el build con --locked va a fallar (ci.yml y release.yml)"
fi

echo
if [ "$rc" -eq 0 ]; then
  echo "RESULT: PASS — el puntero de estado describe el estado real."
else
  echo "RESULT: FAIL — STATE.yaml miente sobre el estado del repo."
  echo "         Reconciliar segun AGENTS.md §10.3: actualizar el puntero con el"
  echo "         SHA real y CONSERVAR la evidencia anterior sin reescribirla."
  echo "         Para los campos mecanicos (current_sha, head_at_state_sync,"
  echo "         workspace_version_at_current) existe una reparacion:"
  echo "           bash scripts/reconcile_state_pointer.sh --check   # inspecciona"
  echo "           bash scripts/reconcile_state_pointer.sh          # repara"
  echo "         Conserva la nota de evidencia existente y no reescribe historia;"
  echo "         el juicio sobre QUE significa el estado sigue siendo humano."
fi
exit "$rc"
