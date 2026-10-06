#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Canario de aislamiento entre proyectos, y su autofalsacion.
#
# ## QUE INTENTA REPRODUCIR
#
# Una sesion que debe continuar el proyecto A entrega contexto del proyecto B.
# El fixture usa dos repos con nombres parecidos (`pipeline-kotlin` /
# `pipelinek-fabric`) porque ese es el incidente reportado, y ADEMAS una segunda
# pareja con **basename identico** en directorios distintos, que es lo que
# separa de verdad "identidad por remote" de "identidad por nombre".
#
# ## LO QUE ESTE CANARIO NO HACE
#
# No arregla. No compensa. Su unico producto es un veredicto: reproducido o no.
# Un canario que no puede fallar no es un canario, asi que su falsador tiene que
# poder ponerlo en rojo.
#
# ## AISLAMIENTO
#
# `SDDK_DATA_DIR` a un temporal. Esta maquina tiene 265 proyectos reales y el
# canario no toca ninguno.
# ---------------------------------------------------------------------------
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || exit 1

WORK="$(mktemp -d)"
export SDDK_DATA_DIR="$WORK/data"
TARGET="$WORK/target"
export CARGO_TARGET_DIR="$TARGET"

# shellcheck disable=SC2329  # invocada por el trap de EXIT, que shellcheck no ve
cleanup() {
    # Se lleva el directorio temporal del canario. `rm` en este entorno es un
    # shim que NO expande variables, asi que el trash se invoca con su ruta
    # absoluta y el objetivo ya expandido aqui.
    [ -d "$WORK" ] || return 0
    /home/rubentxu/.minimax/bin/mavis-trash -- "$WORK" >/dev/null 2>&1
    return 0
}
trap cleanup EXIT

PASS=0; FAIL=0; SKIP=0
BIN="$WORK/target/debug/sddk"

log()  { printf '  %s\n' "$*"; }
ok()   { printf '  [ok]   %s\n' "$*"; PASS=$((PASS+1)); }
ko()   { printf '  [FAIL] %s\n' "$*"; FAIL=$((FAIL+1)); }
skip() { printf '  [SKIP] %s — %s\n' "$1" "$2"; SKIP=$((SKIP+1)); }

# --- fixtures ---------------------------------------------------------------

mk_repo() { # $1=directorio  $2=remote (vacio = sin remote)
    local dir="$1" remote="$2"
    mkdir -p "$dir"
    git -C "$dir" init -q .
    git -C "$dir" config user.email canary@example.invalid
    git -C "$dir" config user.name canary
    if [ -n "$remote" ]; then
        git -C "$dir" remote add origin "$remote"
    fi
    printf 'fixture\n' > "$dir/README.md"
    git -C "$dir" add README.md
    git -C "$dir" commit -qm init
}

build() {
    if ! cargo build -p sddk-cli --bin sddk > "$WORK/build.log" 2>&1; then
        printf 'ERROR: la compilacion fallo\n'
        tail -20 "$WORK/build.log"
        return 1
    fi
    BIN="$TARGET/debug/sddk"
    return 0
}

fixtures() {
    local c="$WORK/canario"
    rm -rf "$c" 2>/dev/null
    mk_repo "$c/pipeline-kotlin"  "https://github.com/example/pipeline-kotlin.git"
    mk_repo "$c/pipelinek-fabric" "https://github.com/example/pipelinek-fabric.git"
    # La pareja que separa remote de nombre: mismo basename, remotes distintos.
    mk_repo "$c/otro-a/proyecto" "https://github.com/example/pipeline-kotlin.git"
    mk_repo "$c/otro-b/proyecto" "https://github.com/example/pipelinek-fabric.git"
    # Y una pareja SIN remote, para poder ejercer el camino de fallback.
    mk_repo "$c/sin-remote-uno/proyecto" ""
    mk_repo "$c/sin-remote-dos/proyecto" ""
}

pid() { "$BIN" project resolve --root "$1" --scope . 2>/dev/null \
        | awk -F': ' '/^project_id:/ {print $2}' | tr -d ' '; }

# --- el canario -------------------------------------------------------------
# Devuelve 0 si NO hay aislamiento roto. Todo lo que encuentra va al log.

# Cada corrida usa un directorio de datos PROPIO.
#
# MEDIDO, y no fue hipotetico: la primera version reutilizaba el mismo
# `SDDK_DATA_DIR` en todas las corridas, asi que los bindings que escribio la
# corrida BASE seguian ahi cuando se ejecutaba una mutacion. Con M2 —que
# justamente hace que todos los proyectos compartan directorio de bindings— la
# comprobacion `el binding de A existe` segia viendo el fichero de la corrida
# base, con el project_id correcto, y el canario daba VERDE CON EL DEFECTO
# PUESTO. El mutado funcionaba; el instrumento no lo veia.
#
# **Un control que pasa por un fichero que dejo la corrida anterior no esta
# mirando lo que dice mirar.** Es la misma clase que el `id` del primer
# PRE-FLIGHT de VA14, con el signo cambiado: alli no mire donde decia, aqui no
# miraba lo que decia.
RUN=0
canary() {
    RUN=$((RUN + 1))
    local DATA="$WORK/data/run-$RUN"
    export SDDK_DATA_DIR="$DATA"

    local c="$WORK/canario"
    local A B A2 B2 N1 N2
    A="$(pid "$c/pipeline-kotlin")"
    B="$(pid "$c/pipelinek-fabric")"
    A2="$(pid "$c/otro-a/proyecto")"
    B2="$(pid "$c/otro-b/proyecto")"
    N1="$(pid "$c/sin-remote-uno/proyecto")"
    N2="$(pid "$c/sin-remote-dos/proyecto")"

    local violaciones=0
    # V1: dos remotes distintos jamas pueden compartir project_id.
    [ "$A" = "$B" ]  && { log "V1 COLISION similar-nombre: $A == $B"; violaciones=$((violaciones+1)); }
    [ "$A2" = "$B2" ] && { log "V1 COLISION mismo-basename: $A2 == $B2"; violaciones=$((violaciones+1)); }
    [ "$N1" = "$N2" ] && { log "V1 COLISION sin-remote: $N1 == $N2"; violaciones=$((violaciones+1)); }
    # V0: un id resuelto por remote jamas puede igualar a uno resuelto por fallback.
    [ -n "$A" ] && [ "$A" = "$N1" ] && { log "V0 COLISION remote/fallback: $A"; violaciones=$((violaciones+1)); }

    [ -z "$A" ] && { log "V- no se pudo resolver project_id"; return 1; }

    # V2: la MISMA session en dos proyectos produce dos bindings, y cada uno
    #     declara SU project_id. Si uno declara el del otro, hay fuga.
    local S=canary-s1
    "$BIN" context bootstrap --root "$c/pipelinek-fabric" --session "$S" >/dev/null 2>&1
    "$BIN" context bootstrap --root "$c/pipeline-kotlin"  --session "$S" >/dev/null 2>&1

    # Donde han CAIDO los bindings de verdad, que es el hecho que hay que mirar y
    # no donde uno supone que deberian estar.
    log "bindings escritos bajo: $DATA"
    find "$DATA" -path "*bindings*" -name "$S.json" 2>/dev/null | sed 's/^/    /'

    local fa fb
    fa="$DATA/sddk/projects/$A/context/bindings/$S.json"
    fb="$DATA/sddk/projects/$B/context/bindings/$S.json"
    [ -f "$fa" ] || { log "V2 el binding de A no esta en el directorio de A"; violaciones=$((violaciones+1)); }
    [ -f "$fb" ] || { log "V2 el binding de B no esta en el directorio de B"; violaciones=$((violaciones+1)); }
    if [ -f "$fa" ]; then
        grep -q "\"project_id\": \"$B\"" "$fa" && {
            log "V2 FUGA: el binding de A declara el project_id de B"; violaciones=$((violaciones+1)); }
        grep -q "\"project_id\": \"$A\"" "$fa" || {
            log "V2 el binding de A no declara su propio project_id"; violaciones=$((violaciones+1)); }
    fi
    if [ -f "$fb" ]; then
        grep -q "\"project_id\": \"$A\"" "$fb" && {
            log "V2 FUGA: el binding de B declara el project_id de A"; violaciones=$((violaciones+1)); }
    fi
    # V3: el directorio de bindings debe depender del project_id. Si todos los
    #     proyectos comparten directorio, la separacion es de fachada.
    if [ -n "$A" ] && [ -n "$B" ] && [ "$A" != "$B" ] \
       && [ "$(dirname "$fa")" = "$(dirname "$fb")" ]; then
        log "V3 el directorio de bindings NO depende del project_id"; violaciones=$((violaciones+1))
    fi

    return $((violaciones > 0))
}

# --- ejecucion --------------------------------------------------------------

printf '== canario de aislamiento entre proyectos ==\n'
fixtures
build || exit 1
log "binario: $BIN"
log "datos aislados en: $SDDK_DATA_DIR"

printf '\n--- BASE ---\n'
if canary; then
    ok "base: NO se reproduce el cross-project leak (0 violaciones)"
else
    ko "base: el aislamiento esta ROTO — el canario reproduce el incidente"
    printf '\nRESULT: REPRODUCIDO — se para aqui. El brief dice no reparar sin saber cual ocurrio.\n'
    exit 1
fi

printf '\n--- MUTACIONES ---\n'

mutar() { # $1=etiqueta  $2=fichero  $3=esperado(ROJO|SKIP)  $4=porque  $5=pycode
    local etiqueta="$1" fichero="$2" esperado="$3" porque="$4" pycode="$5"
    local sha_antes sha_restore

    sha_antes="$(sha256sum "$fichero" | cut -d' ' -f1)"
    cp "$fichero" "$WORK/mut.bak"

    if ! MUT_FILE="$fichero" python3 -c "$pycode" 2>"$WORK/mut.err"; then
        skip "$etiqueta" "el parche lanzo error: $(tail -1 "$WORK/mut.err")"
        cp "$WORK/mut.bak" "$fichero"; return
    fi
    if [ "$sha_antes" = "$(sha256sum "$fichero" | cut -d' ' -f1)" ]; then
        skip "$etiqueta" "el parche no cambio el fichero: no esta midiendo nada"
        cp "$WORK/mut.bak" "$fichero"; return
    fi

    if ! build; then
        skip "$etiqueta" "el parche dejo el codigo sin compilar: mide que no arranca, no la propiedad"
        cp "$WORK/mut.bak" "$fichero"; build >/dev/null 2>&1; return
    fi

    local reproduce="no"
    canary && reproduce="no" || reproduce="si"
    cp "$WORK/mut.bak" "$fichero"
    build >/dev/null 2>&1
    sha_restore="$(sha256sum "$fichero" | cut -d' ' -f1)"
    if [ "$sha_restore" != "$sha_antes" ]; then
        printf '  [FATAL] %s — la restauracion no fue byte-identica\n' "$etiqueta"
        exit 1
    fi

    case "$esperado" in
        ROJO)
            if [ "$reproduce" = "si" ]; then ok "$etiqueta — el canario cae. $porque"
            else ko "$etiqueta — el canario SIGUE VERDE con el defecto introducido. $porque"; fi
            ;;
        SKIP)
            if [ "$reproduce" = "si" ]; then skip "$etiqueta" "se esperaba que NO pudiera caer y cae: el analisis previo era erroneo, $porque"
            else skip "$etiqueta" "$porque"; fi
            ;;
    esac
}

ID=crates/sddk-domain/src/identity.rs
CTX=crates/sddk-cli/src/context_cmd.rs

mutar "M1 el project_id deja de mirar el remote" "$ID" ROJO \
    "con el remote fuera del hash, dos remotes distintos dan el mismo id: es el falsador directo de H1." \
'
import os
p=os.environ["MUT_FILE"]; s=open(p).read()
v="&[normalized_remote, scope]"; n="&[\"\", scope]"
assert v in s, "el hash del remote no tiene la forma que esta mutacion supone"
open(p,"w").write(s.replace(v,n,1))
'

mutar "M2 el directorio de bindings ignora el project_id" "$CTX" ROJO \
    "los tres load_binding siguen leyendolo, pero ahora todos los proyectos comparten directorio: el namespacing era la unica defensa y se la quita." \
'
import os
p=os.environ["MUT_FILE"]; s=open(p).read()
v="""fn bindings_root(project_data: &Path) -> PathBuf {
    project_data.join("context").join("bindings")
}"""
n="""fn bindings_root(project_data: &Path) -> PathBuf {
    project_data
        .parent()
        .map_or_else(|| project_data.to_path_buf(), |p| p.join("bindings-compartidos"))
}"""
assert v in s, "bindings_root no tiene la forma que esta mutacion supone"
open(p,"w").write(s.replace(v,n,1))
'

mutar "M3 el seed de fallback deja de mirar la ruta" "$ID" ROJO \
    "sin remote, la identidad cae al seed; si el seed no mira la ruta, los dos repos sin remote colisionan." \
'
import os
p=os.environ["MUT_FILE"]; s=open(p).read()
v="""    let hex = framed_hash("sddk.project.fallback.seed.v1", &[canonical_workspace_path]);"""
n="""    let _ = canonical_workspace_path;
    let hex = framed_hash("sddk.project.fallback.seed.v1", &[""]);"""
assert v in s, "el seed de fallback no tiene la forma que esta mutacion supone"
open(p,"w").write(s.replace(v,n,1))
'

mutar "M4 el resolutor deja de leer el remote" "$CTX" SKIP \
    "PRE-DECIDO: no puede dar rojo. Sin remote cae al seed de RUTA, y dos rutas distintas dan seeds distintos. El brief espera rojo aqui porque supone que quitar el remote rompe la identidad; medido, la rompe solo si tambien se quita la ruta, que es M3. Un test declarado rojo que no puede estarlo no prueba nada." \
'
import os
p=os.environ["MUT_FILE"]; s=open(p).read()
v="""    let remote =
        resolve_remote(root, None).map_err(|e| ContextBootstrapError::Io(e.to_string()))?;"""
n="""    let remote: Option<String> = None;"""
assert v in s, "la resolucion del remote no tiene la forma que esta mutacion supone"
open(p,"w").write(s.replace(v,n,1))
'

printf '\n----------------------------------------\n'
printf 'PASS=%d FAIL=%d SKIP=%d\n' "$PASS" "$FAIL" "$SKIP"
if [ "$FAIL" -eq 0 ]; then
    printf 'RESULT: PASS — %d comprobacion(es) tienen dientes; %s mutacion(es) se declararon no aplicables.\n' "$PASS" "$SKIP"
    exit 0
fi
printf 'RESULT: FAIL — hay comprobaciones que no vigilan lo que declaran.\n'
exit 1