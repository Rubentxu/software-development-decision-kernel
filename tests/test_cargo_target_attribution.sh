#!/usr/bin/env bash
# test_cargo_target_attribution.sh
#
# MEDIDO (session-82, INC-DEBT-075): `_cargo_uses_target` atribuia a un `cargo`
# la retencion de un target dir que NO era el suyo. La rama 3 decia «el
# directorio de trabajo del proceso es el repo ... un cargo lanzado aqui sin
# declarar target compila donde le digan, QUE SUEDE SER ESTE». Ese «suele» es
# una suposicion, y se puede MEDIR que es falsa: un `cargo` sin target declarado
# con nuestro mismo cwd puede estar compilando en el target dir compartido de
# la maquina, que no es el que se le pregunta.
#
# Por que importa mas de lo que parece: el aviso produced dice «el target dir
# compartido esta retenido: pid N». Si N no esta esperando ese target, el aviso
# se refiere a OTRO directorio, y quien lo lee no tiene forma de saberlo. Un
# aviso que se refiere a otra cosa es peor que no avisar.
#
# EL ARMA: un repo en miniatura con un proyecto cargo real, un sujeto `cargo`
# de verdad, y la libreria REAL. No hace falta abrir un seam en produccion para
# poder romperlo: basta con darle otro sitio donde estar.
#
# TERCEROS, y por que estan:
#   E1  el sujeto NO usa el target preguntado -> NO se debe reportar
#   E2  el sujeto declara OTRO target         -> NO se debe reportar (control)
#   E3  el sujeto SI usa el target preguntado -> SE DEBE reportar
#
# E3 es el que importa: sin el, «E1 ya no reporta» se consigue ROMPIENDO la
# deteccion, que es el modo mas barato de poner en verde un guard.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB="$ROOT/scripts/lib/release_diagnostics.sh"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/sddk-atrib.XXXXXX")"

# MEDIDO: este guard NO era hermetico respecto al entorno, y por eso daba verde
# en una shell y rojo en el 1b de la release 2.11.3. La razon es una asimetria
# entre las dos mitades del experimento:
#
#   - el SUJETO se lanza con `env -u CARGO_TARGET_DIR` (linea 78), luego su
#     target efectivo sale de `build.target-dir` del mini-repo, que es $T3;
#   - la PREGUNTA la hace `_cargo_effective_target`, que corre `cargo metadata`
#     en el proceso del propio guard y por lo tanto HEREDA su entorno.
#
# MEDIDO que esa es la causa y no otra: con la variable puesta, el mini-repo
# declara el target global y el sujeto —que corre sin ella— no aparece nunca en
# el aviso; quitandola, el aviso aparece con el pid correcto.
#
# Y la semántica de la libreria es CORRECTA y no se toca: «el target que cargo
# usaria con este cwd» se resuelve como lo resolvería el proceso que pregunta,
# que es lo unico que un preflight puede afirmar. El guard es el que estaba mal
# armado:hacer una pregunta con un entorno que su propio sujeto no tiene.
# Por eso la variable se quita UNA vez, aqui, y no en cada invocacion.
unset CARGO_TARGET_DIR

# shellcheck disable=SC2329
cleanup() { rm -rf "$WORK" >/dev/null 2>&1 || true; }
trap cleanup EXIT

PASS=0
FAIL=0
asert() {
    if [ "$2" = "1" ]; then
        printf '  [ok]   %s\n' "$1"; PASS=$((PASS + 1))
    else
        printf '  [FAIL] %s -- %s\n' "$1" "${3:-}"; FAIL=$((FAIL + 1))
    fi
}

if [ ! -f "$LIB" ]; then
    printf 'RESULT: FAIL — no existe la libreria que este guard debe ejecutar: %s\n' "$LIB"
    exit 1
fi
# shellcheck source=../scripts/lib/release_diagnostics.sh
# SC1091, MEDIDO: el gate corre shellcheck SIN `-x`, luego el `source=` de
# arriba no evita el aviso. Se declara en vez de dejar que el gate decida.
# shellcheck disable=SC1091
source "$LIB"

printf '=== atribucion del target dir (INC-DEBT-075) ===\n'

# --- el sujeto: un ejecutABLE llamado `cargo`, que es el criterio de ---------
# deteccion (`comm`), con un cwd y un target que el caso decide.
mkdir -p "$WORK/stubs"
cat > "$WORK/stubs/cargo" <<'STUB'
#!/bin/sh
echo $$ > "$PIDFILE"
sleep 30
STUB
chmod +x "$WORK/stubs/cargo"

# lanzar <cwd> <target-declarado-o-vacio> — imprime el pid del sujeto.
lanzar() {
    local cwd="$1" target="${2:-}" i=0
    : > "$WORK/pid"
    if [ -n "$target" ]; then
        ( cd "$cwd" && CARGO_TARGET_DIR="$target" PIDFILE="$WORK/pid" \
            setsid --fork "$WORK/stubs/cargo" </dev/null >/dev/null 2>&1 )
    else
        ( cd "$cwd" && env -u CARGO_TARGET_DIR PIDFILE="$WORK/pid" \
            setsid --fork "$WORK/stubs/cargo" </dev/null >/dev/null 2>&1 )
    fi
    while [ ! -s "$WORK/pid" ] && [ "$i" -lt 80 ]; do sleep 0.1; i=$((i + 1)); done
    cat "$WORK/pid" 2>/dev/null || printf 'NONE'
}
parar() { [ -s "$WORK/pid" ] && kill "$(cat "$WORK/pid")" 2>/dev/null; sleep 0.3; return 0; }

# --- E1: el sujeto NO usa el target que se le pregunta ---------------------
# cwd = un directorio que NO es un proyecto cargo con el target preguntado, y
# sin target declarado. Antes del arreglo, la rama del cwd lo contaba.
#
# MEDIDO al escribir este fichero: estas comprobaciones estaban dentro de
# `( ... )` para poder hacer `cd`, y un `( )` es un SUBSHELL, luego sus
# incrementos de PASS/FAIL se pierden al salir. El resultado medido fue
# `PASS=1 FAIL=0` con un `[FAIL]` impreso delante: un guard VERDE con un caso
# caido. Un contador que no sobrevive al subshell no mide el caso, y para el
# `cd` hace falta cambiar de directorio y volver, no abrir un subshell.
ORIG_DIR="$PWD"
volver() { cd "$ORIG_DIR" || exit 2; }
D1="$WORK/d1"; T1="$WORK/t1"
mkdir -p "$D1" "$T1"
cd "$D1" || exit 2
PID="$(lanzar "$D1" "")"
  vivo=0
  [ -n "$PID" ] && [ "$PID" != "NONE" ] && [ -d "/proc/$PID" ] && vivo=1
  asert "E1: el sujeto arranco de verdad; si no, el caso pasaria por vacuidad" "$vivo" "pid: '$PID'"
salida="$(release_check_cargo_lock "$T1" 2>&1)"
parar
  if printf '%s' "$salida" | grep -qF "pid $PID"; then
      asert "E1: un cargo que NO usa el target preguntado NO se reporta" 0 \
          "lo reporto: $salida"
else
      asert "E1: un cargo que NO usa el target preguntado NO se reporta" 1
fi

# --- E2 (control): el sujeto declara OTRO target --------------------------
cd "$D1" || exit 2
PID="$(lanzar "$D1" "$WORK/otro-cualquiera")"
  salida="$(release_check_cargo_lock "$T1" 2>&1)"
  parar
  asert "E2: un cargo que declara otro target no se reporta" \
      "$([ -z "$salida" ] && echo 1 || echo 0)" "salida: $salida"
volver

# --- E3: el sujeto SI usa el target preguntado -> SE DEBE reportar --------
# El escenario se construye de verdad: un proyecto cargo cuyo `build.target-dir`
# es el target preguntado. Si el escenario no se puede construir, el control
# no probaria nada y se dice, en vez de contarlo como verde.
D3="$WORK/proyecto"; T3="$WORK/target-real"
mkdir -p "$D3/.cargo" "$D3/src" "$T3"
printf '[package]\nname="simulado"\nversion="0.1.0"\nedition="2021"\n' > "$D3/Cargo.toml"
printf '[build]\ntarget-dir = "%s"\n' "$T3" > "$D3/.cargo/config.toml"
printf 'fn main() {}\n' > "$D3/src/main.rs"
# MEDIDO: esta invocacion era `cargo metadata` a secas, sin `env -u
# CARGO_TARGET_DIR`, y eso hacia que E3 DIESE en el 1b de la release 2.11.3 con
# `cargo metadata dice: '/var/home/rubentxu/cargo-targets'`. La causa: una
# variable de entorno `CARGO_TARGET_DIR` AMBIENTAL gana a `build.target-dir` del
# `.cargo/config.toml` del mini-repo, luego el escenario no declaraba el target
# que dice declarar y el control no probaba nada. MEDIDO con el mini-repo a
# mano: sin la variable, `cargo metadata` devuelve `$T3`; con ella, devuelve el
# target global.
#
# `lanzar` ya usaba `env -u CARGO_TARGET_DIR` en su rama de target vacio (linea
# 78) — la asimetria era la senal, y por eso el sujeto se reportaba mientras
# el escenario que lo levanta no. Por que no lo delata antes: E3 es el unico caso
# que exige que el sujeto SI se reporte, luego es el unico cuyo rojo delata un
# escenario roto. E1 y E2 pasaban con el escenario contaminado porque esperan
# «NO se reporta», y un escenario que no se construye nunca lo cumple.
efectivo="$( cd "$D3" && env -u CARGO_TARGET_DIR cargo metadata --no-deps --format-version 1 2>/dev/null \
    | tr ',' '\n' | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p' | head -1 )"
asert "E3: el escenario declara de verdad que el target efectivo es el preguntado" \
    "$([ "$(readlink -f "$efectivo" 2>/dev/null || echo "$efectivo")" = "$(readlink -f "$T3")" ] && echo 1 || echo 0)" \
    "cargo metadata dice: '$efectivo'"

cd "$D3" || exit 2
PID="$(lanzar "$D3" "")"
salida="$(release_check_cargo_lock "$T3" 2>&1)"
parar
  asert "E3: un cargo que SI usa el target preguntado se sigue reportando" \
      "$(printf '%s' "$salida" | grep -qF "pid $PID" && echo 1 || echo 0)" \
      "pid: '$PID' salida: $salida"
volver

printf '\nPASS=%d FAIL=%d\n' "$PASS" "$FAIL"
if [ "$FAIL" = "0" ]; then
    printf 'RESULT: PASS — el aviso se refiere al target dir que el sujeto USA.\n'
    exit 0
fi
printf 'RESULT: FAIL — hay ramas que atribuyen a un cargo un target que no es suyo.\n'
exit 1
