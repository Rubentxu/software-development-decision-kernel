#!/usr/bin/env bash
# Falsifica verificar-medicion.py de C3m.4 con mutaciones aplicadas al SOURCE REAL.
#
# Un verificador de medicion que nunca ha sido contradicho no es un verificador:
# es una prosa con exit code. Cada mutacion rompe UNA afirmacion del SCOPE y el
# verificador tiene que CAER. Se declaran las tres categorias, porque
# NO MEDIBLE no es DETECTADA y una mutacion que sobrevive es informacion sobre el
# alcance del check, no un detalle a ocultar.
#
# Las mutaciones se aplican sobre una COPIA del repo, nunca sobre el real.

set -uo pipefail

RAIZ="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
TRAB="$(mktemp -d)"
trap '"$HOME/.minimax/bin/mavis-trash" -- "$TRAB"' EXIT

DETECTADAS=0
SOBREVIVAS=0
NOMEDIBLES=0
TOTAL=0

aplicar() { # $1=nombre mutacion  $2=subshell que hace el cambio
  local nombre="$1" cambio="$2"
  TOTAL=$((TOTAL + 1))
  rm -rf "$TRAB/repo"
  cp -a "$RAIZ" "$TRAB/repo"
  ( cd "$TRAB/repo" && eval "$cambio" ) >/dev/null 2>&1
  local out ec
  out="$(cd "$TRAB/repo" && python3 docs/roadmap/receipts/c3m4-evidence-states/verificar-medicion.py 2>&1)"
  ec=$?
  if [ "$ec" -ne 0 ]; then
    printf '  DETECTADA  %-52s -> exit %d\n' "$nombre" "$ec"
    DETECTADAS=$((DETECTADAS + 1))
  else
    printf '  SOBREVIVIDA %-51s -> el verificador dio verde\n' "$nombre"
    SOBREVIVAS=$((SOBREVIVAS + 1))
    [ -n "${VERBOSO:-}" ] && printf '%s\n' "$out" | sed 's/^/      /'
  fi
}

echo "Controles: el verificador sin mutaciones debe dar verde y salir 0"
if ! (cd "$RAIZ" && python3 docs/roadmap/receipts/c3m4-evidence-states/verificar-medicion.py >/dev/null 2>&1); then
  echo "  el verificador ya falla en el estado base: la falsificacion no significa nada"
  exit 1
fi
echo "  OK, el estado base da verde"
echo

echo "Mutaciones aplicadas al source real (cada una rompe UNA afirmacion del SCOPE):"

# M1 — la cita rota "se arregla": aparece el SCOPE-CONTRACT.md que :6 cita.
#      Si el verificador solo mira "existe", dejaria de detectar el hallazgo.
aplicar "M1 el SCOPE citado aparece (la cita deja de estar rota)" \
  'mkdir -p tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s7b-storage-snapshot-l1-consumer && printf "# sc\nconfidence 0.95\n" > tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s7b-storage-snapshot-l1-consumer/SCOPE-CONTRACT.md'

# M2 — el SCOPE real de G01 pasa a exigir la clausula de la confianza.
aplicar "M2 G01 si menciona confidence (deja de ser invencion)" \
  'printf "\nG01 ademas exige confidence 0.95/0.5.\n" >> tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s7-secretary-attention/SCOPE-CONTRACT.md'

# M3 — la fila canonica de G01 se duplica: la ambiguedad de "la fila" desaparece.
aplicar "M3 la fila canonica de G01 se duplica" \
  'printf "\n| G01 | IT | otra fila con confidence 0.9 | x |\n" >> docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md'

# M4 — la fila canonica empieza a exigir la confianza.
aplicar "M4 la fila canonica de G01 exige confidence" \
  'sed -i "s#| G01 | IT | snapshot Planning reconciliado, A bloquea B |#| G01 | IT | confidence 0.95/0.5 |#" docs/history/proposals/all-proposals/2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md'

# M5 — el RECEIPT deja de declarar PASS.
aplicar "M5 el RECEIPT deja de declarar PASS" \
  'sed -i "s/| G01 | PASS |/| G01 | BLOCKED |/" tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s7b-storage-snapshot-l1-consumer/RECEIPT.md'

# M6 — la UAT-EVIDENCE deja de mencionar 0.95.
aplicar "M6 la UAT-EVIDENCE deja de mencionar 0.95" \
  'sed -i "s/0.95//g" tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s7b-storage-snapshot-l1-consumer/UAT-EVIDENCE.yaml'

# M7 — aparece un noveno consumidor en produccion de la confianza.
aplicar "M7 aparece un noveno consumidor en produccion" \
  'printf "\npub fn _gate(p: &SecretaryProposal) -> bool { p.confidence > 0.9 }\n" >> crates/sddk-gateway/src/storage_snapshot_l1_consumer.rs'

# M8 — el numero magico se mueve de sitio: la escritura deja de estar en :124.
aplicar "M8 la escritura de 0.95/0.5 se mueve de la linea 124" \
  'sed -i "s#let confidence = if snapshot.log_head > 0 { 0.95 } else { 0.5 };#let confidence = if snapshot.log_head > 0 { 0.90 } else { 0.5 };#" crates/sddk-gateway/src/storage_snapshot_l1_consumer.rs'

# M9 — se devuelve la rama muerta de test_select (0.0 alcanzable).
aplicar "M9 la rama 0.0 de test_select pasa a ser alcanzable" \
  'sed -i "687s/if prop.has_unmapped {/if false {/" crates/sddk-domain/src/test_select.rs'

# M10 — has_unmapped se muta entre el return y la lectura.
aplicar "M10 has_unmapped se muta entre el guard y la lectura" \
  'sed -i "709i\\            prop.has_unmapped = true;" crates/sddk-domain/src/test_select.rs'

# M11 y M12 apuntan a los dos checks que una primera falsificacion dejo
# sobrevividos. Si uno se "arregla" para dar verde y el arreglo no tiene
# dientes propios, vuelven a sobrevivir sin que nadie lo note.
aplicar "M11 el codigo de produccion se esconde tras un modulo cfg(test)" \
  'printf "\npub fn _oculto(p: &SecretaryProposal) -> bool { p.confidence > 0.9 }\n" >> crates/sddk-cli/src/uat.rs'

aplicar "M12 el numero magico se va y la clausula se queda huerfana" \
  'sed -i "s#0.95/0.5 by head, asserted in unit + integration#by head, asserted in unit + integration#" tests/cycle-artifacts/p-63676b11dc0ef88f/aiw-s7b-storage-snapshot-l1-consumer/RECEIPT.md'

echo
echo "RESULTADO: DETECTADAS=$DETECTADAS SOBREVIVAS=$SOBREVIVAS NO_MEDIBLES=$NOMEDIBLES de $TOTAL"
[ "$SOBREVIVAS" -gt 0 ] && echo "Una sobrevida es informacion: dice que el check no cubre esa afirmacion." && \
                           echo "Se corrige el CHECK, no la exigencia, y no se baja el liston."
exit 0
