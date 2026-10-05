#!/usr/bin/env bash
# Gate de los criterios de aceptacion de ADR-0157, medidos uno a uno.
#
# No resume: cada criterio se ejecuta por separado y reporta su propio
# veredicto, para que un PASS agregado no pueda tapar un criterio rojo. La
# aceptacion de un ADR no se declara por suma.
#
# Uso: bash tests/test_adr_0157_criteria.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/var/home/rubentxu/cargo-targets}"
cd "$ROOT" || exit 1

PASS=0
FAIL=0
NOT_APPLIED=0
ok()  { PASS=$((PASS+1)); printf '  [ok]   %s\n' "$1"; }
bad() { FAIL=$((FAIL+1)); printf '  [FAIL] %s\n' "$1"; }
na()  { NOT_APPLIED=$((NOT_APPLIED+1)); printf '  [N/A]  %s\n' "$1"; }

# Corre un test por nombre y decide. Un criterio con varios tests exige que
# PASEN TODOS: basta uno rojo para que el criterio sea rojo.
#
# ## El requisito que este gate no tenia, y por que importa mas de lo que parece
#
# La version anterior de este gate (ADR-0153) decidia con un `grep` sobre
# `test result: ok`. MEDIDO: `cargo test -p sddk-engine --lib <nombre-que-no-existe>`
# imprime `test result: ok. 0 passed; 0 failed` y sale con 0, luego un criterio
# cuyos tests habian desaparecido —que es exactamente lo que paso al migrar el
# registro— reportaba PASS sin haber ejecutado un solo test. Siete criterios en
# verde certificando una migracion que no habia medido nada.
#
# Es la misma clase que el resto del repo ha encontrado seis veces: NOMBRAR no
# es EJECUTAR. Aqui era un gate entero, y habria dado verde en el release.
#
# Por eso un criterio con cero tests ejecutados es NOT_APPLIED, que NO cuenta
# como PASS. Un criterio que no se midio no esta cumplido: no se sabe.
# Corre un test por nombre y decide. Un criterio con varios tests exige que
# PASEN TODOS: basta uno rojo para que el criterio sea rojo.
#
# ## Los dos requisitos que este gate no tenia, y por que importan mas de lo que parece
#
# **Uno: que el test se EJECUTE.** La version anterior de este gate (ADR-0153)
# decidia con un `grep` sobre `test result: ok`. MEDIDO: un criterio cuyos
# tests habian desaparecido reportaba PASS sin ejecutar un solo test, porque
# `cargo test` con un filtro que no casa nada imprime `test result: ok. 0 passed`
# y sale con 0. Siete criterios en verde certificando una migracion que no habia
# medido nada. Es la misma clase que el resto del repo ha encontrado seis veces:
# NOMBRAR no es EJECUTAR. Aqui era un gate entero.
#
# **Dos: que se lea el resultado del binario CORRECTO.** El gate anterior
# tomaba `tail -3` de la salida, y con varios binarios de test eso es la linea
# del ULTIMO —el de doc-tests, que dice `0 passed`— no la del test que se
# pedia. MEDIDO: por eso todos los criterios de este gate salieron NOT_APPLIED
# en su primera pasada, con tests que si existian y ya pasaban. Un instrumento
# que lee la linea equivocada no falla: informa de otra cosa.
#
# Por eso aqui se lee la salida COMPLETA, se exige que ningun binario haya
# fallado, y se exige que al menos uno haya ejecutado AL MENOS UN test. Un
# criterio que no se midio es NOT_APPLIED, que NO cuenta como PASS: no se sabe.
run_one() {
  local target="$1" test_name="$2" out ran failed
  # `--tests` deja fuera los doc-tests, que son los que aportaban la linea de
  # «0 passed» que hacia ilegible el resultado real.
  out="$(cargo test -p "$target" --tests --no-fail-fast "$test_name" 2>&1)"
  if printf '%s' "$out" | grep -qE '^test result: FAILED'; then
    return 1
  fi
  ran="$(printf '%s' "$out" | grep -cE '^test result: ok\. [1-9][0-9]* passed')"
  if [ "$ran" -lt 1 ]; then
    return 2
  fi
  failed="$(printf '%s' "$out" | grep -cE '^test result: FAILED')"
  [ "$failed" -eq 0 ] || return 1
  return 0
}

criterion() {
  local id="$1" desc="$2" target="$3"; shift 3
  local out rc applied=0 all_ok=1
  for t in "$@"; do
    run_one "$target" "$t"; rc=$?
    case $rc in
      0) applied=$((applied+1)) ;;
      2) all_ok=0; na "C$id — $t no se ejecuto (no existe, o la suite no corrio)" ;;
      *) all_ok=0; bad "C$id — $t fallo" ;;
    esac
  done
  if [ "$all_ok" -ne 1 ]; then
    return
  fi
  if [ "$applied" -eq 0 ]; then
    bad "C$id — $desc (ningun test se ejecuto)"
  else
    ok "C$id — $desc ($applied test(s))"
  fi
}

echo "=== criterios de aceptacion de ADR-0157 ==="

# C1. El modulo que DECIDE no nombra tecnologia concreta. En los dos extremos:
# el modelo puro del dominio y la regla del motor. Estructural a proposito: un
# provider que hardcodeara una preferencia se comporta igual en todos los
# fixtures, luego ningun test de comportamiento lo veria.
criterion 1 "el modulo que decide no nombra tecnologia concreta" sddk-domain \
  the_decision_module_names_no_concrete_technology
criterion 1 "el motor tampoco, y su convencion esta escrita" sddk-engine \
  the_version_resolution_module_names_no_concrete_technology

# C2. El escaner puede ver un nombre, y su unica excepcion no puede crecer.
# Sin esto, C1 y su equivalente son un guard que no puede fallar.
criterion 2 "el fitness puede ver un nombre y su excepcion no crece" sddk-domain \
  the_fitness_scanner_can_actually_see_a_name
criterion 2 "el fitness del motor tambien, y su excepcion esta acotada" sddk-engine \
  the_fitness_scanner_can_actually_see_a_name \
  la_excepcion_no_puede_crecer_hasta_ser_un_agujero

# C3. Los ocho ecosistemas del principio resuelven o fallan por una razon
# declarada, medido sobre ficheros reales.
criterion 3 "los ecosistemas del principio resuelven por una razon declarada" sddk-gateway \
  lee_una_declaracion_por_ecosistema \
  las_dos_dialectos_de_python_se_leen_por_separado

# C4. Un repo Rust conserva el lockstep, incluido el texto del rechazo. Criterio
# de paridad con lo que habia antes de la migracion.
criterion 4 "el lockstep se conserva, y el rechazo nombra los dos lados" sddk-engine \
  lockstep_passes_when_the_ref_matches \
  lockstep_passes_when_the_ref_has_no_v_prefix \
  lockstep_fails_and_names_both_sides

# C5. La convencion de la release ref es un prefijo y nada mas. Un recorte mas
# haria que dos valores distintos se presentaran como el mismo.
criterion 5 "la convencion de la release ref no recorta nada mas" sddk-engine \
  la_convencion_no_recorta_nada_mas_que_el_prefijo \
  la_convencion_de_la_release_ref_es_un_prefijo_y_nada_mas

# C6. Dos declaraciones que discrepan son un error duro que nombra LAS DOS.
criterion 6 "una discrepancia no se resuelve eligiendo" sddk-engine \
  una_discrepancia_no_se_resuelve_eligiendo
criterion 6 "y el reducer no colapsa dos versiones distintas en una" sddk-domain \
  dos_declaraciones_distintas_son_ambiguas_y_no_se_elige_una
criterion 6 "y el caso end-to-end nombra las dos" sddk-gateway \
  dos_ficheros_que_declaran_distinto_siguen_siendo_divergentes \
  dos_ficheros_que_declaran_distinto_producen_ambiguedad_y_no_una_elegida

# C7. Un manifiesto roto falla cerrado y NO cede la autoridad a otro.
criterion 7 "un manifiesto roto falla cerrado" sddk-gateway \
  un_manifiesto_que_no_declara_version_sigue_siendo_un_fallo_cerrado \
  un_manifiesto_corrupto_sigue_siendo_fallo_cerrado
criterion 7 "y el reducer no lo salta" sddk-domain \
  un_provider_invalido_falla_cerrado_aunque_otro_declare

# C8. Go y Bazel toman el camino de la release ref, y es DISTINTO de haber sido
# comprobado. Antes sharing el mismo nombre que un proyecto Rust con un solo
# manifiesto.
criterion 8 "un ecosistema sin version de producto declara la convencion" sddk-gateway \
  un_ecosistema_sin_version_de_producto_declara_que_la_lleva_la_release_ref
criterion 8 "una lectura unica no se presenta como comprobacion" sddk-gateway \
  una_declaracion_no_es_un_cross_check
criterion 8 "el reducer mantiene la distincion" sddk-domain \
  una_ausencia_declarada_no_es_silencio \
  una_ausencia_declarada_no_es_un_exito_de_comprobacion

# C9. El caso del consumidor: un fichero presente que no declara version no
# tapa a otro que si. Es el defecto que abrio el bloque.
criterion 9 "un fichero que no declara no tapa al que si" sddk-gateway \
  una_configuracion_que_no_declara_version_no_tapa_a_la_que_si \
  un_escrito_de_construccion_que_existe_no_tapa_a_la_que_si \
  un_escrito_de_python_no_destruye_lo_que_ya_se_declaro \
  un_fichero_sin_version_no_bloquea_a_otro_que_si_declara
criterion 9 "y end-to-end por el binario" sddk-cli \
  cli_release_plan_publishes_past_a_file_that_declares_nothing

# C10. Un mecanismo nuevo se añade registrando un provider, SIN tocar el motor.
# El test de aceptacion del port usa un provider falso definido en el test; si
# el motor tuviera que saber de el, este test no compilaria.
criterion 10 "un provider nuevo no toca el motor" sddk-domain \
  un_provider_nuevo_se_descubre_sin_tocar_el_engine \
  puede_responder_not_applicable \
  un_provider_que_no_habla_la_capability_no_se_pregunta \
  un_target_recibido_llega_al_provider_intacto \
  ninguna_decision_depende_del_identificador_del_provider \
  participa_en_la_resolucion_con_otro_provider
criterion 10 "y el motor no lo menciona" sddk-engine \
  the_version_resolution_module_names_no_concrete_technology

# C11. La resolucion es de solo lectura.
criterion 11 "observar no modifica el arbol" sddk-gateway \
  resolver_es_read_only

# C12. El silencio no es una convencion declarada.
criterion 12 "el silencio no abre la puerta" sddk-gateway \
  un_repositorio_vacio_no_declara_nada \
  un_proyecto_sin_declarar_no_inventa_una \
  una_convencion_declarada_publica_y_el_silencio_no \
  una_declaracion_que_no_se_entiende_no_se_ignora

# C13. La convencion declarada no es una prioridad sobre una version leida.
criterion 13 "una version leida no se tapa con una ausencia declarada" sddk-domain \
  una_version_leida_no_se_tapa_con_una_ausencia_declarada \
  lo_ilegible_no_se_tapa_con_una_ausencia_declarada

# C14. Un provider por fichero, y la identidad no compra autoridad.
criterion 14 "un provider por fichero no es evidencia repetida" sddk-gateway \
  un_provider_por_fichero_no_es_evidencia_repetida \
  dos_lectores_que_coinciden_si_cruzan_validacion
criterion 14 "y el provider no compra autoridad por su nombre" sddk-domain \
  la_identidad_del_provider_compra_autoridad_a_nadie \
  un_provider_consultado_dos_veces_no_es_un_segundo_voto

# C15. El reducer no depende del orden en que le llegan las observaciones.
criterion 15 "el veredicto no depende del orden de llegada" sddk-domain \
  el_orden_de_las_observaciones_no_cambia_el_veredicto

echo
echo "PASS=$PASS FAIL=$FAIL NOT_APPLIED=$NOT_APPLIED"
if [ "$FAIL" -gt 0 ] || [ "$NOT_APPLIED" -gt 0 ]; then
  echo "RESULT: FAIL — ADR-0157 no cumple todos sus criterios; NO se sostiene como accepted."
  exit 1
fi
echo "RESULT: PASS — los quince criterios de ADR-0157 están verdes medidos uno a uno."
exit 0
