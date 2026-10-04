#!/usr/bin/env python3
"""test_grep_q_after_pipe_mutation.py — autofalsacion de
`test_grep_q_after_pipe.py`.

POR QUE EXISTE
--------------
Un guard que nunca ha caido no es un guard: es una asercion. Este fichero
**siembra el defecto, ejecuta el guard importado, exige la caida, y tira**.

No se copia el guard: se IMPORTA, y se le repunta `ROOT` a un sandbox con los
`.sh` del repo. Una copia del codigo no vigila el codigo.

LA REGLA QUE IMPORTA Y QUE ESTE FICHERO SE IMPONE A SI MISMO
------------------------------------------------------------
Cada mutacion cae por SU PROPIA comprobacion, y una mutacion que no se aplica
es `SKIP`, nunca `PASS`. Contar como PASS una mutacion que no llego a aplicarse
daria al guard una comprobacion que no tiene, que es el modo de verde falso mas
caro que existe.

Y hay una segunda regla, mas dura, para las mutaciones que deben dar VERDE —
las que comprueban que una excepcion del guard se sostiene:

    una mutacion "espero verde" NO se acepta por si sola. Se corre DOS veces:
    con el guard real, que debe decir verde, y con una variante a la que se le
    ha quitado ESA excepcion, que debe decir rojo. Si la variante tampoco cae,
    la fixture no contenia el constructo que dice contener, la mutacion no
    midio nada, y su resultado es SKIP.

    Sin ese segundo pase, "el guard dice verde" y "el guard no mira nada" son
    la misma observacion, y el falsador no puede distinguirlas.

REGLAS DE LA MUTACION
---------------------
Se copia solo lo que el guard lee (`*.sh` bajo tests/, scripts/, githooks/). El
sandbox es desechable: la mutacion vive en una copia que se tira, luego no hay
restauracion que pueda fallar. Esa es la diferencia entre "restaure todo" y
"nunca toco el repo".

Salida: exit 0 si cada mutacion aplico y cayo como debia; 1 si no.
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
GUARD_PATH = ROOT / "tests" / "test_grep_q_after_pipe.py"

spec = importlib.util.spec_from_file_location("grep_q_guard", GUARD_PATH)
guard = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guard)

COPY_TREES = ("tests", "scripts", "githooks")

PIPEFAIL_HEADER = "#!/bin/bash\nset -euo pipefail\n\n"

results: list[tuple[str, str, str]] = []  # (id, veredicto, detalle)


def run() -> tuple[int, str]:
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        code = guard.main()
    return code, buf.getvalue()


def with_sandbox(plant=None, patches=None):
    """Copia los `.sh` del repo, planta, ejecuta el guard ahi, y tira todo.

    ORDEN: las rutas del guard se repuntan ANTES de plantar, para que una
    mutacion que parchee el indice opere ya sobre el sandbox. Con el orden
    inverso una mutacion cae contra el arbol real y sobre el sandbox
    equivocado — la misma clase de fallo que hizo que la primera redaccion de
    `test_spec_citation_anchor_mutation.py` diera PASS=1 FAIL=5.

    RESTAURACION: se guardan y devuelven EXACTAMENTE los nombres que esta
    llamada parchea, sin filtrar por mayusculas. La primera version guardaba
    solo los UPPERCASE, de modo que un parche a `strip_comment` o a
    `mask_dollar` —que son minusculas— se FILTRABA a las mutaciones
    siguientes. MEDIDO: M5 media el efecto de M4, no el suyo, y dio
    `PASS=8 FAIL=1` con un fallo que no era del guard. Un falsador que
    transporta el defecto de la mutacion anterior entre casos no mide la
    mutacion que dice medir: es la version silenciosa de contar como PASS lo
    que no llego a ejercitarse.
    """
    patches = patches or {}
    saved = {name: guard.__dict__.get(name) for name in patches}
    saved_root = guard.ROOT
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="grep-q-mutation-"))
    try:
        for tree in COPY_TREES:
            src = ROOT / tree
            if not src.is_dir():
                continue
            for path in src.rglob("*.sh"):
                rel = path.relative_to(ROOT)
                dst = tmp / rel
                dst.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(path, dst)
        guard.ROOT = tmp
        for name, value in patches.items():
            guard.__dict__[name] = value
        if plant is not None:
            plant(tmp)
        return run()
    finally:
        guard.ROOT = saved_root
        for name, value in saved.items():
            if value is None:
                guard.__dict__.pop(name, None)
            else:
                guard.__dict__[name] = value
        shutil.rmtree(tmp, ignore_errors=True)


def record(mid: str, expect_red: bool, code: int, out: str, needle: str = "") -> None:
    """Veredicto de UNA mutacion. Regla: si no se aplico, SKIP."""
    if expect_red:
        if code != 0:
            detail = needle if needle in out else out.strip().splitlines()[-1][:90]
            results.append((mid, "PASS", f"cayo (exit {code}) — {detail}"))
        else:
            results.append((mid, "FAIL", "el guard SIGUO en verde con el defecto sembrado"))
    else:
        if code == 0:
            results.append((mid, "PASS", "sostiene el verde que se le pidio"))
        else:
            results.append((mid, "FAIL", "el guard cayo donde se le pidio verde"))


# ─────────────────────────────────────────────────────────────────────────────
# M1 — un sitio peligroso sembrado en un fichero nuevo.
# Por que debe caer: la propiedad central. Sin esto, el guard podria estar
# verde por no mirar nada en absoluto.
def m1_plant(root: pathlib.Path):
    (root / "tests" / "zz_m1_planted.sh").write_text(
        PIPEFAIL_HEADER
        + 'if sed -n "1,200p" "$F" | grep -q "aguja"; then\n'
        + "    echo ok\n"
        + "fi\n",
        encoding="utf-8",
    )


# ─────────────────────────────────────────────────────────────────────────────
# M2 — escritor BUILTIN. Debe seguir VERDE: un builtin no recibe SIGPIPE.
# Y se prueba con la excepcion retirada, que debe CAER.
def m2_plant(root: pathlib.Path):
    (root / "tests" / "zz_m2_builtin.sh").write_text(
        PIPEFAIL_HEADER
        + 'if printf "%s" "$X" | grep -q "aguja"; then\n'
        + "    echo ok\n"
        + "fi\n",
        encoding="utf-8",
    )


def _dangerous_ignoring_builtin(site):
    return site.consumed and (not site.in_cmdsub)


def m2_neutered() -> tuple[int, str]:
    original = guard.Site.dangerous
    guard.Site.dangerous = property(_dangerous_ignoring_builtin)
    try:
        return with_sandbox(plant=m2_plant)
    finally:
        guard.Site.dangerous = original


# ─────────────────────────────────────────────────────────────────────────────
# M3 — `||` NO es una tuberia. Debe seguir VERDE.
def m3_plant(root: pathlib.Path):
    (root / "tests" / "zz_m3_or.sh").write_text(
        PIPEFAIL_HEADER
        + 'if grep -qE "a" "$F" || grep -qE "b" "$F"; then\n'
        + "    echo ok\n"
        + "fi\n",
        encoding="utf-8",
    )


def m3_neutered() -> tuple[int, str]:
    """Sin el lookbehind, el segundo `|` de `||` casa como tuberia."""
    original = guard.RE_PIPE_Q
    guard.RE_PIPE_Q = re.compile(r"\|\s*(?:e?grep|rg)\s+((?:-[A-Za-z]+\s+)*-q[A-Za-z]*)\b")
    try:
        return with_sandbox(plant=m3_plant)
    finally:
        guard.RE_PIPE_Q = original


# ─────────────────────────────────────────────────────────────────────────────
# M4 — un COMENTARIO que describe el defecto no es un sitio.
# Con `strip_comment` neutralizado debe CAER: eso prueba que el recorte es
#Portador de la propiedad y no decoracion.
def m4_plant(root: pathlib.Path):
    (root / "tests" / "zz_m4_comment.sh").write_text(
        PIPEFAIL_HEADER
        + "# ojo: no escribas `cmd | grep -q needle` aqui\n"
        + 'if grep -qE "real" "$F"; then\n'
        + "    echo ok\n"
        + "fi\n",
        encoding="utf-8",
    )


def m4_neutered() -> tuple[int, str]:
    return with_sandbox(plant=m4_plant, patches={"strip_comment": lambda line: line})


# ─────────────────────────────────────────────────────────────────────────────
# M5 — `$(( ... ))` NO es una sustitucion de comandos.
#
# Esta es la que mas importa, porque su fallo NO hacia ruido: hacia que el
# guard se callara. Con la rama `$((` debajo de la de `$(` — el orden que
# tenia la primera redaccion — el `cmdsub` se marca, el sitio queda exento, y
# el guard dice verde sobre un defecto real. Se mide AL REVES: el guard con el
# bug debe FALLAR a encontrarlo (green) mientras el correcto lo ve.
def m5_plant(root: pathlib.Path):
    (root / "tests" / "zz_m5_arith.sh").write_text(
        PIPEFAIL_HEADER
        + 'if ! sed -n "${A},$((B - 1))p" "$F" | grep -q "aguja"; then\n'
        + "    echo fail\n"
        + "    exit 1\n"
        + "fi\n",
        encoding="utf-8",
    )


def m5_neutered() -> tuple[int, str]:
    """Reproduce el bug de orden: `$((` se captura como `$(`."""
    original = guard.mask_dollar
    buggy_src = (
        "def mask_dollar(text):\n"
        "    out = list(text)\n"
        "    kinds = set()\n"
        "    i, n = 0, len(text)\n"
        "    while i < n:\n"
        "        two = text[i:i+2]\n"
        "        if two == '${':\n"
        "            kind, opener, closer = 'params', '{', '}'\n"
        "        elif two == '$(':            # BUG: captura `$((` como cmdsub\n"
        "            kind, opener, closer = 'cmdsub', '(', ')'\n"
        "        else:\n"
        "            i += 1\n"
        "            continue\n"
        "        depth, j = 0, i + 1\n"
        "        while j < n:\n"
        "            if text[j] == opener: depth += 1\n"
        "            elif text[j] == closer:\n"
        "                depth -= 1\n"
        "                if depth == 0: break\n"
        "            j += 1\n"
        "        if kind == 'cmdsub':\n"
        "            kinds.add('cmdsub')\n"
        "        else:\n"
        "            for k in range(i, min(j+1, n)):\n"
        "                if out[k] != '\\n': out[k] = '\\x00'\n"
        "        i = j + 1\n"
        "    return ''.join(out), kinds\n"
    )
    ns: dict = {}
    exec(compile(buggy_src, "<m5-buggy-mask_dollar>", "exec"), ns)
    return with_sandbox(plant=m5_plant, patches={"mask_dollar": ns["mask_dollar"]})


# ─────────────────────────────────────────────────────────────────────────────
# M6 — el suelo de NO-VACUIDAD. Con MIN_SITES por las nubes debe CAER aunque
# el arbol este limpio: es la unica defensa contra "el parser dejo de casar y
# eso se lee como arbol limpio".
def m6() -> tuple[int, str]:
    return with_sandbox(patches={"MIN_SITES": 10_000})


# ─────────────────────────────────────────────────────────────────────────────
# M7 — `set -euo pipefail` DETECTA errexit. Un pipeline desnudo (sin `if`) cuyo
# rc se consume porque el fichero aborta en el primer fallo sigue siendo
# peligroso. La primera redaccion de `has_errexit` exigia una 'e' o una 'r'
# AISLADAS y por eso declaraba inocuos ficheros que abortan: marco 2
# peligros sobre 91 cuando la clase entera daria 21. Con el detector roto este
# sitio no se ve.
def m7_plant(root: pathlib.Path):
    (root / "tests" / "zz_m7_errexit.sh").write_text(
        PIPEFAIL_HEADER
        + "sed -n '1,200p' \"$F\" | grep -q \"aguja\"\n"
        + 'echo "sigue vivo"\n',
        encoding="utf-8",
    )


def m7_neutered() -> tuple[int, str]:
    return with_sandbox(
        plant=m7_plant,
        patches={"has_errexit": lambda text: False},
    )


# ─────────────────────────────────────────────────────────────────────────────
# CONTROL DEL FALSADOR — M8: el guard neutralizado tiene que SER DETECTADO
# como vacio. Si el falsador no puede distinguir "el guard dice verde porque
# todo esta bien" de "el guard dice verde porque no le pasan nada que mirar",
# no puede afirmar nada de las mutaciones anteriores.
def m8_control() -> tuple[int, str]:
    return with_sandbox(plant=m1_plant, patches={"scan_tree": lambda root: ([], [])})


def main() -> int:
    results.clear()

    # BASE: el sandbox sin sembrar nada tiene que estar verde. Si el sandbox
    # base ya cae, todas las mutaciones de abajo serian ruido.
    base_code, base_out = with_sandbox()
    if base_code != 0:
        print("[FAIL] BASE: el sandbox sin sembrar ya cae; las mutaciones no prueban nada")
        print(base_out)
        return 1
    results.append(("BASE", "PASS", "sandbox limpio en verde"))

    c, o = with_sandbox(plant=m1_plant)
    record("M1 sitio peligroso sembrado", True, c, o, "zz_m1_planted.sh")

    # M2: verde con el guard real, ROJO con la excepcion retirada. El rojo es
    # lo que prueba que la fixture contenia el constructo.
    c, o = with_sandbox(plant=m2_plant)
    record("M2 escritor builtin se exime", False, c, o)
    nc, no = m2_neutered()
    if nc == 0:
        results[-1] = ("M2 escritor builtin se exime", "SKIP",
                       "la variante sin excepcion tampoco cae: la fixture no contenia el builtin")
    elif c == 0:
        results[-1] = ("M2 escritor builtin se exime", "PASS",
                       f"verde con el guard real y CAE ({nc}) sin la excepcion: la excepcion sostiene")
    else:
        results[-1] = ("M2 escritor builtin se exime", "FAIL",
                       f"cayo con el guard real (exit {c})")

    c, o = with_sandbox(plant=m3_plant)
    record("M3 || no es tuberia", False, c, o)
    nc, no = m3_neutered()
    if nc == 0:
        results[-1] = ("M3 || no es tuberia", "SKIP",
                       "sin el lookbehind tampoco cae: la fixture no contenia un || real")
    elif c == 0:
        results[-1] = ("M3 || no es tuberia", "PASS",
                       f"verde con el lookbehind y CAE ({nc}) sin el: el lookbehind sostiene")
    else:
        results[-1] = ("M3 || no es tuberia", "FAIL", f"cayo con el guard real (exit {c})")

    c, o = m4_neutered()
    record("M4 comentario contado con strip_comment roto", True, c, o, "zz_m4_comment.sh")

    c, o = with_sandbox(plant=m5_plant)
    record("M5 el guard ve el sitio con $(( ))", True, c, o, "zz_m5_arith.sh")
    nc, no = m5_neutered()
    if nc != 0:
        results[-1] = ("M5 el guard ve el sitio con $(( ))", "FAIL",
                       f"el bug de orden tambien lo ve ({nc}); la propiedad no se sostiene")
    elif c != 0:
        results[-1] = ("M5 el guard ve el sitio con $(( ))", "PASS",
                       "el guard correcto lo ve y el de orden rota NO: el orden es lo que decidia")
    else:
        results[-1] = ("M5 el guard ve el sitio con $(( ))", "FAIL",
                       "el guard correcto tampoco lo ve")

    c, o = m6()
    record("M6 el suelo de no-vacuedad cae sin sitios", True, c, o, "no-vacuedad")

    c, o = with_sandbox(plant=m7_plant)
    record("M7 el guard ve el pipeline desnudo bajo set -e", True, c, o, "zz_m7_errexit.sh")
    nc, no = m7_neutered()
    if nc != 0:
        results[-1] = ("M7 el guard ve el pipeline desnudo bajo set -e", "FAIL",
                       f"con has_errexit roto tambien lo ve ({nc})")
    elif c != 0:
        results[-1] = ("M7 el guard ve el pipeline desnudo bajo set -e", "PASS",
                       "lo ve con errexit bien detectado y no lo ve sin: el detector sostiene")
    else:
        results[-1] = ("M7 el guard ve el pipeline desnudo bajo set -e", "FAIL",
                       "tampoco con errexit bien detectado")

    c, o = m8_control()
    if c != 0:
        results.append(("M8 CONTROL: sandbox sin nada que mirar cae", "PASS",
                        f"el guard detecta el escaneo vacio (exit {c})"))
    else:
        results.append(("M8 CONTROL: sandbox sin nada que mirar cae", "FAIL",
                        "con el escaneo vacio el guard dice verde: el falsador no puede "
                        "distinguir 'todo bien' de 'no mira'"))

    passed = sum(1 for _, v, _ in results if v == "PASS")
    failed = sum(1 for _, v, _ in results if v == "FAIL")
    skipped = sum(1 for _, v, _ in results if v == "SKIP")
    print("grep-q+pipefail: autofalsacion del guard")
    print("=" * 70)
    for mid, verdict, detail in results:
        print(f"  [{verdict:<4}] {mid}")
        print(f"         {detail}")
    print("-" * 70)
    print(f"PASS={passed} FAIL={failed} SKIP={skipped}")
    if failed:
        print("\nRESULT: FAIL")
        return 1
    if skipped:
        print("\nRESULT: FAIL — una mutacion no se aplico; contarla como PASS seria mentir")
        return 1
    print("\nRESULT: PASS — cada mutacion aplico y cayo por su propia comprobacion")
    return 0


if __name__ == "__main__":
    sys.exit(main())
