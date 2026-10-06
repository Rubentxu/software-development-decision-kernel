#!/usr/bin/env bash
# Autofalsación de los evaluadores que leen el árbol (ARCH004, ARCH005).
#
# Un test que sigue verde cuando se rompe el evaluador que dice verificar no
# esta verificando: esta suite rompe el codigo a proposito y exige que un test
# concreto CAIGA cada vez. Un "PASS" aqui significa "el defecto fue detectado".
#
# Cada mutacion declara el test que debe caer. Si la mutacion se aplica pero el
# test sigue verde, se cuenta como NO DETECTADA — el fallo de este guard, no un
# PASS.
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO" || exit 1
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/var/home/rubentxu/cargo-targets/release/sddk}"

TARGET="crates/sddk-engine/src/rules/evaluators.rs"
BACKUP="$(mktemp)"
cp "$TARGET" "$BACKUP"
PRISTINE_SHA="$(sha256sum "$BACKUP" | cut -d' ' -f1)"

PASS=0
FAIL=0
SKIP=0
declare -a LINES=()

cleanup() { cp "$BACKUP" "$TARGET"; rm -f "$BACKUP"; }
trap cleanup EXIT

restore() {
  cp "$BACKUP" "$TARGET"
  local now
  now="$(sha256sum "$TARGET" | cut -d' ' -f1)"
  if [[ "$now" != "$PRISTINE_SHA" ]]; then
    echo "RESTAURACION NO BYTE-IDENTICA: $now != $PRISTINE_SHA"
    exit 1
  fi
}

run_test() {
  # El nombre completo va como filtro de `--exact`. Con un segundo argumento de
  # filtro delante, cargo lo toma como substring y `--exact` nunca coincide:
  # el control de este guard lo cazó como FAIL antes de que se notara.
  cargo test -q -p sddk-engine --test rules_evaluator -- --exact "$1" 2>&1
}

verdict_for() {
  local name="$1" test="$2" mutation_applies="$3"
  # SIEMPRE se restaura, tambien en SKIP: si un SKIP deja el codigo mutado, los
  # controles finales corren contra un evaluador roto y el guard se acusa a si
  # mismo de no discriminar. Fue lo que paso en la primera ejecucion.
  if [[ "$mutation_applies" != "yes" ]]; then
    restore
    SKIP=$((SKIP + 1))
    LINES+=("SKIP | $name | la mutacion no aplico sobre este codigo")
    return
  fi
  local out rc
  out="$(run_test "$test")"
  rc=$?
  restore
  if [[ $rc -ne 0 ]]; then
    PASS=$((PASS + 1))
    LINES+=("PASS | $name | cae como debe: $test")
  else
    FAIL=$((FAIL + 1))
    # Se imprime la salida: un FAIL aqui sin el "por que" obliga a reproducirlo
    # a mano, y este guard se lee cuando algo ya va mal.
    printf '%s\n' "$out" | sed 's/^/          | /' >&2
    LINES+=("FAIL | $name | $test sigue VERDE con el codigo roto: el test no comprueba lo que dice comprobar")
  fi
}

# yes si el python aplico la mutacion (asserts incluidos), no si no la aplico.
applied() { if [[ $1 -eq 0 ]]; then echo yes; else echo no; fi; }

echo "== mutaciones sobre $TARGET =="

# ── M1: ARCH004 pierde la direccion "undeclared" ──────────────────────────────
python3 - "$TARGET" <<'PY'
import sys
p=sys.argv[1]; s=open(p).read()
old="""        for dep in &actual {
            if !declared_names.contains(&dep.as_str()) {"""
new="""        for dep in &actual.iter().take(0) {
            if !declared_names.contains(&dep.as_str()) {"""
assert s.count(old)==1, "M1 no aplica"
open(p,'w').write(s.replace(old,new))
PY
verdict_for "M1 ARCH004 solo mira declaraciones colgantes" \
  "tree_reading_rules::arch004_fails_on_a_real_dependency_the_manifest_never_names" "$(applied $?)"

# ── M2: ARCH004 vuelve al stub (N/A con root) ────────────────────────────────
python3 - "$TARGET" <<'PY'
import sys
p=sys.argv[1]; s=open(p).read()
# Anclado al texto unico ("the pack manifests"), no a la linea `let Some(root)`
# que aparece DOS veces tras ARCH005: un assert de unicidad sobre una linea
# compartida hacia que esta mutacion dejara de aplicarse en silencio el dia
# que rustfmt partiera la firma en varias lineas.
old='''not_measured(rule, baseline, evaluated_at, "the pack manifests")'''
new='''not_measured_as_stub(rule, baseline, evaluated_at)'''
assert s.count(old)==1, f"M2 no aplica (count={s.count(old)})"
s=s.replace(old,new)
# Y el sustituto declara el stub viejo, que es lo que M2 quiere reproducir.
anchor="""fn not_measured("""
stub='''fn not_measured_as_stub(rule: &sddk_domain::ArchitectureRule, baseline: &Baseline, evaluated_at: &str) -> RuleEvaluation {
    RuleEvaluation {
        rule_id: rule.id.clone(),
        status: RuleStatus::NotApplicable,
        observed: json!({}),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at: evaluated_at.to_owned(),
        evaluated_by: format!("sddk-rules-cli@{EVALUATOR_VERSION}"),
        waiver_id: None,
        evaluator_kind: EvaluatorKind::Schema,
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        provenance: Some("kernel repo, not a pack host".to_owned()),
    }
}

'''
assert s.count(anchor)==1
open(p,'w').write(s.replace(anchor, stub+anchor))
PY
verdict_for "M2 ARCH004 vuelve al stub con motivo falso" \
  "tree_reading_rules::no_repository_root_is_reported_as_not_measured_never_as_a_pass" "$(applied $?)"

# ── M3: ARCH004 se traga un manifest ilegible ────────────────────────────────
python3 - "$TARGET" <<'PY'
import sys,re
p=sys.argv[1]; s=open(p).read()
i=s.index("let declared = match declared_pack_deps(manifest) {")
j=s.index("        let mut declared_names", i)
block=s[i:j]
new=block.replace("""            Err(e) => {""","""            Err(_e) => {
                continue;
            }
            #[allow(unreachable_code)]
            Err(e) => {""",1)
assert new!=block, "M3 no aplica"
open(p,'w').write(s[:i]+new+s[j:])
PY
verdict_for "M3 un manifest ilegible lee como 'no declaro nada'" \
  "tree_reading_rules::arch004_treats_an_unreadable_manifest_as_a_violation_not_as_absence" "$(applied $?)"

# ── M4: ARCH005 escanea tambien los comentarios ──────────────────────────────
python3 - "$TARGET" <<'PY'
import sys
p=sys.argv[1]; s=open(p).read()
old="""            if trimmed.starts_with("//") || trimmed.starts_with("*") {
                continue;
            }"""
new="""            let _ = trimmed;"""
assert s.count(old)==1, "M4 no aplica"
open(p,'w').write(s.replace(old,new))
PY
verdict_for "M4 ARCH005 marca como efecto la documentacion del efecto" \
  "tree_reading_rules::arch005_does_not_flag_a_module_documenting_the_rule_it_obeys" "$(applied $?)"

# ── M5: ARCH005 declara conformidad sin sujeto ───────────────────────────────
python3 - "$TARGET" <<'PY'
import sys
p=sys.argv[1]; s=open(p).read()
old="""    if files.is_empty() {
        return RuleEvaluation {
            rule_id: rule.id.clone(),
            status: RuleStatus::NotApplicable,"""
new="""    if files.is_empty() {
        return RuleEvaluation {
            rule_id: rule.id.clone(),
            status: RuleStatus::Pass,"""
assert s.count(old)==1, "M5 no aplica"
open(p,'w').write(s.replace(old,new))
PY
verdict_for "M5 ARCH005 dice Pass cuando no hay sujeto" \
  "tree_reading_rules::arch005_without_a_subject_is_measured_but_never_conformant" "$(applied $?)"

# ── M6: ARCH005 no distingue escribir de leer ────────────────────────────────
python3 - "$TARGET" <<'PY'
import sys
p=sys.argv[1]; s=open(p).read()
# Solo se quita `write` de la alternancia. La mutacion anterior sustituia la
# linea entera del literal, y rustfmt la habia repartido en varias: la
# mutacion dejo de aplicar sin que nadie lo notara (SKIP, no FAIL, luego peor).
old = r"fs::(write|remove_file|remove_dir_all|rename)"
new = r"fs::(remove_file|remove_dir_all|rename)"
assert s.count(old)==1, f"M6 no aplica (count={s.count(old)})"
open(p,'w').write(s.replace(old,new))
PY
verdict_for "M6 ARCH005 no ve un fs::write" \
  "tree_reading_rules::arch005_fails_when_a_reactive_module_writes_directly" "$(applied $?)"

# ── CONTROLES: el codigo intacto debe dejarlo todo en verde ─────────────────
echo "== controles (codigo intacto) =="
for t in \
  "tree_reading_rules::arch004_fails_on_a_real_dependency_the_manifest_never_names" \
  "tree_reading_rules::arch005_fails_when_a_reactive_module_writes_directly" \
  "tree_reading_rules::no_repository_root_is_reported_as_not_measured_never_as_a_pass" \
  "tree_reading_rules::arch004_treats_an_unreadable_manifest_as_a_violation_not_as_absence"
do
  if run_test "$t" >/dev/null 2>&1; then
    PASS=$((PASS + 1)); LINES+=("PASS | control (codigo intacto) | $t sigue verde, el guard distingue")
  else
    FAIL=$((FAIL + 1)); LINES+=("FAIL | control (codigo intacto) | $t cae sin mutacion: el test no discrimina")
  fi
done

echo
for l in "${LINES[@]}"; do echo "$l"; done
echo
echo "PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
[[ $FAIL -eq 0 ]] && echo "RESULT: PASS" || echo "RESULT: FAIL"
exit $(( FAIL > 0 ))