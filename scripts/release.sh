#!/usr/bin/env bash
# release.sh — Canonical end-to-end release flow for sddk-framework.
#
# Standardized after cycle-46 (install coherence) and cycle-47 (install
# consolidation). Every release MUST pass through this script — or the
# manual equivalent — so that local install stays in lockstep with what
# ships through GitHub Releases.
#
# ── PIPELINE AUTHORITY (read this before changing asset names) ────────────
#
# `.github/workflows/release.yml` is a manually dispatched distribution
# workflow. `release-automation.yml` dispatches it after pushing a release
# tag, so it is an active publisher path even though it is not the local
# SDDK release gate. This script remains the canonical local release gate.
#
# Both paths share `scripts/release-assets-contract.sh`. The local path
# publishes the canonical 9 payloads; the Actions path stages those same 9
# plus three explicitly allowlisted non-x86 unified packages. It signs all
# installable payloads before its single publish job, writes the authority
# receipt through `release-receipt.sh`, and runs the same asset contract plus
# the end-user installer smoke test. The workflow contract is pinned by
# `tests/test_release_pipeline_consistency.sh` and
# `tests/test_release_ci_contract.sh`.
#
# Pipeline (each step is gated on the previous one succeeding):
#   1. Preflight  — workspace green: fmt, clippy -D warnings, tests
#   2. Version    — read current version from Cargo.toml
#   3. Build      — cargo build --release --bin sddk
#   4. Manifest   — regenerate MANIFEST.sha256 from the bundle surface
#   5. Bundle     — tar agents/ skills/ prompts/sddk/ assets/ MANIFEST.sha256
#   6. BUNDLE.toml — inject schema_version=2 + manifest_sha256 into the bundle
#   7. Unified    — repack bin/sddk + framework/ as sddk-<TAG>-<ASSET>.tar.gz
#                   with chmod 0755 on the binary (defensive against CDN cache)
#   8. Checksums  — sha256 + CHECKSUMS + sbom.json for the binary
#   9. Publish    — gh release create with all assets (--clobber if --force)
#  10. Install    — bash scripts/install.sh --version <TAG> against the real
#                   GitHub URL (no SDDK_BASE_URL override)
#  11. Verify     — sddk dev doctor --prefix <P> reports binary.bundle_coherence
#  12. Prune      — sddk dev update --prune-only --keep 1 to clean stale dirs
#  13. Manifest   — print final state (binary version, bundle version,
#                   framework layout, doctor result)
#
# Usage:
#   bash scripts/release.sh                 # full release flow
#   bash scripts/release.sh --dry-run       # walk steps 1-8 only (no publish)
#   bash scripts/release.sh --skip-tests    # skip step 1 (you just ran them)
#   bash scripts/release.sh --skip-install  # steps 1-9 only (no local install)
#   bash scripts/release.sh --force         # overwrite existing GH release
#
# Tag format: vX.Y.Z (semver). The script reads the version from Cargo.toml's
# workspace.package.version and prepends "v" — never accepts a --version
# override (the version is the source of truth; bump it via
# scripts/release-bump.sh or by hand before invoking release.sh).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$ROOT"
REPO="${SDDK_REPO:-Rubentxu/software-development-decision-kernel}"
SDDK_PREFIX="${SDDK_PREFIX:-$HOME/.local/bin}"
SDDK_FRAMEWORK_DIR="${SDDK_FRAMEWORK_DIR:-$HOME/.local/share/sddk/framework}"
cd "$ROOT"
# shellcheck disable=SC1091  # se valida su existencia en tests/test_release_receipt_authority.sh
source "$ROOT/scripts/release-assets-contract.sh"

# Isolate TMPDIR for the whole release run so the test gate is deterministic
# regardless of the ambient TMPDIR. The scratch MUST live OUTSIDE the repo
# tree and OUTSIDE a symlinked home prefix:
#   * Under `$ROOT/target` (inside the repo) tests create temp dirs that walk
#     up and hit the repo's own git/sddk markers, breaking cycle/manifest
#     walk-up tests.
#   * Under a symlinked home (e.g. `/home -> /var/home`) the real path differs
#     from the reported one, breaking XDG canonicalization in integration
#     tests.
# Earlier flakiness blamed the ambient scratch for `PermissionDenied` under
# parallel load; the real cause was two test-isolation races in sddk-cli
# (a global `chdir` in a cycle test and a `copy_tree` test chmod-ing the shared
# temp root to read-only), now fixed in the test suite. We keep the isolation
# for determinism but place it on a real, disk-backed path outside the repo.
mkdir -p "${CARGO_TARGET_DIR:-$HOME}" 2>/dev/null || true
SCRATCH_ROOT="$(readlink -f "${CARGO_TARGET_DIR:-$HOME}" 2>/dev/null || echo "$ROOT")"
mkdir -p "$SCRATCH_ROOT"
RELEASE_SCRATCH="$(mktemp -d "$SCRATCH_ROOT/sddk-release-tmp.XXXXXX")"
export TMPDIR="$RELEASE_SCRATCH"
cleanup_release_scratch() { rm -rf "$RELEASE_SCRATCH"; }
trap cleanup_release_scratch EXIT

# --- args ---

DRY_RUN=0
SKIP_TESTS=0
SKIP_INSTALL=0
FORCE=0
FORCE_VERSION=""

while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run)         DRY_RUN=1; shift ;;
        --skip-tests)      SKIP_TESTS=1; shift ;;
        --skip-install)    SKIP_INSTALL=1; shift ;;
        --force)           FORCE=1; shift ;;
        --force-version)   FORCE_VERSION="$2"; shift 2 ;;
        -h|--help)
            sed -n '2,/^[^#]/p' "$0" | head -50
            exit 0
            ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
done

# --- helpers ---

step() { printf '\n\033[1;34m==>\033[0m %s\n' "$*"; }
ok()   { printf '\033[1;32m  ✓\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m  !\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31m  ✗\033[0m %s\n' "$*" >&2; exit 1; }

require() {
    command -v "$1" >/dev/null 2>&1 \
        || die "required command not found: $1"
}

# --- 0. preflight: tooling + git state ---

step "0/15 — preflight"
require cargo
require git
require gh
require tar
require sha256sum
require curl
require jq

gh auth status >/dev/null 2>&1 \
    || die "gh CLI not authenticated — run: gh auth login"

# Branch check: must be on main, clean working tree (release-bump script
# already stages the version bump; we expect that to be in HEAD or HEAD~1).
BRANCH="$(git rev-parse --abbrev-ref HEAD)"
[ "$BRANCH" = "main" ] \
    || die "must be on main (currently on $BRANCH)"

if ! git diff --quiet || ! git diff --cached --quiet; then
    die "working tree is dirty — commit or stash before releasing"
fi

# A5-1 §6 — release admission is semantic, not textual.
#
# A release HEAD MUST carry a REAL (and monotonically increasing)
# [workspace.package] version change. The commit subject may follow the
# `chore(release): bump version` convention, but the subject alone is NEVER
# the contract: an empty ceremonial marker commit is refused here, so it can
# never be mistaken for a real release.
#
# The comparison is against the LAST PUBLISHED TAG, not against HEAD^.
# This is the v2 check, and it is the default here because the v1 check
# (HEAD vs HEAD^) is wrong for any release whose bump commit is not HEAD.
# The bump legitimately sits N commits behind HEAD: docs, journal and
# pointer commits land after it, by this repo's own convention. With v1,
# HEAD and HEAD^ then carry the SAME version and the gate refuses a
# perfectly valid release with `REJECT non-monotonic 2.1.0 -> 2.1.0`.
# The v1 flaw was documented in scripts/lib/release_admission.sh but the
# publishing path never opted in, so it stayed latent until a release
# actually came due.
#
# v2 fails CLOSED on an unreachable or unauthenticated remote
# (`REJECT query-failed`) rather than degrading to the HEAD^ comparison.
#
# Single source of the invariant: `scripts/lib/release_admission.sh`.
# shellcheck source=lib/release_admission.sh
# shellcheck disable=SC1091
. "$ROOT/scripts/lib/release_admission.sh"

LAST_SUBJECT="$(git log -1 --format=%s)"
ADMISSION="$(release_admission_check_v2 HEAD)" \
    || die "release admission refused: $ADMISSION — release requires a real, monotonic [workspace.package] version bump above the last published release"
if ! echo "$LAST_SUBJECT" | grep -qE '^chore\(release\): bump version'; then
    warn "HEAD subject does not follow the 'chore(release): bump version' convention: $LAST_SUBJECT"
fi
ok "on main, clean tree, release admission: $ADMISSION"

# --- 1. tests ---

if [ "$SKIP_TESTS" = "0" ]; then
    step "1/15 — cargo fmt + clippy + test (workspace)"
    cargo fmt --all -- --check || die "cargo fmt failed"
    cargo clippy --workspace --offline --all-targets -- -D warnings \
        || die "cargo clippy failed"
    cargo test --workspace --offline \
        || die "cargo test --workspace failed"
    ok "workspace green"

    step "1b/15 — shell contract tests (tests/test_*.sh)"
    # The shell contract tests pin invariants that cargo cannot cover
    # (githooks/pre-push behavior, release-receipt authority gate, the
    # cross-crate lockstep between scripts/release-receipt.sh and the
    # engine's infer_actor_kind, the ADR-0001 §3.4 frontmatter
    # convention). They run sequentially; each is expected to exit 0
    # with its own banner. shellcheck is run first as a static gate;
    # the dynamic tests follow.
    if command -v shellcheck >/dev/null 2>&1; then
        # Scope the static gate to scripts/tests authored or extended by
        # this repository's M9+ contracts; legacy tests in
        # tests/test_vault_coherence_alignment.sh have pre-existing
        # SC2034/SC2329 warnings outside our gate (they're exercised by
        # their own dynamic tests, not by shellcheck).
        shellcheck --severity=warning scripts/release-receipt.sh \
            scripts/lib/release_admission.sh \
            githooks/pre-push \
            tests/test_release_admission.sh \
            tests/test_push_prevention_hook.sh \
            tests/test_release_receipt_authority.sh \
            tests/test_authority_helper_lockstep.sh \
            tests/test_adr_promotion_format.sh \
            tests/test_advisory_lint_explanations.sh \
            tests/test_deny_lint_zero_hits.sh \
            tests/test_vault_adr_mirror_coverage.sh \
            tests/test_release_tag_anchoring.sh \
            tests/test_release_ci_contract.sh \
            tests/test_release_pipeline_consistency.sh \
            tests/test_vault_mirror_auto.sh \
            tests/test_install_signature_execution.sh \
            tests/test_install_signature_execution_mutation.sh \
            tests/test_release_sign_artifacts.sh \
            tests/test_release_sign_artifacts_mutation.sh \
            tests/test_release_authenticity_posture.sh \
            tests/test_release_authenticity_posture_mutation.sh \
            || die "shellcheck failed"
        ok "shellcheck clean (scope: release-receipt + release/push admission + 8 cross-crate/M9+ tests)"
    else
        warn "shellcheck not installed — skipping static gate (install shellcheck for full coverage)"
    fi
    for t in tests/test_push_prevention_hook.sh \
             tests/test_release_admission.sh \
             tests/test_release_receipt_authority.sh \
             tests/test_authority_helper_lockstep.sh \
             tests/test_adr_promotion_format.sh \
             tests/test_advisory_lint_explanations.sh \
             tests/test_deny_lint_zero_hits.sh \
             tests/test_vault_adr_mirror_coverage.sh \
             tests/test_release_tag_anchoring.sh \
             tests/test_release_ci_manifest_anchor.sh \
             tests/test_release_ci_staging.sh \
             tests/test_release_ci_contract.sh \
             tests/test_release_pipeline_consistency.sh \
             tests/falsify-ci-anchor-real.sh \
             tests/test_vault_mirror_auto.sh \
             tests/test_debt_index_coherence.sh \
             tests/test_adr_0153_criteria.sh \
             tests/test_dev_install_source_guard.sh \
             tests/test_release_bundle_layout.sh \
             tests/test_release_bundle_step5.sh \
             tests/test_install_asset_contract.sh \
             tests/test_install_signature_execution.sh \
             tests/test_install_signature_execution_mutation.sh \
             tests/test_release_sign_artifacts.sh \
             tests/test_release_sign_artifacts_mutation.sh \
             tests/test_release_authenticity_posture.sh \
             tests/test_release_authenticity_posture_mutation.sh \
             tests/test_release_bump_bundle_sync.sh \
             tests/test_release_unsigned_propagation.sh \
             tests/test_changelog_merge.sh \
             tests/test_release_state_pointer.sh \
             tests/test_vault_coherence_alignment.sh \
             tests/test_build_identity_policy.sh \
             tests/test_kmt_canonical_meaning.sh \
             tests/test_release_build_identity.sh; do
        if [ -x "$t" ]; then
            bash "$t" >/dev/null \
                || die "shell test failed: $t (run manually for details)"
            ok "shell test: $(basename "$t")"
        else
            warn "shell test not executable, skipping: $t"
        fi
    done
    ok "shell contract tests green"

    # Session-65j: los tests en Python tenian la misma forma de fallo que los
    # shell. `ci.yml:58,64` enumera A MANO dos de ellos; los otros cuatro no
    # los corre nadie, y varios son los que fijan contratos que(session-65g/65h
    # escribieron y que se ejecutaban a mano en cada slice. Todos son hermeticos
    # (36-299 ms). Se ejecutan aqui, y `test_gate_coverage.py` es lo que impide
    # que la lista vuelva a quedar por debajo sin que nadie lo note.
    for p in tests/test_bundle_surface_coverage.py \
             tests/test_surface_reference_integrity.py \
             tests/test_adopt_convergence_contract.py \
             tests/test_migrate_project_identity_mirror.py \
             tests/test_migrate_project_identity_write.py \
             tests/test_docs_script_contamination.py \
             tests/test_uat_authority_citations.py \
             tests/test_spec_citation_anchor.py \
             tests/test_contamination_surface_mutation.py \
             tests/test_gate_coverage.py; do
        if [ -f "$p" ]; then
            python3 "$p" >/dev/null \
                || die "python contract test failed: $p (run manually for details)"
            ok "python test: $(basename "$p")"
        else
            warn "python test missing, skipping: $p"
        fi
    done

    # Session-65j: cuatro tests/test_*.sh NO estaban en ningun runner. Tres de
    # ellos —los que sessions 65g/65h escribieron para fijar el contrato del
    # bundle— son hermeticos y corren en 0-2s, asi que desde aqui pasan a ser
    # gates de release: una regresion en el staging de release.sh falla el
    # release en vez de publicarse.
    #
    # Los otros DOS se quedan fuera a proposito, y por razones distintas:
    #
    #   - tests/test_release_routes_parity.sh necesita `act` + `podman` y monta
    #     contenedores. Su cabecera ya dice que la ruta cloud no se puede probar
    #     aqui. Es una comprobacion opt-in, no un gate de cada release.
    #
    #   - tests/test_h05_isolation.sh **pasa sin medir**: si el rlib release no
    #     existe imprime `skip:` y aun asi reporta `PASS=1 FAIL=0`. Cablearlo
    #     hoy devolveria un verde vacio, que es exactamente la forma de
    #     INC-DEBT-054 (`doctor --strict` salia con exit 0 sin medir nada). Un
    #     PASS que no midio nada es peor que un gate ausente, porque ademas
    #     tapa el defecto. Medidos: 0s, PASS, 1 skip.
    #
    # Session-69s: `test_gate_coverage.py` —que corre en este mismo bucle y
    # cuyo fallo hace `die`— estaba en ROJO desde antes de este trabajo, con
    # cuatro tests sin runner y sin motivo. Medido antes de tocar nada:
    # `test_build_identity_policy.sh` PASS=8, `test_kmt_canonical_meaning.sh`
    # PASS=6 y `test_release_build_identity.sh` PASS=22, los tres hermeticos y
    # con forma de `bash test.sh`; asi que los tres entran aqui. El cuarto,
    # `test_doctor_identity_states.sh`, NO: exige dos binarios como argv con
    # procedencia distinta a proposito, porque lo que mide son los cuatro
    # estados de `binary.build_identity` y dos de ellos solo se alcanzan asi.
    # Con `${1:?uso: ...}` sale por argv con codigo 1, o sea un rojo que no mide
    # nada, asi que va a EXCEPTIONS con el motivo escrito -- incluida la
    # consecuencia: los cuatro estados de la identidad NO se verifican en el
    # camino de release hasta que exista el arnes que construya los dos
    # binarios. Una excepcion sin la consecuencia declarada es un hueco
    # silencioso, que es lo que este bloque lleva siete sesiones evitando.
    # MEDIDO, no supuesto: con esto el gate de cobertura da
    # `SIN runner y SIN motivo: 0` y `RESULT: PASS`. Antes de este cambio
    # cualquier `bash scripts/release.sh` moria en 1b.

    # INC-DEBT-055-adjacent (session-65j): el guard de abajo estaba referenciado
    # SOLO por su propio test de fixtures. `ci.yml:46` hace `shellcheck` de
    # `scripts/*.sh` — que es lint, no ejecución — y el bucle de arriba corre
    # `tests/test_*.sh`, que monta un árbol desechable por caso. Nadie ejecutaba
    # el guard contra el repo real, y llevaba ROJO (exit 1) desde session-65b
    # porque INC-DEBT-052 declaraba su estado en un dialecto que no sabe leer.
    #
    # Es la forma del INC-DEBT-033 un nivel más hondo: el propio header del guard
    # advierte que "un test que no puede mover el sujeto bajo test no puede
    # falsificarlo", y aun así sus 10 casos pasaban — porque pasaban contra
    # fixtures, no contra el índice real. Ejecutarlo aquí es lo que convierte
    # los 10 casos en evidencia y no en decoración.
    if bash scripts/check_debt_index_coherence.sh >/dev/null; then
        ok "debt index coherence: indice y documentos coinciden"
    else
        die "debt index coherence guard failed (run scripts/check_debt_index_coherence.sh)"
    fi
else
    warn "skipping step 1 (tests) — assumed already run"
fi

# --- 1c. publish sync: ensure origin/main is at HEAD before step 9 ---
#
# INC-RELEASE-TAG-FIX: step 9 (gh release create --target main) resolves
# `main` to the commit at the tip of origin/main. The release flow's
# previous commits (feat + ceremonial + bump) are local-only and must be
# pushed before publish, otherwise the tag points to a stale commit and
# requires manual repointing. This step pushes HEAD to origin/main
# non-interactively (the pre-push hook enforces the bump predicate) and
# fails closed if origin/main has advanced concurrently — the operator
# must merge before re-running.
#
# Behaviour:
#   1. git fetch origin main → see the current remote tip.
#   2. If origin/main == HEAD → nothing to do, fast path.
#   3. If origin/main behind HEAD → git push origin main (pre-push hook
#      enforces bump-commit + version-bump predicate; no need to repeat
#      that check here).
#   4. If origin/main ahead of HEAD → refuse to release, instruct the
#      operator to merge origin/main into HEAD and re-run.
#
# The --skip-tests flag does not skip this step: pushing is part of the
# release contract, not the test gate.

step "1c/15 — sync HEAD to origin/main (closes INC-RELEASE-TAG-FIX)"
git fetch origin main --quiet \
    || die "git fetch origin main failed — cannot verify remote state"
LOCAL_HEAD="$(git rev-parse HEAD)"
REMOTE_MAIN="$(git rev-parse origin/main)"

if [ "$LOCAL_HEAD" = "$REMOTE_MAIN" ]; then
    ok "HEAD already at origin/main ($LOCAL_HEAD) — no push needed"
elif git merge-base --is-ancestor "$REMOTE_MAIN" "$LOCAL_HEAD"; then
    # origin/main is behind HEAD → fast-forward push.
    # The pre-push hook (githooks/pre-push) is automatically invoked by
    # `git push` and rejects non-release commits to main. We rely on
    # that gate; do not duplicate the predicate here.
    if git push origin main >/dev/null 2>&1; then
        ok "pushed HEAD to origin/main: $LOCAL_HEAD"
    else
        die "git push origin main failed — pre-push hook rejected the push; ensure HEAD carries a real [workspace.package] version bump or a docs-only range"
    fi
else
    # origin/main is ahead of HEAD → concurrent advance. Fail closed.
    die "origin/main ($REMOTE_MAIN) is ahead of HEAD ($LOCAL_HEAD); merge origin/main into HEAD and re-run"
fi

# --- 1d. EXT auto-activation (closes FU-A6-EXT-AUTO) ---
#
# When the operator exports $COGNICODE_MCP_BIN and/or $CHRONOS_MCP_BIN
# before invoking release.sh, run the previously-#[ignore] EXT tests
# against the real provider binaries, capture pass/fail per provider, and
# write an EXT-RECEIPT.md to the cycle artifacts. Without the env vars,
# the step is a no-op (the EXT tests stay #[ignore] and the release
# proceeds normally — operators without the binaries are not blocked).
#
# This step MUST run before version reading because the receipt dir
# embeds the version string. We read VERSION early here.
step "1d/15 — EXT auto-activation (cognicode-mcp / chronos-mcp, opt-in)"
VERSION="$(awk '/^\[workspace\.package\]/{flag=1; next} flag && /^version = /{print $3; exit}' Cargo.toml \
    | tr -d '\"')"
TAG="v$VERSION"
[ -n "$VERSION" ] || die "could not parse version from Cargo.toml"

EXT_RECEIPT_DIR=""
EXT_FAIL=0
if [ -n "${COGNICODE_MCP_BIN:-}" ] || [ -n "${CHRONOS_MCP_BIN:-}" ]; then
    require jq
    EXT_RECEIPT_DIR="tests/cycle-artifacts/p-63676b11dc0ef88f/ext-auto-activation-$VERSION"
    mkdir -p "$EXT_RECEIPT_DIR"
    : > "$EXT_RECEIPT_DIR/EXT-RECEIPT.md"
    {
        echo "# EXT-RECEIPT — $TAG (release-time auto-activation)"
        echo
        echo "- HEAD: $LOCAL_HEAD"
        echo "- COGNICODE_MCP_BIN: ${COGNICODE_MCP_BIN:-NOT SET}"
        echo "- CHRONOS_MCP_BIN: ${CHRONOS_MCP_BIN:-NOT SET}"
        echo "- date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
        echo
    } >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md"

    if [ -n "$COGNICODE_MCP_BIN" ]; then
        echo "## CogniCode EXT (COGNICODE_MCP_BIN=$COGNICODE_MCP_BIN)" \
            >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md"
        if COGNICODE_MCP_BIN="$COGNICODE_MCP_BIN" \
            cargo test --workspace --offline -- \
                --include-ignored \
                a6_cc_s1 aiw_s1_cognicode_real 2>&1 \
                | tee "$EXT_RECEIPT_DIR/cognicode-ext.log" >/dev/null; then
            echo "- result: PASS" >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md"
        else
            echo "- result: FAIL" >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md"
            EXT_FAIL=1
        fi
        # Extract test counts from the log.
        grep -E 'test result:' "$EXT_RECEIPT_DIR/cognicode-ext.log" \
            >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md" || true
    fi

    if [ -n "$CHRONOS_MCP_BIN" ]; then
        echo "## Chronos EXT (CHRONOS_MCP_BIN=$CHRONOS_MCP_BIN)" \
            >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md"
        if CHRONOS_MCP_BIN="$CHRONOS_MCP_BIN" \
            cargo test --workspace --offline -- \
                --include-ignored \
                aiw_s5_chronos_real a7_s1_runtime_uat 2>&1 \
                | tee "$EXT_RECEIPT_DIR/chronos-ext.log" >/dev/null; then
            echo "- result: PASS" >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md"
        else
            echo "- result: FAIL" >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md"
            EXT_FAIL=1
        fi
        grep -E 'test result:' "$EXT_RECEIPT_DIR/chronos-ext.log" \
            >> "$EXT_RECEIPT_DIR/EXT-RECEIPT.md" || true
    fi

    ok "EXT receipt written to $EXT_RECEIPT_DIR/EXT-RECEIPT.md"
    if [ "$EXT_FAIL" = "1" ]; then
        die "EXT tests failed against real binaries — refusing to release (see $EXT_RECEIPT_DIR/EXT-RECEIPT.md)"
    fi
else
    ok "no EXT env vars set — skipping (EXT tests stay #[ignore] in this release)"
fi

# --- 2. version ---

# VERSION/TAG were already read in step 1d above. Re-read defensively in
# case step 1d was added as an insert (the read is idempotent).
VERSION="${VERSION:-$(awk '/^\[workspace\.package\]/{flag=1; next} flag && /^version = /{print $3; exit}' Cargo.toml \
    | tr -d '\"')}"
TAG="${TAG:-v$VERSION}"
[ -n "$VERSION" ] || die "could not parse version from Cargo.toml"
ok "version: $VERSION → tag: $TAG"

# --- 2b. changelog coverage (INC-DEBT-047, session-61) ---
#
# The version above is already declared, so the release can proceed without any
# further check that the CHANGELOG section describes the work. That gap was
# live: 26 commits sat between the last tag and HEAD — including a whole
# `feat(architecture)` slice — while `[2.5.0]` listed only the two features
# present when the bump was committed. Shipping then would have published an
# artifact whose changelog misdescribed its own contents: the same defect shape
# as C3l.7, one layer down.
#
# Fail-closed before the build, because a missing entry found at step 9 (after
# a `gh release create`) costs a deletion; found here it costs a commit.
if [[ "$DRY_RUN" == "0" ]]; then
    step "2b/15 — changelog coverage"
    bash tests/test_changelog_coverage.sh \
        || die "changelog coverage failed: the declared section does not describe the work this release ships. Add the missing entries (git log --format=%s <last-tag>..HEAD) and re-run."
    ok "changelog describes the shipped work"
fi

# --- 2.5 semver-correct tag (cycle-c2 bug fix) ---
#
# The workspace version above is a CEREMONIAL per-push pointer (incremented by
# the pre-push hook for every source-touching commit, not SemVer-strict).
# `scripts/release-bump.sh` computes the actual SemVer-correct next tag from
# the conventional commits accumulated since the last published release tag.
# Without this step the released tag would be the workspace version literal
# (e.g. v1.169.152) instead of the SemVer-bumped tag (e.g. v1.170.0).
#
# Invocation: invoke release-bump.sh in dry-run mode and parse its output.
# Honors the operator's --force-version flag (passed through if set).
BUMP_ARGS=(--dry-run)
if [ -n "$FORCE_VERSION" ]; then
    BUMP_ARGS+=(--force-version "$FORCE_VERSION")
    warn "operator forced version override: $FORCE_VERSION (SemVer algorithm bypassed)"
fi
STEP2P5_OUTPUT="$(bash "$ROOT/scripts/release-bump.sh" "${BUMP_ARGS[@]}" 2>&1)" \
    || die "scripts/release-bump.sh failed (cannot compute SemVer tag)"
SEMVER_TAG="$(echo "$STEP2P5_OUTPUT" \
    | awk '/^new tag: / {print $3; exit}')"
if [ -z "$SEMVER_TAG" ]; then
    # release-bump.sh exits 0 with "no commits since <tag>" message — keep TAG.
    warn "release-bump.sh did not produce a tag; keeping workspace-derived TAG=$TAG"
else
    if [ "$SEMVER_TAG" != "$TAG" ]; then
        warn "semver tag overrides workspace-derived tag: $TAG → $SEMVER_TAG"
        TAG="$SEMVER_TAG"
    else
        ok "semver tag matches workspace tag: $TAG"
    fi
fi
ok "final tag: $TAG"

# --- 3. build ---
#
# INC-DEBT-021: el nombre del asset decía "musl" y el contenido era glibc.
# La causa era que este script compilaba con `cargo build --release` — el
# target del host — y luego empaquetaba el resultado con nombre musl.
#
# Aqui se compila con el target musl REAL. El binario resultante es
# static-pie: no necesita glibc y corre en cualquier distro, incluidos
# Alpine y Debian 12 (verificado en session-19).
#
# El target es configurable porque no todos los hosts de release tienen el
# toolchain. El default es musl porque es lo que el nombre del asset
# promete. Si se pide musl y el toolchain no esta, el script ABORTA: es
# preferible no publicar a publicar un binario con el nombre equivocado.
# Esa era exactamente la mentira que INC-021 documentaba.

step "3/15 — cargo build --release --bin sddk"

BUILD_TARGET="${SDDK_RELEASE_BUILD_TARGET:-x86_64-unknown-linux-musl}"

if [ "$BUILD_TARGET" != "x86_64-unknown-linux-musl" ]; then
    warn "SDDK_RELEASE_BUILD_TARGET=$BUILD_TARGET: el nombre del asset dice musl pero el target no es musl"
    warn "esto reintroduce INC-021. Se requiere una decision explicita del operador."
fi

if ! rustup target list --installed 2>/dev/null | grep -qx "$BUILD_TARGET"; then
    die "el target $BUILD_TARGET no esta instalado (rustup target add $BUILD_TARGET).
         release.sh publica assets musl; compilar contra otro target rompe el
         contrato del installer. Para publicar de otro modo, cambia tambien el
         nombre del asset y el contrato de install.sh. Ver INC-DEBT-021."
fi

# La identidad la fija quien lanza el build (crates/sddk-cli/build.rs lee
# SDDK_GIT_SHA y la declara como fuente de verdad; STOP 6 del SCOPE de
# cl-build-identity prohibe que el fallback a .git decida nada). Sin esto, un
# binario publicado declararia `source: git` y `sddk dev build-id --check` no
# saldria nunca de `unknown` — el detector no podria cumplir su funcion
# precisamente en el caso que motiva INC-DEBT-064, que es un binario PUBLICADO
# y obsoleto. Es el unico punto entre el hallazgo y el remedio completo.
#
# El SHA se MIDE, no se supone, y por el mismo motivo del que hay unas lineas
# mas arriba: un detector que emite un valor obsoleto sin senal es peor que no
# tener detector. Y la suciedad tambien se mide, porque un commit no identifica
# contenido que tenia cambios sin commitear encima: afirmar `false` sobre un
# arbol sucio seria publicar una identidad que miente.
RELEASE_BUILD_SHA="$(git rev-parse HEAD 2>/dev/null || true)"
# Mismo predicado que `is_hex_sha` en crates/sddk-cli/build.rs: de 7 a 40
# caracteres, TODOS hexadecimales. La primera version de esta comprobacion
# anclaba solo los 7 primeros digitos con un glob `[0-9a-f]{7}*`, y eso
# aceptaba `abc1234 (HEAD detached)` — porque `*` se come lo que venga
# detras. La encontro el falsificador de este mismo commit, no la revision:
# un validador que acepta un valor con basura pegada no valida.
if ! [[ "$RELEASE_BUILD_SHA" =~ ^[0-9a-f]{7,40}$ ]]; then
    die "no se pudo resolver el commit a publicar (git rev-parse HEAD devolvio
         '$RELEASE_BUILD_SHA'). Un binario sin identidad declarada es
         indistinguible de uno obsoleto, que es INC-DEBT-064. No se publica."
fi

if [ -n "$(git status --porcelain --untracked-files=no 2>/dev/null)" ]; then
    die "el arbol tiene cambios RASTREADOS sin commitear en el paso 3 (build). El
         binario se construiria desde contenido que ningun commit identifica, y
         declararlo 'dirty: false' seria publicar una identidad que miente.
         El preflight (paso 0) ya exige arbol limpio; si has llegado aqui con el
         arbol sucio, algo lo ensucio entre medias. Ver INC-DEBT-064."
fi

# Sin seguimiento solo se bloquea lo que de verdad entra en el binario: un
# `crates/algo.rs` sin seguimiento lo compila cargo y ningun commit lo
# identifica, luego la identidad seria falsa por la misma razon que arriba.
# Un fichero suelto en docs/ o un log no cambia el binario, y bloquear la
# release por eso seria endurecer el gate del operador sin que nadie lo pidiera:
# el preflight (paso 0) usa `git diff --quiet`, que no ve ficheros sin
# seguimiento, y este paso no puede ser mas estricto que el sin avisar.
UNTRACKED_SOURCES="$(git status --porcelain 2>/dev/null \
    | sed -n 's/^?? //p' \
    | grep -E '^(Cargo\.(toml|lock)|crates/|build\.rs)' || true)"
if [ -n "$UNTRACKED_SOURCES" ]; then
    die "hay ficheros SIN SEGUIMIENTO que entran en el binario:
$UNTRACKED_SOURCES
         cargo los compila y ningun commit los identifica, luego el SHA que se
         declare no identifica el binario. Sube esos ficheros o muevelos antes
         de publicar. Ver INC-DEBT-064."
fi

export SDDK_GIT_SHA="$RELEASE_BUILD_SHA"
export SDDK_BUILD_DIRTY=false
printf '    identidad del binario: commit=%s dirty=false (SDDK_GIT_SHA)\n' "$SDDK_GIT_SHA"

cargo build --release --offline --bin sddk --target "$BUILD_TARGET" \
    || die "cargo build failed para target $BUILD_TARGET"

# Locate the binary via cargo metadata so we respect CARGO_TARGET_DIR.
TARGET_DIR="$(cargo metadata --format-version 1 --offline \
        | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])' \
        || true)"
BIN="$TARGET_DIR/$BUILD_TARGET/release/sddk"
[ -x "$BIN" ] || die "binary not found at $BIN"

# Verificar que el binario es REALMENTE lo que el nombre del asset promete.
# Un binario dinamico en un asset llamado musl es INC-021 reincidente y
# tiene que abortar ANTES de publicar, no despues. `file` es la fuente.
#
# INC-021-FALSE-NEGATIVE (session-30): este guard solo buscaba el literal
# "statically linked", pero `file` 5.46 describe un binario static-pie como
#
#   ELF 64-bit LSB pie executable, x86-64, static-pie linked, stripped
#
# sin la palabra "statically". El guard por tanto RECHAZABA el binario musl
# correcto — el fallo era del guard, no del toolchain. Se aceptan las dos
# grafias ("statically linked" y "static-pie linked"), que son las dos
# formas de un binario sin dependencias dinamicas, y se sigue rechazando
# cualquier "dynamically linked".
if [ "$BUILD_TARGET" = "x86_64-unknown-linux-musl" ]; then
    FILE_DESC="$(file -b "$BIN")"
    if printf '%s' "$FILE_DESC" | grep -qE 'statically linked|static-pie linked'; then
        ok "binario verificado estatico: $FILE_DESC"
    else
        die "el target de build es musl pero el binario NO es estatico.
         file dice: $FILE_DESC
         Publicar esto seria reincidir en INC-021. Verificar el linker
         (musl-tools / CC=musl-gcc) antes de reintentar."
    fi
fi
# `sddk --version` escribe en STDERR, no en stdout. Con `$(...)` a secas esta
# linea imprimia la version vacia -- `binary: /ruta (), target=...` -- en la
# unica linea que dice que binario se va a publicar. El mismo defecto se
# encontro el dia antes en el guard de reconciliacion (INC-DEBT-060) y aqui
# seguia vivo: es la clase de error que aparece en dos sitios porque nadie
# ejecuto la linea y la leyo.
ok "binary: $BIN ($("$BIN" --version 2>&1)), target=$BUILD_TARGET"

# --- 3b. reconciliación del artefacto contra la autoridad ---

# INC-DEBT-060. El guard contrasta lo que `sddk cycle list` DECLARA contra las
# filas que la autoridad tiene para el proyecto que el propio producto declara.
# Es la unica comparacion con dientes, porque el total declarado sale del mismo
# vector que el comando emite y por eso `declarado == emitido` no puede fallar.
#
# POR QUE AQUI Y NO EN 1b. La 1b corre los shell tests ANTES de compilar, luego
# en ese punto no hay binario de release: el guard fallaria cerrado y mataria
# la release por un binario que todavia no existe. Este paso corre DESPUES del
# build, con lo que el guard reconcilia el ARTEFACTO QUE SE VA A PUBLICAR, que
# es mas fuerte que reconciliar un binario de desarrollo: es el mismo criterio
# que el gate de R1 pide para cerrar INC-DEBT-064.
#
# NO va detras de --skip-tests a proposito. --skip-tests dice "ya he corrido los
# gates"; esto no es un gate de codigo, es una comprobacion del artefacto, y es
# exactamente la que 9b no se deja saltar con --skip-install por el mismo motivo:
# publicar un binario cuya enumeracion no cuadra con su almacenamiento es
# exactamente el defecto que este guard existe para que no llegue a un tag.
#
# El log cae en $RELEASE_SCRATCH, que ya existe (linea 83) y que el trap de la
# linea 86 limpia: no hace falta mkdir, el directorio esta. Ademas TMPDIR apunta
# ahi, con lo que los temporales del propio guard caen en el scratch de la
# release y no en el /tmp del operador.
#
# Y por eso aqui caen los DOS, no solo el guard. Se conecto primero solo el
# guard, con el argumento de que su autofalsacion cuesta 36,5 s contra 3,6 s
# --medido con `time`-- y que re-falsar un guard que no ha cambiado no vale esa
# espera. `tests/test_gate_coverage.py` lo veto por la via correcta: sin runner
# y sin motivo en EXCEPTIONS es un FAIL, y los motivos que esa lista acepta son
# SEMANTICOS --un test que pasa sin medir, o uno que necesita contenedores--, no
# "es lento". Cuesta 40 s a una release que ya compila el workspace entero y
# construye en release: LA FALSACION OPCIONAL ES UNA FALSACION QUE NO CORRE, y
# una autofalsacion que solo corre cuando alguien edita el guard es una
# autofalsacion que envejece sin que nadie lo note.
step "3b/15 — reconciliación del artefacto contra la autoridad (INC-DEBT-060)"
RECON_LOG="$RELEASE_SCRATCH/reconciliation.log"
if SDDK_GUARD_BIN="$BIN" bash tests/test_cycle_list_total_reconciliation.sh \
        >"$RECON_LOG" 2>&1; then
    ok "reconciliación del artefacto: $(grep -m1 '^PASS=' "$RECON_LOG" || echo 'PASS')"
else
    tail -25 "$RECON_LOG" >&2
    die "la reconciliacion del artefacto no pasa, y el motivo concreto esta en
         $RECON_LOG (que se imprime arriba). El motivo puede ser que el binario
         declare una poblacion que su almacenamiento no tiene, o que no pueda
         ejecutar la comprobacion; NO se afirma aqui cual de los dos es, porque
         este bloque no lo midio -- solo midio que el guard no quedo en verde.
         Ver tests/test_cycle_list_total_reconciliation.sh e INC-DEBT-060."
fi

# La autofalsacion va en la MISMA release y no como excepcion, por el parrafo
# de arriba. Le pide al guard que siga distinguiendo el caso bueno de los nueve
# modos de mentira, y exige que cada comprobacion siga siendo load-bearing por
# separado: sin esto el guard puede rechazar las nueve por efecto colateral y
# seguir pareciendo que vigila.
step "3c/15 — autofalsación de la reconciliación (cada comprobación con dientes)"
RECON_MUT_LOG="$RELEASE_SCRATCH/reconciliation_mutation.log"
if SDDK_GUARD_BIN="$BIN" bash tests/test_cycle_list_total_reconciliation_mutation.sh \
        >"$RECON_MUT_LOG" 2>&1; then
    ok "autofalsación: $(grep -m1 '^PASS=' "$RECON_MUT_LOG" || echo 'PASS')"
else
    tail -25 "$RECON_MUT_LOG" >&2
    die "la autofalsacion de la reconciliacion falla: o el guard dejo de
         distinguir el caso bueno de los modos de mentira, o una de sus
         comprobaciones dejo de ser load-bearing, que es codigo muerto.
         Log: $RECON_MUT_LOG"
fi

# Los CUATRO estados de `binary.build_identity` de `dev doctor`. Se ejecuta en
# MODO DE UN SOLO BINARIO, y el modo existe por una medicion, no por comodidad:
# pasando el binario concluyente en los dos huecos, O2, O3, O4, O5 y O7 pasan y
# SOLO O6 falla -- O6 es el unico objetivo que depende de la procedencia NO
# concluyente, y los demas dependen solo de la identidad concluyente y del
# checkout contra el que se compara.
#
# MEDIDO Y POR QUE NO SE CONSTRUYE EL SEGUNDO BINARIO AQUI: una compilacion en
# frio del perfil debug son 118,77 s (medido con `time`, target dir limpio).
# Pagar ~2 min por publicacion para medir UN estado --el de STOP 6, en el que el
# check por diseno NO decide-- no es lo que este paso tiene que decidir. El modo
# de uno DECLARA O6 como NOT_RUN con su motivo y baja la cuenta de veredictos de
# 5 a 4, que es lo que de verdad se midio; no lo cuenta como passed ni lo salta
# en silencio. Para la version completa con los dos binarios, construirlos fuera
# y pasar las dos rutas, que es como la uso el verify de cl-doctor-build-identity.
#
# LO QUE ESTE PASO NO CUBRE, y queda escrito para que no se lea al reves: el
# estado de procedencia no concluyente sigue siendo cobertura de SESION, no de
# pipeline. Y eso no se arregla con este paso: se arregla con el arnes que
# construya los dos binarios, que es trabajo declarado, no una decision tomada
# aqui.
step "3d/15 — los cuatro estados de binary.build_identity (dev doctor)"
IDENT_LOG="$RELEASE_SCRATCH/doctor_identity_states.log"
if bash tests/test_doctor_identity_states.sh "$BIN" >"$IDENT_LOG" 2>&1; then
    ok "estados de la identidad: $(grep -m1 '^PASS=' "$IDENT_LOG" || echo 'PASS') (O6 declarado NOT_RUN)"
else
    tail -25 "$IDENT_LOG" >&2
    die "dev doctor no distingue los estados de identidad sobre el binario que se
         va a publicar. Un rojo falso aqui es PEOR que no comprobar: entrena a
         ignorar los rojos de doctor. Log: $IDENT_LOG"
fi

# 3e entra porque su comprobacion (e) es la que impide que este repo vuelva a
# tener el conjunto de niveles exigentes declarado en dos sitios que no coinciden
# -- que es exactamente como se produjo el hueco de 13 filas de C3n.1. Y porque
# la mutacion que hace (e) escribe sobre el SPEC, no sobre una copia: por eso el
# paso va aqui, con el arbol ya en su forma final, y no en el 1b.
step "3e/15 — receipt de frontera exigible (C3n.1 / AT-UAT-023)"
UAT_BOUNDARY_LOG="$RELEASE_SCRATCH/uat_boundary_receipt.log"
if bash tests/test_uat_boundary_receipt.sh >"$UAT_BOUNDARY_LOG" 2>&1; then
    ok "receipt de frontera: $(grep -m1 '^PASS=' "$UAT_BOUNDARY_LOG" || echo 'PASS')"
else
    tail -25 "$UAT_BOUNDARY_LOG" >&2
    die "un receipt de UAT en un nivel que exige frontera no la declara, o el
         conjunto canonico ha divergido de la autoridad. La segunda causa es la
         grave: significa que el gate estaria validando contra un conjunto que
         ya no es el de ROADMAP-ACCEPTANCE-TRUTHFULNESS.md §C3n.1.
         Log: $UAT_BOUNDARY_LOG"
fi

# 3f y 3g van juntos porque son las dos mitades de la regla 4 de
# ACCEPTANCE-TRUTHFULNESS-MATRIX.md, que decia -- falsamente medido -- que C3n.1
# era su aplicacion mecanica. C3n.1 no la cubria. Y el guard, al construirse,
# encontro el defecto que la regla describe: cuatro de los cinco tests de AIW-S7a
# se llamaban *_e2e sin cruzar ninguna frontera. Corregido con rename.
step "3f/15 — la regla 4: un nombre de test no promete una frontera que no cruza"
NAMES_LOG="$RELEASE_SCRATCH/uat_naming_policy.log"
if bash tests/test_uat_naming_boundary_policy.sh >"$NAMES_LOG" 2>&1; then
    ok "politica de nombres: $(grep -m1 '^PASS=' "$NAMES_LOG" || echo 'PASS')"
else
    tail -25 "$NAMES_LOG" >&2
    die "un nombre de test promete una frontera que su fila no declara, o el guard
         ha dejado de medir. Lo primero informa mal a quien lee el nombre sin
         abrir el fichero; lo segundo es peor, porque un guard que no mide no
         avisa de que no mide. Log: $NAMES_LOG"
fi

# La autofalsacion va en la MISMA release y no como excepcion, por la misma razon
# que 3c: una falsacion que solo corre cuando alguien edita el guard envejece
# sin que nadie lo note. Ademas, este guard tiene la propriedade de poder quedar
# VERDE con su propio veto desconectado, y eso solo se ve falsandolo.
step "3g/15 — autofalsación de la política de nombres (cada comprobación con dientes)"
NAMES_MUT_LOG="$RELEASE_SCRATCH/uat_naming_policy_mutation.log"
if bash tests/test_uat_naming_boundary_policy_mutation.sh >"$NAMES_MUT_LOG" 2>&1; then
    ok "autofalsación de nombres: $(grep -m1 '^PASS=' "$NAMES_MUT_LOG" || echo 'PASS')"
else
    tail -25 "$NAMES_MUT_LOG" >&2
    die "la autofalsacion de la politica de nombres no pasa: o una comprobacion
         del guard ha dejado de tener dientes, o una mutacion no llego a aplicarse
         y se esta contando como deteccion. Log: $NAMES_MUT_LOG"
fi

# 3h. Una cita de spec tiene que anclar a UN documento. Este guard cierra el
# hueco que el de citas (paso 1, linea 276) declara en su propio docstring: el
# token SPEC-NNN no estaba en su AUTHORITY_TOKENS, y 202 citas en crates/ no
# las vigilaba nadie. Va con su autofalsacion en la MISMA release, porque este
# guard tiene una propiedad particular -- con cero defectos que medir esta
# indistinguible de uno sin dientes -- y eso solo se ve sembrando el defecto.
step "3h/15 — una cita de spec ancla a un documento, y se le aplican seis mutaciones"
SPEC_CITATION_LOG="$RELEASE_SCRATCH/spec_citation_anchor.log"
if python3 tests/test_spec_citation_anchor.py >"$SPEC_CITATION_LOG" 2>&1; then
    ok "citas de spec ancladas: $(grep -m1 'citas ambiguas SIN ancla' "$SPEC_CITATION_LOG" || echo PASS)"
else
    tail -25 "$SPEC_CITATION_LOG" >&2
    die "una cita de spec no ancla a un documento. Un SPEC-NNN que sirve cuatro
         documentos no nombra ninguno, y el guard de citas la daba por buena
         porque el ID existia en algun sitio. Log: $SPEC_CITATION_LOG"
fi
SPEC_CITATION_MUT_LOG="$RELEASE_SCRATCH/spec_citation_anchor_mutation.log"
if python3 tests/test_spec_citation_anchor_mutation.py >"$SPEC_CITATION_MUT_LOG" 2>&1; then
    ok "autofalsación de citas de spec: $(grep -m1 '^  PASS=' "$SPEC_CITATION_MUT_LOG" | tr -s ' ')"
else
    tail -25 "$SPEC_CITATION_MUT_LOG" >&2
    die "la autofalsacion de citas de spec no pasa: o una propiedad dejo de tener
         dientes, o una mutacion cayo por una razon que no es la que dice medir.
         Log: $SPEC_CITATION_MUT_LOG"
fi

# --- 4. manifest ---

step "4/15 — regenerate MANIFEST.sha256"
"$BIN" dev manifest --root . --format text \
    || die "sddk dev manifest failed"
"$BIN" dev manifest --verify --root . --format text \
    || die "manifest verification failed (RDI)"
ok "MANIFEST.sha256 regenerated and verified"

# --- 5. bundle tarball ---

step "5/15 — bundle tarball"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP" "$RELEASE_SCRATCH"' EXIT

BUNDLE_TARBALL="$TMP/software-development-decision-kernel.tar.gz"
# Stage the EXACT tarball contents (repo surfaces + injected BUNDLE.toml) in
# one directory, then pack with the uniform `--xform` wrapper. Packing repo
# paths and an external file in one command does NOT work: the xform applies
# to every member, so an absolute second path leaks its mktemp prefix (two
# falsation runs failed before this layout; see session-34 receipt).
BUNDLE_STAGE="$TMP/bundle-stage/software-development-decision-kernel"
mkdir -p "$BUNDLE_STAGE"
# The staging is derived FROM the manifest, not from a hand-written surface
# list. Two defects forced this (session-65h, both measured):
#
#   (a) The `tar` below named surfaces the `cp -r` never staged, so the tar
#       failed on a non-existent member. `set -euo pipefail` turns that into an
#       aborted release, not a corrupt one -- but the release would not have
#       completed at all. A subdirectory surface (prompts/sddk,
#       docs/impeccable-reference) additionally needs its PARENT created
#       before `cp -r`, or it lands flat as dst/<leaf> and the tar, which asks
#       for the prefix path, finds nothing to stat.
#   (b) `cp -r <surface>` copies whatever is on disk, INCLUDING gitignored
#       files the manifest does not list. Measured on 2.5.3: the bundle shipped
#       agents/.atl/.skill-registry.cache.json and assets/agent-models.yaml.bak
#       (both matched by .gitignore:26 / :17) as bundle content outside the
#       manifest, so `manifest_sha256` in BUNDLE.toml did not describe the
#       tarball and an install could not verify them.
#
# The manifest is already the authoritative statement of what ships (step 4
# verifies it, fail-closed). Reading the staging from it makes that authority
# load-bearing for BOTH what is included and what is excluded, and removes the
# hand-maintained list that had to be edited every time a surface was added.
awk '{print $2}' MANIFEST.sha256 | xargs -d '\n' cp --parents -t "$BUNDLE_STAGE"
# MANIFEST.sha256 must ship too, and the manifest cannot list itself: a file
# cannot contain its own digest. Deriving the staging from the manifest
# therefore drops the one file the bundle most needs — OBSERVED end-to-end in
# session-65h, where an extracted tarball had no MANIFEST.sha256 and
# release.yml:230 ("bundle lacks MANIFEST.sha256") plus
# `update.rs` (required) would both have refused it. Add it explicitly, and
# let the fail-closed check below confirm the result is exactly
# manifest-paths + MANIFEST.sha256 + BUNDLE.toml.
cp MANIFEST.sha256 "$BUNDLE_STAGE/"
# BUNDLE.toml must ship INSIDE the standalone bundle tarball. It used to be
# written only AFTER the tar was created (step 6), so the published tarball
# never contained it: `sddk dev update` resolves its layout from BUNDLE.toml,
# finds it missing, and falls back to the legacy ROOT layout, whose
# destructive swap deleted the version dirs and `current` that a previous
# install.sh had created (observed live against v2.2.23, session-34). The
# unified tarball (step 7) picked BUNDLE.toml up via `cp -r`, which is why
# install.sh installs were unaffected.
# The manifest's OWN sha256, not the hash of its first line (INC-DEBT-025,
# long comment in step 6 below).
# The `sha256:` prefix matches `dev manifest --bundle` (manifest.rs) and both
# cloud jobs, so the three producers stop diverging. `verify_manifest_anchor`
# normalises both forms, so this is convergence, not a correctness change;
# tests/test_release_ci_manifest_anchor.sh pins all three.
MANIFEST_SHA="$(sha256sum MANIFEST.sha256 | awk '{print $1}')"
printf '%s\n' \
    '[bundle]' 'schema_version = 2' \
    "version = \"$VERSION\"" \
    "binary_min_version = \"$VERSION\"" \
    "binary_max_version = \"$VERSION\"" \
    '' '[contents]' "manifest_sha256 = \"sha256:$MANIFEST_SHA\"" \
    > "$BUNDLE_STAGE/BUNDLE.toml"
# Pack the staged tree. The `--xform` prefix must be applied to the members
# RELATIVE to the staging parent, never to the wrapper directory itself:
# `tar -C $parent software-development-decision-kernel --xform 's|^|.../|'`
# transforms the MEMBER NAME, which is already `software-development-decision-kernel`,
# and yields `software-development-decision-kernel/software-development-decision-kernel/…`
# — a doubled prefix on all 396 members. OBSERVED end-to-end in session-65h:
# every consumer check ran against a path one level too deep. `cd` into the
# parent and pass `.` so the members are `./agents/...` and the xform produces
# the single `software-development-decision-kernel/` prefix the contract
# expects (checked by `grep -qx` on the tar listing, below).
tar czf "$BUNDLE_TARBALL" \
    --xform "s|^\./|software-development-decision-kernel/|" \
    -C "$TMP/bundle-stage" software-development-decision-kernel
sha256sum "$BUNDLE_TARBALL" | awk '{print $1}' > "$BUNDLE_TARBALL.sha256"
# Contract check: the standalone tarball MUST carry BUNDLE.toml now.
tar tzf "$BUNDLE_TARBALL" | grep -qx "software-development-decision-kernel/BUNDLE.toml" \
    || { echo "FATAL: bundle tarball is missing software-development-decision-kernel/BUNDLE.toml" >&2; exit 1; }
# Contract check (INC-DEBT-056, session-65h): the staged tree must be EXACTLY
# the manifest's paths plus the two files the manifest cannot list itself —
# MANIFEST.sha256 (a file cannot contain its own digest) and BUNDLE.toml
# (injected). This is the check that would have caught the gitignored-artefact
# leak. It compares the staged tree on disk rather than the tar listing, so a
# member that silently went missing is caught too.
DIFF_OUT="$(diff <(awk '{print $2}' MANIFEST.sha256 | sort) \
                 <(cd "$BUNDLE_STAGE" && find . -type f -printf '%P\n' \
                    | grep -vx -e '^BUNDLE.toml$' -e '^MANIFEST\.sha256$' | sort) || true)"
[ -z "$DIFF_OUT" ] || { echo "FATAL: staged bundle does not match the manifest:" >&2; echo "$DIFF_OUT" >&2; exit 1; }
for REQUIRED in MANIFEST.sha256 BUNDLE.toml; do
    [ -f "$BUNDLE_STAGE/$REQUIRED" ] \
        || { echo "FATAL: bundle is missing required $REQUIRED" >&2; exit 1; }
done
ok "bundle: $(basename "$BUNDLE_TARBALL") ($(stat -c%s "$BUNDLE_TARBALL") bytes, BUNDLE.toml included)"
ok "bundle staging matches MANIFEST.sha256 exactly ($(awk 'END{print NR}' MANIFEST.sha256) + MANIFEST.sha256 + BUNDLE.toml)"

# --- 6. BUNDLE.toml ---

step "6/15 — BUNDLE.toml in tarball (verified in step 5)"
# BUNDLE.toml is now written INTO the standalone tarball by step 5 (the
# stage-then-pack layout); this step only re-derives FW_DIR for the unified
# tarball below. Kept as an extraction+assertion so a future refactor of
# step 5 that drops the file fails here, loudly, before anything publishes.
BUNDLE_DIR="$TMP/bundle"
mkdir -p "$BUNDLE_DIR"
tar xzf "$BUNDLE_TARBALL" -C "$BUNDLE_DIR"
# The manifest's OWN sha256, not the hash of its first line.
#
# `awk 'NR==1 {print $1}' MANIFEST.sha256` returned the sha256 of the first
# FILE listed in the manifest (e.g. agents/analytics-judge.md), which has
# nothing to do with the manifest itself — the field then named one bundled
# file while claiming to be the manifest digest. The consumer's own doc
# comment states the intent: "sha256 of MANIFEST.sha256 itself"
# (crates/sddk-cli/src/dev/bundle_manifest.rs:24).
#
# Nothing validated the old value (install.sh never reads it; the Rust side
# parses it as an Option and never compares), so the field was inert AND
# wrong. Fixing it changes a published value, which is safe for that
# reason. See INC-DEBT-025-MANIFEST-SHA-FROM-FIRST-LINE.
FW_DIR="$BUNDLE_DIR/software-development-decision-kernel"
[ -f "$FW_DIR/BUNDLE.toml" ] || { echo "FATAL: step 5 did not ship BUNDLE.toml inside the tarball" >&2; exit 1; }
SHIPPED_VERSION="$(awk -F'"' '/^version = /{print $2; exit}' "$FW_DIR/BUNDLE.toml")"
[ "$SHIPPED_VERSION" = "$VERSION" ] || { echo "FATAL: BUNDLE.toml version $SHIPPED_VERSION != $VERSION" >&2; exit 1; }
ok "BUNDLE.toml inside tarball (version=$SHIPPED_VERSION)"

# --- 7. unified tarball ---

step "7/15 — unified tarball (bin/sddk + framework/)"
UNIFIED="$TMP/sddk-${TAG}-sddk-linux-x86_64-musl.tar.gz"
PACK="$TMP/pack"
rm -rf "$PACK"
mkdir -p "$PACK/bin" "$PACK/framework"
chmod 0755 "$BIN"
cp "$BIN" "$PACK/bin/sddk"
# Copy the CONTENTS of FW_DIR (which is wrapped under
# software-development-decision-kernel/) directly into pack/framework/, so
# install.sh finds BUNDLE.toml / MANIFEST.sha256 at framework/BUNDLE.toml
# (its expected location) instead of framework/software-development-decision-kernel/BUNDLE.toml.
cp -r "$FW_DIR/." "$PACK/framework/"
tar -C "$PACK" -czf "$UNIFIED" bin framework
chmod 0755 "$PACK/bin/sddk"  # post-extract defensive chmod (already 0755)
# Verify the exec bit survives in the archive AND that BUNDLE.toml lives
# at the install.sh-expected path (not nested under software-development-decision-kernel/).
# CRITICAL: do NOT use `grep -q` here. With `set -o pipefail`, `grep -q`
# exits on the first match and closes stdin, which causes `tar` to receive
# SIGPIPE (exit 141) and the pipeline to be reported as failed even though
# grep found the match. Capture tar output to a variable and grep it
# afterwards.
TAR_LISTING="$(tar tvzf "$UNIFIED")"
if ! grep -q -- '-rwxr-xr-x.* bin/sddk' <<<"$TAR_LISTING"; then
    die "unified tarball lost the exec bit on bin/sddk — refusing to ship"
fi
if ! grep -q -- 'framework/BUNDLE.toml$' <<<"$TAR_LISTING"; then
    die "unified tarball lacks framework/BUNDLE.toml at the install.sh-expected path"
fi
sha256sum "$UNIFIED" | awk '{print $1}' > "$UNIFIED.sha256"
ok "unified: $(basename "$UNIFIED") ($(stat -c%s "$UNIFIED") bytes, exec bit + BUNDLE.toml OK)"

# --- 8. checksums + sbom ---

step "8/15 — sha256 + CHECKSUMS + sbom.json"
BIN_SHA="$(sha256sum "$BIN" | awk '{print $1}')"
# El binario DESNUDO se copia a $TMP junto a su .sha256, y no por comodidad.
#
# El bucle de firma (8c) resuelve sus tres artefactos como "$TMP/$artifact", y
# el binario solo vivia en $BIN y dentro de $PACK/bin/sddk — luego en $TMP no
# estaba. Consecuencia medida, no supuesta: el bucle hacia `continue` sin
# incrementar el contador, el "all-or-nothing" contaba 2 de 3 y la release
# moria. **Las dos vias de firma estaban rotas por lo mismo**: con clave, el
# binario nunca se firmaba; con SDDK_SKIP_SIGNING=1, el contador nunca llegaba
# a 3 y la via documentada para publicar sin firmar era insatisfacible. El
# mensaje de error decia "set SDDK_SKIP_SIGNING=1" y eso no hacia nada.
#
# El arreglo es que $TMP sea de verdad el unico sitio donde vive cada
# artefacto firmable, que es lo que el bucle ya asumia. La alternativa —
#ensenar al bucle que el primero vive en $BIN— anade un caso especial y otra
# fuente de verdad sobre el layout, que es la clase de defecto que este repo
# ya ha pagado dos veces.
cp "$BIN" "$TMP/$(basename "$BIN")"
echo "$BIN_SHA  $(basename "$BIN")" > "$TMP/$(basename "$BIN").sha256"
( cd "$TMP" && sha256sum "$(basename "$UNIFIED")" "$(basename "$BUNDLE_TARBALL")" ) \
    > "$TMP/CHECKSUMS"
cat > "$TMP/sbom.json" <<EOF
{"bomFormat":"CycloneDX","specVersion":"1.5","version":1,"components":[{"type":"application","name":"sddk","version":"$VERSION","purl":"pkg:generic/sddk@$VERSION"}]}
EOF
ok "checksums + sbom ready (binary sha256: ${BIN_SHA:0:16}…)"


# --- 8b. vault ADR mirror sync (best-effort, fail-soft) ---
#
# INC-VAULT-MIRROR-AUTO: vault mirrors at
# ~/.sddk-knowledge/sddk-framework/adrs/ must stay in sync with the
# accepted ADRs in docs/architecture/adrs/. Without this step the
# operator must invoke `python3 scripts/mirror_adrs_to_vault.py` manually
# after each release — easy to forget, drifts the human-knowledge source
# from the runtime authority.
#
# The mirror script is idempotent (skips existing mirrors) and the
# vault is human knowledge per AGENTS §2.7 (the repo ADR remains
# canonical). Therefore this step is best-effort:
#   - exit 0 → log counts as ok
#   - non-zero exit → log as warn, do NOT abort the release; the bump
#     commit is the canonical record and is already published
#
# Always executed (also under --skip-tests and --dry-run) — vault sync
# is part of the release contract, not the test gate.

step "8b/15 — vault ADR mirror sync (closes INC-VAULT-MIRROR-AUTO)"
if MIRROR_OUT="$(python3 "$ROOT/scripts/mirror_adrs_to_vault.py" 2>&1)"; then
    ok "vault mirror sync: $(echo "$MIRROR_OUT" | tr '\n' ' ')"
else
    warn "vault mirror sync failed (non-fatal — repo ADR is canonical): $MIRROR_OUT"
fi

if [ "$DRY_RUN" = "1" ]; then
    ok "dry-run: stopping before gh release create"
    echo
    echo "Assets staged in $TMP:"
    ls -la "$TMP"
    exit 0
fi

# --- 8c. cosign signatures (authenticity) ---
#
# INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY option (a). install.sh and
# `sddk dev update` now refuse an unsigned artifact unless the operator
# opts out explicitly. This step is what makes the default path work: the
# release publishes the signatures both consumers require.
#
# Sign the three consumable artifacts: the bare binary, the unified
# tarball, and the bundle tarball. Signatures travel as `.sig` (detached)
# AND `.bundle.json` (the current cosign format), because the two
# consumers probe for both — a signature published in only one format
# would be a coin flip on which path works.
#
# KEYLESS IS THE DEFAULT and the only option that avoids a long-lived
# signing key on the release host. It needs OIDC: in CI that is
# ACTIONS_ID_TOKEN_REQUEST; locally cosign falls back to a device flow
# that requires a human to open a browser. If neither is available, the
# release ABORTS rather than shipping unsigned artifacts whose consumers
# will reject — publishing artifacts that the installer refuses is
# strictly worse than failing here.
step "8c/15 — cosign signatures"

# Pull the base64 signature out of a cosign bundle so a detached `.sig`
# can be published alongside it. The bundle is the current format; the
# detached form is what older cosign reads via `--signature`. Publishing a
# file that merely *looks* like a signature would be worse than publishing
# none: it either fails verification confusingly or, in the worst case,
# gets treated as valid.
extract_detached_sig() {
    local bundle_path="$1" out_path="$2"
    python3 - "$bundle_path" "$out_path" <<'PY'
import base64, json, sys
bundle = json.load(open(sys.argv[1]))
sig = (bundle.get("base64Signature")
       or bundle.get("signedBlob", {}).get("signature")
       or bundle.get("dsseEnvelope", {}).get("payload"))
if not sig:
    sys.exit(1)
open(sys.argv[2], "wb").write(base64.b64decode(sig))
PY
}

# The OIDC issuer the certificate MUST carry. Byte-identical to
# DEFAULT_CERT_ISSUER in crates/sddk-cli/src/cosign.rs, which is the single
# source of truth both consumers verify against. If these drift, the release
# signs with one identity and the installers demand another.
#
# Why this is a hard failure and not a warning: INC-DEBT-024. The keyless
# identity depends on WHERE you sign, and the two are mutually exclusive —
# GitHub Actions mints issuer token.actions.githubusercontent.com, the local
# device flow mints oauth2.sigstore.dev/auth with a PERSON's subject. cosign
# is installed either way, so `command -v cosign` passes locally and the
# signing step runs. The result is a release that looks correctly signed and
# that this project's own install.sh refuses, because the pin cannot match.
# Publishing that is the worst outcome available: the failure surfaces at the
# user's install, with a signature error that reads like tampering.
SIGN_ARTIFACTS=(
    "$(basename "$BIN")"
    "$(basename "$UNIFIED")"
    "$(basename "$BUNDLE_TARBALL")"
)

# ── Politica de autenticidad del 9c (una sola decision, derivada) ───────────
# Vive aqui, y no dentro del paso 9c, por una razon que ya ha salido cara dos
# veces en este repo: una decision que solo existe dentro de un `if` anidado
# no se puede ejecutar fuera de el, luego ningun guard la puede falsificar y
# la unica forma de comprobarla es leerla. Leyendola se ve que de la tercera
# salida —unsigned— no habia ninguna, que es exactamente por lo que una
# release publicada sin firma moria en 9c con un 404.
#
# UNSIGNED NO significa "el flag lo dice": significa "el flag lo dice Y no hay
# ninguna firma sobre la mesa". Con firmas presentes se verifica igual, porque
# una propiedad de supply-chain que se puede comprobar y se deja sin comprobar
# por obedecer una bandera es un downgrade silencioso.
release_authenticity_posture() {
    local sig_files_present="${1:-0}"
    if [ "${SDDK_SKIP_SIGNING:-0}" = "1" ] && [ "$sig_files_present" -eq 0 ]; then
        echo "UNSIGNED"
        return 0
    fi
    if [ "${SDDK_SKIP_AUTHENTICITY_CHECK:-0}" = "1" ]; then
        echo "DECLARED_SKIP"
        return 0
    fi
    echo "VERIFY"
    return 0
}

# --- 8c. cosign signatures (key-based anchor, ADR-0151) -------------------
#
# The anchor used to be a Fulcio certificate minted by GitHub Actions'
# OIDC provider, which only exists inside an Actions runner. This project no
# longer uses Actions as its CI, so requiring one made releases impossible
# to produce: the v2.5.3 attempt aborted here at step 8c. The anchor is now
# key-based and the signing key lives in a KMS (ADR-0151).
#
# The invariant this block still enforces is the one that matters: PUBLISHING
# A SIGNATURE NOBODY CAN VERIFY. It is checked below by verifying what was
# actually produced against the pinned public key, which is strictly
# stronger than reading a certificate issuer — the old check proved "signed
# by something", the new one proves "signed by us".
#
# SDDK_RELEASE_SIGNING_KEY selects the signer:
#   * a KMS reference (awskms://…, gcpkms://…, azurekms://…, hashivault://…)
#     — the private key never leaves the KMS. This is the supported path.
#   * a path to a private key file — NOT the supported path. A file is a
#     secret that can be copied out of a backup; it exists so the signing
#     half can be exercised in tests without a cloud account. Refused unless
#     the operator says so explicitly.
#
# Cosign reads COSIGN_PASSWORD from the environment. It must never be
# exported or logged: this script never echoes the signing ref beyond its
# scheme.
signing_ref="${SDDK_RELEASE_SIGNING_KEY:-}"

if [ "${SDDK_SKIP_SIGNING:-0}" != "1" ]; then
    if [ -z "$signing_ref" ]; then
        die "no signing key configured.
               The release anchor is key-based as of ADR-0151, so a release
               needs a signer. Set SDDK_RELEASE_SIGNING_KEY to a KMS
               reference, e.g.:

                 SDDK_RELEASE_SIGNING_KEY=awskms:///projects/P/locations/global/keyRings/SDDK/cryptoKeys/release \
                 bash scripts/release.sh

               To publish fully unsigned on purpose, set SDDK_SKIP_SIGNING=1 —
               the installers will then require SDDK_ALLOW_UNSIGNED=1."
    fi

    # The anchor the consumers pin. If it is still the transition marker,
    # publishing would ship a release nobody can install.
    anchor_file="$ROOT/assets/trust/release-verify-key.pub"
    anchor_body="$(tr -d '\n' < "$anchor_file" 2>/dev/null | sed 's/[[:space:]]*$//')"
    if [ -z "$anchor_body" ] || [ "$anchor_body" = "@@SDDK_TRANSITION_ANCHOR_NOT_A_REAL_KEY@@" ]; then
        die "the release signing key has not been provisioned.
               $anchor_file holds the transition placeholder, not a public key.

               Create the key in the KMS, then copy its public key body (one
               line, base64, no PEM framing) into BOTH:
                 - $anchor_file
                 - scripts/install.sh (SDDK_RELEASE_VERIFY_KEY_BODY)

               tests/test_install_asset_contract.sh fails until the two
               agree. Refusing to publish: a release signed by a key its own
               installers cannot verify is worse than no release. See
               ADR-0151."
    fi

    # A private key FILE is the shape that breaks ADR-0151's premise. It is
    # refused by default so the cheap path is not taken by accident; the
    # escape hatch exists so CI can exercise the signing path without a cloud
    # account.
    case "$signing_ref" in
        *.key|*.pem|/*|*.json)
            if [ "${SDDK_ALLOW_FILE_SIGNER:-0}" != "1" ]; then
                die "SDDK_RELEASE_SIGNING_KEY looks like a private key FILE.
                       That is not the supported path under ADR-0151: the point
                       of the KMS is that the private key never lands on a host.

                       Use a KMS reference (awskms://…, gcpkms://…,
                       azurekms://…, hashivault://…). If this is a test run,
                       set SDDK_ALLOW_FILE_SIGNER=1 to acknowledge the
                       difference."
            fi
            warn "SDDK_ALLOW_FILE_SIGNER=1 — signing from a private key FILE."
            warn "  The key material exists on this host. Never use this for a real release."
            ;;
    esac
    # Only the scheme is printed, never the reference itself: a KMS reference
    # names a key ring and a crypto key, and this script does not put one in a
    # log. An unrecognised scheme is not rejected here — cosign rejects it at
    # signing time with a precise error, and a check here would only duplicate
    # that with a worse message. The claim is deliberately narrow: an anchor is
    # PRESENT, not that it is correct. Correctness is settled below, by
    # verifying the signed artifact against it, which is the only check that
    # cannot be satisfied by a signer's optimism.
    ok "signing anchor present; signer scheme: ${signing_ref%%:*}"
fi

SIGNED_COUNT=0
if command -v cosign >/dev/null 2>&1; then
    for artifact in "${SIGN_ARTIFACTS[@]}"; do
        src_artifact="$TMP/$artifact"
        [ -f "$src_artifact" ] || { warn "artifact missing for signing: $artifact"; continue; }
        if [ "${SDDK_SKIP_SIGNING:-0}" = "1" ]; then
            SIGNED_COUNT=$((SIGNED_COUNT + 1))
            continue
        fi
        # --new-bundle-format is required by cosign v3.x for --bundle;
        # observed on v3.1.3, where the older --output-signature form was
        # removed. The detached .sig is extracted FROM the bundle so it is a
        # real signature rather than a copy of the artifact.
        if COSIGN_PASSWORD="${COSIGN_PASSWORD:-}" cosign sign-blob --yes \
             --new-bundle-format --key "$signing_ref" \
             --bundle "$TMP/$artifact.bundle.json" "$src_artifact" \
             2>"$TMP/sign-$artifact.log"; then
            extract_detached_sig "$TMP/$artifact.bundle.json" "$TMP/$artifact.sig" \
                || warn "could not extract detached .sig from bundle for $artifact"
            ok "signed: $artifact (.sig + .bundle.json)"
            SIGNED_COUNT=$((SIGNED_COUNT + 1))
        else
            warn "cosign could not sign $artifact:"
            sed 's/^/    /' "$TMP/sign-$artifact.log" | head -5
        fi
    done

    # ── The invariant: what was published must verify against the pin ──
    #
    # The old check read the issuer out of the minted certificate and
    # compared it to a constant. That proved "signed by something that
    # Fulcio vouched for", and it ran AFTER signing, so a laptop-signed
    # release had already cost a browser round-trip before being refused.
    #
    # This verifies the artifact against the SAME key the installers pin, so
    # the two cannot disagree. It is a real check: a wrong key, a corrupted
    # bundle or a signature that never landed all fail here rather than in
    # the user's terminal.
    if [ "${SDDK_SKIP_SIGNING:-0}" != "1" ] && [ "$SIGNED_COUNT" -gt 0 ]; then
        anchor_pem="$TMP/release-anchor.pem"
        {
            printf -- '-----BEGIN PUBLIC KEY-----\n'
            printf -- '%s\n' "$anchor_body"
            printf -- '-----END PUBLIC KEY-----\n'
        } > "$anchor_pem"

        first_artifact="${SIGN_ARTIFACTS[0]}"
        if cosign verify-blob --key "$anchor_pem" \
             --bundle "$TMP/$first_artifact.bundle.json" "$TMP/$first_artifact" \
             >/dev/null 2>&1; then
            ok "signing identity verified: the release verifies against the pinned anchor"
        else
            die "the signed artifact does NOT verify against the pinned public key.

               This is the release nobody can install: install.sh and
               \`sddk dev update\` pin the anchor in
               assets/trust/release-verify-key.pub, and this
               artifact does not satisfy it.

               Refusing to publish. Check that SDDK_RELEASE_SIGNING_KEY is the
               private key matching that anchor, and that the anchor file and
               scripts/install.sh carry the same body."
        fi
    fi

    # All-or-nothing. A partially signed release is worse than an unsigned
    # one: the binary verifies while the bundle tarball does not, so the
    # failure lands on the user at install time instead of here.
    if [ "$SIGNED_COUNT" -ne "${#SIGN_ARTIFACTS[@]}" ]; then
        die "signed $SIGNED_COUNT of ${#SIGN_ARTIFACTS[@]} artifacts. Refusing to publish a partial set: a release whose bundle tarball has no signature is a release that install.sh refuses, and the user finds out instead of us.

         To sign, set SDDK_RELEASE_SIGNING_KEY to a KMS reference. Or set
         SDDK_SKIP_SIGNING=1 to publish fully unsigned on purpose — the
         installers will then require SDDK_ALLOW_UNSIGNED=1."
    fi
    if [ "${SDDK_SKIP_SIGNING:-0}" = "1" ]; then
        warn "SDDK_SKIP_SIGNING=1 — publishing UNSIGNED artifacts. Both installers"
        warn "will refuse this release unless the operator opts out explicitly."
    else
        ok "$SIGNED_COUNT/${#SIGN_ARTIFACTS[@]} artifacts signed"
    fi
else
    if [ "${SDDK_SKIP_SIGNING:-0}" = "1" ]; then
        warn "cosign not installed; SDDK_SKIP_SIGNING=1 — publishing UNSIGNED artifacts."
    else
        die "cosign is not installed. Install cosign, or set SDDK_SKIP_SIGNING=1 to accept
         that knowingly — both installers require a signature, so an unsigned
         release would be uninstallable. See ADR-0151 and
         INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY."
    fi
fi

# --- 9. publish ---

step "9/15 — gh release create $TAG"
# Anchor the release to the current branch (not the tag SHA). `gh release
# create --target` accepts a branch name or tag name; passing the raw SHA
# of HEAD fails with HTTP 422 ("Release.target_commitish is invalid")
# because that SHA has no ref yet. Using the current branch lets `gh`
# resolve to HEAD automatically and avoids the manual `gh release edit
# --target` repoint that v1.89.7 needed.
RELEASE_TARGET="$(git rev-parse --abbrev-ref HEAD)"
ok "release target: $RELEASE_TARGET"

# ARCH-HEX-001 slice 3: GitHub Releases is a System-only writable surface
# (ADR-069 §3, row 7). Emit an actor-kind authority receipt via
# scripts/release-receipt.sh and fail-closed if the actor is not System.
# The helper applies the locked v1.81.x prefix heuristic
# (crates/sddk-engine/src/authority.rs::infer_actor_kind) and writes a
# JSON receipt that downstream tooling can consume. Same shape as the
# engine-side checks introduced in v1.168.22 (apply_cycle_start) and
# v1.168.24 (sddk dev install).
RECEIPT_PATH="$TMP/gh-release-receipt.json"
RECEIPT_ACTOR="${SDDK_ACTOR:-system}"
bash "$ROOT/scripts/release-receipt.sh" \
    --actor-id "$RECEIPT_ACTOR" \
    --tag "$TAG" \
    --out "$RECEIPT_PATH" \
    || die "github_releases authority check failed (actor_kind must be System); refusing to publish"
ok "github_releases receipt: $RECEIPT_PATH (actor=$RECEIPT_ACTOR)"

RECEIPT_SUMMARY="$(python3 -c '
import json, sys
with open(sys.argv[1]) as f:
    data = json.load(f)
ak = data["actor_kind"]
aid = data["actor_id"]
sv = data["schema_version"]
print("actor_kind=" + ak + " actor_id=" + aid + " schema_version=" + str(sv))
' "$RECEIPT_PATH")"

RELEASE_ARGS=(
    "$TAG"
    --repo "$REPO"
    --target "$RELEASE_TARGET"
    --title "sddk $TAG"
    --notes "Release $TAG — published by scripts/release.sh.

ARCH-HEX-001 receipt: ${RECEIPT_SUMMARY}"
)
# Note: --clobber is NOT supported on `gh release create` in gh <2.99 (only on `upload`).
# We pass --clobber to `gh release upload` below, which is the path that actually needs it.
# Forcing re-create of an existing release is handled by deleting first.

ASSETS=(
    # El binario se publica desde la copia de $TMP, la misma que firma el 8c.
    # Con "$BIN" aqui se publicaba un fichero y se firmaba otro: los bytes
    # eran iguales por construccion, no por garantia, y una release cuya firma no
    # corresponde a lo publicado es un fallo que solo aparece en el usuario.
    "$TMP/$(basename "$BIN")"
    "$TMP/$(basename "$BIN").sha256"
    "$TMP/CHECKSUMS"
    "$TMP/sbom.json"
    "$UNIFIED"
    "$UNIFIED.sha256"
    "$BUNDLE_TARBALL"
    "$BUNDLE_TARBALL.sha256"
    "$RECEIPT_PATH"
)
# If step 1d ran, include the EXT-RECEIPT.md as a release asset so the
# provider-exercised evidence ships alongside the binary. The release
# gate (step 9b) does not poll this asset — it is evidence-of-record,
# not part of the 9-asset public-release contract — but adding it here
# keeps the EXT profile auditable from the GitHub Release page itself.
if [ -n "$EXT_RECEIPT_DIR" ] && [ -f "$EXT_RECEIPT_DIR/EXT-RECEIPT.md" ]; then
    ASSETS+=("$EXT_RECEIPT_DIR/EXT-RECEIPT.md")
fi

# Signature assets. Both consumers (install.sh and `sddk dev update`) look
# for `.bundle.json` first and fall back to `.sig`, so both must ship. They
# are additive to the 9-asset canonical contract, not part of it: the
# canonical list below is a public-release gate and its shape is defined
# elsewhere (tests/test_release_public_gate.sh).
for sig_artifact in "${SIGN_ARTIFACTS[@]}"; do
    # .bundle.json already carries the certificate, so the bundle path works
    # on its own. The .pem is published anyway so the DETACHED path works
    # too: the CI signs detached, and both consumers now require a
    # certificate before accepting a detached signature. Shipping only the
    # bundle would make the two signing paths produce releases that are
    # mutually un-verifiable depending on which consumer fetched what.
    for ext in .sig .bundle.json .pem; do
        if [ -f "$TMP/$sig_artifact$ext" ]; then
            ASSETS+=("$TMP/$sig_artifact$ext")
        fi
    done
done

if gh release view "$TAG" --repo "$REPO" \
        >/dev/null 2>&1; then
    if [ "$FORCE" = "1" ]; then
        gh release upload "$TAG" --repo "$REPO" \
            --clobber "${ASSETS[@]}" \
            || die "gh release upload --clobber failed"
    else
        die "release $TAG already exists — pass --force to overwrite"
    fi
else
    gh release create "${RELEASE_ARGS[@]}" "${ASSETS[@]}" \
        || die "gh release create failed"
fi
ok "release $TAG published"

# --- 9b. public-release gate ---
# >>> REL-1 public-release gate begin >>>
# Closes FU-A4-4A-REL-1. Before the install step, assert the GitHub
# Release is publicly distributable. Fail closed on any mismatch.
#
# Skipped under --dry-run (no publish happened) and --skip-install
# (no install will run, so the gate is moot).
if [ "$DRY_RUN" = "1" ]; then
    warn "skipping step 9b (--dry-run)"
elif [ "$SKIP_INSTALL" = "1" ]; then
    warn "skipping step 9b (--skip-install)"
else
    step "9b/15 — public-release gate for $TAG"
    require jq

    # 1. Tag SHA anchoring: refs/tags/$TAG must equal the release commit
    #    we just published. `main` will keep moving; the tag is durable.
    EXPECTED_RELEASE_SHA="$(git rev-parse HEAD)"
    ACTUAL_TAG_SHA="$(git ls-remote origin "$TAG" | awk '{print $1}')"
    if [ -z "$ACTUAL_TAG_SHA" ]; then
        die "tag $TAG not found on origin — refusing to install"
    fi
    if [ "$EXPECTED_RELEASE_SHA" != "$ACTUAL_TAG_SHA" ]; then
        die "tag $TAG SHA drift: HEAD=$EXPECTED_RELEASE_SHA tag=$ACTUAL_TAG_SHA — refusing to install"
    fi
    ok "tag SHA anchored: $ACTUAL_TAG_SHA"

    # 2. Release metadata: query gh release view, parse JSON, assert state.
    #    isDraft=false, isPrerelease=false, tagName=expected, asset set
    #    equals the canonical 9-asset contract.
    RELEASE_JSON="$(gh release view "$TAG" --repo "$REPO" --json tagName,isDraft,isPrerelease,assets 2>/dev/null)" \
        || die "gh release view $TAG failed — refusing to install"
    GOTTEN_TAG="$(echo "$RELEASE_JSON" | jq -r '.tagName')"
    if [ "$GOTTEN_TAG" != "$TAG" ]; then
        die "tagName drift: expected $TAG got $GOTTEN_TAG"
    fi
    ok "tagName match: $TAG"

    IS_DRAFT="$(echo "$RELEASE_JSON" | jq -r '.isDraft')"
    if [ "$IS_DRAFT" != "false" ]; then
        die "release $TAG is in draft state (isDraft=$IS_DRAFT) — run: gh release edit $TAG --draft=false"
    fi
    ok "isDraft=false"

    IS_PRERELEASE="$(echo "$RELEASE_JSON" | jq -r '.isPrerelease')"
    if [ "$IS_PRERELEASE" != "false" ]; then
        die "release $TAG is a prerelease (isPrerelease=$IS_PRERELEASE) — Base releases must be non-prerelease"
    fi
    ok "isPrerelease=false"

    # 3. Asset contract. Keep the production rule in one place and exercise
    #    the exact same function from tests/lib_public_release_gate.sh.
    validate_release_asset_contract "$RELEASE_JSON" "${TAG#v}"
    CANONICAL_ASSETS=("${RELEASE_CANONICAL_ASSETS[@]}")


    # 4. Public URL HTTP probes: each canonical asset must respond
    #    200 from the public releases/download/$TAG/<asset> URL.
    #    CDN refresh can lag a few minutes after `gh release create`.
    #    Per-asset budget: 60s (6 attempts × 10s). Across 9 assets
    #    the worst case is ~9 minutes if every asset is stale. In
    #    practice the CDN catches up within seconds; this is a safety
    #    net, not a retry loop.
    URL_FAILS=""
    for asset in "${CANONICAL_ASSETS[@]}"; do
        URL="https://github.com/$REPO/releases/download/$TAG/$asset"
        ok_remote=0
        last_rc="000"
        for i in $(seq 1 6); do
            rc="$(curl -fsSL -o /dev/null -w '%{http_code}' "$URL" 2>/dev/null || echo "000")"
            last_rc="$rc"
            if [ "$rc" = "200" ]; then
                ok_remote=1
                break
            fi
            sleep 10
        done
        if [ "$ok_remote" = "0" ]; then
            URL_FAILS="$URL_FAILS $asset(HTTP $last_rc)"
        fi
    done
    if [ -n "$URL_FAILS" ]; then
        die "public URL probes failed after 60s-per-asset budget:$URL_FAILS"
    fi
    ok "9/9 canonical assets reachable from public CDN (HTTP 200)"

    # --- 9c. supply-chain authenticity against the PUBLISHED release ------
    # The 9b gate proves the assets EXIST and are the right names. This
    # proves they are AUTHENTIC: that a Fulcio certificate minted by
    # release.yml on a SemVer tag of this repo verifies them under the
    # exact pin the product ships.
    #
    # INC-AUDIT-S14 sat at `distribution-open` for several sessions because
    # nothing in the release path ever performed this verification, so the
    # one property that makes a signed release trustworthy was never
    # observed at the moment of release. It is cheap (it re-uses the CDN
    # round trip already paid for) and it is the difference between "the
    # upload worked" and "the upload is trustworthy".
    step "9c/15 — verify supply-chain authenticity of the published release"

    # La postura se DERIVA, no se decide otra vez. Antes el 9c exigia `.sig` y
    # `.pem` sin condicion, y una release sin firmar —que el propio 8c admite
    # con un aviso, y que el 9b declara completa porque el contrato canonico de
    # 9 assets no incluye firmas— se publicaba y luego moria aqui con un 404.
    # La secuencia era incoherente consigo misma: 9b decia "el conjunto esta
    # completo" y 9c decia "falta la mitad", DESPUES de publicar, con un
    # exit != 0 que hace creer que la release no salio.
    #
    # Que la postura sea UNSIGNED exige DOS cosas, no una:
    #   (1) que el operador lo declarase (SDDK_SKIP_SIGNING=1), y
    #   (2) que de verdad no haya ninguna firma que verificar.
    # La segunda es la que carga el peso. Saltarse porque el flag lo dice
    # seria afirmar "no hay nada que verificar" sin mirar. Si alguien dejara
    # un camino que firma pese al flag, esto verifica en vez de callar: una
    # propiedad de supply-chain que se puede comprobar y se deja sin
    # comprobar es un downgrade silencioso, que es peor que no tener el paso.
    SIG_FILES_PRESENT=0
    for sig_probe in "${SIGN_ARTIFACTS[@]}"; do
        for sig_ext in .sig .bundle.json .pem; do
            if [ -f "$TMP/$sig_probe$sig_ext" ]; then
                SIG_FILES_PRESENT=$((SIG_FILES_PRESENT + 1))
            fi
        done
    done
    AUTH_POSTURE="$(release_authenticity_posture "$SIG_FILES_PRESENT")"
    ok "9c posture: $AUTH_POSTURE (signature files present: $SIG_FILES_PRESENT)"

    if [ "$AUTH_POSTURE" = "UNSIGNED" ]; then
        warn "9c NOT_RUN — la release se publico SIN FIRMAR (SDDK_SKIP_SIGNING=1) y no"
        warn "hay ninguna firma que verificar. Authenticity NOT verified. Los dos"
        warn "instaladores exigiran SDDK_ALLOW_UNSIGNED=1 para instalar este tag."
        ok "9c declarado NOT_RUN con su motivo (release unsigned por decision del operador)"
    elif [ "$AUTH_POSTURE" = "DECLARED_SKIP" ]; then
        warn "skipping step 9c (SDDK_SKIP_AUTHENTICITY_CHECK=1) — authenticity NOT verified"
    elif ! command -v cosign >/dev/null 2>&1; then
        die "cosign not found — refusing to publish a signed release whose authenticity cannot be verified. Install cosign, or set SDDK_SKIP_AUTHENTICITY_CHECK=1 to acknowledge the gap."
    else
        AUTH_TMP="$(mktemp -d)"
        trap 'rm -rf "$AUTH_TMP"' RETURN
        for asset in sddk sddk.sig sddk.pem \
            "sddk-v$VERSION-sddk-linux-x86_64-musl.tar.gz" \
            "sddk-v$VERSION-sddk-linux-x86_64-musl.tar.gz.sig" \
            "sddk-v$VERSION-sddk-linux-x86_64-musl.tar.gz.pem" \
            software-development-decision-kernel.tar.gz \
            software-development-decision-kernel.tar.gz.sig \
            software-development-decision-kernel.tar.gz.pem; do
            curl -fsSL -o "$AUTH_TMP/$asset" \
                "https://github.com/$REPO/releases/download/$TAG/$asset" \
                || die "9c: could not fetch $asset for authenticity verification"
        done

        SDDK_RELEASE_REPO="$REPO" bash tests/test_supply_chain_authenticity.sh \
            --tag "$TAG" --assets-dir "$AUTH_TMP" >"$AUTH_TMP/auth.out" 2>&1 || {
                cat "$AUTH_TMP/auth.out"
                die "9c: supply-chain authenticity check FAILED — the published release does not verify under the shipped trust root"
            }
        grep -E "PASS=[0-9]+ FAIL=0" "$AUTH_TMP/auth.out" >/dev/null \
            || die "9c: authenticity check did not report FAIL=0"
        ok "published release verifies under the pinned trust root (9c)"
        ok "9c verified the CDN-served bytes, not a separate download"
    fi
    if [ "$AUTH_POSTURE" = "UNSIGNED" ] || [ "$AUTH_POSTURE" = "DECLARED_SKIP" ]; then
        warn "public-release gate PASS — pero la release se publico SIN verificar autenticidad"
    else
        ok "public-release gate PASS"
    fi
fi
# <<< REL-1 public-release gate end <<<

if [ "$SKIP_INSTALL" = "1" ]; then
    warn "skipping step 10-13 (--skip-install)"
    exit 0
fi

# --- 10. install from real GH URL ---

step "10/15 — install from GitHub Release URL"
# Defense against GH CDN caching: the URL may serve a stale tarball for
# up to a few minutes after upload. We poll the binary sha256 until it
# matches what we just uploaded, with a 5-minute budget.
EXPECTED_SHA="$(sha256sum "$BIN" | awk '{print $1}')"
URL_BIN="https://github.com/$REPO/releases/download/$TAG/$(basename "$BIN")"
ATTEMPTS=30
SLEEP_SECS=10
for i in $(seq 1 "$ATTEMPTS"); do
    ACTUAL="$(curl -fsSL "$URL_BIN" 2>/dev/null | sha256sum | awk '{print $1}' || true)"
    if [ "$ACTUAL" = "$EXPECTED_SHA" ]; then
        ok "CDN served correct binary sha256 after $((i * SLEEP_SECS))s"
        break
    fi
    if [ "$i" = "$ATTEMPTS" ]; then
        die "CDN still serving stale binary after $((ATTEMPTS * SLEEP_SECS))s — refusing to install"
    fi
    warn "CDN stale (got ${ACTUAL:-empty}, want ${EXPECTED_SHA:0:16}…) — retry $i/$ATTEMPTS"
    sleep "$SLEEP_SECS"
done

unset SDDK_BASE_URL SDDK_VERSION
# El pipeline YA decidio en el 8c que esta release se publica SIN FIRMAR, y el
# instalador no puede saberlo: solo ve que no hay `.sig` en el CDN. Sin esto el
# paso 10 moria con `no signature asset published` sobre una release que el
# propio pipeline acababa de publicar a proposito y que su propio 9c acababa
# de declarar NOT_RUN con ese motivo.
#
# Es la TERCERA vez que la misma verdad —"esta release no esta firmada"— se
# vuelve a decidir en un sitio distinto del que se decidio: el 8c la admitio, el
# 9c la declaro, y el 10 la vuelve a negar. **El instalador hace bien en
# negarse**: negarse es exactamente lo que significa unsigned, y por eso lleva
# la palabra ALLOW en el nombre de la bandera. Lo que esta mal es no
# comunicarle que la decision ya esta tomada y por quien. Un gate que
#murio en un tramo porque nadie le dijo lo que el tramo anterior ya habia
#decidido no es un gate que protege: es un gate que estorba.
if [ "${SDDK_SKIP_SIGNING:-0}" = "1" ]; then
    export SDDK_ALLOW_UNSIGNED=1
    warn "SDDK_SKIP_SIGNING=1 propagado como SDDK_ALLOW_UNSIGNED=1 al instalador"
    warn "— la release se publico SIN FIRMAR por decision declarada, y sin esto"
    warn "el paso 10 se negaria a instalar exactamente lo que el pipeline publico."
fi

# SDDK_PREFIX y SDDK_FRAMEWORK_DIR ya estan resueltos arriba, desde $HOME y con
# sobreescritura por entorno. Este paso los sobreescribia con rutas absolutas
# literales: el script llevaba 1600 lineas respetando el entorno y en la
# installacion lo dejaba de respetar, escribiendo en el home del operador que
# desarrollo el repo. En otra maquina —o con otro usuario— instalaba ahi y
# informaba de un exito que no era de este prefix.
# Los dos `export` de arriba no son un no-op: las variables se resuelven al
# principio del script como asignaciones simples, y sin exportarlas install.sh
# las recibiria vacias. Lo que estaba mal era su VALOR, no su export.
export SDDK_PREFIX="$SDDK_PREFIX"
export SDDK_FRAMEWORK_DIR="$SDDK_FRAMEWORK_DIR"
export SDDK_EDITOR="all"
bash scripts/install.sh --version "$TAG" --editor all \
    || die "install.sh failed"

# --- 11. doctor ---

step "11/15 — sddk dev doctor --prefix $SDDK_PREFIX"
DOCTOR_OUT="$("$SDDK_PREFIX/sddk" dev doctor --prefix "$SDDK_PREFIX" --format text)"
echo "$DOCTOR_OUT" | grep -E "binary\.bundle_coherence|^all_present" \
    || die "doctor output missing expected checks"
echo "$DOCTOR_OUT" | grep -q "binary\.bundle_coherence: present" \
    || die "binary.bundle_coherence not present"
echo "$DOCTOR_OUT" | grep -q "all_present: true" \
    || die "all_present is not true"
ok "binary.bundle_coherence: present, all_present: true"

# --- 12. prune ---

step "12/15 — sddk dev update --prune-only --keep 1"
"$SDDK_PREFIX/sddk" dev update --prune-only --keep 1 \
    --root "$SDDK_FRAMEWORK_DIR" --format text \
    || die "prune failed"
ok "stale bundles pruned"

# --- 13. re-install from distrib (smoke test the published artefact) ---

step "13/15 — re-install from URL (distrib smoke test)"
# After step 11 (install from real URL) and step 12 (prune stale bundles),
# re-run install.sh --editor none against the same GH URL to confirm the
# published artefact round-trips through `sddk dev install` + `dev use`
# + `dev link` without any housekeeping needed. This is the same code
# path that an external user would take after `gh release create`. We
# pass --editor none to avoid relinking editors every release; the
# doctor check at the end confirms the install was successful.
SDDK_REPO="$REPO" \
SDDK_VERSION="$TAG" \
SDDK_BASE_URL="${SDDK_BASE_URL:-https://github.com/$REPO/releases}" \
SDDK_EDITOR="none" \
bash "$REPO_ROOT/scripts/install.sh" \
    --version "$TAG" \
    --prefix "$SDDK_PREFIX" \
    --editor none \
    || die "re-install from distrib failed; the published artefact is broken"
ok "distrib round-trip OK (binary + bundle coherent after prune)"

# --- 15. final state ---

step "15/15 — final state"
echo
BIN_VER="$("$SDDK_PREFIX/sddk" --version 2>&1 | head -1)"
BUNDLE_VER="$("$SDDK_PREFIX/sddk" dev doctor --prefix "$SDDK_PREFIX" --format json 2>/dev/null \
    | python3 -c '
import json, sys
data = json.load(sys.stdin)
# binary.bundle_coherence lives under checks[]; the bundle version itself
# comes from the receipt (sddk-install.json).
print(json.loads(open("'"$SDDK_PREFIX"'/sddk-install.json").read()).get("bundle_version", "?"))
' 2>/dev/null || echo "?")"
CURRENT_VER="$(basename "$(readlink "$SDDK_FRAMEWORK_DIR/current" 2>/dev/null || echo "?")")"
echo "  binary:        $BIN_VER"
echo "  bundle:        $BUNDLE_VER"
echo "  current:       $CURRENT_VER"
echo "  framework/:"
find "$SDDK_FRAMEWORK_DIR" -mindepth 1 -maxdepth 1 -printf '    %f\n' | sort
echo
ok "release $TAG shipped and installed locally"
