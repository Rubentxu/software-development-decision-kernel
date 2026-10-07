#!/bin/bash
# Contract test for scripts/release-bump.sh version DERIVATION.
#
# Session-28 defect: `next_version` derived from the last TAG, ignoring the
# version the workspace is actually at. A workspace at 2.1.1 with last tag
# v2.0.1 derived v2.1.0 — LOWER than the workspace. The CI release branch
# ("Open release PR when a bump is pending") runs this script and
# auto-merges the result, so CI would have landed a version ROLLBACK on main.
#
# Reproduced in an isolated clone before fixing; see the journal.
#
# Temp repos only. No network. Never mutates the real repo.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUMP="$REPO_ROOT/scripts/release-bump.sh"
ADMISSION="$REPO_ROOT/scripts/lib/release_admission.sh"

if [[ ! -f "$BUMP" ]]; then
    echo "FAIL: $BUMP missing"
    exit 1
fi

TMPROOT=$(mktemp -d)
# shellcheck disable=SC2329  # se referencia via trap EXIT (no captado por shellcheck)
cleanup() {
    local code=$?
    chmod -R u+rw "$TMPROOT" 2>/dev/null || true
    rm -rf "$TMPROOT" >/dev/null 2>&1
    exit "$code"
}
trap cleanup EXIT

PASS=0
FAIL=0

# ── la segunda autoridad de admision, tambien aqui ────────────────────────
#
# `release-bump.sh` deriva el baseline publicado por
# `scripts/lib/release_admission.sh`, que reconcilia `git ls-remote` contra
# la API de GitHub antes de devolverlo. Los remotos de estos fixtures son
# bare locales, luego sin lo que sigue TODOS los casos con remoto cierran con
# `crosscheck_unavailable:remote-is-not-github` y no derivan nada.
#
# El remoto se DECLARA de github y las conexiones se traducen al bare local
# con `url.<x>.insteadOf`: `git config --get remote.origin.url` —de donde sale
# el slug— devuelve la identidad de github, y `git ls-remote origin` responde
# contra el bare. MEDIDO: es la unica combinacion en la que las dos cosas son
# verdad a la vez; con `git remote get-url` (la URL expandida) el slug decia
# que el remoto no era de github.
BIN="$TMPROOT/bin"
mkdir -p "$BIN"
SDDK_TEST_REAL_GIT="$(command -v git)"
export SDDK_TEST_REAL_GIT
cat > "$BIN/gh" <<'SHIM'
#!/usr/bin/env bash
for a in "$@"; do
    case "$a" in repos/*/tags*) spec="$a" ;; esac
done
[[ "${1:-}" == "api" && -n "${spec:-}" ]] || exit 64
# URL EXPANDIDA con el git real: es la que `ls-remote` va a usar.
url="$("$SDDK_TEST_REAL_GIT" remote get-url origin 2>/dev/null)" || exit 1
[[ -n "$url" ]] || exit 1
"$SDDK_TEST_REAL_GIT" ls-remote --tags "$url" 2>/dev/null \
    | awk '{print $2}' | grep -vF '^{}' \
    | sed -n 's|^refs/tags/\(v[0-9][0-9]*\.[0-9][0-9]*\.[0-9][0-9]*\)$|\1|p'
exit 0
SHIM
chmod +x "$BIN/gh"
export PATH="$BIN:$PATH"

# Declara el remoto local como repositorio de github. Se llama DESPUES de
# `git remote add`.
remote_looks_like_github() { # $1 = ruta del bare, $2 = dir del repo
    git -C "$2" config remote.origin.url "git@github.com:acme/widgets.git"
    git -C "$2" config "url.$1.insteadOf" "git@github.com:acme/widgets.git"
}

# Build a repo with a given workspace version and a given last tag, then ask
# the tool to derive. Echoes the "new tag: vX.Y.Z" it reports.
#
# The script does `cd "$ROOT"` where ROOT is its OWN repo root, so invoking it
# from another directory silently operates on this repo instead of the temp
# one. Each fixture therefore gets its own COPY of the script, exactly as CI
# does (CI checks out the repo, so $ROOT == the fixture).
#
#   $1 = workspace version
#   $2 = last tag version
#   $3 = commit subject used for the level detection
derive() {
    local ws="$1" tag="$2" subject="$3"
    local dir="$TMPROOT/$RANDOM-$$"
    mkdir -p "$dir/scripts"
    (
        cd "$dir" || exit 2
        git init -q .
        git config user.email t@example.com
        git config user.name t
        cp "$BUMP" scripts/release-bump.sh
        mkdir -p crates/fake
        cat > Cargo.toml <<EOF
[workspace]
members = ["crates/fake"]

[workspace.package]
version = "$ws"
edition = "2021"
EOF
        cat > crates/fake/Cargo.toml <<EOF
[package]
name = "fake"
version.workspace = true
EOF
        printf 'version = "%s"\n' "$ws" > manifest.toml
        git add -A
        git commit -qm "chore: base"
        git tag "v$tag"
        git commit -q --allow-empty -m "$subject"
    )
    local out derived
    out=$(cd "$dir" && bash scripts/release-bump.sh --dry-run 2>&1)
    # El valor de retorno de esta funcion ES lo que el caller lee. Cualquier
    # salida en stdout del cleanup se concatenaria al tag y haria fallar la
    # comparacion, asi que se silencia explicitamente en vez de confiar en que
    # `rm` no imprime nada.
    rm -rf "$dir" >/dev/null 2>&1
    derived=$(printf '%s\n' "$out" | grep -oE '^new tag: v[0-9]+\.[0-9]+\.[0-9]+$' | awk '{print $3}')
    printf '%s\n' "$derived"
}

# Igual que `derive`, pero el commit lleva CUERPO. El nivel de SemVer se
# decide con el cuerpo, luego un fixture que solo pone el subject no puede
# medir el defecto: haria falta meter la frase en el subject, que es un
# caso distinto y mas Easy de pasar de lo que parece.
#   $1 = workspace version, $2 = tag, $3 = subject, $4 = cuerpo
derive_with_body() {
    local ws="$1" tag="$2" subject="$3" body="$4"
    local dir="$TMPROOT/$RANDOM-$$"
    mkdir -p "$dir/scripts"
    (
        cd "$dir" || exit 2
        git init -q .
        git config user.email t@example.com
        git config user.name t
        cp "$BUMP" scripts/release-bump.sh
        mkdir -p crates/fake
        cat > Cargo.toml <<EOF
[workspace]
members = ["crates/fake"]

[workspace.package]
version = "$ws"
edition = "2021"
EOF
        cat > crates/fake/Cargo.toml <<EOF
[package]
name = "fake"
version.workspace = true
EOF
        printf 'version = "%s"\n' "$ws" > manifest.toml
        git add -A
        git commit -qm "chore: base"
        git tag "v$tag"
        git commit -q --allow-empty -m "$subject" -m "$body"
    )
    local out derived
    out=$(cd "$dir" && bash scripts/release-bump.sh --dry-run 2>&1)
    rm -rf "$dir" >/dev/null 2>&1
    derived=$(printf '%s\n' "$out" | grep -oE '^new tag: v[0-9]+\.[0-9]+\.[0-9]+$' | awk '{print $3}')
    printf '%s\n' "$derived"
}

check() {
    local name="$1" expect="$2" got="$3"
    if [[ "$got" == "$expect" ]]; then
        echo "PASS  [$expect] $name"
        PASS=$((PASS + 1))
    else
        echo "FAIL  [expected $expect, got ${got:-<none>}] $name"
        FAIL=$((FAIL + 1))
    fi
}

# Same fixture as `derive`, but passes an explicit --force-version.
#
#   $1 = workspace version
#   $2 = last tag version
#   $3 = commit subject used for the level detection
#   $4 = version to force
derive_forced() {
    local ws="$1" tag="$2" subject="$3" force="$4"
    local dir="$TMPROOT/$RANDOM-$$"
    mkdir -p "$dir/scripts"
    (
        cd "$dir" || exit 2
        git init -q .
        git config user.email t@example.com
        git config user.name t
        cp "$BUMP" scripts/release-bump.sh
        mkdir -p crates/fake
        cat > Cargo.toml <<EOF
[workspace]
members = ["crates/fake"]

[workspace.package]
version = "$ws"
edition = "2021"
EOF
        cat > crates/fake/Cargo.toml <<EOF
[package]
name = "fake"
version.workspace = true
EOF
        printf 'version = "%s"\n' "$ws" > manifest.toml
        git add -A
        git commit -qm "chore: base"
        git tag "v$tag"
        git commit -q --allow-empty -m "$subject"
    )
    local out derived
    out=$(cd "$dir" && bash scripts/release-bump.sh --dry-run --force-version "$force" 2>&1)
    # Mismo motivo que en derive(): el retorno se lee como valor, no como
    # salida acumulada.
    rm -rf "$dir" >/dev/null 2>&1
    derived=$(printf '%s\n' "$out" | grep -oE '^new tag: v[0-9]+\.[0-9]+\.[0-9]+$' | awk '{print $3}')
    printf '%s\n' "$derived"
}

# --- The regression: workspace ahead of the tag -----------------------------
#
# A manual `--force-version` bump (or a pending ceremonial release) leaves
# the workspace ABOVE the last tag. AGENTS.md §2.3 makes the workspace
# version the *ceremonial release pointer*: it declares the version that is
# going to appear as the tag. So the release IS the workspace version and
# there is nothing left to derive.
#
# Deriving from the tag alone rolls the workspace BACK (session-28).
# Deriving max(workspace, tag) and then bumping SKIPS the declared release
# (session-29: 2.2.0 -> v2.3.0, publishing a version nobody asked for).
# Both are wrong; the answer is "no pending bump".
check "workspace ahead of tag, minor commit derives no bump" \
    "" "$(derive 2.1.1 2.0.1 'feat: something')"

check "workspace ahead of tag, patch commit derives no bump" \
    "" "$(derive 2.1.1 2.0.1 'fix: something')"

check "workspace ahead of tag, breaking commit derives no bump" \
    "" "$(derive 2.1.1 2.0.1 'feat!: breaking change')"

# ── BREAKING CHANGE: FOOTER, NO PALABRA ───────────────────────────────────
#
# MEDIDO (session-91): el nivel se derivaba con `grep -iE 'breaking change'`
# sobre el cuerpo ENTERO de todos los commits. En el historial de este repo
# eso casa 11 veces y los footers canonicos son 2 — 5,5 veces mas. Y no era un
# detalle: el commit que arreglaba el SemVer documentaba el patron en su
# cuerpo, luego se declaraba a si mismo breaking y dos `fix(...)` salian como
# `release bump: v2.14.0 -> v3.0.0 (major)`. Ejecutado y medido.
#
# Los dos casos de aqui son los lados del defecto y ambos tienen que caer:
# el que MENCIONA la frase sin ser breaking, y el que ES breaking de verdad.
# Con solo el segundo, un detector que no detectase nada pasaria.
check "un commit que MENCIONA breaking change en el cuerpo no es major" \
    "v2.0.2" "$(derive_with_body 2.0.1 2.0.1 'fix: algo' 'el patron grep -i breaking change casa once veces')"

check "un footer BREAKING CHANGE canonico SI es major" \
    "v3.0.0" "$(derive_with_body 2.0.1 2.0.1 'fix: algo' 'BREAKING CHANGE: la API cambia')"

check "un footer BREAKING-CHANGE con guion SI es major" \
    "v3.0.0" "$(derive_with_body 2.0.1 2.0.1 'fix: algo' 'BREAKING-CHANGE: la API cambia')"

check "un subject con ! SI es major aunque el cuerpo no diga nada" \
    "v3.0.0" "$(derive 2.0.1 2.0.1 'feat!: la API cambia')"

# --- The normal case: workspace == tag -------------------------------------
#
# Nothing pending. The tag is the base, as before.
check "workspace equals tag, minor commit" \
    "v2.1.0" "$(derive 2.0.1 2.0.1 'feat: something')"

check "workspace equals tag, patch commit" \
    "v2.0.2" "$(derive 2.0.1 2.0.1 'fix: something')"

# --- Workspace BELOW the tag is impossible in a healthy repo, but if it
# --- happened the tag must still win (never derive downward from nothing).
check "workspace behind tag keeps tag as base" \
    "v2.0.2" "$(derive 2.0.0 2.0.1 'fix: something')"

# --- --force-version still wins over the short-circuit ----------------------
#
# The short-circuit above stops the accidental double bump. But the operator
# must keep a way to say "actually make it this", even with the workspace
# already ahead of the tag. If --force-version were swallowed by the
# short-circuit, the escape hatch documented in AGENTS.md §2.3 would be dead.
check "--force-version overrides the declared-release short-circuit" \
    "v2.5.0" "$(derive_forced 2.1.1 2.0.1 'feat: something' 2.5.0)"

# --- The baseline comes from the REMOTE, not the local clone ----------------
#
# Session-77. The bump script used to read the baseline from `git tag` on the
# local clone while `release_admission.sh` read it from `git ls-remote --tags
# origin`. Two answers to the same question, and the local one goes stale:
# `gh release create` publishes the tag on the remote and nothing in the
# pipeline updates the clone.
#
# Measured on this repo: `v2.5.6` was published and present in
# `git ls-remote --tags origin`, while `git tag` stopped at `v2.5.5`. The bump
# then reported "the workspace declares the pending release (2.5.6)" and would
# have handed `release.sh` a tag that was ALREADY PUBLISHED.
#
# The fixture below reproduces exactly that shape: the remote carries a tag the
# clone does not have. The assertion is the one that discriminates — the local
# baseline v2.5.5 would say "workspace 2.5.6 is ahead, nothing to derive" and
# exit 0 with no tag, which is the bug; the remote baseline v2.5.6 derives a
# fresh tag from the commits since it.
#
#   $1 = workspace version
#   $2 = tag the CLONE knows about
#   $3 = tag the REMOTE additionally carries
#   $4 = commit subject used for the level detection
derive_remote_ahead() {
    local ws="$1" local_tag="$2" remote_tag="$3" subject="$4"
    local dir="$TMPROOT/$RANDOM-$$"
    local bare="$dir/.remote.git"
    mkdir -p "$dir/scripts/lib" "$bare"
    (
        cd "$dir" || exit 2
        git init -q --bare "$bare"
        git init -q .
        git config user.email t@example.com
        git config user.name t
        git remote add origin "$bare"
        remote_looks_like_github "$bare" "$dir"
        cp "$BUMP" scripts/release-bump.sh
        cp "$ADMISSION" scripts/lib/release_admission.sh
        mkdir -p crates/fake
        cat > Cargo.toml <<EOF
[workspace]
members = ["crates/fake"]

[workspace.package]
version = "$ws"
edition = "2021"
EOF
        cat > crates/fake/Cargo.toml <<EOF
[package]
name = "fake"
version.workspace = true
EOF
        printf 'version = "%s"\n' "$ws" > manifest.toml
        git add -A
        git commit -qm "chore: base"
        git tag "v$local_tag"
        # A commit that was released ELSEWHERE, so the remote tag can sit on it.
        git commit -q --allow-empty -m "chore: released from another clone"
        # The only unreleased commit; its type is what decides the level.
        git commit -q --allow-empty -m "$subject"
        git push -q origin "refs/tags/v$local_tag:refs/tags/v$local_tag"
        git push -q origin "HEAD:refs/heads/main"
        # The later tag exists ONLY on the remote and is never fetched back.
        # This is the state `gh release create` leaves a clone in.
        git -C "$bare" tag "v$remote_tag" HEAD^
    )
    local out derived
    out=$(cd "$dir" && bash scripts/release-bump.sh --dry-run 2>&1)
    rm -rf "$dir" >/dev/null 2>&1
    derived=$(printf '%s\n' "$out" | grep -oE '^new tag: v[0-9]+\.[0-9]+\.[0-9]+$' | awk '{print $3}')
    printf '%s\n' "$derived"
}

# The remote baseline is one ahead of what the clone knows, and the commit since
# it is a `fix`. Reading the local tag instead makes the workspace look "ahead
# of the tag" and derives nothing at all -- which is the defect, and is exactly
# the shape that would have re-published an existing tag.
check "remote baseline wins over a stale local tag list (fix)" \
    "v2.5.4" "$(derive_remote_ahead 2.5.3 2.5.2 2.5.3 'fix: something')"

# Same fixture, minor commit: the derivation must follow the REMOTE baseline,
# not the local one, or the level is computed from the wrong commit range.
check "remote baseline yields a minor when the commit is a feat" \
    "v2.6.0" "$(derive_remote_ahead 2.5.3 2.5.2 2.5.3 'feat: something')"

# A repo with no remote at all must still derive from its local tags — the
# bootstrap path, and the one the original fixtures above exercise. The remote
# fix must not turn "never published" into a hard error.
check "no remote configured still derives from local tags" \
    "v2.1.0" "$(derive 2.0.1 2.0.1 'feat: something')"

echo ""
echo "=== matrix result: PASS=$PASS FAIL=$FAIL ==="
[[ $FAIL -eq 0 ]] || exit 1
exit 0
