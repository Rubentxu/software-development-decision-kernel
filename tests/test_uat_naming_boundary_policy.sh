#!/usr/bin/env bash
# Gate de la regla 4 de ACCEPTANCE-TRUTHFULNESS-MATRIX.md: un test no puede
# llamarse `e2e`, `real`, `external`, `two-cli` ni `second-binary` si su fila no
# declara una frontera que el test cruce de verdad.
#
# POR QUE ESTE GUARD EXISTE, Y NO ES "PORQUE LA REGLA LO DICE"
# -------------------------------------------------------------
# La regla 4 dice: "La aplicacion mecanica de esta politica es C3n.1 (taxonomy
# gate)". Las dos mitades de esa frase eran FALSAS y estan medidas:
#   (a) el guard de C3n.1 lee las 26 filas AT-UAT de UAT-MATRIX.md y su overlay.
#       No lee ESTA matriz, que es donde vive la regla 4, ni mira ningun nombre.
#   (b) la propia fila que nombra C3n.2 como Trigger de reapertura -- AIW-S7a --
#       tenia CUATRO de sus cinco tests con sufijo prohibido, y los cuatro
#       construian su evento a mano y lo despachaban in-process.
# El (b) esta corregido (rename con el mapeo escrito en los receipts que citan
# los nombres viejos; mismos 5 tests, mismo 5 passed). Este guard es lo que hace
# que no vuelva.
#
# QUE ES UN NOMBRE MENTIROSO, Y POR QUE ES UN DEFECTO Y NO UN ESTILO
# ------------------------------------------------------------------
# El nombre de un test es su unico contrato con quien lo lee sin abrirlo. Un
# `*_e2e` que no cruza proceso, ni binario, ni provider, no es un nombre
# conservador: es una afirmacion falsa con el formato de una verdadera. Y este
# repo ya pago dos veces por nombres que cambiaban el significado: el rename de
# los pins SDDK_LEGACY_CERT_* (rompio la certificacion de cinco releases) y el
# PROCESS/SQLITE_DURABLE de cuatro filas que decidia si exigian receipt o no.
#
# POR QUE TODA LA LOGICA ESTA EN UN SOLO SITIO
# --------------------------------------------
# La segunda version de este guard fallaba en verde: una comprobacion de control
# evaluaba SU PROPIA copia de la funcion que decidia si una fila exige frontera.
# Mutar la funcion real no movia el control, porque el control no la usaba. Es la
# septima vez que esta serie paga lo mismo -- una copia del codigo no vigila el
# codigo -- y aqui la habia repetido yo. Por eso `exige()` y `contar_sufijos()`
# viven en UN solo fichero, y las tres comprobaciones -- la principal, el control
# de exenciones y el control de clasificacion -- los IMPORTAN.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MATRIX="$ROOT/docs/roadmap/ACCEPTANCE-TRUTHFULNESS-MATRIX.md"
SPEC="$ROOT/docs/sddk-roadmap-acceptance-truthfulness-2026-09-30/ROADMAP-ACCEPTANCE-TRUTHFULNESS.md"
PASS=0
FAIL=0

ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

echo "=== regla 4: un sufijo de frontera sin frontera cruzada es un nombre falso ==="

for f in "$MATRIX" "$SPEC"; do
    [ -f "$f" ] || { echo "FAIL: falta $f"; exit 1; }
done

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# ── LA politica, en un solo fichero ─────────────────────────────────────────
cat >"$WORK/politica.py" <<'PY'
"""La politica de la regla 4. Una sola implementacion, importada por todo."""
import re

SUFIJOS_PROHIBIDOS = ("_e2e", "_real", "_external", "_two_cli", "_second_binary")
SEC_HEADING = "### El vocabulario de frontera"


def niveles_exigentes(spec_path):
    """LEE el conjunto exigente de la autoridad. No lo copia: lo extrae."""
    spec = open(spec_path, encoding="utf-8").read()
    i = spec.find(SEC_HEADING)
    if i < 0:
        raise SystemExit("SIN-SECCION: %r no aparece en %s" % (SEC_HEADING, spec_path))
    m = re.search(r"```text\n(.*?)```", spec[i:], re.S)
    if not m:
        raise SystemExit("SIN-BLOQUE: no hay bloque cercado tras la seccion")
    return {l.strip() for l in m.group(1).split("\n")
            if re.fullmatch(r"[A-Z][A-Z_]*", l.strip())}


def filas(matriz_path):
    """(item, boundary_class, [citas]) de cada fila con frontera declarada."""
    out = []
    for ln in open(matriz_path, encoding="utf-8"):
        if not re.match(r'^\|\s*\*{0,2}(AIW-S|R)[0-9]', ln):
            continue
        celdas = [c.strip().strip("*") for c in ln.split("|")]
        if len(celdas) < 5:
            continue
        out.append((celdas[1], celdas[4],
                    sorted(set(re.findall(r'`([A-Za-z0-9_./-]+\.rs)`', ln)))))
    return out


def exige(frontera, niveles):
    """Una fila exige si su boundary_class MENCIONA un nivel exigente.

    La lectura es por TOKEN, no por sufijo de cadena. La primera version comparaba
    con `in`, y "PROCESS" es SUBCADENA de "IN_PROCESS": la fila de AIW-S7a se
    contaba como exigente y su fila entera se saltaba. El guard daba verde con
    los cuatro nombres falsos puestos, y solo lo noto la falsificacion. Un `in`
    que confunde un nivel con otro que lo contiene no es una lectura permisiva: es
    un apagador, porque el caso que se queria vigilar es el de los que NO exigen.
    """
    tokens = set(re.findall(r'[A-Z][A-Z_]*', frontera))
    return bool(tokens & niveles)


def nombres_de_test(fuente):
    """Los nombres de fn que llevan #[test] (con atributos intermedios opcionales)."""
    return re.findall(r'#\[test\]\s*\n(?:#\[[^\]]*\]\s*\n)*fn\s+([a-z0-9_]+)',
                      open(fuente, encoding="utf-8").read())


def contar_sufijos(fuente):
    return sum(1 for n in nombres_de_test(fuente) if n.endswith(SUFIJOS_PROHIBIDOS))


def resolver(raiz, cita):
    """Una cita puede ser ruta, modulo, o basename anidado. Se resuelve o se dice.

    Las tres formas son legitimas: la columna Test cita a veces `algo.rs` y a
    veces `runner_receipt`, que vive en `src/` o un nivel mas abajo. Lo que no es
    legitimo es no resolver ninguna, porque entonces no se puede leer el fichero
    y el veredicto sobre su nombre es nulo, no limpio.
    """
    import glob, os
    directo = os.path.join(raiz, cita)
    if os.path.isfile(directo):
        return [directo]
    base = os.path.basename(cita)
    if not base.endswith(".rs"):
        base += ".rs"
    return sorted(glob.glob(os.path.join(raiz, "crates", "**", base), recursive=True))


def escanear(filas, raiz, niveles):
    """Aplica el VETO. Devuelve (hallazgos, n_filas_exentas, n_filas).

    Es la UNICA implementacion del veto, y la usan el check principal y el
    control 4. La primera version tenia el veto en linea dentro de un heredoc de
    bash, y los controles solo mediaban el extractor y la clasificacion: con el
    veto desconectado el guard se quedaba VERDE, porque no habia nada que lo
    midiera. La autofalsacion lo destapo componiendo M1 con la desconexion; un
    veto sin control propio es decoracion, y la decorator de la que este repo mas
    ha suspeito.
    """
    hallazgos, n_exentas = [], 0
    for item, frontera, citas in filas:
        if exige(frontera, niveles):
            n_exentas += 1
            continue
        for cita in citas:
            rutas = resolver(raiz, cita)
            if not rutas:
                hallazgos.append(("NO-RESUELTA", item, cita,
                                  "la cita no apunta a ningun fichero"))
                continue
            for ruta in rutas:
                import os as _o
                for n in nombres_de_test(ruta):
                    if n.endswith(SUFIJOS_PROHIBIDOS):
                        hallazgos.append(("NOMBRE", item, _o.path.relpath(ruta, raiz), n,
                                          "frontera declarada: " + frontera))
    return hallazgos, n_exentas, len(filas)
PY

export SDDK_POLITICA="$WORK/politica.py"
export SDDK_RAIZ="$ROOT"

EXIGENTES="$(python3 - "$SPEC" <<'PY'
import os, sys
sys.path.insert(0, os.path.dirname(os.environ["SDDK_POLITICA"]))
import politica
print(" ".join(sorted(politica.niveles_exigentes(sys.argv[1]))))
PY
)" || { bad "no se pudo leer el conjunto exigente de la autoridad"; EXIGENTES=""; }
if [[ -n "$EXIGENTES" ]]; then
    ok "conjunto exigente leido de la autoridad: $EXIGENTES"
fi

# ── (a) ninguna fila NO exigiendo cita un test con sufijo prohibido ──────────
SALIDA="$(python3 - "$MATRIX" "$SPEC" "$ROOT" <<'PY'
import os, sys
sys.path.insert(0, os.path.dirname(os.environ["SDDK_POLITICA"]))
import politica

niveles = politica.niveles_exigentes(sys.argv[2])
filas = politica.filas(sys.argv[1])
if not filas:
    print("SIN-FILAS\tla matriz no devolvio ninguna fila con boundary_class")
    raise SystemExit
hallazgos, n_exentas, n_total = politica.escanear(filas, sys.argv[3], niveles)
for h in hallazgos:
    print("\t".join(h))
print("EXENTAS\t%d\t%d" % (n_exentas, n_total))
PY
)"

EXENTAS="$(printf '%s\n' "$SALIDA" | awk -F'\t' '$1=="EXENTAS"{print $2}')"
TOTAL="$(printf '%s\n' "$SALIDA" | awk -F'\t' '$1=="EXENTAS"{print $3}')"
HITS="$(printf '%s\n' "$SALIDA" | grep -c '^\(NOMBRE\|NO-RESUELTA\|SIN-FILAS\)' || true)"

if [[ "${TOTAL:-0}" -eq 0 ]]; then
    bad "la matriz no devolvio ninguna fila con boundary_class: el guard no mide nada"
else
    ok "filas de la matriz con frontera declarada: $TOTAL"
fi
if [[ "${HITS:-0}" -eq 0 ]]; then
    ok "ningun sufijo de frontera en un test cuya fila no declara una exigente"
else
    bad "$HITS hallazgo(s) de nombre o de cita:"
    printf '%s\n' "$SALIDA" | grep '^\(NOMBRE\|NO-RESUELTA\|SIN-FILAS\)' | sed 's/^/        /'
fi

# ── CONTROLES: sin esto, "cero infractores" no distingue de "no mide" ────────
# Control 1: tiene que haber filas que SI exigen, o el veto de nombres no tiene
# contra que ser y el guard pasa por vacuidad.
if [[ "${EXENTAS:-0}" -gt 0 ]]; then
    ok "control: $EXENTAS de $TOTAL fila(s) SI exigen frontera y quedan exentas del veto"
else
    bad "control: ninguna fila exige frontera, luego el veto de nombres no tiene contra que ser"
fi

# Control 2: el extractor ve lo que se le planta. Compara la COPIA contra el
# ORIGINAL en vez de contra una constante, para que siga valiendo aunque el
# fichero real este limpio o sucio: si contase contra 1, con el fichero real
# sucio contaria 5 y caeria por un motivo que no es el suyo.
Copia="$WORK/inyectado.rs"
Original="$ROOT/crates/sddk-gateway/tests/aiw_s7a_producer_l0.rs"
cp "$Original" "$Copia"
printf '\n#[test]\nfn control_plantado_second_binary() {}\n' >>"$Copia"
read -r VISTOS BASE <<<"$(python3 - "$Copia" "$Original" <<'PY'
import os, sys
sys.path.insert(0, os.path.dirname(os.environ["SDDK_POLITICA"]))
import politica
print(politica.contar_sufijos(sys.argv[1]), politica.contar_sufijos(sys.argv[2]))
PY
)"
if [[ $((VISTOS - BASE)) -eq 1 ]]; then
    ok "control: el extractor ve el sufijo prohibido plantado (+1 sobre los $BASE reales)"
else
    bad "control: el extractor no ve el sufijo plantado (copia $VISTOS, original $BASE, esperaba +1)"
fi

# Control 3: LA CLASIFICACION, usando la MISMA `exige` que el check principal.
# Este control existe porque la primera version de este guard comparaba con `in`
# y contaba IN_PROCESS como PROCESS, con lo que la fila de AIW-S7a se saltaba
# entera y el guard pasaba con los cuatro nombres falsos puestos. Como el control
# evaluaba su propia copia, mutar la funcion real no lo movia y la falsificacion
# lo destapo: una copia del codigo no vigila el codigo.
CLASIF="$(python3 - "$SPEC" <<'PY'
import os, sys
sys.path.insert(0, os.path.dirname(os.environ["SDDK_POLITICA"]))
import politica
niveles = politica.niveles_exigentes(sys.argv[1])
CASOS = [
    ("IN_PROCESS (tests unit del gateway)", False,
     "IN_PROCESS contiene la subcadena PROCESS pero no es un nivel exigente"),
    ("PROCESS / SQLITE_DURABLE", True,
     "menciona PROCESS: X04 es concurrencia de >=2 PIDs reales"),
    ("MCP_EXTERNAL (provider externo por MCP)", True, "menciona MCP_EXTERNAL"),
    ("PURE", False, "no cruza nada"),
    ("IN_PROCESS/SQLITE", False, "compuesto, y ninguno de sus dos token exige"),
    ("RELEASE_ARTIFACT", True, "el artefacto publicado, tal como lo instala un usuario"),
]
malos = 0
for frontera, esperado, porque in CASOS:
    real = politica.exige(frontera, niveles)
    if real != esperado:
        print("  MAL  %-42s esperaba exige=%s, obtuvo %s (%s)" % (frontera, esperado, real, porque))
        malos += 1
print(malos)
PY
)"
if [[ "${CLASIF:-99}" == "0" ]]; then
    ok "control: IN_PROCESS no cuenta como PROCESS, y PROCESS / SQLITE_DURABLE si cuenta"
else
    bad "control: la clasificacion de niveles exigentes confunde un nivel con otro que lo contiene"
    python3 - "$SPEC" <<'PY'
import os, sys
sys.path.insert(0, os.path.dirname(os.environ["SDDK_POLITICA"]))
import politica
niveles = politica.niveles_exigentes(sys.argv[1])
print("        niveles leidos: %s" % ", ".join(sorted(niveles)))
for f in ("IN_PROCESS (tests unit del gateway)", "PROCESS / SQLITE_DURABLE", "PURE"):
    print("        %-42s -> exige=%s" % (f, politica.exige(f, niveles)))
PY
fi

# Control 4: EL VETO PROPIAMENTE DICHO, con datos que el guard controla.
# Los controles 2 y 3 miden el extractor y la clasificacion; ninguno merma el
# veto. Con el veto desconectado y el repo limpio, el guard daba VERDE: nadie
# miraba lo que el veto hacia, solo lo que sus ayudantes hacian. Este control
# construye una fila y un test sinteticos con un sufijo prohibido y exige que
# escanear() los senale. Es el control que convierte el veto en algo con dientes
# en vez de una rama que se podria desconectar sin consecuencias.
VETO="$(python3 - "$SPEC" "$WORK" <<'PY'
import os, sys
sys.path.insert(0, os.path.dirname(os.environ["SDDK_POLITICA"]))
import politica

niveles = politica.niveles_exigentes(sys.argv[1])
lab = sys.argv[2]
plantado = os.path.join(lab, "sintetico_e2e.rs")
with open(plantado, "w", encoding="utf-8") as fh:
    fh.write("#[test]\nfn planta_real_e2e() {}\n#[test]\nfn planta_limpia() {}\n")
casos = [
    # (frontera, citado, debe_ser_señalado, porque)
    ("IN_PROCESS (tests unit del gateway)", plantado, True,
     "IN_PROCESS no exige, luego un sufijo de frontera es un nombre falso"),
    ("PROCESS / SQLITE_DURABLE", plantado, False,
     "la fila SI exige: el mismo nombre es honesto aqui"),
]
malos = 0
for frontera, citado, esperado, porque in casos:
    filas = [("SINTETICA", frontera, [citado])]
    hallazgos, _, _ = politica.escanear(filas, lab, niveles)
    hay = any(h[0] == "NOMBRE" for h in hallazgos)
    if hay != esperado:
        print("  MAL  %-40s esperaba senalado=%s, obtuvo %s (%s)" % (frontera, esperado, hay, porque))
        malos += 1
print(malos)
PY
)"
if [[ "${VETO:-99}" == "0" ]]; then
    ok "control: el veto senala un nombre prohibido en una fila que no exige, y lo respeta si exige"
else
    bad "control: el veto no hace lo que dice: no senala lo prohibido, o senala lo permitido"
fi

echo
echo "PASS=$PASS FAIL=$FAIL"
if [[ $FAIL -eq 0 ]]; then
    echo "RESULT: PASS -- ningun nombre de test promete una frontera que su fila no declara."
else
    echo "RESULT: FAIL -- hay nombres que prometen una frontera que el test no cruza,"
    echo "o el guard ha dejado de medir (control caido)."
fi
[[ $FAIL -eq 0 ]]
