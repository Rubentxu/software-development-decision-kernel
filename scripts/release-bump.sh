#!/usr/bin/env bash
# release-bump.sh — Compute and apply the next semver release bump.
#
# Usage:
#   bash scripts/release-bump.sh --dry-run   # print what would change (no writes)
#   bash scripts/release-bump.sh             # apply bump: versions + lock + CHANGELOG
#   bash scripts/release-bump.sh --force-version 1.1.0   # explicit version
#
# Bump rules from conventional commits since the last tag:
#   BREAKING CHANGE / <type>!  -> major
#   feat                       -> minor
#   fix|refactor|perf|docs|ci|chore|style|test|build -> patch
#   otherwise                  -> no release
#
# Updates: workspace Cargo.toml, manifest.toml, BUNDLE.toml (las TRES claves
# del rango), Cargo.lock (via cargo check), and CHANGELOG.md.
#
# Los 7 `crates/*/Cargo.toml` tambien entran en el bucle de abajo, pero MEDIDO:
# todos declaran `version.workspace = true`, luego el sed no encuentra ninguna
# linea `version = "..."` que cambiar y ninguno aparece en la lista de
# "changed files". Sigue estando en el bucle por si un crate deja de heredar, y
# por eso el informe NO los anuncia: una superficie que el resumen nombra y la
# accion no toca es la misma asimetria lista/accion que ya produjo dos
# defectos.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DRY_RUN=0
FORCE_VERSION=""

while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run) DRY_RUN=1; shift ;;
        --force-version) FORCE_VERSION="$2"; shift 2 ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
done

cd "$ROOT"

# --- Git state ---

# The last PUBLISHED version comes from the REMOTE, through the same
# authority the push admission and the public-release gate already use
# (`scripts/lib/release_admission.sh`, `git ls-remote --tags`).
#
# It used to come from `git tag` on the local clone. That is a SECOND answer
# to the same question, and it goes stale: `gh release create` publishes the
# tag on the remote and nothing in the pipeline updates the local clone.
# Session-77 measured it on this repo -- `v2.5.6` was published and present in
# `git ls-remote --tags origin` while `git tag` stopped at `v2.5.5` -- and the
# consequence was not a wrong number in a report. The script concluded
# "the workspace declares the pending release (2.5.6)" and would have handed
# `release.sh` a tag that was ALREADY PUBLISHED. Two answers to "what is out
# there", one of them a release ago; that is the same shape as the identity
# defect of session-76, one layer up.
#
# Three outcomes, and the caller cannot confuse them:
#
#   remote configured and answering   -> authoritative; the remote is used
#   no remote configured at all       -> the local list, which cannot be
#                                       stale about a remote that does not
#                                       exist (bootstrap, isolated fixture)
#   remote configured, not answering  -> FAIL CLOSED. "I cannot see the
#                                       remote" and "nothing is published"
#                                       must not read the same, because one
#                                       means a fresh release and the other
#                                       means a wrong one.
LAST_PUB_REMOTE="${SDDK_RELEASE_ADMISSION_REMOTE:-origin}"
LAST_PUB_SOURCE=""

if git remote | grep -qx "$LAST_PUB_REMOTE"; then
    ADMISSION_LIB="$ROOT/scripts/lib/release_admission.sh"
    if [ ! -f "$ADMISSION_LIB" ]; then
        echo "error: '$LAST_PUB_REMOTE' is configured but $ADMISSION_LIB is missing;" >&2
        echo "       without it there is no authority for the published version." >&2
        exit 1
    fi
    # shellcheck source=scripts/lib/release_admission.sh
    . "$ADMISSION_LIB"
    if ! _last_published_resolve; then
        echo "error: cannot read published tags from '$LAST_PUB_REMOTE' ($LAST_PUB_OUTCOME)" >&2
        echo "       refusing to derive a version from a local tag list that may be" >&2
        echo "       stale. Fetch the remote, or pass --force-version explicitly." >&2
        exit 1
    fi
    if [ "$LAST_PUB_OUTCOME" = "bootstrap" ]; then
        LAST_TAG=""
        LAST_PUB_SOURCE="remote ($LAST_PUB_REMOTE: no v* tags yet)"
    else
        LAST_TAG="v${LAST_PUB_OUTCOME#v}"
        LAST_PUB_SOURCE="remote ($LAST_PUB_REMOTE)"
    fi
else
    LAST_TAG="$(git tag --sort=-v:refname | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | head -1 || true)"
    LAST_PUB_SOURCE="local (no '$LAST_PUB_REMOTE' remote configured)"
fi

if [ -n "$LAST_TAG" ]; then
    # The range below is computed as "${LAST_TAG}..HEAD", so the ref has to
    # exist HERE. A tag published from this clone exists on the remote and is
    # absent locally until fetched -- the exact state session-77 hit. The
    # commit it points at is already in this history, so fetching the tag ref
    # brings no new objects. If it cannot be fetched, the range is not
    # computable and guessing at it would fabricate the commit list that
    # decides the SemVer level.
    if ! git rev-parse -q --verify "refs/tags/$LAST_TAG" >/dev/null; then
        echo "resolving the last published tag: $LAST_TAG is on '$LAST_PUB_REMOTE' but not locally; fetching it" >&2
        if ! GIT_TERMINAL_PROMPT=0 git fetch -q --tags "$LAST_PUB_REMOTE" "refs/tags/$LAST_TAG:refs/tags/$LAST_TAG" >&2; then
            echo "error: cannot fetch '$LAST_TAG' from '$LAST_PUB_REMOTE'." >&2
            echo "       '$LAST_TAG' is published but unavailable here, so the commits since" >&2
            echo "       the last release cannot be read. Refusing to guess the level." >&2
            exit 1
        fi
    fi
elif [ -z "$LAST_PUB_SOURCE" ]; then
    LAST_PUB_SOURCE="local (none)"
fi

CURRENT="${LAST_TAG#v}"

if [ -z "$LAST_TAG" ]; then
    echo "error: no published semver tag found ($LAST_PUB_SOURCE)" >&2
    echo "       with no baseline there is no commit range to derive a level from." >&2
    echo "       Pass --force-version to declare the release version explicitly." >&2
    exit 1
fi

# Where the baseline came from, printed on every run. Two answers to "what is
# published" is how session-77 nearly re-published a tag that already existed;
# a caller that cannot see which one answered cannot tell a fresh bootstrap
# from a stale clone.
echo "last published: $LAST_TAG ($LAST_PUB_SOURCE)" >&2

# The version the workspace is actually AT, which may be ahead of the last
# tag (a manual `--force-version` bump, or the ceremonial-release-pending
# state described in AGENTS.md §2.3).
#
# Deriving the next version from the TAG alone is wrong: it cannot see a
# manual bump, so a workspace at 2.1.1 with last tag v2.0.1 derived 2.1.0
# and the CI release branch would have REGRESSED the workspace to 2.1.0
# (session-28, reproduced in an isolated clone).
#
# But deriving max(workspace, tag) and then BUMPING it is also wrong, in the
# opposite direction: if the workspace is already ahead of the tag, the
# operator has declared the release version (AGENTS.md §2.3 — "el workspace
# version es puntero ceremonial del release ... declara la versión que va a
# aparecer como tag"). With workspace 2.2.0 and last tag v2.0.1, a minor
# commit derived v2.3.0, and CI's "Open release PR when a bump is pending"
# would have opened and auto-merged a PR taking 2.2.0 -> 2.3.0 — publishing
# a version the operator never asked for, and desynchronising workspace,
# tag and CHANGELOG (session-29).
#
# So: ahead of the tag means the workspace IS the pending release, and there
# is nothing to derive. Equal means derive from the tag as usual.
WORKSPACE_VERSION="$(grep -A1 '^\[workspace\.package\]' Cargo.toml | grep '^version' | sed -E 's/.*"([^"]+)".*/\1/')"

# semver_gt <a> <b> → 0 when a > b
semver_gt() {
    local a="$1" b="$2" i
    local -a av bv
    IFS='.' read -r -a av <<<"$a"
    IFS='.' read -r -a bv <<<"$b"
    for i in 0 1 2; do
        local ai bi
        ai="${av[$i]:-0}"
        bi="${bv[$i]:-0}"
        [[ "$ai" =~ ^[0-9]+$ ]] || ai=0
        [[ "$bi" =~ ^[0-9]+$ ]] || bi=0
        if ((10#$ai > 10#$bi)); then return 0; fi
        if ((10#$ai < 10#$bi)); then return 1; fi
    done
    return 1
}

if [ -n "$WORKSPACE_VERSION" ] && semver_gt "$WORKSPACE_VERSION" "$CURRENT"; then
    # The operator has already declared the release version by hand
    # (`--force-version`, or the committed ceremonial bump). The release IS
    # that version — re-deriving one on top would skip the declared release.
    BASE_VERSION="$WORKSPACE_VERSION"
    PENDING_RELEASE_IS_WORKSPACE=1
    echo "workspace ($WORKSPACE_VERSION) is ahead of last tag ($CURRENT); the workspace declares the pending release ($WORKSPACE_VERSION), no bump to derive"
else
    BASE_VERSION="$CURRENT"
    PENDING_RELEASE_IS_WORKSPACE=0
fi

COMMITS="$(git log --oneline --no-merges "${LAST_TAG}..HEAD" 2>/dev/null | grep -vE 'chore\(release\)' || true)"
if [ -z "$COMMITS" ]; then
    echo "no commits since $LAST_TAG — nothing to release"
    exit 0
fi

# --- Bump level ---

LEVEL="none"
if [ -n "$FORCE_VERSION" ]; then
    LEVEL="forced"
elif echo "$COMMITS" | grep -qiE 'breaking change|^[a-z]+!:' || git log --format=%B "${LAST_TAG}..HEAD" | grep -qiE 'breaking change'; then
    LEVEL="major"
elif echo "$COMMITS" | grep -qE '^[a-f0-9]+ feat'; then
    LEVEL="minor"
elif echo "$COMMITS" | grep -qE '^[a-f0-9]+ (fix|refactor|perf|docs|ci|chore|style|test|build)'; then
    LEVEL="patch"
fi

if [ -z "$FORCE_VERSION" ] && [ "$PENDING_RELEASE_IS_WORKSPACE" = "1" ]; then
    # The workspace already declares the release version. There is nothing to
    # derive — emitting a bump here would make CI publish a version the
    # operator never declared. `--force-version` bypasses this on purpose:
    # it is the explicit way to say "and actually make it this".
    echo "no bump to derive: the workspace already declares the pending release ($WORKSPACE_VERSION)"
    exit 0
fi

if [ "$LEVEL" = "none" ]; then
    echo "no release-worthy commits since $LAST_TAG"
    exit 0
fi

next_version() {
    local cur="$1" level="$2"
    local major minor patch
    IFS='.' read -r major minor patch <<<"$cur"
    case "$level" in
        major) echo "$((major + 1)).0.0" ;;
        minor) echo "$major.$((minor + 1)).0" ;;
        patch) echo "$major.$minor.$((patch + 1))" ;;
    esac
}

if [ "$LEVEL" = "forced" ]; then
    NEXT="$FORCE_VERSION"
else
    NEXT="$(next_version "$BASE_VERSION" "$LEVEL")"
fi
NEW_TAG="v$NEXT"

echo "release bump: $LAST_TAG -> $NEW_TAG ($LEVEL)"
echo "new tag: $NEW_TAG"
if [ "$DRY_RUN" = "1" ]; then
    echo "--- commits ---"
    echo "$COMMITS"
    echo "--- files to update ---"
    # Session-75: esta linea decia cinco superficies y el bloque de aplicacion
    # mueve seis -- `BUNDLE.toml` faltaba en el resumen, no en el codigo. Es la
    # misma asimetria lista/accion que produjo los dos ultimos defectos: el
    # informe de una herramienta tiene que decir lo que la herramienta hace,
    # porque es lo que uno lee antes de confiar en ella. El dry-run es la unica
    # occasion de verlo sin aplicar el bump.
    echo "  Cargo.toml (workspace) + manifest.toml + BUNDLE.toml (3 claves) + Cargo.lock + CHANGELOG.md"
    exit 0
fi

# --- Apply version bump ---

# Anchor the sed to the version currently in workspace Cargo.toml, not the
# last tag — workspace version may be ahead of the tag between releases
# (AGENTS.md §2.3 makes workspace == release tag MANDATORY, but historical
# commits kept the workspace 1+ patch ahead for testing). Without this,
# `s/^version = "$CURRENT"/.../` silently misses when workspace ≠ last tag.
# Reuse the WORKSPACE_VERSION already read above so the anchor and the
# derivation can never disagree.
for f in Cargo.toml crates/*/Cargo.toml; do
    sed -i "s/^version = \"$WORKSPACE_VERSION\"/version = \"$NEXT\"/" "$f"
done
# manifest.toml can drift from the tag version across manual bumps; set its
# single top-level `version = "…"` line unconditionally (`schema_version`
# starts with a different anchor and is never touched).
sed -i "s/^version = \"[^\"]*\"/version = \"$NEXT\"/" manifest.toml

# BUNDLE.toml lleva las TRES versiones del rango, y las tres tienen que moverse
# juntas. Este paso no es cosmetico: `tests/test_dev_install_source_guard.sh`
# (paso 1b) exige que la version del bundle sea la del workspace, y sin esto
# **toda release futura muere en 1b en el primer bump**, con un mensaje que
# habla de un fosil y no de la causa —que es que el bump se dejaba untracked
# file por delante sin avisar. Medido en session-70: el bump 2.5.3 -> 2.5.4
# dejo `BUNDLE.toml` en 2.5.3 y la release murio ahi.
#
# `schema_version` empieza por otro ancla y no se toca, igual que en
# manifest.toml: solo las claves del rango.
if [ -f BUNDLE.toml ]; then
    for bundle_key in version binary_min_version binary_max_version; do
        sed -i "s/^${bundle_key} = \"[^\"]*\"/${bundle_key} = \"$NEXT\"/" BUNDLE.toml
    done
    # El rango del bundle es [min, max] y ambos se fijan a la misma version.
    # Si alguien lo abriera a mano, `dev install --source`
    # rechazaria un binario que el release acaba de construir; se verifica aqui
    # en vez de dejarlo para el usuario.
    B_MIN="$(sed -n 's/^binary_min_version *= *"\([^"]*\)".*/\1/p' BUNDLE.toml | head -1)"
    B_MAX="$(sed -n 's/^binary_max_version *= *"\([^"]*\)".*/\1/p' BUNDLE.toml | head -1)"
    [ "$B_MIN" = "$NEXT" ] && [ "$B_MAX" = "$NEXT" ] \
        || { echo "BUNDLE.toml: el rango [$B_MIN, $B_MAX] no quedo en $NEXT" >&2; exit 1; }
fi

# Regenerate Cargo.lock from the bumped manifests.
cargo check --workspace --quiet 2>/dev/null || cargo check --workspace

# --- CHANGELOG ---

if [ ! -f CHANGELOG.md ]; then
    cat > CHANGELOG.md <<'EOF'
# Changelog

All notable changes to this project are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

EOF
fi

TODAY="$(date -u +%Y-%m-%d)"
ENTRY_FILE="$(mktemp)"
trap 'rm -f "$ENTRY_FILE"' EXIT
{
    echo "## [$NEXT] - $TODAY"
    echo
    features="$(echo "$COMMITS" | grep -E '^[a-f0-9]+ feat' || true)"
    fixes="$(echo "$COMMITS" | grep -E '^[a-f0-9]+ fix' || true)"
    other="$(echo "$COMMITS" | grep -vE '^[a-f0-9]+ (feat|fix)' || true)"
    if [ -n "$features" ]; then
        echo "### Features"
        echo "$features" | sed -E 's/^[a-f0-9]+ (feat)(\([^)]*\))?: /\1\2: /' | sed -E 's/^/  - /'
        echo
    fi
    if [ -n "$fixes" ]; then
        echo "### Fixes"
        echo "$fixes" | sed -E 's/^[a-f0-9]+ (fix)(\([^)]*\))?: /\1\2: /' | sed -E 's/^/  - /'
        echo
    fi
    if [ -n "$other" ]; then
        echo "### Other"
        echo "$other" | sed -E 's/^[a-f0-9]+ //' | sed -E 's/^/  - /'
        echo
    fi
} > "$ENTRY_FILE"

# Keep-a-Changelog ordering: newest first. Insert the new entry right after
# the header, before the first existing `## [` section.
#
# session-30 retrospectiva: insertar a ciegas creaba cabeceras duplicadas.
# `--force-version` existe precisamente para re-declarar una version que ya
# estaba declarada (asi se bumpeo 2.2.4 -> 2.2.5 cuando 2.2.4 nunca llego a
# publicarse), y al no comprobar si la seccion existia ya, un re-bump dejaba
# dos cabeceras `## [2.2.0]` identicas (observado: separadas por 53 lineas).
# Una cabecera duplicada hace ambiguo cual entrada manda.
if [ -n "$(grep -nE "^## \[$NEXT\]" CHANGELOG.md 2>/dev/null | head -1)" ]; then
    # La seccion de $NEXT ya existe: se AÑADEN sus items al final de la
    # seccion existente, sin crear una segunda cabecera. El bloque va desde su
    # cabecera hasta justo antes de la siguiente cabecera `## [` (o EOF).
    EXIST_LINE="$(grep -nE "^## \[$NEXT\]" CHANGELOG.md | head -1 | cut -d: -f1)"
    AFTER_LINE="$(grep -nE '^## \[' CHANGELOG.md | awk -F: -v s="$EXIST_LINE" '$1 > s {print $1; exit}')"
    TOTAL="$(wc -l < CHANGELOG.md)"
    [ -z "$AFTER_LINE" ] && AFTER_LINE="$((TOTAL + 1))"
    TMP_MERGE="$(mktemp)"
    {
        # Todo hasta el final de la seccion existente, con las lineas en blanco
        # finales recortadas para poder anadir sin acumular huecos.
        head -n "$((AFTER_LINE - 1))" CHANGELOG.md | sed -e :a -e '/^\n*$/{$d;N;ba' -e '}'
        # Los items nuevos, ya tal cual los genera $ENTRY_FILE (con su
        # indentacion), pero sin repetir la cabecera `## [$NEXT]`.
        echo
        tail -n +2 "$ENTRY_FILE"
        echo
        [ "$AFTER_LINE" -le "$TOTAL" ] && tail -n "+$AFTER_LINE" CHANGELOG.md
    } > "$TMP_MERGE" && mv "$TMP_MERGE" CHANGELOG.md
    echo "  CHANGELOG.md: merged into the existing '## [$NEXT]' section (line $EXIST_LINE)"
elif grep -qE '^## \[' CHANGELOG.md; then
    FIRST="$(grep -n -m1 '^## \[' CHANGELOG.md | cut -d: -f1)"
    {
        head -n "$((FIRST - 1))" CHANGELOG.md
        cat "$ENTRY_FILE"
        tail -n "+$FIRST" CHANGELOG.md
    } > CHANGELOG.md.new && mv CHANGELOG.md.new CHANGELOG.md
else
    cat "$ENTRY_FILE" >> CHANGELOG.md
fi

echo "applied: $CURRENT -> $NEXT"
echo "changed files:"
git status --short | head -20
