#!/usr/bin/env python3
"""test_contamination_surface_mutation.py — autofalsación de la ampliación del
guard de contaminación (session-69s bis 10).

POR QUÉ EXISTE
--------------
El guard de contaminación (session-66) barria `docs/` y nada más, y llevaba
meses en verde. Al medir el problema sobre todo lo trackeado appeared que la
superficie real era **el doble**: hay CJK y cirílico en `crates/`,
`skills/`, `specs/`, `tests/cycle-artifacts/` y `CHANGELOG.md`. **Un guard que
vigila el 8% del problema y está en verde** es la misma clase que el token
`SPEC-NNN` fuera del vocabulario del guard de citas: una comprobación que
parece cubrir porque tiene nombre de guard, y solo cubre lo que su primer
comitter quiso mirar.

La ampliación añadió la distinción PROSA / ARTE, y esa distinción necesitó
**cuatro versiones**, cada una con un modo de verde falso distinto:

  v1  «si no está entre dos letras, es arte»  → 6 falsos positivos, entre ellos
      una URL de vault que acababa en `ADR-0072`
  v2  «y además la línea es sobre todo barra» → 2 falsos negativos: separaba por
      longitud de barra, y `── Pin 15: … ──` tiene menos barra que una URL
  v3  «el símbolo es U+FFFD y toca la barra» → fallaba en el segundo U+FFFD de
      una racha, que solo tiene U+FFFD a ambos lados
  v4  «el símbolo es U+FFFD y la RACHA toca la barra» → la que quedó

**Cuatro versiones para una regla de dos líneas es el dato más caro de este
fichero**, y por eso existe como prueba permanente y no como nota: la regla
final no se deduce, se falsifica. Cada mutación de aquí rompe UNA de las
condiciones y exige que el guard reporte la línea contaminada.

LA REGLA, QUE ES LO QUE ESTE FICHERO FALSA
------------------------------------------
`is_box_drawing(line)` es True (arte, no veredicto) si y solo si:
  1. la línea tiene un símbolo de barra, y
  2. todo símbolo contaminado de la línea es `U+FFFD`, y
  3. cada RACHA de `U+FFFD` está pegada a la barra por alguno de sus dos lados.

Medido sobre los datos reales: el arte usa `U+FFFD` (un carácter de reemplazo:
un símbolo que se perdió al codificar) y la prosa usa CJK o cirílico real. Una
barra que perdió símbolos sigue siendo una barra; una palabra que los perdió
ya no es la palabra que alguien escribió, que es el defecto que el guard existe
para declarar.

Salida: exit 0 si toda mutación aplicó y detectó; 1 si no.
"""

from __future__ import annotations

import importlib.util
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
GUARD_PATH = ROOT / "tests" / "test_docs_script_contamination.py"

spec = importlib.util.spec_from_file_location("contamination_guard", GUARD_PATH)
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)

FFFD = "�"
CJK = "收"
CYR = "力"
BAR = "─"

# (nombre, linea, decision esperada por la regla, que seccion se rompe)
# El `esperado` es la CORRECTA segun la regla documentada arriba. Una mutacion
# que cambia ese valor es la que esta midiendo el fallo: si la v1, la v2 o la v3
# hubieran sobrevivido, estas lineas darian el valor contrario.
CASES: list[tuple[str, str, bool]] = [
    ("barra simple, sin contaminar", f"// {BAR*9}", False),
    ("barra con texto, sin contaminar", f"// {BAR*3} Pin 5: X + Y -> NO violation {BAR*3}", False),
    # Las dos primeras son barras SIN simbolo perdido, y por eso la regla dice
    # False: no hay nada que clasificar. Exigir True ahi obligaria a la regla a
    # devolver True para toda linea con barra, que es exactamente la version
    # que clasifica `/// The runtime[CJK]es` como arte.
    ("racha de FFFD en medio de la barra", f"// {BAR*3}{FFFD*2}{BAR*3} titulo", True),
    ("racha de FFFD al final de la barra", f"// {BAR*3} titulo {BAR*2}{FFFD*2}{BAR}", True),
    # Una barra SIN contaminar no es arte "danado": no es contaminacion. La
    # regla responde a "esta linea esta contaminada y es arte?", y si no esta
    # contaminada la pregunta no se hace. Ponerla aqui como caso de arte
    # obligaria a la regla a devolver True para toda linea con barra, que es
    # justo la version que clasifica `/// The runtime[CJK]es` como arte.
    ("barra limpia, sin contaminar", f"// {BAR*20}", False),
    # --- y estas seis son las que la v1, v2 y v3 clasificaban MAL ---
    ("prosa en un doc-comment", f"/// The runtime{CJK}es when all children complete.", False),
    ("URL que acaba en ID", f"> `ADR-0072` en vault (`~/.sddk/x.md`){CJK}", False),
    ("vinieta con FFFD suelto", f"-  **Policy Resistance**: blogs diciendo cosas {FFFD}", False),
    ("celda de tabla", f"| Criterio | Valor | {CJK} |", False),
    ("cirilico en prosa", f"El metodo {CYR} una transaccion completa.", False),
    ("prosa entre dos barras", f"// {BAR*3} S-3: Pareto {CJK} {BAR*3}", False),
]

passed = 0
sets: dict[str, frozenset] = {}
failed: list[str] = []


def mutar_regex(patron: str, replacement: str):
    """Devuelve una funcion que cambia el patron BOX_DRAWING del guard."""

    def aplicar():
        original = guard.BOX_DRAWING
        guard.BOX_DRAWING = re.compile(replacement)
        return original

    return aplicar


def con_patron(restore, fn):
    restore()
    try:
        return fn()
    finally:
        guard.BOX_DRAWING = restore.__closure__[0].cell_contents if False else restore


print("== autofalsación: guard de contaminación ampliado (session-69s bis 10) ==")
print(f"  base: {len(CASES)} casos, todos con la regla vigente")
print()

# ── CONTROL DE NO-VACUIDAD: la regla tiene que acertar los 11 casos ─────────
aciertos = 0
for nombre, linea, esperado in CASES:
    real = guard.is_box_drawing(linea)
    if real == esperado:
        aciertos += 1
    else:
        failed.append(
            f"control: «{nombre}» es {'arte' if real else 'prosa'}, "
            f"la regla dice {'arte' if esperado else 'prosa'}"
        )
        print(f"  [FAIL] control: {nombre}: esperado {esperado}, real {real}")

if aciertos == len(CASES):
    passed += 1
    print(f"  [ok]   control de no-vacuidad: {aciertos}/{len(CASES)} casos con la "
          f"regla vigente, arte y prosa separados")
else:
    print(f"  [FAIL] control de no-vacuidad: {aciertos}/{len(CASES)}")

# ── M1: el simbolo deja de ser U+FFFD y cualquier cosa cuenta como arte ──────
# Rompe la condicion 2. Con ella caída, `runtime[CJK]es` —que es `converges`
# truncado— pasa a clasificarse como arte y el guard deja de reportar la
# prosa mas grave del repo.
def m1():
    original = guard.is_box_drawing
    guard.is_box_drawing = lambda line: bool(guard.BOX_DRAWING.search(line))
    try:
        return [n for n, l, exp in CASES if guard.is_box_drawing(l) != exp]
    finally:
        guard.is_box_drawing = original


# ── M2: la racha deja de exigir que toque la barra ─────────────────────────
# Rompe la condicion 3. Es la v1 del guard: cualquier simbolo que no este
# entre dos letras pasa a ser arte, y con el una URL de vault.
def m2():
    original = guard.is_box_drawing
    guard.is_box_drawing = lambda line: (
        bool(guard.BOX_DRAWING.search(line))
        and all(m.group()[0] == FFFD for m in guard.CONTAMINATION.finditer(line))
    )
    try:
        return [n for n, l, exp in CASES if guard.is_box_drawing(l) != exp]
    finally:
        guard.is_box_drawing = original


# ── M3: se exige barra a la izquierda Y a la derecha ───────────────────────
# Rompe la condicion 3 por el otro lado. Es la v3: una racha de FFFD en medio
# de la barra no toca barra por ningun lado, y se|reportaba como prosa.
def m3():
    original = guard.is_box_drawing
    def solo_izquierda(line: str) -> bool:
        if guard.BOX_DRAWING.search(line) is None:
            return False
        for m in re.finditer(FFFD + "+", line):
            before = line[m.start() - 1] if m.start() > 0 else ""
            if not guard.BOX_DRAWING.match(before or " "):
                return False
        return all(m.group()[0] == FFFD for m in guard.CONTAMINATION.finditer(line))
    guard.is_box_drawing = solo_izquierda
    try:
        return [n for n, l, exp in CASES if guard.is_box_drawing(l) != exp]
    finally:
        guard.is_box_drawing = original


# ── M4: la barra deja de contar y todo es prosa ─────────────────────────────
# Rompe la condicion 1. Sin barra, el arte se reporta como prosa y el guard
# exige redibujar arte de comentario: escribir para el guard en vez de para
# quien lee.
def m4():
    original = guard.BOX_DRAWING
    guard.BOX_DRAWING = re.compile(r"(?!x)x")  # nunca casa
    try:
        return [n for n, l, exp in CASES if guard.is_box_drawing(l) != exp]
    finally:
        guard.BOX_DRAWING = original


MUTACIONES = [
    ("M1", "el simbolo deja de ser U+FFFD (condicion 2)", m1),
    ("M2", "la racha deja de exigir que toque la barra (condicion 3)", m2),
    ("M3", "se exige barra solo por la izquierda (condicion 3, el otro lado)", m3),
    ("M4", "la barra deja de contar (condicion 1)", m4),
]

for name, why, mut in MUTACIONES:
    if not all(guard.is_box_drawing(l) == exp for _n, l, exp in CASES):
        failed.append(f"{name}: la base no está verde, la mutación no mide nada")
        print(f"  [FAIL] {name}: la base no esta verde")
        continue
    rotas = mut()
    if not rotas:
        failed.append(f"{name} NO CAE: {why}")
        print(f"  [FAIL] {name}: no cambio ningun caso. {why}")
    else:
        passed += 1
        print(f"  [ok]   {name} cae: {why}")
        print(f"           casos que clasifica mal: {', '.join(rotas[:3])}")
        sets[name] = frozenset(rotas)

# ── SEPARACION: una mutacion no puede caer por el efecto colateral de otra ──
# Que las cuatro caigan no basta. Si M2 y M3 rompieran el MISMO conjunto de
# casos, estarian midiendo lo mismo y la que quede en pie podria caerse sin
# que nadie lo notara. Se exige que los conjuntos sean DISTINTOS.
print()
for a, b in (("M1", "M2"), ("M2", "M3"), ("M1", "M3"), ("M3", "M4")):
    if a in sets and b in sets and sets[a] == sets[b]:
        failed.append(
            f"{a} y {b} rompen EXACTAMENTE los mismos casos: una de las dos esta "
            f"midiendo la condicion de la otra"
        )
        print(f"  [FAIL] {a} y {b} no se distinguen: rompen los mismos {len(sets[a])} casos")
    elif a in sets and b in sets:
        passed += 1
        print(f"  [ok]   {a} y {b} rompen conjuntos distintos "
              f"({len(sets[a])} vs {len(sets[b])} casos): cada una mide su condicion")

# La regla vigente tiene que seguir acertando despues de todo.
if not all(guard.is_box_drawing(l) == exp for _n, l, exp in CASES):
    failed.append("la regla vigente quedo alterada por una mutacion")
    print("  [FAIL] la regla vigente quedo alterada")
else:
    print("  [ok]   la regla vigente sigue acertando los 11 casos tras restaurar")

print()
print(f"  PASS={passed} FAIL={len(failed)} SKIP=0")
if failed:
    print()
    for f in failed:
        print(f"  [FAIL] {f}")
    print()
    print("RESULT: FAIL — una condicion de la regla arte/prosa no tiene dientes, o la")
    print("         regla vigente no separa los casos. Un guard que no distingue arte de")
    print("         prosa o prohibe dibujar, y las dos cosas se cumplen de forma formal.")
    sys.exit(1)

print()
print("RESULT: PASS — las tres condiciones tienen dientes por separado, el control de")
print("         no-vacuidad acepta el caso bueno, y la regla separa arte de prosa.")
sys.exit(0)
