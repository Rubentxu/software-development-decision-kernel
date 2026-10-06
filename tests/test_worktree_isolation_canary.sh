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
# MEDIDO (session-86): el canario solo aislaba SDDK_DATA_DIR, y el LEDGER no
# cuelga de ahi. `paths.rs:173` lo resuelve por `state_home`, que tiene su
# propia precedencia: SDDK_STATE_HOME, luego XDG_STATE_HOME, luego
# $HOME/.local/state. Sin esta linea el canario CREABA un
# `~/.local/state/sddk/projects/<project_id>/ledger.sqlite` de verdad en cada
# corrida — medido: `p-f1bf640403f38968/ledger.sqlite` con mtime fresco, en
# el estado real de una maquina con 362 proyectos. Un canario que promete no
# tocar ninguno y escribe en el ultimo es peor que uno sin promesa: el
# `find` del W4 miraba solo data y por eso no lo veia.
export SDDK_STATE_HOME="$WORK/state"
export XDG_STATE_HOME="$WORK/state"
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

# El papel de este canario CAMBIO con VA17, y la constante es lo que lo deja
# escrito.
#
# Con EXPECT_LEAK="si" (VA16) el canario afirmaba que la propiedad NO existia: lo
# que media era el leak. Su base verde significaba "el leak sigue ahi".
#
# Con VA17 los cuatro almacenes de sesion pasan a ser POR WORKTREE, luego la
# propiedad EXISTE y el canario pasa a ser guard estricto: su base verde
# significa "el aislamiento se sostiene", y sus mutaciones tienen que
# CONSTRUIR la defensa (lo que ya hacen, forma inversa a VA15) para comprobar
# que el veredicto se mueve.
#
# La constante sigue siendo una constante, no un comentario, por el motivo de la
# version anterior y porque ahora el riesgo es el inverso: si el producto
# volviera a filtrar entre worktrees, una base en "si" seguiria dando verde
# mientras el defecto regresa. Con "no", el defecto da rojo.
EXPECT_LEAK="no"

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

    # MEDIDO (session-85): el directorio era el MISMO en la base y en cada
    # mutacion, luego los bindings escritos en la base seguian ahi al medir M2.
    # M2 quita el `#[serde(default)]` de `workspace_id`, esos bindings dejaron de
    # ser legibles, y el fixture quedo sin poder medir — lo que el harness
    # cuenta como SKIP. Peor: M3, el CONTROL NEGATIVO, heredaba ese mismo estado
    # roto y tambien quedo en SKIP, o sea el control que debia probar que el
    # canario no reacciona a ruido no llego a ejecutarse. Mutaciones que
    # comparten estado no son independientes, y la segunda que pagaba el
    # pato era justamente la que no podia mentir.
    local c
    c="$(mktemp -d "$WORK/med.XXXXXX")"
    # Mismo remote en los dos: mismo project_id, distinta ruta, distinto workspace_id.
    mk_repo "$c/wt-a" "https://github.com/example/obra.git"
    mk_repo "$c/wt-b" "https://github.com/example/obra.git"

    local out_a out_b rc_a rc_b
    out_a="$("$BIN" context bootstrap --root "$c/wt-a" --session S-wt 2>&1)"; rc_a=$?
    out_b="$("$BIN" context bootstrap --root "$c/wt-b" --session S-wt 2>&1)"; rc_b=$?

    PID_A="$(bfield "$out_a" project)";  WID_A="$(bfield "$out_a" workspace)"
    PID_B="$(bfield "$out_b" project)";  WID_B="$(bfield "$out_b" workspace)"
    WRITTEN_B="$(bwritten "$out_b")"

    log "A: project=$PID_A workspace=$WID_A"
    log "B: project=$PID_B workspace=$WID_B"

    # --- W0: precondicion. Sin esto el fixture no reproduce el caso. ---
    #
    # MEDIDO (session-86): cuando B RECHAZA arrancar —que es lo que pasa con
    # la mutacion M1, porque las cuatro raices vuelven a `project_data` y B
    # pisa el binding de A— no imprime identidad. Antes eso caia en
    # "PRECONDICION FALLIDA: project_id distintos" y el harness lo contaba
    # como SKIP: fixture no medible. O sea que la mutacion que DESHACE el
    # aislamiento apagaba el canario justo cuando tenia que hablar. Es la
    # sexta vez de la misma clase, y la mas cara: aqui la mutacion era la
    # unica que podia mover el veredicto y se midio como decoracion.
    #
    # Un rechazo de B no es "el fixture no mide": es el producto fallando
    # closed ante un binding ajeno, y eso es una VIOLACION de primer orden.
    # Por eso `rc_b` se distingue ahora: si B fallo, se cuenta y se sigue.
    if [ -z "$PID_A" ] || [ -z "$WID_A" ]; then
        # rc_a se mira aqui y no por gusto: si A tampoco arranca, el fallo no
        # es del caso cross-worktree sino del binario o del fixture, y son
        # cosas que hay que distinguir en vez de agrupar bajo "no medible".
        log "W0 A no resolvio identidad (rc=$rc_a): ${out_a:0:140}"; MEDIBLE=0; return 0
    fi
    if [ -z "$PID_B" ] || [ -z "$WID_B" ]; then
        log "W0 B no resolvio identidad porque RECHAZO (rc=$rc_b): ${out_b:0:140}"
        log "W0 VIOLACION: B no puede ni arrancar con el estado que A dejo. Es el aislamiento roto haciendose dano a si mismo."
        violations=$((violations+1))
        # MEDIBLE sigue a 1 a proposito: esto ES un veredicto medido, no una
        # falta de medicion. Lo que no corre despues (W1..W5) no puede correr
        # porque B no dejo binding, y se declara en voz alta en vez de
        # declararse un SKIP que el harness contaria como falsacion muerta.
        log "W0 W1..W5 no se evaluan en esta corrida: B no dejo binding contra el que medirlos."
        MEDIBLE=1
        return 0
    fi
    if [ "$PID_A" != "$PID_B" ]; then
        log "W0 PRECONDICION FALLIDA: project_id distintos, no es el caso X6"; MEDIBLE=0; return 0
    fi
    if [ "$WID_A" = "$WID_B" ]; then
        log "W0 el workspace_id NO distingue worktrees: no hay caso que medir"; MEDIBLE=0; return 0
    fi
    log "W0 ok: mismo project_id, distinto workspace_id"

    # --- localizar el binding, donde CAE y no donde se supone ---
    #
    # MEDIDO (session-85, al volver el canario estricto con VA17): esto era
    # `find "$DATA" -path '*bindings*' -name 'S-wt.json' | head -1`, y con DOS
    # worktrees eso devuelve el que salga primero, SIN decir de quien es. W3b
    # plantaba la marca en ese fichero y luego comprobaba si seguia ahi tras
    # arrancar B — luego, si el fichero era el de B, la prueba se media a si
    # misma: B conservando sus propias referencias es correcto, y se contaba
    # como "B absorbio el trabajo de A". Un FAIL del instrumento es
    # indistinguible de un FAIL real si no se distingue de quien lo pide, y este
    # es el cuarto caso de esa clase en el bloque.
    #
    # Ahora cada binding se resuelve POR WORKTREE, con el `workspace_id` que el
    # propio canario resolvio antes. Si alguno no aparece, el fixture no mide
    # el caso y se dice, en vez de medir el binding equivocado.
    local binding_a binding_b
    binding_a="$(find "$DATA" -path "*$WID_A*/context/bindings*" -name 'S-wt.json' 2>/dev/null | head -1)"
    binding_b="$(find "$DATA" -path "*$WID_B*/context/bindings*" -name 'S-wt.json' 2>/dev/null | head -1)"
    if [ -z "$binding_a" ] || [ -z "$binding_b" ]; then
        log "W0 no se localizaron los bindings por worktree (A='${binding_a:-vacio}' B='${binding_b:-vacio}'); el fixture no mide el caso"
        MEDIBLE=0; return 0
    fi
    if [ "$binding_a" = "$binding_b" ]; then
        log "W0 los dos worktrees resuelven al MISMO binding: no hay caso que medir"
        MEDIBLE=0; return 0
    fi
    binding="$binding_a"
    log "binding de A: ${binding_a#"$DATA"/}"
    log "binding de B: ${binding_b#"$DATA"/}"

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
    # Se marca el binding de **A** como si A hubiera acumulado refs, se pasa B,
    # y se mira las DOS mitades. MEDIDO (session-85): antes plantaba la marca en
    # un binding arbitrario, luego cuando le tocaba el de B la comprobacion se
    # media a si misma — B conservando sus propias referencias es correcto y se
    # contaba como herencia ajena.
    #
    # Las dos mitades importan y no son la misma asercion:
    #   - B no tiene la marca  => B no absorbio el trabajo de A (la propiedad).
    #   - A sigue con la marca => B no piso el estado de A al escribir el suyo.
    python3 - "$binding_a" "$MARCA" <<'PYEOF'
import json, sys
ruta, marca = sys.argv[1], sys.argv[2]
d = json.load(open(ruta, encoding="utf-8"))
d["semantic_refs"] = [marca]
json.dump(d, open(ruta, "w", encoding="utf-8"), indent=2)
PYEOF
    "$BIN" context bootstrap --root "$c/wt-b" --session S-wt >/dev/null 2>&1
    if grep -q "$MARCA" "$binding_b" 2>/dev/null; then
        log "W3b VIOLACION: el binding de B contiene el trabajo acumulado por A"
        violations=$((violations+1))
    elif ! grep -q "$MARCA" "$binding_a" 2>/dev/null; then
        log "W3b VIOLACION: B piso el estado de A al escribir el suyo (la marca desaparecio de A)"
        violations=$((violations+1))
    else
        log "W3b ok: B no hereda el trabajo de A"
    fi

    # --- W5: que se haga de un binding ANTIGUO (sin `workspace_id`).
    #
    # MEDIDO (session-86): esta comprobacion afirmaba lo CONTRARIO de lo que
    # el producto decide. Decia "un binding antiguo se lee y sobrevive al
    # bootstrap", y lo moria exigiendo rc 0 o 4 — o sea, EXIGIENDO que un
    # binding sin workspace se adoptara en silencio. Pero el contrato escrito
    # en `agentic_session_binding.rs:103` es "Unknown is NOT a wildcard", y el
    # producto lo cumple: rc=1 con `carries no workspace` y un recovery que
    # nombra `--rebind`. La comprobacion estaba fallando en la BASE por
    # funcionar bien el producto, y su falsificacion (M2) era ademas MUERTA:
    # quitar el `#[serde(default)]` no cambia nada, porque serde_derive ya
    # trata `Option<T>` ausente como `None` sin ese atributo — medido y
    # fijado en `crates/sddk-engine/tests/va17_legacy_binding_serde.rs`.
    #
    # Un guard que exige lo contrario del contrato no vigila el contrato:
    # obliga a que el defecto se elimine para que el guard pase. Eso es un
    # guard invertido, y es la quinta vez que esta clase aparece en el
    # bloque.
    #
    # NOTA sobre los ficheros huerfanos: la PRE-FLIGHT de este bloque se
    # escribio contando 212 ficheros de sesion en el layout antiguo (91
    # bindings, 94 deltas, 26 capsules, 1 reads). MEDIDO (session-86): ya NO
    # EXISTEN — `find` sobre `~/.local/state/sddk` no devuelve ni un
    # `bindings/`, `deltas/` ni `capsules/`. El fixture de W5 los fabrica en
    # su propio temporal, asi que la propiedad sigue midiendo lo que dice
    # medir; lo que cambio es el recuento y con el la justificacion de
    # "212", que ya no describe esta maquina.
    #
    # LO QUE SI se afirma, en dos mitades que no son la misma asercion:
    #   - sin `--rebind`: RECHAZA, y el error nombra el comando que repara.
    #     Un fallo de serde ("missing field") tambien rejects, pero no dice
    #     como arreglarlo: por eso se comprueba el texto del recovery.
    #   - con `--rebind`: ADOPTA y CONSERVA el contenido del binding antiguo.
    #     Read preserva lo que hay escrito; sin esto, "se rechaza" podria
    #     querer decir "se pierde".
    local legacy_marker="ref-legado-sin-workspace"
    python3 - "$binding_a" "$legacy_marker" <<'PYEOF'
import json, sys
ruta, marca = sys.argv[1], sys.argv[2]
d = json.load(open(ruta, encoding="utf-8"))
# Se quita el campo para simular un binding escrito antes de VA17.
d.pop("workspace_id", None)
d["semantic_refs"] = [marca]
json.dump(d, open(ruta, "w", encoding="utf-8"), indent=2)
PYEOF
    local boot_legacy rc_legacy
    boot_legacy="$("$BIN" context bootstrap --root "$c/wt-a" --session S-wt 2>&1)"
    rc_legacy=$?
    # Mitad 1: fail-closed, y el fallo dice que hacer.
    if [ "$rc_legacy" -eq 0 ] || [ "$rc_legacy" -eq 4 ]; then
        log "W5 VIOLACION: un binding sin workspace_id se adopto en SILENCIO (rc=$rc_legacy). Unknown no es comodin."
        violations=$((violations+1))
    elif ! printf '%s' "$boot_legacy" | grep -q -- '--rebind'; then
        log "W5 VIOLACION: rechazo sin recovery (rc=$rc_legacy): ${boot_legacy:0:120}"
        violations=$((violations+1))
    else
        log "W5 ok: sin --rebind rechaza y nombra --rebind"
    fi
    # Mitad 2: con el rebind explicito, el contenido antiguo sobrevive.
    local boot_rebind rc_rebind
    boot_rebind="$("$BIN" context bootstrap --root "$c/wt-a" --session S-wt --rebind 2>&1)"
    rc_rebind=$?
    if [ "$rc_rebind" -ne 0 ] && [ "$rc_rebind" -ne 4 ]; then
        log "W5 VIOLACION: --rebind no recupera un binding antiguo (rc=$rc_rebind): ${boot_rebind:0:120}"
        violations=$((violations+1))
    elif ! grep -q "$legacy_marker" "$binding_a" 2>/dev/null; then
        log "W5 VIOLACION: el binding antiguo se leyo pero se descarta su contenido al reescribir"
        violations=$((violations+1))
    else
        log "W5 ok: --rebind lo adopta y su contenido sobrevive"
    fi
    # No hace falta restaurar el binding de A: el `--rebind` de la segunda
    # mitad de W5 ya lo dejo con `workspace_id` y el producto lo reescribe en
    # esa operacion. MEDIDO (session-86): el bootstrap de restauracion que
    # estaba aqui era ademas un NO-OP con riesgo — sin `--rebind` leeria el
    # binding con `workspace_id` ausente, lo rechazaria, y no tocaria nada.

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
# Con la propiedad YA EXISTENTE (VA17), las mutaciones la DESHACEN y el canario
# tiene que notar que el veredicto empeora. Antes (VA16) la construian, porque la
# propiedad no existia: mismo mecanismo, signo opuesto. M3 es el control sin el
# cual "el veredicto empeoro" no significa nada: prueba que el canario no
# reacciona a ruido.

mutar() { # $1=etiqueta $2=fichero $3=efecto(empeora|cambia|igual) $4=porque $5=pycode
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
        # `empeora` es el signo de VA17 y el unico valido mientras la propiedad
        # EXISTE: el canario afirma que el aislamiento se sostiene, luego
        # deshacerlo tiene que MEJORAR el veredicto de reds, o sea EMPEORAR el
        # estado. `cambia` (mejorar) era el signo de VA16, cuando la propiedad
        # no existia y anadirla era el arreglo. Con la propiedad ya existente,
        # `cambia` daria por bueno un canario que solo sabe confirmar fixes, y
        # eso no es un guard: es un cheerleader.
        empeora)
            if [ "$violations" -gt "$violations_base" ]; then
                ok "$etiqueta — el veredicto empeoro ($violations_base -> $violations violaciones), como debe. $porque"
            else
                ko "$etiqueta — el veredicto NO empeoro ($violations_base -> $violations). El canario no ve deshacer el aislamiento. $porque"
            fi
            ;;
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

# --- LAS MUTACIONES CAMBIAN DE SIGNO CON LA PROPIEDAD ------------------------
#
# MEDIDO (session-85, al volver este canario estricto con VA17): F1, F2 y F3
# eran el conjunto de VA16, y en VA16 la propiedad NO existia, luego su trabajo
# era CONSTRUIR la defensa. Con la propiedad ya existente eso es un error de
# categoria: F1 "anade el workspace a project_data" sobre un layout que ya lo
# lleva produce `projects/<p>/<w>/workspaces/<w>/...` — el workspace_id DOS
# veces — y ademas no mueve el veredicto. F2 y F3 ni siquiera aplicaron: su
# patch buscaba `bindings_root(project_data)` y el codigo ya dice
# `bindings_root(session_root)`, luego el `assert` los tiro y ambos quedaron en
# SKIP. Tres falsaciones, ninguna midiendo.
#
# Con la propiedad ya existente, el trabajo de las mutaciones es el INVERSO:
# DESHACER el aislamiento y comprobar que el canario lo nota. Un guard cuya
# mutaciones construyen la defensa solo sabe direccion: no puede afirmar que la
# defensa este puesta, que es justo lo que hay que afirmar.

CTX=crates/sddk-cli/src/context_cmd.rs
BINDING=crates/sddk-engine/src/agentic_session_binding.rs

mutar "M1 las cuatro raices vuelven a colgar de project_data" "$CTX" empeora \
    "deshace el aislamiento por worktree: A y B vuelven a compartir el arbol, que es el leak que VA16 midio. El veredicto TIENE que empeorar." \
'
import os
p=os.environ["MUT_FILE"]; s=open(p).read()
antes = s.count("paths.session_root")
assert antes >= 4, "esperaba las cuatro raices colgadas de session_root, encontre %d" % antes
s = s.replace("paths.session_root", "paths.project_data")
# Y el nucleo del layout se queda sin el escalon de workspaces/.
open(p,"w").write(s)
'

# MEDIDO (session-86): esta mutacion era MUERTA, y no por poco. Quitar el
# `#[serde(default)]` de `workspace_id` NO cambia el comportamiento, porque
# serde_derive ya trata un `Option<T>` ausente como `None` sin ese atributo.
# Medido y fijado en `crates/sddk-engine/tests/va17_legacy_binding_serde.rs`,
# con un control que demuestra que el mismo approach sobre un `String` si
# falla. Su justificacion ("212 bindings quedan ilegibles") era ademas falsa
# por partida doble: los 212 ya no existen (ver nota de W5), y aunque
# existieran no se volverian ilegibles.
#
# Se sustituye por la mutacion que SI deshace la propiedad que W5 afirma:
# tratar `Unknown` como si fuera `Same`. Es el defecto exacto que el
# fail-closed existe para impedir — un worktree que continua el trabajo de
# otro sin saberlo — y el que W5 mide al exigir el rechazo con `--rebind`.
mutar "M2 un binding sin workspace pasa a adoptarse en silencio" "$BINDING" empeora \
    "trata Unknown como Same: el bootstrap se traga un binding sin workspace en vez de negarse. Es el leak silencioso que el fail-closed de VA17 existe para cerrar, y W5 TIENE que notarlo." \
'
import os
p=os.environ["MUT_FILE"]; s=open(p).read()
v = """    pub fn adoption_verdict(&self, observed_workspace: &str) -> WorkspaceAdoption {
        match self.workspace_id.as_deref() {"""
n = """    pub fn adoption_verdict(&self, observed_workspace: &str) -> WorkspaceAdoption {
        if self.workspace_id.is_none() {
            return WorkspaceAdoption::Same;
        }
        match self.workspace_id.as_deref() {"""
assert v in s, "adoption_verdict no tiene la forma que esta mutacion supone"
open(p,"w").write(s.replace(v,n,1))
'

mutar "M3 comentario inocuo (control negativo)" "$CTX" igual \
    "si esto moviera el veredicto, M1 y M2 no significarian nada: probarian que el canario detecta ruido." \
'
import os
p=os.environ["MUT_FILE"]; s=open(p).read()
v = "fn bindings_root(session_root: &Path) -> PathBuf {"
n = """fn bindings_root(session_root: &Path) -> PathBuf {
    // control negativo del canario de worktree: este comentario no cambia nada"""
assert v in s, "bindings_root no tiene la forma que esta mutacion supone"
open(p,"w").write(s.replace(v,n,1))
'

printf '\n----------------------------------------\n'
printf 'PASS=%d FAIL=%d SKIP=%d\n' "$PASS" "$FAIL" "$SKIP"
# MEDIDO (session-85): el resumen gateaba solo con `FAIL -eq 0`, luego una
# comprobacion en SKIP —que no produjo veredicto— era invisible y el canario
# cerraba `RESULT: PASS` con 3 de 4 comprobaciones muertas. Es la misma clase
# que un guard que imprime PASS y sale con codigo de rojo: el veredicto y lo
# que el veredicto cubre no coinciden. Un SKIP aqui es una FALSACION MUERTA,
# porque todas las mutaciones de este fichero tienen sujetos reales.
if [ "$SKIP" -gt 0 ]; then
    printf 'RESULT: FAIL — %s comprobacion(es) no produjeron veredicto, y una comprobacion sin\n' "$SKIP"
    printf '  veredicto no es cobertura: es decoracion. Cada SKIP es una falsacion cuya\n'
    printf '  mutacion no se aplico o cuyo sujeto dejo de medir.\n'
    exit 1
fi
if [ "$FAIL" -eq 0 ]; then
    printf 'RESULT: PASS — el canario mide lo que declara medir (%d violaciones medidas, %d comprobaciones con dientes).\n' \
        "$violations_base" "$PASS"
    exit 0
fi
printf 'RESULT: FAIL — hay comprobaciones que no vigilan lo que declaran.\n'
exit 1