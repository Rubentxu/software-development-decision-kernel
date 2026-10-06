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
# MEDIDO (session-86, al arreglar el hermano de worktree): faltaba aislar el
# ESTADO, y sin el el canario escribia en la maquina. `paths.rs:173` resuelve
# el ledger por `state_home`, cuya precedencia es SDDK_STATE_HOME, luego
# XDG_STATE_HOME, luego $HOME/.local/state — independiente de SDDK_DATA_DIR.
# Este canario creaba un `~/.local/state/sddk/projects/<pid>/ledger.sqlite`
# de verdad en cada corrida. Mismo defecto, misma causa, dos ficheros: lo que
# faltaba no era un override, era la LISTA de overrides.
export SDDK_STATE_HOME="$WORK/state"
export XDG_STATE_HOME="$WORK/state"
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

# Localiza el binding de una sesion por el project_id que DECLARA, no por el
# directorio en el que cae.
#
# MEDIDO (session-86): la primera version lo busco por la ruta
# (`find -path "*$A*"`), y eso es exactamente la propiedad que la mutacion M2
# deshace — con las cuatro raices en un temporal compartido el path ya no
# contiene el project_id y el `find` no encontraba nada. Un localizador que
# depende del layout midio bien la base y dejo muda a la unica mutacion que
# quita ese layout. Por eso aqui se mira el CONTENIDO: el project_id escrito
# dentro del fichero es la ley, y sobrevive a que el directorio cambie.
#
# MEDIDO (session-86, tercera vuelta): con M2 el canario seguia en SKIP, y ya
# no por la ruta sino por el ALCANCE — la mutacion cuelga las cuatro raices de
# `/tmp/sddk-bindings-compartidos`, que queda FUERA de `$DATA`, luego el `find`
# no lo veia. Un localizador que solo busca donde se SUPO que escribir no puede
# detectar que el producto escribe donde no deberia, que es justo la propiedad
# que se quiere medir. Por eso se mira tambien `BIND_SEARCH_EXTRA`: el canario
# busca donde el producto ha escrito de verdad, no solo donde el fixture
# esperaba que escribiera.
BIND_SEARCH_EXTRA=/tmp/sddk-bindings-compartidos
export BIND_SEARCH_EXTRA
bind_por_proyecto() { # $1=project_id  $2=session
    local p
    for p in $(find "$DATA" -name "$2.json" -type f 2>/dev/null | sort) \
             $(find "${BIND_SEARCH_EXTRA:-/nonexistent}" -name "$2.json" -type f 2>/dev/null | sort); do
        if grep -q "\"project_id\": *\"$1\"" "$p" 2>/dev/null; then
            printf '%s\n' "$p"
            return 0
        fi
    done
    return 0
}

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
    # El temporal COMPARTIDO de M2 sobrevive entre corridas —esta fuera del
    # directorio por corrida— y con el el mismo estado compartido que esta
    # cabecera describe, pero por otra via: un binding de la corrida anterior
    # seguiria ahi y `bind_por_proyecto` lo encontraria. Por eso se limpia
    # aqui, no al terminar la mutacion: limpiar despues deja que la medicion
    # de la siguiente vea el rastro de la anterior.
    if [ -d "$BIND_SEARCH_EXTRA" ]; then
        unlink "$BIND_SEARCH_EXTRA/canary-s1.json" 2>/dev/null || true
    fi

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
    #
    # MEDIDO (session-86, con VA17): este bloque IMPRIMIA la localization con
    # `find` y despues comprobaba contra una ruta escrita a mano —
    # `$DATA/sddk/projects/$A/context/bindings/`. Con el layout por worktree el
    # binding cae en `projects/<p>/workspaces/<w>/context/bindings/`, luego el
    # `find` de tres lineas mas arriba lo veia y las dos comprobaciones de
    # abajo decian "no esta". El canario cantaba REPRODUCIDO por un path
    # viejo: el incidente que declara medir no ocurria, lo que no ocurria era
    # la suposicion del instrumento. Septima vez de esta clase en el bloque, y
    # la mas cara hasta ahora porque el falso positivo de un canario de
    # REPRODUCCION invita a "reparar" un producto que no esta roto.
    #
    # Por eso los bindings se LOCALIZAN por el project_id que el canario ya
    # resolvio, y se afirma la PROPIEDAD (cada binding declara SU project_id),
    # no la ruta. La forma del directorio es del producto; el nombre del
    # proyecto dentro del binding es la ley.
    log "bindings escritos bajo: $DATA"
    find "$DATA" -path "*bindings*" -name "$S.json" 2>/dev/null | sed 's/^/    /'

    local fa fb
    # MEDIDO (session-86, segunda vuelta): localizo por `*$A*` en la RUTA, y
    # eso es la misma suposicion que M2 quita — con las raices en un temporal
    # compartido, el path ya no contiene el project_id, el `find` no encuentra
    # nada, y la mutacion quedaba en SKIP. O sea que el arreglo del falso
    # REPRODUCIDO habia dejado la mutacion muda, que es el mismo fallo
    # cubierto por el otro lado: un instrumento que supone donde esta lo que
    # busca.
    #
    # Se localiza por CONTENIDO —el binding que declara ESE project_id— y no
    # por la ruta. El project_id es la ley; el directorio es del producto.
    fa="$(bind_por_proyecto "$A" "$S")"
    fb="$(bind_por_proyecto "$B" "$S")"
    [ -n "$fa" ] || log "V- A no deja binding que declare su project_id: lo que hay ahi es de otro"
    [ -n "$fb" ] || log "V- B no deja binding que declare su project_id: lo que hay ahi es de otro"
    # MEDIBLE solo cuando NO se escribio nada. Si uno de los dos falta, el
    # otro project's binding fue pisado — que es la fuga, no una falta de
    # medicion. MEDIDO (session-86): con M2 los dos proyectos escriben el
    # MISMO fichero, y el segundo borra el project_id del primero; tratar eso
    # como "no medible" convertia la mutacion en un SKIP mudo, cuando es
    # precisamente el defecto que la mutacion introduce.
    if [ -z "$fa" ] && [ -z "$fb" ]; then
        MEDIBLE=0
        return 1
    fi
    if [ -z "$fa" ] || [ -z "$fb" ]; then
        log "V2 FUGA: los dos proyectos comparten fichero y uno piso al otro (A='${fa:-ninguno}' B='${fb:-ninguno}')"
        violaciones=$((violaciones+1))
        if [ -n "$fa" ] || [ -n "$fb" ]; then
            return 1
        fi
        MEDIBLE=0
        return 1
    fi
    [ "$fa" = "$fb" ] && { log "V2 ambos proyectos resuelven al MISMO binding: hay fuga"; violaciones=$((violaciones+1)); }
    log "V2 binding de A: ${fa#"$DATA"/}"
    log "V2 binding de B: ${fb#"$DATA"/}"
    if grep -q "\"project_id\": *\"$B\"" "$fa"; then
        log "V2 FUGA: el binding de A declara el project_id de B"; violaciones=$((violaciones+1))
    fi
    grep -q "\"project_id\": *\"$A\"" "$fa" || {
        log "V2 el binding de A no declara su propio project_id"; violaciones=$((violaciones+1)); }
    if grep -q "\"project_id\": *\"$A\"" "$fb"; then
        log "V2 FUGA: el binding de B declara el project_id de A"; violaciones=$((violaciones+1))
    fi
    grep -q "\"project_id\": *\"$B\"" "$fb" || {
        log "V2 el binding de B no declara su propio project_id"; violaciones=$((violaciones+1)); }
    # V3: el directorio de bindings debe depender del project_id. Si todos los
    #     proyectos comparten directorio, la separacion es de fachada.
    if [ "$(dirname "$fa")" = "$(dirname "$fb")" ]; then
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

# MEDIBLE va por canal aparte, y no por el codigo de retorno: `canary` devuelve
# 0/1 (pasa/falla) y un 2 se confundiria con "reproducido", que es justo la
# lectura que este canario no quiere que se tome cuando no midio nada.
MEDIBLE=1

printf '\n--- BASE ---\n'
canary
canary_rc=$?
if [ "$MEDIBLE" -eq 0 ]; then
    printf '\nRESULT: NO SE PUEDE MEDIR — el fixture no reproduce el caso. No cuenta.\n'
    exit 1
fi
if [ "$canary_rc" -eq 0 ]; then
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
    MEDIBLE=1
    canary && reproduce="no" || reproduce="si"
    # Un `canary` que no pudo medir no es un canario que no cae: es uno que no
    # dijo nada. Antes de meter esto en un veredicto hay que distinguirlo, o una
    # mutacion cuyo sujeto dejo de medir se contaria como "el canario aguanto".
    local medible="$MEDIBLE"
    cp "$WORK/mut.bak" "$fichero"
    build >/dev/null 2>&1
    sha_restore="$(sha256sum "$fichero" | cut -d' ' -f1)"
    if [ "$sha_restore" != "$sha_antes" ]; then
        printf '  [FATAL] %s — la restauracion no fue byte-identica\n' "$etiqueta"
        exit 1
    fi
    if [ "$medible" -eq 0 ]; then
        skip "$etiqueta" "con la mutacion el fixture ya no mide el caso"
        return
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

# MEDIDO (session-86): esta mutacion era MUERTA. Parcheaba la forma PRE-VA17
# de `bindings_root(project_data: &Path)`, y el codigo ya dice
# `bindings_root(session_root: &Path)`, luego el `assert` la tiro y el canario
# lo conto como SKIP — una falsacion que no puede mover nada. Mismo modo de
# fallo que las F2/F3 de VA16: una mutacion escrita contra la forma del codigo
# que existia cuando se concibio, no contra la que existe.
#
# Se rehace contra la forma real. La propiedad que V3 afirma es que el
# directorio de bindings DEPENDE del project_id, y la unica forma de dejarla de
# depender sin tocar los tres `load_binding` es colgar el path de un temporal
# compartido: dos proyectos distintos escriben el MISMO fichero.
mutar "M2 el directorio de bindings deja de depender del project_id" "$CTX" ROJO \
    "los tres load_binding siguen leyendolo, pero ahora el path cuelga de un temporal compartido: dos proyectos distintos escriben el MISMO fichero y el namespacing era la unica defensa." \
'
import os
p=os.environ["MUT_FILE"]; s=open(p).read()
v="""fn bindings_root(session_root: &Path) -> PathBuf {
    session_root.join("bindings")
}"""
n="""fn bindings_root(session_root: &Path) -> PathBuf {
    std::env::temp_dir().join("sddk-bindings-compartidos")
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