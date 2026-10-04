#!/usr/bin/env bash
# Gate: the declared CHANGELOG section must COVER the commits it describes.
#
# Why this exists (session-61). `tests/test_changelog_merge.sh` proves the
# merge in `release-bump.sh` does not duplicate a version header. It says
# nothing about whether the section under that header *describes the work*.
# Those are different properties, and the gap was live: 26 commits sat
# between v2.4.2 and HEAD, including a whole `feat(architecture)` slice, while
# the `[2.5.0]` section listed only the two features present when the bump was
# committed. Had the release shipped, the published artifact's changelog would
# not have described its own contents.
#
# The same shape as the C3l.7 defect — an artifact that does not declare what
# it is — one layer down.
#
# Contract checked here:
#   (a) the section for the workspace version exists exactly once;
#   (b) every feat/fix commit since the last published tag appears in that
#       section, matched on a normalised subject (scope and punctuation are
#       allowed to drift, the payload is not);
#   (c) test(...) commits are represented too — a test that is the only
#       evidence for a fix is part of what shipped.
#
# Fail-closed: an unparseable changelog, a missing section, or an empty commit
# range each fail with a distinct message. Exit 0 only when coverage holds.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CHANGELOG="$ROOT/CHANGELOG.md"
PASS=0
FAIL=0

ok()   { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad()  { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

[[ -f "$CHANGELOG" ]] || { echo "FAIL: no CHANGELOG.md at $CHANGELOG"; exit 1; }

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
[[ -n "$VERSION" ]] || { echo "FAIL: cannot read workspace version from Cargo.toml"; exit 1; }

echo "=== changelog coverage gate (workspace $VERSION) ==="

# ── (a) the section exists exactly once ─────────────────────────────────────
HEADER="## [$VERSION]"
COUNT="$(grep -cF "$HEADER" "$CHANGELOG" || true)"
if [[ "$COUNT" -eq 1 ]]; then
  ok "exactly one '$HEADER' header (got $COUNT)"
else
  bad "expected exactly one '$HEADER' header, got $COUNT"
fi

# ── the section body: from its header to the next '## [' ────────────────────
SECTION="$(awk -v h="$HEADER" '
  index($0, h) == 1 { inb = 1; next }
  inb && /^## \[/ { exit }
  inb { print }
' "$CHANGELOG")"

if [[ -z "$(echo "$SECTION" | tr -d '[:space:]')" ]]; then
  bad "section '$HEADER' is empty — it declares nothing about this release"
else
  ok "section '$HEADER' has content"
fi

# ── (b)/(c) every feat/fix/test commit since the last published tag ─────────
#
# "Que version esta publicada" tiene DOS respuestas, y por eso INC-DEBT-070:
# `git ls-remote` lee el remoto y `git tag` lee el clon local. `gh release create`
# publica en el remoto y NADA del pipeline actualiza el clon, luego la lista local
# se queda vieja version tras version — MEDIDO en session-79: tras publicar
# v2.8.0 el clon seguia en v2.7.0, y este gate habria comparado `v2.7.0..HEAD`
# con 4 commits feat/fix/test YA publicados en 2.8.0. La forma facil de ponerlo
# verde era duplicarlos en la seccion siguiente, que es un changelog describiendo
# trabajo que ya salio.
#
# La autoridad es la MISMA que ya usa `release-bump.sh` y el gate 9b
# (`scripts/lib/release_admission.sh`), con sus tres resultados que no se pueden
# confundir, y los tres se tratan distinto a proposito:
#   remoto responde y hay tag  -> autoridad (el unico caso que decide)
#   remoto responde sin tags   -> bootstrap legitimo
#   remoto no responde         -> FAIL CERRADO. Degradar a la lista local aqui es
#                                 exactamente el defecto: "no veo el remoto" y "no
#                                 hay nada publicado" no pueden leerse igual.
LAST_PUB_REMOTE="${SDDK_RELEASE_ADMISSION_REMOTE:-origin}"
LAST_PUB_SOURCE=""
LAST_TAG=""

if git -C "$ROOT" remote | grep -qx "$LAST_PUB_REMOTE"; then
    ADMISSION_LIB="$ROOT/scripts/lib/release_admission.sh"
    if [ ! -f "$ADMISSION_LIB" ]; then
        bad "remote '$LAST_PUB_REMOTE' is configured but $ADMISSION_LIB is missing: there is no authority for the published version"
        echo
        echo "PASS=$PASS FAIL=$FAIL"
        echo "RESULT: FAIL -- the published version has no authority to read."
        exit 1
    fi
    # shellcheck source=scripts/lib/release_admission.sh
    # shellcheck disable=SC1091
    # SC1091: la ruta del source es una variable porque este gate se copia a
    # fixtures aislados donde $ROOT no es este repo, y el analisis estatico no
    # puede seguir un source dinamico. La existencia se comprueba justo arriba.
    . "$ADMISSION_LIB"
    if ! _last_published_resolve; then
        bad "cannot read published tags from '$LAST_PUB_REMOTE' ($LAST_PUB_OUTCOME): refusing to compare against a local tag list that may be stale"
        echo
        echo "PASS=$PASS FAIL=$FAIL"
        echo "RESULT: FAIL -- the remote did not answer, and a stale local list is not an answer."
        exit 1
    fi
    if [ "$LAST_PUB_OUTCOME" = "bootstrap" ]; then
        LAST_PUB_SOURCE="remote ($LAST_PUB_REMOTE: no v* tags yet)"
    else
        LAST_TAG="v${LAST_PUB_OUTCOME#v}"
        LAST_PUB_SOURCE="remote ($LAST_PUB_REMOTE)"
    fi
else
    # Sin remoto no puede haber un clon viejo RESPECTO de un remoto que no
    # existe: la lista local es la autoridad entera. Mismo criterio que
    # `release-bump.sh`, y por el mismo motivo.
    LAST_TAG="$(git -C "$ROOT" tag --sort=-version:refname | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | head -1 || true)"
    LAST_PUB_SOURCE="local (no '$LAST_PUB_REMOTE' remote configured)"
fi

if [ -n "$LAST_TAG" ]; then
    # El rango se calcula como "${LAST_TAG}..HEAD", luego la referencia tiene que
    # existir AQUI. Un tag publicado desde este clon esta en el remoto y falta
    # localmente hasta que se trae — el estado exacto que se midio al cerrar
    # 2.8.0. El commit al que apunta ya esta en este historial, luego traer la
    # referencia no introduce objetos nuevos. Si no se puede traer, el rango NO es
    # calculable, y adivinarlo fabricaria la lista de commits que decide el nivel.
    if ! git -C "$ROOT" rev-parse -q --verify "refs/tags/$LAST_TAG" >/dev/null; then
        echo "  resolving $LAST_TAG: published on '$LAST_PUB_REMOTE' but absent locally; fetching the ref"
        if ! GIT_TERMINAL_PROMPT=0 git -C "$ROOT" fetch -q --tags "$LAST_PUB_REMOTE" "refs/tags/$LAST_TAG:refs/tags/$LAST_TAG" 2>&1; then
            bad "cannot fetch '$LAST_TAG' from '$LAST_PUB_REMOTE': it is published but unavailable here, so the commits since the last release cannot be read. Refusing to guess."
            echo
            echo "PASS=$PASS FAIL=$FAIL"
            echo "RESULT: FAIL -- an uncomputable range is not a range."
            exit 1
        fi
    fi
fi

# De donde salio la linea base, en cada corrida. Un lector que no puede ver QUE
# authority respondio no puede distinguir un bootstrap limpio de un clon viejo,
# y esa es la distincion que decidio este arreglo.
echo "  (published-version authority: $LAST_PUB_SOURCE)"

if [[ -z "$LAST_TAG" ]]; then
  echo "  [skip] no published tag found; coverage cannot be checked"
  echo
  echo "PASS=$PASS FAIL=$FAIL"
  echo "RESULT: PASS (nothing to compare against)"
  exit 0
fi

echo "  (comparing against last published tag: $LAST_TAG)"

# ── La seccion congelada: el workspace YA es la version publicada ───────────
#
# MEDIDO al cerrar 2.8.0, y este caso lo destapo el arreglo de INC-DEBT-070: con
# la linea base vieja (clon en v2.7.0) este gate **pasaba por el motivo
# equivocado** —comparaba contra v2.7.0 y la seccion 2.8.0 si declaraba esos
# commits— mientras la seccion que de verdad se iba a publicar ya estaba
# congelada. Con la linea base correcta el caso es visible y es real.
#
# Si el workspace ES el tag publicado, su seccion salio: **no se puede ampliar**,
# y pedirle que declare commits posteriores es pedirle que mienta sobre un
# artefacto ya distribuido. Esos commits pertenecen a la release siguiente, que
# todavia no tiene version, luego no hay nada que comprobar aqui.
#
# Por que esto NO es una puerta trasera: dentro de `release.sh` este estado es
# imposible, porque el paso 0 (`release_admission_check_v2`) exige que la version
# del workspace en HEAD sea SEMVER MAYOR que el mayor tag publicado. Dos gates
# independientes, y ninguno de los dos miente: este declara que no comprueba
# nada y por que, y aquel se niega a publicar sin bumpear. Un gate que solo
# puede decir "no" es el que entrena a ignorar los rojos, y eso lo impone el
# propio gate en el caso del rango vacio (session-75).
WS_VER="$(sed -n 's/^version = "\([^"]*\)".*/\1/p' "$ROOT/Cargo.toml" | head -1)"
TAG_VER="${LAST_TAG#v}"
if [[ "$WS_VER" == "$TAG_VER" ]]; then
    PENDING="$(git -C "$ROOT" log --format=%s "$LAST_TAG"..HEAD \
        | grep -cE '^(feat|fix|test)' || true)"
    if [[ "$PENDING" -eq 0 ]]; then
        ok "nothing to ship: workspace ($WS_VER) is the published tag ($LAST_TAG) and the range is empty"
    else
        ok "nothing to compare: workspace ($WS_VER) is the published tag ($LAST_TAG) and $PENDING \
feat/fix/test commit(s) since it belong to a release that has no version yet"
    fi
    echo
    echo "PASS=$PASS FAIL=$FAIL"
    echo "RESULT: PASS (the section under test is already published and cannot be extended)"
    exit 0
fi

# Normalise a subject for comparison: lowercase, collapse whitespace.
norm() { tr '[:upper:]' '[:lower:]' | tr -s '[:space:]' ' '; }

MISSING=0
CHECKED=0
while IFS= read -r SUBJECT; do
  [[ -z "$SUBJECT" ]] && continue
  # Only the kinds a reader uses to learn what shipped. docs/chore are
  # deliberately excluded: they do not change behaviour and a changelog that
  # lists every pointer sync is noise.
  case "$SUBJECT" in
    feat*|fix*|test*) ;;
    *) continue ;;
  esac
  CHECKED=$((CHECKED + 1))
  KEY="${SUBJECT%%:*}"                 # type(scope)
  PAYLOAD_NORM="$(printf '%s' "${SUBJECT#*: }" | norm)"
  SECTION_NORM="$(printf '%s' "$SECTION" | norm)"
  if printf '%s' "$SECTION_NORM" | grep -qF "$KEY"; then
    # type present; require a distinctive slice of the payload too, so a bare
    # "feat(lease):" line cannot satisfy an unrelated feat.
    # Use the first 4 significant words of the payload as a fingerprint.
    FINGERPRINT="$(printf '%s' "$PAYLOAD_NORM" | cut -d' ' -f1-4)"
    if printf '%s' "$SECTION_NORM" | grep -qF "$FINGERPRINT"; then
      ok "covered: $SUBJECT"
    else
      bad "declared type '$KEY' present but not this commit: $SUBJECT"
      MISSING=$((MISSING + 1))
    fi
  else
    bad "missing from section '$HEADER': $SUBJECT"
    MISSING=$((MISSING + 1))
  fi
done < <(git -C "$ROOT" log --format=%s "$LAST_TAG"..HEAD)

if [[ "$CHECKED" -eq 0 ]]; then
  # Rango vacio: hay que distinguir dos cosas que se parecian y no lo son.
  #
  # Session-75: al publicar v2.5.5, el workspace queda en la MISMA version que el
  # ultimo tag, asi que `v2.5.5..HEAD` esta vacio y este gate se quedaba en rojo
  # permanente con "no commits to vouch for". Eso es un gate que solo puede decir
  # que no -- y un gate permanentemente rojo es exactamente la condicion que
  # entrena a ignorar los rojos, que es el defecto que este gate existe para
  # cazar, aplicado a si mismo.
  #
  # La distincion que decide: esta la version del workspace IGUAL a la del tag
  # publicado, o es ANTERIOR?
  #   igual     -> no hay nada que enviar. No es un fallo: es el estado limpio
  #               de un repo recien publicado. Se declara NOT_APPLICABLE y sale 0.
  #   anterior  -> el tip se ha retardado respecto a lo publicado. Eso SI es un
  #               fallo: la autoridad de version esta detras de la release, que
  #               es la razon por la que INC-DEBT-040 existe. Sigue siendo FAIL.
  WS_VER="$(sed -n 's/^version = "\([^"]*\)".*/\1/p' "$ROOT/Cargo.toml" | head -1)"
  TAG_VER="${LAST_TAG#v}"
  if [[ "$WS_VER" == "$TAG_VER" ]]; then
    ok "nothing to ship: workspace ($WS_VER) is the published tag ($LAST_TAG) and the range is empty"
  else
    bad "no feat/fix/test commits in range AND workspace ($WS_VER) != published tag ($TAG_VER): the version pointer is behind the release, which is the INC-DEBT-040 shape"
  fi
elif [[ "$MISSING" -eq 0 ]]; then
  ok "all $CHECKED feat/fix/test commits since $LAST_TAG are represented"
fi

echo
echo "PASS=$PASS FAIL=$FAIL"
if [[ "$FAIL" -eq 0 ]]; then
  echo "RESULT: PASS — the declared section describes the work it ships."
  exit 0
fi
echo "RESULT: FAIL — shipping this would publish a changelog that misdescribes the artifact."
exit 1
