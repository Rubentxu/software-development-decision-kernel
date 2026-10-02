#!/usr/bin/env bash
# Gate de los siete criterios de aceptacion de ADR-0153, medidos uno a uno.
#
# No resume: cada criterio se ejecuta por separado y reporta su propio
# veredicto, para que un PASS agregado no pueda tapar un criterio rojo. La
# aceptacion de un ADR no se declara por suma.
#
# Uso: bash tests/test_adr_0153_criteria.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/var/home/rubentxu/cargo-targets}"
cd "$ROOT" || exit 1

PASS=0
FAIL=0
ok()  { PASS=$((PASS+1)); printf '  [ok]   %s\n' "$1"; }
bad() { FAIL=$((FAIL+1)); printf '  [FAIL] %s\n' "$1"; }

# Corre un test por nombre y decide. Un criterio con varios tests exige que
# PASEN TODOS: basta uno rojo para que el criterio sea rojo.
criterion() {
  local id="$1" desc="$2"; shift 2
  local out all_ok=1
  for t in "$@"; do
    out="$(cargo test -p sddk-engine --lib "$t" 2>&1 | tail -3)"
    printf '%s' "$out" | grep -q 'test result: ok' || { all_ok=0; break; }
  done
  if [ "$all_ok" = 1 ]; then ok "C$id — $desc"; else bad "C$id — $desc"; fi
}

echo "=== criterios de aceptacion de ADR-0153 ==="

# C1. El codigo que resuelve no nombra ningun manifiesto. Es estructural a
# proposito: un reader que hardcodea un nombre se comporta igual mientras el
# nombre siga ahi, luego ningun test de comportamiento lo veria.
criterion 1 "el codigo de resolucion no nombra ningun manifiesto" \
  the_resolution_code_names_no_manifest

# C2. Los ocho ecosistemas resuelven o fallan por una razon declarada, medido
# uno a uno sobre fixtures, no por inspeccion.
criterion 2 "los ocho ecosistemas del principio tienen fila y se resuelven" \
  every_ecosystem_in_the_principle_has_a_row \
  rust_resolves_workspace_package \
  typescript_resolves_package_json \
  jvm_gradle_resolves_gradle_properties \
  dotnet_resolves_the_directory_build_props_tag \
  cpp_cmake_resolves_the_project_call \
  go_has_no_version_to_check_and_says_so \
  bazel_has_no_version_to_check_and_says_so

# C3. Anadir un ecosistema es solo datos.
criterion 3 "anadir un ecosistema es solo datos, sin tocar codigo" \
  a_new_ecosystem_is_data_and_needs_no_code

# C4. Un repo Rust se comporta exactamente igual que antes, incluido el
# mensaje de error. Criterio de paridad: estos tests son los que existian
# cuando la funcion abria Cargo.toml a pelo, y no se reescribieron.
criterion 4 "un repo Rust conserva el lockstep, incluido el texto del refusal" \
  lockstep_passes_when_tag_matches_workspace_version \
  lockstep_passes_when_tag_has_no_v_prefix \
  lockstep_fails_when_tag_diverges_from_workspace \
  lockstep_errors_when_no_known_manifest_is_present \
  lockstep_uses_the_project_version_not_a_dependencies \
  lockstep_accepts_a_single_quoted_version \
  lockstep_reports_a_parse_failure_as_such \
  lockstep_distinguishes_a_missing_version_from_a_broken_file \
  lockstep_does_not_confuse_package_and_workspace_tables \
  lockstep_errors_when_version_key_absent_in_workspace

# C5. Dos manifiestos con versiones distintas => error duro que nombra ambos.
criterion 5 "dos manifiestos que discrepan son un error duro que nombra ambos" \
  two_manifests_that_disagree_are_a_hard_error_naming_both

# C6. Un manifiesto corrupto no cede la autoridad al siguiente candidato.
criterion 6 "un manifiesto roto no cede la autoridad al siguiente candidato" \
  a_broken_manifest_does_not_hand_authority_to_the_next_candidate

# C7. Go y Bazel toman el camino del tag, y es DISTINTO de cross-checked.
criterion 7 "Go y Bazel toman el camino del tag, distinto de cross-checked" \
  go_has_no_version_to_check_and_says_so \
  bazel_has_no_version_to_check_and_says_so \
  rust_never_takes_the_tag_only_path

echo
echo "PASS=$PASS FAIL=$FAIL"
if [ "$FAIL" -eq 0 ]; then
  echo "RESULT: PASS — los siete criterios de ADR-0153 están verdes medidos uno a uno."
else
  echo "RESULT: FAIL — ADR-0153 no cumple todos sus criterios; NO se promueve a accepted."
fi
exit $(( FAIL > 0 ? 1 : 0 ))
