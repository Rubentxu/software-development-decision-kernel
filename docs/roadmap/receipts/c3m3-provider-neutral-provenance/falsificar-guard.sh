#!/usr/bin/env bash
# Falsifica las afirmaciones de ADR-0155 / C3m.3 con mutaciones al SOURCE REAL.
#
# El guard reescrito (t_ar_5c_no_provider_name_in_the_core_port) es el unico
# mecanismo que impide que el nombre de un proveedor vuelva al enum del core.
# Un guard que nunca ha sido contradicho no es un guard: es prosa con codigo de
# salida. Cada mutacion reintroduce el defecto de una forma distinta y el guard
# tiene que CAER.
#
# Se declaran las tres categorias. NO MEDIBLE no es DETECTADA, y una mutacion
# que sobrevive es informacion sobre el alcance del check, no un detalle.
#
# Las mutaciones se aplican sobre una COPIA del repo, nunca sobre el real.

set -uo pipefail

RAIZ="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
TRAB="$(mktemp -d)"
trap '"$HOME/.minimax/bin/mavis-trash" -- "$TRAB"' EXIT

PUERTO="crates/sddk-engine/src/code_intelligence_port.rs"
FAKE="crates/sddk-engine/src/code_intelligence_port_fake.rs"
GUARD="crates/sddk-engine/tests/a6_cc_s1_static_graph_completeness.rs"

DETECTADAS=0
SOBREVIVAS=0
TOTAL=0

aplicar() { # $1=nombre  $2=subshell con el cambio
  local nombre="$1" cambio="$2"
  TOTAL=$((TOTAL + 1))
  rm -rf "$TRAB/repo"
  cp -a "$RAIZ" "$TRAB/repo"
  ( cd "$TRAB/repo" && eval "$cambio" ) >/dev/null 2>&1
  local out ec
  out="$(cd "$TRAB/repo" && CARGO_TARGET_DIR=/var/home/rubentxu/cargo-targets \
         cargo test -p sddk-engine --test a6_cc_s1_static_graph_completeness \
         t_ar_5c 2>&1)"
  ec=$?
  if [ "$ec" -ne 0 ]; then
    printf '  DETECTADA  %-56s -> exit %d\n' "$nombre" "$ec"
    DETECTADAS=$((DETECTADAS + 1))
  else
    printf '  SOBREVIVIDA %-55s -> el guard dio verde\n' "$nombre"
    SOBREVIVAS=$((SOBREVIVAS + 1))
    [ -n "${VERBOSO:-}" ] && printf '%s\n' "$out" | tail -20 | sed 's/^/      /'
  fi
}

echo "Control: el guard sin mutaciones debe pasar"
if ! (cd "$RAIZ" && CARGO_TARGET_DIR=/var/home/rubentxu/cargo-targets \
      cargo test -p sddk-engine --test a6_cc_s1_static_graph_completeness t_ar_5c >/dev/null 2>&1); then
  echo "  el guard ya falla en el estado base: la falsificacion no significa nada"
  exit 1
fi
echo "  OK, el estado base pasa"
echo
echo "Mutaciones aplicadas al source real:"

# M1 — el defecto original vuelve: la variante con nombre de producto.
aplicar "M1 vuelve la variante ProviderKind::CogniCode" \
  "sed -i 's/^    External,/    External,\n    \/**\/ reintroducido *\/\n    #[allow(dead_code)]\n    CogniCode,/' $PUERTO"

# M2 — vuelve con OTRO nombre de proveedor, para comprobar que el guard no
# estaba escrito contra una sola cadena sino contra la lista.
aplicar "M2 vuelve la variante con otro nombre (Chronos)" \
  "sed -i 's/^    External,/    External,\n    Chronos,/' $PUERTO"

# M3 — se cuela un tipo publico llamado como el proveedor (lo que el guard
# viejo prohibia y el nuevo sigue prohibiendo).
aplicar "M3 se filtra un tipo pub CogniCode al puerto" \
  "sed -i 's/^pub enum ProviderKind {/pub struct CogniCodeLeak;\npub enum ProviderKind {/' $PUERTO"

# M4 — se borra la identidad como dato: el enum queda sin forma de decir QUE
# proveedor produjo los datos. El guard debe caer por el punto 3, no por el 1.
aplicar "M4 desaparece provider_id (la identidad deja de ser expresable)" \
  "sed -i '/pub provider_id: String,/d' $PUERTO"

# M5 — se borra la categoria External: el adaptador real solo podria reportar
# Null o Fake, las dos falsas.
aplicar "M5 desaparece ProviderKind::External" \
  "sed -i 's/^    External,/    \/**\/ eliminado *\//' $PUERTO"

# M6 — Null vuelve a llevar un id, que es la misma mentira que el cambio quita.
# La primera pasada de esta falsificacion dio VERDE aqui: el codigo estaba
# escrito y comentado, pero NADA lo vigilaba. Se anadio el punto (4) al guard.
aplicar "M6 Null vuelve a declarar un provider_id (mentira reintroducida)" \
  "sed -i 's/provider_id: String::new()/provider_id: \"algo\".to_owned()/' $FAKE"

# M7 — el guard mismo se debilita. Esto NO es detectable desde dentro del guard
# por construccion: un guard mas debil sobre un fuente sano DEBE dar verde, y
# es lo correcto. Se mide con el VERIFICADOR, que comprueba la lista de nombres
# del guard como propiedad del guard. Se declara como NO MEDIBLE para el guard
# y MEDIBLE para el verificador, en vez de contarlo como una sobrevida del guard
# que no lo es.
TOTAL=$((TOTAL + 1))
rm -rf "$TRAB/repo"; cp -a "$RAIZ" "$TRAB/repo"
( cd "$TRAB/repo" && sed -i 's/\["CogniCode", "Chronos", "CodeIntelligence"\]/["CogniCode"]/' "$GUARD" ) >/dev/null 2>&1
if (cd "$TRAB/repo" && python3 docs/roadmap/receipts/c3m3-provider-neutral-provenance/verificar-medicion.py >/dev/null 2>&1); then
  printf '  SOBREVIVIDA %-55s -> el VERIFICADOR no lo ve\n' "M7 el guard vuelve a buscar solo un nombre fijo"
  SOBREVIVAS=$((SOBREVIVAS + 1))
else
  printf '  DETECTADA  %-56s -> exit 1 (via verificador)\n' "M7 el guard vuelve a buscar solo un nombre fijo"
  DETECTADAS=$((DETECTADAS + 1))
fi

# M8 — el guard se debilita Y el defecto vuelve a la vez. Esta es la
# combinacion que M7 en solitario no puede detectar, y la que de verdad importa:
# un guard mas debil solo es peligroso cuando ademas hay algo que vigilar.
TOTAL=$((TOTAL + 1))
rm -rf "$TRAB/repo"; cp -a "$RAIZ" "$TRAB/repo"
( cd "$TRAB/repo" \
  && sed -i 's/\["CogniCode", "Chronos", "CodeIntelligence"\]/["CogniCode"]/' "$GUARD" \
  && sed -i 's/^    External,/    External,\n    Chronos,/' "$PUERTO" ) >/dev/null 2>&1
out="$(cd "$TRAB/repo" && CARGO_TARGET_DIR=/var/home/rubentxu/cargo-targets \
       cargo test -p sddk-engine --test a6_cc_s1_static_graph_completeness t_ar_5c 2>&1)"
if ! (cd "$TRAB/repo" && CARGO_TARGET_DIR=/var/home/rubentxu/cargo-targets \
       cargo test -p sddk-engine --test a6_cc_s1_static_graph_completeness t_ar_5c >/dev/null 2>&1); then
  printf '  DETECTADA  %-56s -> exit 101\n' "M8 guard debilitado + Chronos vuelve a la vez"
  DETECTADAS=$((DETECTADAS + 1))
else
  printf '  SOBREVIVIDA %-55s -> el guard NO lo ve\n' "M8 guard debilitado + Chronos vuelve a la vez"
  SOBREVIVAS=$((SOBREVIVAS + 1))
  [ -n "${VERBOSO:-}" ] && printf '%s\n' "$out" | tail -8 | sed 's/^/      /'
fi

echo
echo "RESULTADO: DETECTADAS=$DETECTADAS SOBREVIVAS=$SOBREVIVAS de $TOTAL"
[ "$SOBREVIVAS" -gt 0 ] && echo "Una sobrevida es informacion: acota que vigila el guard, no que el defecto volviera."
exit 0
