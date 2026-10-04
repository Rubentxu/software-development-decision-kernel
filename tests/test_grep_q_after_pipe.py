#!/usr/bin/env python3
"""test_grep_q_after_pipe.py — ninguna tuberia puede decidir un veredicto por
accidente (cierra INC-DEBT-071).

EL DEFECTO
----------
Con `set -o pipefail`, si el LECTOR de una tuberia sale pronto, el pipeline
devuelve el fallo del ESCRITOR:

    set -o pipefail
    sed -n '527,706p' release.sh | grep -q 'git merge-base --is-ancestor'

`grep -q` sale en cuanto casa. Si al escritor le queda trabajo, recibe SIGPIPE
y muere con 141, y `pipefail` propaga ESE codigo. Un `if !` entra entonces por
la rama de "no lo encontre" aunque lo haya encontrado.

MEDIDO (session-81), con la slice real de 8 471 B y 180 lineas:

    sin carga ..............  2,10 %  (42 de 2 000)
    con 32 procesos spin ...  9,17 %  (55 de 600)
    el guard entero ........  3 caidas de 40 con carga

Un release compila con `cargo`, o sea con carga. Por eso el release 2.9.0
murio en el 1b con un aserto que, aislado, daba 5 de 5.

LO QUE ESTE GUARD MIDE, Y POR QUE NO ES "CUANTOS HAY"
-----------------------------------------------------
`grep -rlE '\\|[[:space:]]*grep[^|]*-q'` cuenta 91 sitios en 32 ficheros. **No
son 91 defectos.** Un sitio es PELIGROSO solo si se cumplen las cuatro:

  1. el fichero activa `pipefail`
  2. hay una tuberia real `<escritor> | grep -q ...`
  3. `<escritor>` es EXTERNO — un builtin de bash no recibe SIGPIPE
  4. el rc del pipeline se CONSUME: abre un `if`/`while`, encadena con
     `&&`/`||`, se niega con `!`, o queda desnudo bajo `set -e`

MEDIDO: de los 91 sitios, **22 son peligrosos**, en 14 ficheros. Los otros 69
son estructuralmente inmunes y quedan EXENTOS, y el guard imprime el reparto
para que la cuenta se pueda falsar a ojo en vez de creerla.

LO QUE ESTE GUARD MIDE, Y POR QUE EL TAMANO NO ENTRA
-----------------------------------------------------
La primera redaccion de INC-DEBT-071 proponia triar "por tamano de la entrada".
**Medido, es falso como criterio.** Poner la aguja AL FINAL de la slice —con el
escritor sin trabajo pendiente que leer— no la hace segura:

    needle al final, sin carga ....  1,50 %
    needle al final, con carga ....  8,33 %

Lo que decide no es el tamano de la entrada sino si al escritor le queda algo
por escribir cuando el lector cierra, y eso no se lee del tamano. Por eso aqui
no hay lista de excepciones escrita a mano: la excepcion es la ESTRUCTURA
(escritor builtin, o rc descartado), y se imprime. Una lista escrita a mano es
la forma mas rapida de que la cuenta vuelva a mentir sin que nadie lo note.

Salida: exit 0 si no hay ningun sitio peligroso; 1 si hay alguno.
"""

from __future__ import annotations

import os
import pathlib
import re
import sys
from typing import NamedTuple

# El repo real por defecto; el falsador lo repunta a su sandbox.
ROOT = pathlib.Path(os.environ.get("GREP_Q_SCAN_ROOT", "")).resolve() \
    if os.environ.get("GREP_Q_SCAN_ROOT") \
    else pathlib.Path(__file__).resolve().parent.parent

SCAN_DIRS = ("tests", "scripts", "githooks")

# Suelo de NO-VACUIDAD. Medido sobre el arbol real: 91 sitios en 32 ficheros.
# Si el parser dejara de encontrar la clase —porque una mutacion rompa el
# enmascarado, o porque alguien borre el `| grep -q` sin querer— este numero
# cae y el guard FALLA, en vez de dar un verde que no midio nada.
MIN_SITES = 40

# Builtins de bash: no son procesos, no reciben SIGPIPE.
BUILTINS = frozenset(
    """
    printf echo read cd pwd test [ ]] true false declare local export let
    eval exec source . trap return break continue shift set unset wait jobs
    kill exit umask alias type command mapfile
    """.split()
)

CTRL_WORDS = frozenset(
    "if then else elif do done ! while until time".split()
)

# `| grep -q`, `| grep -qE`, `| grep -Eq`, `| rg -q` ...
#
# EL LOOKBEHIND NO ES COSMETICO. Sin `(?<!\|)`, el segundo `|` de un `||`
# casa como si fuera una tuberia: `a || grep -qE 'x' f` es un OR, no un
# pipeline, y el guard lo reportaba como sitio peligroso. MEDIDO: asi marcaba
# un sitio de test_release_admission.sh que no existe, y un guard que cuenta
# sitios que no existen es la forma mas corta de que la cuenta no cuadre con
# la realidad — que es justo lo que este guard viene a impedir.
# `|&` SI es tuberia y sigue casando: el `&` no es `|`.
RE_PIPE_Q = re.compile(r"(?<!\|)\|\s*(?:e?grep|rg)\s+((?:-[A-Za-z]+\s+)*-q[A-Za-z]*)\b")
RE_SET_PIPEFAIL = re.compile(r"^\s*set\s+(-\w+\s+)*.*\bpipefail\b", re.M)
RE_SET_SHORT = re.compile(r"^\s*set\s+(-\w+)", re.M)


class Site(NamedTuple):
    file: str
    line: int
    writer: str
    builtin: bool
    consumed: bool
    in_cmdsub: bool
    snippet: str

    @property
    def dangerous(self) -> bool:
        return (not self.builtin) and self.consumed and (not self.in_cmdsub)


# ── ENMASCARADO ─────────────────────────────────────────────────────────────
def mask_dollar(text: str) -> tuple[str, set[str]]:
    """Enmascara `${...}` y `$((...))` enteros; deja las tuberias de `$(...)`.

    Por que: el `|` de `${V#*|}` NO es una tuberia, y casarlo como tal produce
    sitios que no existen —un falso positivo que hace que la cuenta mienta.
    El interior de un `$( ... )` SI contiene tuberias reales y se conserva.
    """
    out = list(text)
    kinds: set[str] = set()
    i, n = 0, len(text)
    while i < n:
        two = text[i : i + 2]
        # ORDEN IMPORTA. `$((LINE_9 - 1))` empieza por `$(`, luego si se
        # comprueba `$(` antes que `((` la aritmetica se toma por
        # sustitucion de comandos, se marca `cmdsub`, y CADA sitio con
        # aritmetica queda exculpado. La primera redaccion tenia la rama de
        # `$((` como `elif`, debajo de un `if ... in ("${", "$(")`: codigo
        # muerto. Medido: 5 sitios de test_vault_mirror_auto.sh salian
        # exentos por eso, y el guard reporting 17 en vez de 22.
        if two == "${":
            kind, opener, closer = "params", "{", "}"
        elif text[i : i + 3] == "$((":
            kind, opener, closer = "arith", "(", ")"
        elif two == "$(":
            kind, opener, closer = "cmdsub", "(", ")"
        else:
            i += 1
            continue
        depth, j = 0, i + 1
        while j < n:
            if text[j] == opener:
                depth += 1
            elif text[j] == closer:
                depth -= 1
                if depth == 0:
                    break
            j += 1
        if kind == "cmdsub":
            # el interior de un `$( ... )` SI contiene tuberias reales
            kinds.add("cmdsub")
        else:
            for k in range(i, min(j + 1, n)):
                if out[k] != "\n":
                    out[k] = "\x00"
        i = j + 1
    return "".join(out), kinds


def strip_comment(line: str) -> str:
    """Quita el comentario respetando comillas simples y dobles.

    Por que: dos comentarios que DESCRIBEN este defecto vivian en
    `test_release_diagnostics_wiring.sh`, y una version anterior de este
    clasificador los contaba como sitios. Un guard que cuenta su propia
    documentacion no esta midiendo el producto.
    """
    out: list[str] = []
    i, n, sq, dq = 0, len(line), False, False
    while i < n:
        ch = line[i]
        if ch == "'" and not dq:
            sq = not sq
        elif ch == '"' and not sq:
            dq = not dq
        elif ch == "\\" and not sq:
            out.append(ch)
            if i + 1 < n:
                out.append(line[i + 1])
            i += 2
            continue
        elif ch == "#" and not sq and not dq:
            if i == 0 or line[i - 1] in " \t":
                break
        out.append(ch)
        i += 1
    return "".join(out)


# ── LINEAS LOGICAS ───────────────────────────────────────────────────────────
def logical_lines(text: str) -> list[tuple[int, str]]:
    """Une continuaciones de linea. Sin esto, la mitad de los sitios reales se
    escapan, porque la forma del repo es partir el pipe en su propia linea."""
    out: list[tuple[int, str]] = []
    buf, start = "", None
    for i, raw in enumerate(text.splitlines(), 1):
        if start is None:
            start = i
        s = raw.rstrip()
        if s.endswith("\\"):
            buf += s[:-1] + " "
            continue
        buf += raw
        out.append((start, buf))
        buf, start = "", None
    if start is not None:
        out.append((start, buf))
    return out


# ── AUTOR DE LA TUBERIA ──────────────────────────────────────────────────────
def head_command(head: str) -> str:
    """Primer token del ULTIMO comando antes del pipe.

    Dos errores medidos en este detector, ambos por tomar el token equivocado:
      - tomar el ULTIMO token (el argumento) dio "0 builtins" sobre 98 sitios.
      - no enmascarar literales dio `'.State.Running}}'` como autor de un
        `podman inspect`. La clasificacion acertaba y la etiqueta mentia, y un
        guard que miente en la etiqueta no es citable.
    """
    h = re.sub(r"'[^']*'", " '\x01' ", head)
    h = re.sub(r'"[^"]*"', ' "\x01" ', h)
    opens: list[int] = []
    for idx, ch in enumerate(h):
        if ch == "(":
            opens.append(idx)
        elif ch == ")" and opens:
            opens.pop()
    if opens:
        h = h[opens[-1] + 1 :]
    for sep in ("||", "&&", ";", "{"):
        k = h.rfind(sep)
        if k != -1:
            h = h[k + len(sep) :]
    for tok in h.split():
        if tok in CTRL_WORDS:
            continue
        return tok
    return ""


# ── SE CONSUME EL RC ────────────────────────────────────────────────────────
def rc_consumed(logical: str) -> bool:
    """El rc del pipeline decide algo.

    En `if ! sed ... | grep -q X; then` el `if` abre la LINEA, no la tuberia.
    Exigir que `if` estuviera pegado al pipe —que fue la primera redaccion—
    tablero el caso real que motivo esta deuda y marco 2 peligros sobre 91.
    """
    m = RE_PIPE_Q.search(logical)
    if not m:
        return False
    before = logical[: m.start()]
    if re.match(r"\s*(if|while|until|elif)\b", before):
        return True
    if re.search(r"(&&|\|\|)\s*$", before):
        return True
    if re.search(r"!\s*$", before):
        return True
    return logical.rstrip().endswith(("&&", "||"))


def has_errexit(text: str) -> bool:
    """`set -euo pipefail` activa errexit.

    La primera redaccion exigia una 'e' o una 'r' AISLADAS y nunca las
    encontraba: declaraba inocuos ficheros que abortan en el primer fallo."""
    return any("e" in m.group(1)[1:] for m in RE_SET_SHORT.finditer(text))


def scan_file(path: pathlib.Path, rel: str) -> list[Site]:
    text = path.read_text(encoding="utf-8", errors="replace")
    if not RE_SET_PIPEFAIL.search(text):
        return []
    errexit = has_errexit(text)
    sites: list[Site] = []
    for lineno, logical in logical_lines(text):
        code = strip_comment(logical)
        if not code.strip():
            continue
        masked, kinds = mask_dollar(code)
        m = RE_PIPE_Q.search(masked)
        if not m:
            continue
        writer = head_command(masked[: m.start()])
        if not writer:
            continue
        sites.append(
            Site(
                file=rel,
                line=lineno,
                writer=writer,
                builtin=writer in BUILTINS,
                consumed=rc_consumed(code) or errexit,
                in_cmdsub="cmdsub" in kinds,
                snippet=code.strip()[:150],
            )
        )
    return sites


def scan_tree(root: pathlib.Path) -> tuple[list[Site], list[str]]:
    sites: list[Site] = []
    with_class: list[str] = []
    for top in SCAN_DIRS:
        base = root / top
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.sh")):
            rel = str(path.relative_to(root))
            found = scan_file(path, rel)
            if found:
                with_class.append(rel)
                sites.extend(found)
    return sites, with_class


# ── EL GUARD ────────────────────────────────────────────────────────────────
def main() -> int:
    sites, files = scan_tree(ROOT)
    dangerous = [s for s in sites if s.dangerous]
    exempt = [s for s in sites if not s.dangerous]
    failures: list[str] = []

    print("grep-q tras tuberia con pipefail (INC-DEBT-071)")
    print("=" * 70)
    print(f"  raiz escaneada ................. {ROOT}")
    print(f"  ficheros con pipefail + clase ... {len(files)}")
    print(f"  sitios de la clase ............. {len(sites)}")
    print(f"  EXENTOS (builtin / rc no consumido) {len(exempt)}")
    print(f"  PELIGROSOS ..................... {len(dangerous)}")

    if dangerous:
        print("\n  sitios PELIGROSOS (escritor externo con rc consumido):")
        for s in dangerous:
            print(f"\n    {s.file}:{s.line}  escritor={s.writer}")
            print(f"        {s.snippet}")
        failures.append(f"{len(dangerous)} sitios peligrosos de la clase grep-q+pipefail")

    # --- no-vacuedad: un parser que no encuentra nada no es un verde ---
    if len(sites) < MIN_SITES:
        failures.append(
            f"no-vacuedad: solo {len(sites)} sitios de la clase, el suelo es "
            f"{MIN_SITES}. Un parser que deja de casar NO es un arbol limpio."
        )
    if not files:
        failures.append(
            "no-vacuedad: ningun fichero con pipefail y la clase. El escaneo no "
            "llego a leerse, o el patron se rompio."
        )
    # --- el reparto tiene que cuadrar, o una de las dos cuentas miente ---
    if len(sites) != len(dangerous) + len(exempt):
        failures.append(
            f"reparto inconsistente: {len(sites)} sitios no son "
            f"{len(dangerous)} peligrosos + {len(exempt)} exentos"
        )

    print()
    for f in failures:
        print(f"[FAIL] {f}")
    if failures:
        print(f"\nRESULT: FAIL — {len(failures)} comprobacion(es) fallida(s)")
        return 1
    print("RESULT: PASS — ninguna tuberia decide un veredicto por accidente")
    return 0


if __name__ == "__main__":
    sys.exit(main())
