#!/usr/bin/env bash
# reconcile_state_pointer.sh
#
# Repara el puntero de docs/roadmap/STATE.yaml cuando
# test_release_state_pointer.sh reporta drift.
#
# Por que existe (sesion-23, evidencia):
#   El guard test_release_state_pointer.sh DETECTA que STATE.yaml
#   miente sobre el estado del repo. Eso ya es mejor que nada, pero
#   obliga a que alguien recuerde reconciliar a mano, y se ha fallado
#   tres sesiones seguidas por el mismo motivo:
#
#     - session-18:  puntero en 5ce4bca/2.0.1, repo en 971e0ea/2.0.4
#     - session-22:  puntero en aaed465/2.0.7, repo en 62d4728/2.0.8
#     - session-23:  puntero en 8136bbf/2.0.9, repo en f8ef219/2.0.10
#
#   El motivo por el que se repite es ESTRUCTURAL, no un descuido: el
#   pre-push hook exige que el bump de version viaje en un commit
#   `chore(release): bump version` PROPIO, siempre posterior al commit
#   de trabajo. Y STATE.yaml, al ser un fichero, no puede contenerse a
#   si mismo. Asi que el puntero nunca puede senalar al bump sin quedar
#   al menos un commit por detras, y cada cierre de sesion anade mas
#   commits documentales encima. Detectar era inevitablemente tarde.
#
#   Este script cierra el bucle: mueve el puntero al commit real y deja
#   el anterior como `superseded_pointer`, conservando la evidencia.
#
# Lo que NO hace, a proposito:
#   - No inventa evidencia ni toca CURRENT.md / SESSION-JOURNAL.md.
#     El juicio ("que significa este estado") sigue siendo humano.
#   - No bumpea versiones ni commitea. Solo edita el fichero.
#   - No reescribe entradas anteriores del puntero.
#   - No inventa evidencia. El juicio sobre que significa el estado sigue
#     siendo humano; esto solo mueve campos mecanicos.
#
# Uso:
#   bash scripts/reconcile_state_pointer.sh --check     # solo informa, no escribe
#   bash scripts/reconcile_state_pointer.sh            # repara
#
# Exit: 0 = puntero coherente (nada que hacer) o reparado
#       1 = no se pudo reparar / el estado no es reparable automaticamente
set -uo pipefail

cd "$(git rev-parse --show-toplevel 2>/dev/null)" || {
  echo "FATAL: no estoy dentro de un repo git" >&2
  exit 1
}

STATE="docs/roadmap/STATE.yaml"
CARGO="Cargo.toml"
MODE="repair"

case "${1:-}" in
  --check) MODE="check" ;;
  --repair|"") MODE="repair" ;;
  -h|--help)
    sed -n '2,40p' "$0" | sed 's/^# \{0,1\}//'
    exit 0
    ;;
  *)
    echo "opcion desconocida: $1 (usa --check o --repair)" >&2
    exit 1
    ;;
esac

[ -f "$STATE" ] || { echo "FATAL: no existe $STATE" >&2; exit 1; }
command -v python3 >/dev/null || { echo "FATAL: falta python3" >&2; exit 1; }

# --- QueVersionDice: extrae workspace_version_at_current de STATE.yaml ------
declared_ver() {
  sed -n 's/^  workspace_version_at_current: *"\([^"]*\)".*/\1/p' "$STATE" | head -1
}

# --- Version real de Cargo.toml -------------------------------------------
real_ver=$(sed -n '/^\[workspace\.package\]/,/^\[/ s/^version *= *"\([^"]*\)".*/\1/p' "$CARGO" | head -1)

# --- A donde debe apuntar el puntero ---------------------------------------
# El puntero describe el estado PUBLICADO. Por eso el objetivo es el
# ultimo commit de main, no un SHA inventado: si hay commits sin pushear,
# el puntero queda uno atras, que es exactamente el estado que el guard
# ya acepta como normal (tolerancia 3).
target_sha=$(git rev-parse main)
target_short=$(git rev-parse --short "$target_sha")
target_subject=$(git log -1 --format=%s "$target_sha")

current_sha=$(sed -n 's/^  current_sha: *"\([0-9a-f]\{7,40\}\)".*/\1/p' "$STATE" | head -1)
current_ver=$(declared_ver)

echo "== reconciliacion del puntero de estado =="
echo "  STATE.yaml         : $STATE"
echo "  current_sha        : ${current_sha:-<none>} (workspace_version_at_current=${current_ver:-<none>})"
echo "  main               : $target_short ($target_subject)"
echo "  Cargo.toml version : ${real_ver:-<none>}"
echo

drift=0
sha_reconcilable=1
# Importa: perseguir la punta de main NO es reconciliar. El puntero
# describe el estado PUBLICADO y su SHA puede estar legitimamente unos
# commits atras: el propio guard acepta un retraso <= PUNCTUAL_TOLERANCE
# porque el bump de version viaja en su propio commit (impuesto por el
# pre-push hook) y STATE.yaml no puede contenerse a si mismo. Si este
# script se moviera a HEAD en cada cierre, borraria la nota de evidencia
# que hay que conservar y convertiria un estado SANO en escritura.
#
# Aqui se replica la misma tolerancia que el guard, con la misma
# constante, para que los dos no puedan discrepar. Si alguno cambia, el
# otro se queda obsoleto en silencio: por eso el valor va en la variable
# de entorno PUNCTUAL_TOLERANCE_OVERRIDE y no hardcodeado dos veces.
TOLERANCE="${PUNCTUAL_TOLERANCE_OVERRIDE:-3}"

if [ -n "$current_sha" ] && git rev-parse --verify --quiet "$current_sha^{commit}" >/dev/null; then
  behind=$(git rev-list --count "$current_sha..$target_sha")
  if [ "$behind" -le "$TOLERANCE" ]; then
    # El SHA NO se mueve (esta dentro de tolerancia y su nota de
    # evidencia es más informativa que la que escribiria este script),
    # pero la version puede seguir desalineada: son dos campos
    # independientes y drift en uno no absuelve al otro.
    echo "  current_sha: $current_sha, $behind commit(s) por detras (tolerancia $TOLERANCE) -- se conserva"
  else
    echo "  current_sha va $behind commit(s) por detras (tolerancia $TOLERANCE) -- se reconcilia"
    drift=1
    sha_reconcilable=0
  fi
else
  echo "  current_sha ausente o irresoluble: drift estructural -- se reconcilia"
  drift=1
  sha_reconcilable=0
fi

# La version se comprueba SIEMPRE, incluso cuando el SHA esta sano. Un
# exit temprano aqui hacía que el script anunciara PASS con la version
# rota mientras el guard decia FAIL: el peor resultado posible para una
# herramienta cuyo unico trabajo es que ambos coincidan.
if [ "$current_ver" != "$real_ver" ]; then
  echo "  workspace_version_at_current $current_ver != Cargo.toml $real_ver -- se reconcilia"
  drift=1
else
  echo "  workspace_version_at_current $current_ver: alineada"
fi

if [ "$drift" -eq 0 ]; then
  echo
  echo "RESULT: PASS — el puntero ya describe el estado real; nada que reparar."
  exit 0
fi

echo "DRIFT DETECTADO:"
echo

if [ "$MODE" = "check" ]; then
  echo "RESULT: DRIFT (--check, no se escribio nada)."
  echo "         Corre 'bash scripts/reconcile_state_pointer.sh' para repararlo."
  exit 0
fi

# --- Aviso honesto sobre el flag de version antes de escribir ---------------
# Mover el puntero a un commit cuyo subject NO es un bump, mientras el
# puntero declara una version distinta de la de ese commit, es
# precisamente el estado que el check 3c del guard considera incoherente.
# No es un error de este script: es que el humano deberia decidir si el
# bump es correcto. Se avisa, no se bloquea, porque el drift del puntero
# es peor que la ventana de auto-referencia.
if ! printf '%s' "$target_subject" | grep -qE 'bump .*-> *[0-9]+\.[0-9]+\.[0-9]+'; then
  echo "AVISO: el commit objetivo ($target_short) NO es un bump de version, pero el"
  echo "       puntero declarara $real_ver. El check 3c del guard aceptara esto"
  echo "       como 'subject del puntero no es un bump', pero conviene que"
  echo "       confirmes que $real_ver es la version que ese commit describe."
  echo
fi

# --- Escritura atomica ------------------------------------------------------
# Se escribe a un temporal y se renombra: un fallo a mitad de camino no
# deja STATE.yaml truncado, que seria peor que la deriva que se intenta
# corregir.
tmp=$(mktemp "${STATE}.XXXXXX") || { echo "FATAL: mktemp fallo" >&2; exit 1; }
trap 'rm -f "$tmp"' EXIT

python3 - "$STATE" "$tmp" "$target_sha" "$target_short" "$real_ver" "$current_sha" "$current_ver" "$sha_reconcilable" <<'PY'
import re
import sys

(state_path, out_path, target_sha, target_short,
 real_ver, current_sha, current_ver, sha_reconcilable) = sys.argv[1:9]

with open(state_path, encoding="utf-8") as fh:
    text = fh.read()

if sha_reconcilable == "1":
    # Solo la version estaba mal. NO se toca current_sha ni su nota: el
    # puntero ya senalaba al commit correcto y su evidencia vale mas que
    # la marca de este script.
    pass
else:
    new_ver = f'  current_sha: "{target_short}"  # reconciliado por scripts/reconcile_state_pointer.sh sobre main={target_short} ({real_ver}). El puntero anterior ({current_sha or "<none>"}/{current_ver or "<none>"}) y su nota de evidencia quedan en superseded_pointer y en SESSION-JOURNAL.md; este script no reescribe historia.'
    text, n1 = re.subn(r'^  current_sha: *"[0-9a-f]{7,40}".*$', new_ver, text,
                        count=1, flags=re.M)
    if n1 != 1:
        sys.exit(f"no encontre exactamente una linea current_sha (n={n1})")

    text, n2 = re.subn(r'^  head_at_state_sync: *"[^"]*".*$',
                       f'  head_at_state_sync: "{target_short}"', text,
                       count=1, flags=re.M)
    if n2 == 0:
        sys.exit("no encontre head_at_state_sync")

# superseded_pointer: se inserta SOLO si no existia, y solo cuando se
# movio el SHA (mover el puntero sin dejar constancia de a donde venia
# seria reescribir historia por la puerta de atras).
if sha_reconcilable != "1" and not re.search(r'^  superseded_pointer:.*$', text, flags=re.M):
    superseded = (
        f'  superseded_pointer: "{current_sha or "<none>"}/{current_ver or "<none>"}'
        f' (auto-registrado por scripts/reconcile_state_pointer.sh al reconciliar'
        f' {state_path}). La evidencia del puntero anterior se conserva en'
        f' SESSION-JOURNAL.md; este script no reescribe historia, solo mueve el'
        f' puntero y deja constancia de a donde venia.'
    )
    text, n5 = re.subn(r'^(  head_at_state_sync:.*)$', rf'\1\n{superseded}',
                       text, count=1, flags=re.M)
    if n5 != 1:
        sys.exit("no pude insertar superseded_pointer tras head_at_state_sync")
    note = "superseded_pointer: INSERTADO (no existia)"
else:
    note = "superseded_pointer: preservado tal cual (contiene nota manual)"

# La version se corrige SIEMPRE que sea este el unico drift.
text, n4 = re.subn(r'^(  workspace_version_at_current: *")[^"]*(")',
                    rf'\g<1>{real_ver}\g<2>', text, count=1, flags=re.M)
if n4 != 1:
    sys.exit(f"no encontre exactamente una linea workspace_version_at_current (n={n4})")

with open(out_path, "w", encoding="utf-8") as fh:
    fh.write(text)

print(f"  {note}")
PY
write_rc=$?
[ "$write_rc" -eq 0 ] || { echo "FATAL: la escritura fallo, STATE.yaml intacto" >&2; exit 1; }

# Valida el YAML antes de sustituir el fichero real. Una escritura que
# produce YAML invalido es peor que no escribir: el proximo guard no
# podria ni leer el puntero.
#
# if-then-else explicito y NO `A && B || C` (SC2015): aqui `mv` puede
# fallar de verdad, y con la forma `&&`/`||` el fallback se ejecutaria
# tambien, moviendo el temporal por segunda vez. La forma explicita
# distingue "no se puede validar" de "la escritura fallo".
if python3 -c "import yaml,sys; yaml.safe_load(open(sys.argv[1],encoding='utf-8'))" "$tmp" 2>/dev/null; then
  if mv "$tmp" "$STATE"; then
    echo "  escritura: OK (YAML valido, fichero sustituido)"
  else
    echo "FATAL: mv fallo, STATE.yaml intacto" >&2
    exit 1
  fi
else
  # Sin PyYAML no se puede validar, pero el temporal tampoco se descarta:
  # se conserva para que un humano pueda inspeccionarlo.
  backup="${STATE}.reconcile-candidate"
  mv "$tmp" "$backup"
  echo "  escritura: SIN VALIDAR (no hay PyYAML instalado)"
  echo "  el resultado quedo en $backup; revisalo y muevelo a mano si es correcto:"
  echo "    mv $backup $STATE"
  exit 0
fi

echo
if [ "$sha_reconcilable" = "1" ]; then
  echo "RESULT: RECONCILIADO — solo la version estaba desalineada; ahora es $real_ver."
  echo "         current_sha NO se toco ($current_sha, dentro de tolerancia): su nota"
  echo "         de evidencia es mas informativa que la marca de este script."
else
  echo "RESULT: RECONCILIADO — el puntero ahora apunta a $target_short / $real_ver."
fi
echo "         Falta commitear (docs/**, no requiere bump) y, si quieres dejar"
echo "         constancia, una linea en CURRENT.md y SESSION-JOURNAL.md:"
echo "         el juicio sobre QUE significa este estado sigue siendo humano."
