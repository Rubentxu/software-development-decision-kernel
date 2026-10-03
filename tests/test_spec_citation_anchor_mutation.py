#!/usr/bin/env python3
"""test_spec_citation_anchor_mutation.py — autofalsación de
`test_spec_citation_anchor.py`.

POR QUÉ EXISTE
--------------
Un guard que no ha caído nunca no es un guard: es una aserción. Este fichero
**siembra el defecto, ejecuta el guard, exige la caída, y restaura**.

EL HALLAZGO QUE OBLIGÓ A REESCRIBIR ESTE FICHERO
------------------------------------------------
La primera versión mutaba **la detección**: rompía `ANCHOR_NATIVE`,
`declares_provenance`, `history_candidates` y el barrido de `crates/`, y
exigía que el guard cayera. Resultado: **`PASS=1 FAIL=5`**. Cuatro de las cinco
mutaciones no caían, y la razón es la misma que la quinta: **el guard estaba
verde porque yo ya había corregido las tres citas antes de construirlo.** No
había ningún defecto en la base sobre el que una detección rota pudiera fallar.

Eso no es un defecto del falsificador: es un hecho sobre qué prueba un guard
verde. **Un guard con cero defectos que medir es indistinguible de un guard sin
teeth**, y la única forma de separarlos es sembrar el defecto. Por eso cada
mutación aquí:
  1. copia el árbol relevante a un directorio temporal,
  2. aplica UNA rotura,
  3. apunta el guard importado a ese árbol,
  4. exige exit != 0 y que el veredicto nombre la propiedad rota,
  5. descarta el árbol.

NADA se muta en el repo real, y por eso no hay restauración que pueda fallar:
la mutación vive en una copia que se tira. Es la diferencia entre «restauré
todo» y «nunca toqué el repo».

MECÁNICA
--------
Se **importa** el guard, no se copia: una copia del código no vigila el código.
Y se parchea la comprobación concreta de cada mutación, para que una no pueda
caer por el efecto colateral de otra.

Salida: exit 0 si cada mutación aplicó y cayó; 1 si no.
"""

from __future__ import annotations

import contextlib
import importlib.util
import io
import pathlib
import re
import shutil
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
GUARD_PATH = ROOT / "tests" / "test_spec_citation_anchor.py"

spec = importlib.util.spec_from_file_location("spec_anchor_guard", GUARD_PATH)
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)

# Qué se copia al árbol temporal. Lo minimo que las propiedades necesitan:
# las specs canonicas (para el indice y la procedencia) y las tablas de verdad
# (para las citas). `crates/` se copia entero porque las citas del codigo son
# la mayor parte del volumen, y sin ellas la propiedad 1 no tiene entrada.
COPY_TREES = [
    "docs/architecture/specs",
    "docs/history",
    "docs/roadmap",
    "crates",
]

# Las rutas del modulo que hay que repuntar al arbol temporal.
PATHS_TO_REPOINT = ["ROOT", "SPEC_DIR", "HISTORY_DIR", "TRUTH_TABLES"]


def run() -> tuple[int, str]:
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        code = guard.main()
    return code, buf.getvalue()


def with_sandbox(mutate) -> tuple[int, str]:
    """Copia el arbol, aplica `mutate`, ejecuta el guard ahi, y tira todo.

    ORDEN: las rutas del guard se repuntan ANTES de llamar a `mutate`, para que
    una mutacion que parchee el indice opere ya sobre el sandbox. Con el orden
    inverso, `spec_index` se parcheaba contra el arbol real y la mutacion se
    ejecutaba sobre el sandbox equivocado: el guard corria con el indice roto
    pero con rutas del repo, y no caia. Es la misma clase de fallo que el resto
    del dia -- una mutacion que no se aplica donde dice aplicarse, contada
    como si se hubiera ejercitado.
    """
    saved = {name: getattr(guard, name) for name in PATHS_TO_REPOINT}
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="spec-anchor-mutation-"))
    try:
        for tree in COPY_TREES:
            src = ROOT / tree
            if src.is_dir():
                dst = tmp / tree
                dst.parent.mkdir(parents=True, exist_ok=True)
                shutil.copytree(src, dst)
        sandbox_root = tmp
        guard.ROOT = sandbox_root
        guard.SPEC_DIR = sandbox_root / "docs" / "architecture" / "specs"
        guard.HISTORY_DIR = sandbox_root / "docs" / "history"
        guard.TRUTH_TABLES = [
            sandbox_root / "docs" / "roadmap" / "ACCEPTANCE-TRUTHFULNESS-MATRIX.md",
            sandbox_root / "docs" / "roadmap" / "UAT-MATRIX.md",
        ]
        # Una mutacion puede devolver un dict de PARCHES que se aplican DESPUES
        # de repuntar las rutas y ANTES de correr el guard. Existe por M6: su
        # rotura esta en la FUNCION del indice, no en el arbol, y un parche
        # aplicado antes del repunto opera contra el repo real.
        patches = mutate(sandbox_root)
        if patches:
            for name, value in patches.items():
                saved.setdefault(name, getattr(guard, name))
                setattr(guard, name, value)
        return run()
    finally:
        for name, value in saved.items():
            setattr(guard, name, value)
        shutil.rmtree(tmp, ignore_errors=True)


# ── M1: una spec canonica pierde su procedencia ──────────────────────────────
# Por que debe caer: propiedad 2. Una spec canonica que no dice de que paquete
# absorbio deja la ambiguedad de su ID sin quien la resuelva. Se borra el
# frontmatter entero, que es la rotura mas fuerte posible de esa propiedad.
def m1(sandbox: pathlib.Path):
    target = sandbox / "docs/architecture/specs/arch-spec-043-generic-verify.md"
    text = target.read_text(encoding="utf-8")
    # Se quita el bloque de frontmatter entero: la rotura mas fuerte posible
    # de la propiedad 2.
    stripped = re.sub(r"\A---\n.*?\n---\n", "", text, flags=re.S)
    assert stripped != text, "M1 no aplico: el fichero no tiene frontmatter"
    target.write_text(stripped, encoding="utf-8")
    return None


# ── M2: una fila de verdad cita un SPEC-NNN sin ancla ────────────────────────
# Por que debe caer: propiedad 3 en su superficie de veredicto. Se inyecta en
# la tabla una fila que cita `SPEC-012` a pelo, que es exactamente el defecto
# que motiva el guard.
def m2(sandbox: pathlib.Path):
    target = sandbox / "docs/roadmap/ACCEPTANCE-TRUTHFULNESS-MATRIX.md"
    text = target.read_text(encoding="utf-8")
    line = "| M2ROW | spec ambigua | SPEC-012 | x | x | x | x | x |\n"
    anchor = text.index("\n", text.index("| R1 |")) + 1
    target.write_text(text[:anchor] + line + text[anchor:], encoding="utf-8")
    return None


# ── M3: el codigo cita una spec que no existe ───────────────────────────────
# Por que debe caer: propiedad 1. Se cita `SPEC-999` desde un fuente de
# `crates/`, que no tiene canonico. Es el caso `INC-DEBT-048` del que habla el
# guard de citas, en la superficie donde ocurre de verdad.
def m3(sandbox: pathlib.Path):
    target = sandbox / "crates/sddk-domain/src/staleness.rs"
    target.write_text(
        target.read_text(encoding="utf-8") + "\n// implements SPEC-999\n",
        encoding="utf-8",
    )
    return None


# ── M4: la cita ambigua pierde su ancla en una fila de verdad ───────────────
# Por que debe caer: propiedad 3 por la via de la ANCLA. Se quita el ancla de
# una fila que hoy la tiene (`SPEC-004 -> arch-spec-004-decision-memory`) y se
# deja el ID a pelo. Es la misma propiedad que M2 pero por la operacion
# contraria: M2 anade una cita nueva, M4 deja de anclar una existente, y hace
# falta que las dos caigan para que se vea que el guard mide presencia Y
# separabilidad por separado.
#
# NOTA DE METODO, y la segunda mutacion mala que este fichero produce: la
# primera version renombraba el fichero de la spec a `arch-spec-912-…`
# esperando que el ID quedara vacio de autoridad, y el guard siguio en verde.
# Motivo: el indice lee `package_local_id:` del FRONTMATTER, no el nombre, luego
# renombrar no cambia el ID. **Una mutacion que no toca la propiedad que dice
# medir no cae; no caer no la hace automaticamente mala, la hace mal
# elegida.** Y contarla como PASS habria dado al guard una comprobacion que no
# tiene -- que es el modo de verde falso mas caro que existe.
def m4(sandbox: pathlib.Path):
    target = sandbox / "docs/roadmap/ACCEPTANCE-TRUTHFULNESS-MATRIX.md"
    text = target.read_text(encoding="utf-8")
    before = text
    text = text.replace("SPEC-004 → `arch-spec-004-decision-memory`", "SPEC-004")
    assert text != before, "M4 no aplico: el ancla de SPEC-004 no esta donde se esperaba"
    target.write_text(text, encoding="utf-8")
    return None


# ── M5: la ambiguedad en FILA se degrada a prosa y el veredicto desaparece ──
# Por que se mide AL REVES: esta mutacion no puede exigir exit != 0, porque su
# efecto correcto es precisamente el contrario -- el guard debe SEGUIR en verde
# y REPORTAR la cita como aviso en vez de como fallo. Exigir que caiga seria
# exigir que el guard se rompa: un falso positivo disfrazado de prueba.
#
# Lo que si se exige es que el reparto cambie: la cita pasa de FILA a PROSA y el
# veredicto que la nombraba tiene que desaparecer del texto. **Un guard que no
# distingue una afirmacion de una descripcion de esa afirmacion trata las dos
# igual, y el precio lo paga quien intenta documentar el defecto sin que el
# guard le exija silenciar la evidencia que lo explica.**
def m5(sandbox: pathlib.Path):
    target = sandbox / "docs/roadmap/ACCEPTANCE-TRUTHFULNESS-MATRIX.md"
    text = target.read_text(encoding="utf-8")
    # Se SEMBRA la fila ambigua y luego se mueve a prosa, en el mismo sandbox.
    # Cada mutacion tiene el suyo, asi que no puede confiar en el estado que
    # M2 dejo: compartirlo haria que la quinta dependiera de la segunda, que
    # es justo la dependencia que este fichero existe para evitar.
    anchor = text.index("\n", text.index("| R1 |")) + 1
    seeded = text[:anchor] + "| M5ROW | ambigua en FILA | SPEC-012 | x | x | x | x | x |\n" + text[anchor:]
    assert "M5ROW" in seeded, "M5 no aplico: no se pudo sembrar la fila ambigua"
    final = seeded.replace("| M5ROW | ambigua en FILA | SPEC-012 | x | x | x | x | x |\n", "")
    final += "\nLa nota de prosa cita SPEC-012 sin anclar y describe el defecto.\n"
    target.write_text(final, encoding="utf-8")
    return None


# ── M6: el indice deja de derivar el id del nombre del fichero ──────────────
# Por que debe caer: es la rotura que, durante la construcción, produjo **35
# falsos positivos** de `SPEC-043` huerfano con 35 citas. Sin la derivacion
# `arch-spec-NNN-`, las specs nativas desaparecen del indice. Es la mutacion
# mas importante del fichero: **falsifica el modo de fallo que casi me hace
# publicar un defecto inexistente.**
def m6(sandbox: pathlib.Path):
    # El indice se parchea para que lea SOLO el frontmatter y el nombre
    # `SPEC-NNN-` literal, sin derivar `arch-spec-NNN-` del nombre del fichero.
    # Como `with_sandbox` repunta las rutas ANTES de aplicar este parche, opera
    # sobre el sandbox correcto.
    def broken():
        index: dict[str, list[str]] = {}
        for path in sorted(guard.SPEC_DIR.glob("*.md")):
            text = path.read_text(encoding="utf-8", errors="replace")
            local = None
            for key in ("package_local_id", "id"):
                m = re.search(rf"^{key}:\s*(SPEC-\d+)\b", text, re.M)
                if m:
                    local = m.group(1)
                    break
            if local is None:
                m = re.match(r"SPEC-(\d+)", path.name)
                if m:
                    local = f"SPEC-{m.group(1)}"
            if local:
                index.setdefault(local, []).append(path.relative_to(guard.ROOT).as_posix())
        return index

    # Se devuelve como parche, no se aplica aqui: un parche aplicado antes del
    # repunto de rutas opera contra el repo real y el guard nunca ve el indice
    # roto. Fue el fallo de la primera version de esta mutacion.
    return {"spec_index": broken}


MUTATIONS = [
    ("M1", "una spec canonica pierde su procedencia (propiedad 2)", m1, "NO declara de donde viene"),
    ("M2", "una fila cita SPEC-012 sin ancla (propiedad 3, veredicto)", m2, "AMBIGUA"),
    ("M3", "el codigo cita SPEC-999, que no existe (propiedad 1)", m3, "NO tiene documento"),
    ("M4", "una fila pierde el ancla que hacia separables su cita", m4, "AMBIGUA"),
    ("M5", "la ambiguedad pasa de fila a prosa: el veredicto debe DESAPARECER", m5, "PROSA"),
    ("M6", "el indice deja de derivar el id del nombre (35 falsos positivos de SPEC-043)", m6, "NO tiene documento"),
]

# M5 se mide al reves que las demas: su efecto correcto es que el guard SIGA en
# verde y reporte la cita como AVISO. Exigirle exit != 0 seria exigir que se
# rompa. Se declara aqui su expectativa para que la diferencia sea explicita y
# no un accidente del bucle.
EXPECT_GREEN = {"M5"}

print("== autofalsacion: test_spec_citation_anchor.py ==")
base_code, _ = run()
print(f"  base: exit {base_code} (se exige 0)")

if base_code != 0:
    print()
    print("RESULT: FAIL — el guard no esta verde en la base. Una autofalsacion de un")
    print("         guard que ya falla no mide nada: solo contaria ruido.")
    sys.exit(1)

passed = 0
failed: list[str] = []

for name, why, mutate, expect in MUTATIONS:
    code, out = with_sandbox(mutate)
    fails = [l for l in out.splitlines() if l.strip().startswith("[FAIL]")]
    avisos = [l for l in out.splitlines() if l.strip().startswith("[aviso]")]

    if name in EXPECT_GREEN:
        # El veredicto de esta cita debe ser AVISO, nunca FAIL.
        if any("M5ROW" in f for f in fails) or any("FILA" in f and "SPEC-012" in f for f in fails):
            failed.append(f"{name} reporta la prosa como fallo: {why}")
            print(f"  [FAIL] {name}: una cita en prosa produjo FAIL. Debe ser aviso.")
        elif not any("SPEC-012" in a for a in avisos):
            failed.append(f"{name} dejo de reportar la cita: {why}")
            print(f"  [FAIL] {name}: la cita desaparecio del reporte en vez de pasar a aviso.")
        else:
            passed += 1
            print(f"  [ok]   {name}: la cita paso de veredicto a aviso y el guard sigue verde")
            for a in avisos[:1]:
                print(f"           {a.strip()[:110]}")
        continue

    if code == 0:
        failed.append(f"{name} NO CAE: {why}")
        print(f"  [FAIL] {name}: el guard siguio en verde. {why}")
    elif not any(expect in l for l in fails):
        failed.append(f"{name} cae pero por larazon equivocada: {why}")
        print(f"  [FAIL] {name}: cayo, pero ningun veredicto nombra «{expect}».")
        for f in fails[:2]:
            print(f"           {f.strip()[:110]}")
    else:
        passed += 1
        print(f"  [ok]   {name} cae: {why}")
        for f in fails[:1]:
            print(f"           {f.strip()[:110]}")

# El repo real tiene que seguir verde: nada se toco.
final_code, _ = run()
if final_code != 0:
    failed.append("el repo real quedo modificado por el falsificador")
    print("  [FAIL] el guard real NO sigue verde: el falsificador toco el repo")
else:
    print("  [ok]   el repo real sigue verde: la mutacion vivio en la copia y se tiro")

print()
print(f"  PASS={passed} FAIL={len(failed)} SKIP=0")
if failed:
    print()
    for f in failed:
        print(f"  [FAIL] {f}")
    print()
    print("RESULT: FAIL — una comprobacion del guard no tiene dientes, o cayo por la")
    print("         razon equivocada. Un guard que no cae no vigila, y uno que cae")
    print("         por casualidad no distingue el defecto que importa.")
    sys.exit(1)

print()
print("RESULT: PASS — cada propiedad cae por separado, por su propia razon, y nada")
print("         del repo real se toco.")
sys.exit(0)
