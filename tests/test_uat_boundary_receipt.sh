#!/usr/bin/env bash
# Gate de C3n.1 / AT-UAT-023: un receipt de UAT en un nivel que exige frontera
# tiene que DECLARAR esa frontera, y el guard rechaza los que no.
#
# QUE GUARDA, Y POR QUE NO PUEDE SER UNA COPIA
# --------------------------------------------
# El conjunto de niveles exigentes esta declarado en UN sitio --
# ROADMAP-ACCEPTANCE-TRUTHFULNESS.md §C3n.1 -- y este guard lo LEE de ahi en
# vez de escribirlo. Session-69s bis 5 lo midio: el conjunto estaba escrito en
# dos sitios que no coincidian, y el NOMBRE del nivel decidia si una fila exigia
# receipt o no. Un guard con el conjunto copiado dentro habria sido una tercera
# declaracion, y la tercera es la que diverge.
#
# Ademas el guard exige que las dos matrices coincidan fila a fila en su nivel.
# Hoy coinciden (se reconciliaron en esta sesion); si divergen otra vez, el gate
# lo dice, porque un receipt puede ser valido para un nivel y no para el otro.
#
# POR QUE COMPRUEBA EL CONTENIDO Y NO SOLO QUE LA CLAVE EXISTA
# -----------------------------------------------------------
# Una comprobacion de presencia se satisface con `process_count: por medir`, que
# es justo el texto de un receipt que no midio nada. Por eso los dos campos que
# sostienen la frontera exigen TIPO: `process_count` entero >= 1 y
# `binary_sha256` de 64 hexadecimales. La presencia la pone cualquiera; el tipo
# no.
#
# Y el hueco declarado, que este guard NO cierra y por tanto no puede tapar:
# el operador decidio (session-69s) el conjunto pequeno -- PROCESS,
# MCP_EXTERNAL, RELEASE_ARTIFACT --, de modo que 13 de las 26 filas quedan
# exentas, 5 de ellas cruzando estado durable entre dos procesos. Aqui se
# COMPRUEBA que el numero de exentas es el declarado, para que crecer en
# silencio sea visible; no se exige boundary a esas filas.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SPEC="$ROOT/docs/sddk-roadmap-acceptance-truthfulness-2026-09-30/ROADMAP-ACCEPTANCE-TRUTHFULNESS.md"
MATRIX="$ROOT/docs/roadmap/UAT-MATRIX.md"
OVERLAY="$ROOT/docs/sddk-roadmap-acceptance-truthfulness-2026-09-30/UAT-MATRIX-OVERLAY.md"
EXENTAS_DECLARADAS=13
# El titulo de la seccion se DECLARA UNA VEZ y se pasa a los tres lectores.
# Session-69s bis 5 lo pago. El literal de la mutacion tenia las tres ultimas
# letras de la palabra "vocabulario" transpuestas (a-r-i-e donde el documento
# dice a-r-i-o), asi que no existia en el spec y `find` devolvio -1 sin error.
# En python, -1 en el segundo argumento de `find` significa "buscar desde el
# final": la mutacion no muto, escribio un fichero de 42 KB --el documento
# entero mas el bloque nuevo-- en vez del documento con el bloque cambiado.
# Un literal escrito a mano que no casa no degrada a un fallo: degrada a
# silencioso. Los tres lectores ahora reciben el titulo y salen con codigo 2.
SEC_HEADING='### El vocabulario de frontera'
PASS=0
FAIL=0

ok()  { echo "  [ok]   $1"; PASS=$((PASS + 1)); }
bad() { echo "  [FAIL] $1"; FAIL=$((FAIL + 1)); }

echo "=== C3n.1 / AT-UAT-023: receipt de frontera exigible ==="

for f in "$SPEC" "$MATRIX" "$OVERLAY"; do
    [ -f "$f" ] || { echo "FAIL: falta $f"; exit 1; }
done
python3 -c 'import yaml' 2>/dev/null || {
    echo "FAIL: este gate necesita PyYAML para leer receipts (se lee fail-closed a proposito,"
    echo "      porque un gate que se salta la validacion por falta de una dependencia no valida)."
    exit 1
}

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# El extractor del conjunto vive en un fichero, no en dos heredocs: (a) y (e)
# ejecutan el MISMO codigo, que es lo que hace que (e) falsifique al gate
# y no a una copia suya.
cat >"$WORK/leer_conjunto.py" <<'PY'
import re, sys
spec = open(sys.argv[1], encoding="utf-8").read()
i = spec.find(sys.argv[2])
if i < 0:
    print("SIN-SECCION: %r no aparece en el spec" % sys.argv[2], file=sys.stderr)
    raise SystemExit(2)
m = re.search(r"```text\n(.*?)```", spec[i:], re.S)
if not m:
    print("SIN-BLOQUE: no hay bloque cercado tras la seccion", file=sys.stderr)
    raise SystemExit(2)
for line in m.group(1).split("\n"):
    line = line.strip()
    if re.fullmatch(r"[A-Z][A-Z_]*", line):
        print(line)
PY

# ── (a) el conjunto canonico se LEE, y existe ──────────────────────────────
# Se extrae del bloque cercado de texto que sigue al titulo de la seccion, no de
# una copia local. Si el documento dejara de declararlo, el gate falla en vez de
# seguir con un conjunto inventado.
leer_conjunto() {
    python3 "$WORK/leer_conjunto.py" "$SPEC" "$SEC_HEADING" >"$WORK/exigentes.txt"
}
leer_conjunto || bad "el extractor no pudo leer el conjunto canonico del spec"
mapfile -t EXIGENTES <"$WORK/exigentes.txt"
if [[ ${#EXIGENTES[@]} -eq 0 ]]; then
    bad "el conjunto canonico de niveles exigentes no se puede leer de $SPEC"
    echo "        (el gate lo LIE de ahi a proposito; si no esta, no inventa uno)"
else
    ok "conjunto canonico leido de la autoridad: ${#EXIGENTES[@]} niveles (${EXIGENTES[*]})"
fi

# ── (b) las dos matrices coinciden en el nivel de cada fila ────────────────
if diff <(python3 - "$MATRIX" <<'PY'
import re, sys
for l in open(sys.argv[1], encoding="utf-8"):
    m = re.match(r'\|\s*(AT-UAT-\d+)\s*\|\s*([^|]+?)\s*\|\s*([A-Z_/]+)\s*\|', l)
    if m: print(m.group(1), m.group(3))
PY
) <(python3 - "$OVERLAY" <<'PY'
import re, sys
for l in open(sys.argv[1], encoding="utf-8"):
    m = re.match(r'\|\s*(AT-UAT-\d+)\s*\|\s*([^|]+?)\s*\|\s*([A-Z_/]+)\s*\|', l)
    if m: print(m.group(1), m.group(3))
PY
) >"$WORK/level.diff" 2>&1; then
    ok "las dos matrices coinciden fila a fila en el nivel"
else
    bad "las dos matrices discrepan en el nivel de alguna fila"
    head -8 "$WORK/level.diff" | sed 's/^/        /'
fi

# ── el validador ───────────────────────────────────────────────────────────
# Unico sitio que decide. Entrada: receipt + conjunto. Salida: ACCEPT/REJECT.
validar() {
    python3 - "$1" "$WORK/exigentes.txt" <<'PY'
import re, sys, yaml
path, exig = sys.argv[1], sys.argv[2]
niveles = {l.strip() for l in open(exig) if re.fullmatch(r"[A-Z][A-Z_]*", l.strip())}
try:
    d = yaml.safe_load(open(path, encoding="utf-8")) or {}
except Exception as e:
    print("REJECT"); print(f"  - el receipt no es YAML legible: {e}"); raise SystemExit(1)
if not isinstance(d, dict):
    print("REJECT"); print("  - el receipt no es un mapa"); raise SystemExit(1)
nivel = d.get("boundary_level")
problemas = []
if not nivel:
    problemas.append("el receipt no declara 'boundary_level', sin el cual no se puede saber si exige")
elif nivel not in niveles:
    # Nivel no exigiente: no se exige frontera. Y NO se cuenta como PASS por
    # tener campos deboundary; se sale con ACCEPT y sin haber medido nada, que es
    # exactamente lo que el hueco declarado dice que es esta zona.
    print("ACCEPT"); print(f"  - nivel {nivel} no es exigente: no se exige frontera (hueco declarado)")
    raise SystemExit(0)
for campo in ("boundary", "mode", "provider", "binary_sha256", "storage",
              "process_count", "falsifier"):
    v = d.get(campo)
    if v is None or (isinstance(v, str) and not v.strip()):
        problemas.append(f"falta o vacio: {campo}")
pc = d.get("process_count")
if pc is not None and not (isinstance(pc, int) and not isinstance(pc, bool) and pc >= 1):
    problemas.append(f"process_count debe ser entero >= 1, no {pc!r}")
bs = d.get("binary_sha256")
if bs is not None and not re.fullmatch(r"[0-9a-f]{64}", str(bs)):
    problemas.append(f"binary_sha256 debe ser 64 hexadecimales en minuscula, no {str(bs)[:24]!r}")
if problemas:
    print("REJECT")
    for p in problemas: print("  - " + p)
    raise SystemExit(1)
print("ACCEPT")
PY
}

# ── (c) el hueco declarado no crece en silencio ─────────────────────────────
python3 - "$MATRIX" "$WORK/exigentes.txt" >"$WORK/hueco.txt" <<'PY'
import re, sys
niveles = {l.strip() for l in open(sys.argv[2]) if re.fullmatch(r"[A-Z][A-Z_]*", l.strip())}
exentas = 0
for l in open(sys.argv[1], encoding="utf-8"):
    m = re.match(r'\|\s*(AT-UAT-\d+)\s*\|\s*([^|]+?)\s*\|\s*([A-Z_/]+)\s*\|', l)
    if m and m.group(3) not in niveles and m.group(3) != "PURE":
        exentas += 1
print(exentas)
PY
HUECO="$(cat "$WORK/hueco.txt")"
if [[ "$HUECO" == "$EXENTAS_DECLARADAS" ]]; then
    ok "el hueco declarado sigue siendo $HUECO filas (no crece en silencio)"
else
    bad "el hueco declarado cambio: son $HUECO filas y la cifra escrita es $EXENTAS_DECLARADAS"
    echo "        Si el conjunto canonico cambio a proposito, actualiza TAMBIEN el"
    echo "        §C3n.1 del spec y esta cifra, o el gate dejara de describir la realidad."
fi

# ── (d) autoprueba: RECHAZA cada receipt incompleto, y ACEPTA el bueno ──────
# El control es lo que separa un gate de un gate vacio: uno que rechazase todo
# pasaria esta seccion sin medir nada.
receipt_ok() {
    cat >"$1" <<EOF
id: AT-UAT-013
boundary_level: PROCESS
boundary: proceso hijo real del producto
mode: e2e
provider: n/a
binary_sha256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
storage: ledger aislado
process_count: 2
falsifier: el lector no observa el estado durable
EOF
}

receipt_ok "$WORK/bueno.yaml"
if validar "$WORK/bueno.yaml" >/dev/null 2>&1; then
    ok "control: el receipt completo se ACEPTA (el gate no es vacio)"
else
    bad "control: el receipt completo se RECHAZA -- el gate es vacio o la plantilla esta mal"
fi

# Cada mutacion quita o corrompe UNA cosa, y el nombre de la asercion dice cual.
rechazar() {
    local nombre="$1" mutador="$2"
    receipt_ok "$WORK/m.yaml"
    python3 - "$WORK/m.yaml" "$mutador" <<'PY'
import re, sys
p, regla = sys.argv[1], sys.argv[2]
t = open(p, encoding="utf-8").read()
if regla == "sin_process_count":   t = re.sub(r"^process_count: .*\n", "", t, flags=re.M)
elif regla == "sin_binary":        t = re.sub(r"^binary_sha256: .*\n", "", t, flags=re.M)
elif regla == "sin_falsifier":     t = re.sub(r"^falsifier: .*\n", "", t, flags=re.M)
elif regla == "sin_nivel":         t = re.sub(r"^boundary_level: .*\n", "", t, flags=re.M)
elif regla == "process_count_texto":
    t = re.sub(r"^process_count: .*$", "process_count: por medir", t, flags=re.M)
elif regla == "binary_no_hex":
    t = re.sub(r"^binary_sha256: .*$", "binary_sha256: NO-ES-UN-HASH", t, flags=re.M)
elif regla == "process_count_cero":
    t = re.sub(r"^process_count: .*$", "process_count: 0", t, flags=re.M)
elif regla == "nivel_no_exigente":
    t = re.sub(r"^boundary_level: .*$", "boundary_level: PURE", t, flags=re.M)
else: raise SystemExit("regla desconocida: " + regla)
open(p, "w", encoding="utf-8").write(t)
PY
    if validar "$WORK/m.yaml" >/dev/null 2>&1; then
        if [[ "$mutador" == "nivel_no_exigente" ]]; then
            ok "$nombre (no exigente: se acepta sin exigir frontera, que es el hueco declarado)"
        else
            bad "$nombre (el gate lo ACEPTO -- no vigila)"
        fi
    else
        if [[ "$mutador" == "nivel_no_exigente" ]]; then
            bad "$nombre (un nivel no exigente debe ACEPTARSE: exigirle frontera seria inventar cobertura)"
        else
            ok "$nombre"
        fi
    fi
}

rechazar "M1 sin process_count"                     sin_process_count
rechazar "M2 sin binary_sha256"                     sin_binary
rechazar "M3 sin falsifier"                         sin_falsifier
rechazar "M4 sin boundary_level (no se puede saber si exige)" sin_nivel
rechazar "M5 process_count en texto"                process_count_texto
rechazar "M6 binary_sha256 que no es hash"          binary_no_hex
rechazar "M7 process_count = 0"                     process_count_cero
rechazar "M8 nivel PURE: no se exige"               nivel_no_exigente

# ── (e) la DECISION sigue al spec, no a una copia ──────────────────────────
# Lo que esta comprobacion afirma es que el gate no tiene una tercera
# declaracion del conjunto. La primera version solo afirmaba otra cosa: que el
# spec se puede mutar y que un lector independiente ve el valor nuevo. Con el
# conjunto copiado dentro del gate, esa version daba PASS igual, porque el
# lector de (e) no era el del gate.
#
# Ahora se prueba la DECISION: se muta el conjunto de la autoridad a un nivel
# que hoy NO exige (MIXED), se reextrae por el mismo codigo que usa (a), y se
# pasa un receipt de ese nivel sin campos de frontera por el validador REAL.
# Si el gate tuviera el conjunto escrito dentro, seguiria aceptandolo.
SPEC_BAK="$WORK/spec.bak"
cp "$SPEC" "$SPEC_BAK"
# El trap restaura el spec aunque el gate muera a mitad: la mutacion escribe
# sobre la FUENTE, y un fallo dejaria el documento peor que antes de empezar.
trap 'cp "$SPEC_BAK" "$SPEC" 2>/dev/null; rm -rf "$WORK"' EXIT
MIXED_NIVEL=MIXED
MUT_OK=1
python3 - "$SPEC" "$SEC_HEADING" "$MIXED_NIVEL" <<'PY' || MUT_OK=0
import re, sys
p, heading, nivel = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(p, encoding="utf-8").read()
i = s.find(heading)
if i < 0:
    print("SIN-SECCION: %r no aparece en %s" % (heading, p), file=sys.stderr)
    raise SystemExit(2)
j = s.find("```text", i)
k = s.find("```", j + 7) if j >= 0 else -1
if j < 0 or k <= j:
    print("SIN-BLOQUE: no hay bloque cercado tras la seccion", file=sys.stderr)
    raise SystemExit(2)
nuevo = "```text\n%s\n```" % nivel
s2 = s[:j] + nuevo + s[k + 3:]
if s2 == s:
    print("NO-OP: el conjunto ya era %r; la mutacion no probaria nada" % nivel, file=sys.stderr)
    raise SystemExit(2)
open(p, "w", encoding="utf-8").write(s2)
# Se relee lo escrito: una mutacion que no se puede observar no es una
# mutacion, es un cambio de fichero.
check = open(p, encoding="utf-8").read()
c = check.find(heading)
cj = check.find("```text", c) if c >= 0 else -1
ck = check.find("```", cj + 7) if cj >= 0 else -1
if c < 0 or cj < 0 or check[cj:ck + 3] != nuevo:
    print("SIN-EFECTO: el bloque tras escribir es %r" % (check[cj:ck + 3] if cj >= 0 else None), file=sys.stderr)
    raise SystemExit(2)
PY

# El receipt es de un nivel que HOY es exento: con el conjunto real se acepta
# (hueco declarado), y con el conjunto mutado tiene que RECHAZARSE. Ese cambio
# de veredicto es la prueba de que la decision se lee del spec.
cat >"$WORK/mezclado.yaml" <<EOF
id: AT-UAT-024
boundary_level: $MIXED_NIVEL
boundary:
mode:
provider:
binary_sha256:
storage:
process_count:
falsifier:
EOF

if [[ $MUT_OK -ne 1 ]]; then
    bad "no se pudo mutar el spec de forma controlada: (e) no se ha ejecutado"
    echo "        (ver arriba el motivo; el veredicto sobre el gate queda sin emitir)"
else
    # Se repite la FUNCION de extraccion, no una copia de su codigo, y se llama
    # a `validar` con la misma forma que usa (d). Un solo camino: si el conjunto
    # estuviera escrito dentro de `leer_conjunto` en vez de leido del spec, el
    # receipt se aceptaria y esta comprobacion caeria.
    leer_conjunto 2>/dev/null
    MUT_SET="$(tr '\n' ' ' <"$WORK/exigentes.txt")"
    if validar "$WORK/mezclado.yaml" >/dev/null 2>&1; then
        bad "el gate lee su conjunto del spec: mutarlo NO lo cambia"
        echo "        (conjunto leido tras mutar: $MUT_SET -- si el gate tuviera"
        echo "         el conjunto escrito dentro, este receipt pasaria igual)"
    else
        ok "el gate lee su conjunto del spec: mutarlo lo cambia (no hay copia dentro)"
        echo "        (tras mutar, el conjunto es $MUT_SET: un receipt de ese nivel"
        echo "         sin frontera pasa de ACEPTAR a RECHAZAR)"
    fi
fi
cp "$SPEC_BAK" "$SPEC"
if diff -q "$SPEC" "$SPEC_BAK" >/dev/null; then
    ok "el spec quedo restaurado byte-identico tras la mutacion"
else
    bad "el spec NO quedo restaurado: la mutacion de prueba se llevo por delante la fuente"
fi

echo
echo "PASS=$PASS FAIL=$FAIL"
if [[ $FAIL -eq 0 ]]; then
    echo "RESULT: PASS -- un receipt en un nivel exigente declara su frontera o es invalido."
else
    echo "RESULT: FAIL -- el gate acepta receipts que no declaran la frontera que dicen medir,"
    echo "o su conjunto canonico ha dejado de leerse."
fi
[[ $FAIL -eq 0 ]]
