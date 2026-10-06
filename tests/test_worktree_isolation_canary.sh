#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# Canario de aislamiento POR WORKTREE, y su autofalsacion.
#
# ## QUE INTENTA REPRODUCIR
#
# Dos checkouts del MISMO proyecto (mismo remote, mismo project_id) con
# distinto workspace_id. El binding de sesion se escribe en A y se lee en B.
# Lo que no deberia ocurrir es que B lo absorba sin ningun discriminante.
#
# ## POR QUE EXISTE SI EL CASO ANTERIOR NO SE REPRODUCE
#
# El canario cross-project (VA15) concluyo que un binding de un proyecto no es
# alcanzable desde otro: el namespacing en disco por project_id lo impide. Este
# canario mide el caso que SI queda, y es peor, porque aqui el project_id
# COINCIDE legitimamente. Ninguna ley por project_id puede detectarlo.
#
# ## LO QUE ESTE CANARIO NO HACE
#
# No arregla. No decide la politica de reattach cross-worktree. Su producto es
# un veredicto con falsificadores.
#
# ## AISLAMIENTO
#
# `SDDK_DATA_DIR` a un temporal. Esta maquina tiene 265 proyectos reales y el
# canario no toca ninguno. El target de cargo se mantiene FUERA del temporal a
# proposito: si no, cada mutacion recompila el mundo desde cero.
# ---------------------------------------------------------------------------
set -uo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT" || exit 1

WORK="$(mktemp -d)"
export SDDK_DATA_DIR="$WORK/data"
TARGET="${SDDK_CANARIO_WT_TARGET:-/tmp/sddk-canario-worktree-target}"
mkdir -p "$TARGET"
export CARGO_TARGET_DIR="$TARGET"
BIN="$TARGET/debug/sddk"

# shellcheck disable=SC2329  # invocada por el trap de EXIT, que shellcheck no ve
cleanup() {
    # `rm` en este entorno es un shim que NO expande variables, asi que el trash
    # se invoca con su ruta absoluta y el objetivo ya expandido aqui.
    # El target NO se borra: es lo que hace incrementales las recompilaciones.
    [ -d "$WORK" ] || return 0
    /home/rubentxu/.minimax/bin/mavis-trash -- "$WORK" >/dev/null 2>&1
    return 0
}
trap cleanup EXIT

PASS=0; FAIL=0; SKIP=0
log()  { printf '  %s\n' "$*"; }
ok()   { printf '  [ok]   %s\n' "$*"; PASS=$((PASS+1)); }
ko()   { printf '  [FAIL] %s\n' "$*"; FAIL=$((FAIL+1)); }
skip() { printf '  [SKIP] %s — %s\n' "$1" "$2"; SKIP=$((SKIP+1)); }

# Hoy la propiedad NO existe y eso es exactamente lo que este bloque mide. Si
# el canario dejara de encontrar el leak, alguien lo habria arreglado y la
# constante habria que cambiar a mano. Por eso es una constante y no un
# comentario: para que "el veredicto cambio sin que nadie lo tocara" sea visible.
EXPECT_LEAK="si"

# Marcador que simula trabajo acumulado por A. Es lo que hace observable la
# herencia sin necesitar un ciclo real, que seria caro de montar.
MARCA="ref-acumulado-en-A"

mk_repo() { # $1=directorio  $2=remote
    mkdir -p "$1"
    git -C "$1" init -q .
    git -C "$1" config user.email canary@example.invalid
    git -C "$1" config user.name canary
    # `remote add` no es idempotente y measure corre varias veces sobre el mismo
    # path: sin esto, la segunda corrida perderia el remote en silencio.
    git -C "$1" remote set-url origin "$2" 2>/dev/null \
        || git -C "$1" remote add origin "$2"
    [ -f "$1/README.md" ] || printf 'fixture\n' > "$1/README.md"
    git -C "$1" add README.md 2>/dev/null
    git -C "$1" commit -qm init >/dev/null 2>&1 || true
}

build() {
    if ! cargo build -p sddk-cli --bin sddk > "$WORK/build.log" 2>&1; then
        printf 'ERROR: la compilacion fallo\n'
        tail -20 "$WORK/build.log"
        return 1
    fi
    return 0
}

# El texto de `context bootstrap`; el codigo de salida es 4 sin ciclo, y eso NO
# es un fallo del canario, asi que no se comprueba.
bfield() { # $1=salida  $2=campo
    printf '%s' "$1" | awk -F': ' -v k="$2:" '$0 ~ "^"k" " {print $2; exit}'
}
bwritten() {
    printf '%s' "$1" | grep -o 'written: true\|written: false' | head -1 | sed 's/written: //'
}

# --- el canario -------------------------------------------------------------
# Devuelve el numero de violaciones, o 2 si el fixture no mide el caso.
#
# MEDIDO EN ESTE MISMO FICHERO, y es la clase de error que el canario cross-
# project ya delato: buscar el binding donde uno SUPONE que esta en vez de donde
# CAE. La primera version de este canario construia la ruta a mano
# (`$DATA/sddk/projects/<pid>/context/bindings/S-wt.json`), y con F1 puesto el
# binding se mueve de directorio — que es justo lo que F1 hace — y el canario
# devolvia "no se encontro binding" en vez de medir. Un instrumento que se
# apaga justo cuando la mutacion funciona no mide la propiedad: mide su propia
# suposicion. Por eso el binding se LOCALIZA con `find`.

RUN=0
violations=0
violations_base=-1
# MEDIBLE va por canal aparte. Antes `return 2` hacia de centinela y
# `violations == 2` es un veredicto valido: las dos SKIP iniciales fueron eso.
MEDIBLE=1
PID_A=""; PID_B=""; WID_A=""; WID_B=""
WRITTEN_B=""
NBIND=0

measure() {
    RUN=$((RUN + 1))
    local DATA="$WORK/data/run-$RUN"
    export SDDK_DATA_DIR="$DATA"
    violations=0

    local c="$WORK/canario"
    # Mismo remote en los dos: mismo project_id, distinta ruta, distinto workspace_id.
    mk_repo "$c/wt-a" "https://github.com/example/obra.git"
    mk_repo "$c/wt-b" "https://github.com/example/obra.git"

    local out_a out_b
    out_a="$("$BIN" context bootstrap --root "$c/wt-a" --session S-wt 2>&1)"
    out_b="$("$BIN" context bootstrap --root "$c/wt-b" --session S-wt 2>&1)"

    PID_A="$(bfield "$out_a" project)";  WID_A="$(bfield "$out_a" workspace)"
    PID_B="$(bfield "$out_b" project)";  WID_B="$(bfield "$out_b" workspace)"
    WRITTEN_B="$(bwritten "$out_b")"

    log "A: project=$PID_A workspace=$WID_A"
    log "B: project=$PID_B workspace=$WID_B"

    # --- W0: precondicion. Sin esto el fixture no reproduce el caso. ---
    if [ -z "$PID_A" ] || [ -z "$WID_A" ]; then
        log "W0 no se pudo resolver identidad en A"; MEDIBLE=0; return 0
    fi
    if [ "$PID_A" != "$PID_B" ]; then
        log "W0 PRECONDICION FALLIDA: project_id distintos, no es el caso X6"; MEDIBLE=0; return 0
    fi
    if [ "$WID_A" = "$WID_B" ]; then
        log "W0 el workspace_id NO distingue worktrees: no hay caso que medir"; MEDIBLE=0; return 0
    fi
    log "W0 ok: mismo project_id, distinto workspace_id"

    # --- localizar el binding, donde CAE y no donde se supone ---
    local binding
    binding="$(find "$DATA" -path '*bindings*' -name 'S-wt.json' 2>/dev/null | head -1)"
    if [ -z "$binding" ]; then
        log "W0 no se escribio ningun binding; el fixture no mide el caso"; MEDIBLE=0; return 0
    fi
    log "binding localizado en: ${binding#"$DATA"/}"

    # --- W1: el binding de A tiene que ser inalcanzable desde B ---
    NBIND="$(find "$DATA" -path '*bindings*' -name 'S-wt.json' 2>/dev/null | wc -l)"
    if [ "$NBIND" -gt 1 ]; then
        log "W1 ok: A y B tienen bindings separados ($NBIND)"
    else
        log "W1 VIOLACION: un unico binding compartido por dos worktrees distintos"
        violations=$((violations+1))
    fi

    # --- W2: el binding debe declarar de que worktree vino ---
    log "W2 contenido: $(tr -d '\n ' < "$binding")"
    if grep -q 'workspace_id' "$binding"; then
        log "W2 ok: el binding declara su workspace"
    else
        log "W2 VIOLACION: el binding no declara ningun workspace; no se puede saber de que worktree vino"
        violations=$((violations+1))
    fi

    # --- W3: B no debe absorber en silencio la sesion de A ---
    log "W3 bootstrap en B reporto: $WRITTEN_B"
    if [ "$WRITTEN_B" = "true" ]; then
        log "W3 ok: B escribio su propio binding"
    else
        log "W3 VIOLACION: B absorbio en silencio el binding de A (written: false)"
        violations=$((violations+1))
    fi

    # --- W3b: y no debe heredar el TRABAJO que A acumulo en el ---
    # Se marca el binding de A como si A hubiera acumulado refs, y se mira lo que
    # queda despues de que B pase por el binding. Es la forma barata de observar
    # la herencia sin montar un ciclo real.
    python3 - "$binding" "$MARCA" <<'PYEOF'
import json, sys
ruta, marca = sys.argv[1], sys.argv[2]
d = json.load(open(ruta, encoding="utf-8"))
d["semantic_refs"] = [marca]
json.dump(d, open(ruta, "w", encoding="utf-8"), indent=2)
PYEOF
    "$BIN" context bootstrap --root "$c/wt-b" --session S-wt >/dev/null 2>&1
    if grep -q "$MARCA" "$binding"; then
        log "W3b VIOLACION: tras pasar B, el binding sigue llevando el trabajo acumulado por A"
        violations=$((violations+1))
    else
        log "W3b ok: B no hereda el trabajo de A"
    fi

    # --- W4: control POSITIVO. El namespacing por workspace se sabe hacer ---
    local adoption_n
    adoption_n="$(find "$DATA" -path '*workspaces*' -name 'adoption.json' 2>/dev/null | wc -l)"
    if [ "$adoption_n" -ge 2 ]; then
        log "W4 ok: adoption SI esta separado por workspace ($adoption_n ficheros, uno por worktree)"
    else
        log "W4 el control positivo fallo: el sistema tampoco separa adoption por workspace ($adoption_n)"
    fi
    log "W4 inventado de lo que hay bajo projects/:"
    find "$DATA/sddk/projects" -type f 2>/dev/null | sed "s|$DATA/sddk/projects/||; s|^|      |" | sort | head -12

    return 0
}

# --- ejecucion --------------------------------------------------------------

printf '== canario de aislamiento por worktree ==\n'
build || exit 1
log "binario: $BIN"
log "target persistente: $TARGET (fuera del temporal, para rebuilds incrementales)"

printf '\n--- BASE ---\n'
measure
if [ "$MEDIBLE" -eq 0 ]; then
    printf '\nRESULT: NO SE PUEDE MEDIR — el fixture no reproduce el caso. No cuenta.\n'
    exit 1
fi
violations_base="$violations"

if [ "$violations" -gt 0 ]; then
    log "VEREDICTO: $violations violaciones. El aislamiento por worktree NO existe."
    if [ "$EXPECT_LEAK" = "si" ]; then
        ok "base: el canario midio el leak que declara medir ($violations violaciones)"
    else
        ko "base: hay $violations violaciones y este canario ya no las espera. El producto cambio: revisar EXPECT_LEAK."
    fi
else
    if [ "$EXPECT_LEAK" = "si" ]; then
        ko "base: el canario NO encuentra el leak y deberia. El fixture perdio su poder, o el producto se arreglo."
    else
        ok "base: el aislamiento por worktree se sostiene ($violations violaciones)"
    fi
fi

printf '\n--- MUTACIONES ---\n'
# Aqui la forma es la INVERSA de VA15: no hay defensa que romper, asi que las
# mutaciones la CONSTRUYEN y el canario tiene que notar el cambio del veredicto.
# F3 es el control sin el cual "el veredicto cambio" no significa nada: prueba
# que el canario no reacciona a ruido.

mutar() { # $1=etiqueta $2=fichero $3=efecto(cambia|igual) $4=porque $5=pycode
    local etiqueta="$1" fichero="$2" efecto="$3" porque="$4" pycode="$5"
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

    measure
    cp "$WORK/mut.bak" "$fichero"
    build >/dev/null 2>&1
    sha_restore="$(sha256sum "$fichero" | cut -d' ' -f1)"
    if [ "$sha_restore" != "$sha_antes" ]; then
        printf '  [FATAL] %s — la restauracion no fue byte-identica\n' "$etiqueta"
        exit 1
    fi
    if [ "$MEDIBLE" -eq 0 ]; then
        skip "$etiqueta" "con la mutacion el fixture ya no mide el caso"
        return
    fi
    case "$efecto" in
        cambia)
            if [ "$violations" -lt "$violations_base" ]; then
                ok "$etiqueta — el veredicto mejoro ($violations_base -> $violations violaciones). $porque"
            else
                ko "$etiqueta — el veredicto NO cambio ($violations_base -> $violations). El canario no ve lo que dice medir. $porque"
            fi
            ;;
        igual)
            if [ "$violations" -eq "$violations_base" ]; then
                ok "$etiqueta — el veredicto NO cambio, como debe. $porque"
            else
                ko "$etiqueta — el veredicto cambio ($violations_base -> $violations) con una mutacion neutra. El canario reacciona a ruido."
            fi
            ;;
    esac
}

log "violaciones de referencia (base): $violations_base"

PATHF=crates/sddk-engine/src/paths.rs
CTX=crates/sddk-cli/src/context_cmd.rs

mutar "F1 project_data cuelga del workspace_id" "$PATHF" cambia \
    "es el FIX REAL en una linea: A y B dejan de compartir todo lo que cuelga de project_data, incluidos bindings y capsules." \
'
import os
p=os.environ["MUT_FILE"]; s=open(p).read()
v = "    let project_data = data_home.join(\"sddk/projects\").join(project_id);"
n = "    let project_data = data_home.join(\"sddk/projects\").join(project_id).join(workspace_id);"
assert v in s, "project_data no tiene la forma que esta mutacion supone"
open(p,"w").write(s.replace(v,n,1))
'

mutar "F2 bootstrap deja de cargar el binding previo" "$CTX" cambia \
    "si B no carga el estado de A, no puede absorberlo: es la otra mitad del fix y ataca la herencia, no el directorio." \
'
import os
p=os.environ["MUT_FILE"]; s=open(p).read()
v = """    let existing = load_binding(&bindings_root(&paths.project_data), &session)
        .map_err(|e| ContextBootstrapError::Durable(e.to_string()))?;"""
n = """    let existing: Option<AgenticBinding> = None;
    let _probe = load_binding(&bindings_root(&paths.project_data), &session);"""
assert v in s, "la carga del binding previo no tiene la forma que esta mutacion supone"
open(p,"w").write(s.replace(v,n,1))
'

mutar "F3 comentario inocuo (control negativo)" "$CTX" igual \
    "si esto moviera el veredicto, F1 y F2 no significarian nada: probarian que el canario detecta ruido." \
'
import os
p=os.environ["MUT_FILE"]; s=open(p).read()
v = """fn bindings_root(project_data: &Path) -> PathBuf {
    project_data.join("context").join("bindings")
}"""
n = """fn bindings_root(project_data: &Path) -> PathBuf {
    // control negativo del canario de worktree: este comentario no cambia nada
    project_data.join("context").join("bindings")
}"""
assert v in s, "bindings_root no tiene la forma que esta mutacion supone"
open(p,"w").write(s.replace(v,n,1))
'

printf '\n----------------------------------------\n'
printf 'PASS=%d FAIL=%d SKIP=%d\n' "$PASS" "$FAIL" "$SKIP"
if [ "$FAIL" -eq 0 ]; then
    printf 'RESULT: PASS — el canario mide lo que declara medir (%d violaciones medidas, %d comprobaciones con dientes).\n' \
        "$violations_base" "$PASS"
    exit 0
fi
printf 'RESULT: FAIL — hay comprobaciones que no vigilan lo que declaran.\n'
exit 1