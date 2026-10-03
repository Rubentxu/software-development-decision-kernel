#!/usr/bin/env python3
"""test_docs_script_contamination.py — prosa en español con otro sistema de escritura pegado.

POR QUÉ EXISTE (session-66)
---------------------------
Trece documentos de `docs/` tienen caracteres CJK, cirílicos o de reemplazo
ocupando exactamente la posición de una palabra española. Se detectó
midiendo; lo que faltaba era que **nada lo impidiera**. Es la misma clase que
`test_uat_authority_citations.py` (una cita que no resuelve no gobierna nada) y
que `test_gate_coverage.py` (un invariante que nadie ejecuta no es un
invariante): un defecto que no tiene guard vuelve.

POR QUÉ UNA ALLOWLIST Y NO UN FAIL SECO
---------------------------------------
INC-DEBT-057 documenta por qué la corrupción vieja **no se puede corregir
automáticamente**: la palabra original no es reconstruible con fiabilidad.
Corregir a ciegas sería fabricar texto normativo, y varios de esos ficheros son
ADRs `accepted`. Así que el preexistente se declara uno a uno, con su motivo, y
lo que este guard protege es lo que viene **después**: la corrupción nueva es un
FAIL, no una nota más en un informe que nadie lee.

Dos reglas impiden que la allowlist se pudra, las mesmas que en
`test_gate_coverage.py`:

1. Una entrada que apunta a una línea que ya no coincide es un FAIL. Sin esto,
   un `git mv` o una reescritura dejarían la allowlist apuntando al vacío y
   la exclusión seguiría valiendo para un texto que ya no es el que se revisó.
2. Una entrada cuyo fichero ya no contiene el carácter es un FAIL. Sin esto,
   borrar la corrupción del fichero y olvidar quitar la entrada dejaría una
   exclusión permanente de algo que ya está bien.

Y una tercera, que es la que hace útil la allowlist: **el recuento tiene que
salir del contenido, no de una constante escrita a mano**. Un guard que dice
"27" porque alguien lo escribió se queda callado cuando aparecen 28.

LO QUE NO COMPRUEBA
-------------------
No sabe si un texto en japonés, coreano o ruso dentro de `docs/` es legítimo.
No hay ninguno hoy — los paquetes de `docs/history/` están en inglés y español
— pero un guard que corrige por patrón y no por contenido es exactamente la
fuente de los 13 que ya existen. Éste reporta y bloquea lo nuevo; no reescribe.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "docs"

# Bloques no latinos que no tienen nada que hacer en la prosa de este repo.
# U+FFFD (replacement char) se incluye a proposito: un U+FFFD en un .md es
# una perdida de datos, no un signo de puntuacion.
CONTAMINATION = re.compile(
    "["
    "Ѐ-ӿ"      # cirílico
    "؀-ۿ"      # árabe
    "ऀ-෿"      # indio
    "฀-๿"      # tailandés
    "ᄀ-ᇿ"      # hangul jamo
    "㄰-㆏"      # hangul compat
    "぀-ヿ"      # hiragana/katakana
    "㐀-䶿"      # Ext-A
    "一-鿿"      # CJK ideographs
    "가-힯"      # hangul syllables
    "豈-﫿"      # CJK compatibility
    "ｦ-ﾟ"      # halfwidth katakana
    "�"         # replacement char
    "]"
)

# `docs/history/` es inmutable por politica de traslados (AGENTS.md) y
# `SESSION-JOURNAL.md` es append-only: una entrada antigua no se reescribe
# nunca. Se miden, no se corrigen.
EXCLUDED_PREFIXES = ("docs/history/",)
EXCLUDED_FILES = ("docs/roadmap/SESSION-JOURNAL.md",)

# ─────────────────────────────────────────────────────────────────────────────
# AMPLIACION session-69s bis 10: el guard barria `docs/` y nada mas
# ─────────────────────────────────────────────────────────────────────────────
# Se midio el problema sobre TODO lo trackeado y el alcance real es el doble
# del que este guard vigilaba. Fuera de `docs/` hay contaminacion real en:
#
#   crates/sddk-engine/src/operator.rs            "The runtime<CJK>es when..."
#   crates/sddk-engine/src/decision_lab_baseline.rs
#   crates/sddk-engine/src/paradigm_profile/tests.rs
#   crates/sddk-engine/tests/a4_3r2_namespace_safe_targets.rs
#   skills/deep-research/sub/*/SKILL.md          (3 ficheros)
#   specs/E14-uat-guided-pipeline/E14.4-TEST-DISCOVERY-AGENT.md
#   tests/cycle-artifacts/p-.../session48-.../RECEIPT.md
#   tests/cycle-artifacts/p-.../session58-.../SCOPE-CONTRACT.md
#
# O sea: **un guard que vigila el 8% del problema y esta en verde**. Es la
# misma clase que el token `SPEC-NNN` fuera del vocabulario del guard de citas
# (session-69s bis 10): una comprobacion que parece cubrir porque tiene nombre
# de guard, y solo cubre lo que su primer committer quiso mirar.
#
# LO QUE NO SE HACE aqui, y es deliberado: no se corrige el preexistente.
# INC-DEBT-057 explica por que la corrupcion vieja no es reconstruible con
# fiabilidad -- la palabra original no se sabe, y varios de esos ficheros son
# ADRs `accepted` o receipts ya publicados. Sustituir a ciegas seria fabricar
# texto normativo. Se DECLARA uno a uno con su motivo, y lo que el guard
# protege es lo que viene despues.
#
# `crates/` entra porque un doc-comment es prosa con la misma propiedad que una
# linea de `.md`: si el verbo esta en otro sistema de escritura, quien lo lee no
# puede reconstruirlo, y el compilador no avisa porque el UTF-8 es valido.

# Superficies que se barren, en orden. Cada entrada dice SI su extension
# importa: `docs/` y `skills/` son prosa, `crates/` es prosa embebida en codigo.
SCAN = (
    ("docs", "**/*.md"),
    ("crates", "**/*.rs"),
    ("skills", "**/*.md"),
    ("specs", "**/*.md"),
    ("tests", "**/*.md"),
    # `CHANGELOG.md` entra por la misma razon que el resto: es la superficie
    # que un usuario lee para saber que se publico, y esta publicada con el
    # repositorio. Se measure que traia una contaminacion propia —la palabra
    # `converges` de un doc-comment, escrita como `继续`— y la cita que
    # documenta este commit la reproduce a proposito, asi que necesita
    # entrada propia en la allowlist. Sin esta superficie, un `CHANGELOG.md`
    # contaminado pasaria sin que nadie lo viera, que es el mismo defecto que
    # venia a cerrar y por el mismo motivo: cobertura parcial.
    ("CHANGELOG.md", None),
)

# `crates/`, `skills/`, `specs/` y `tests/` no tienen el problema de `docs/`
# (que mezcla historia inmutable con documentos vivos), asi que se barren
# enteros salvo los ficheros que si son inmutables por otra politica.

# Las cajas de dibujo de los comentarios de seccion (`// ─── S-3: …`) no son
# prosa: son caracteres de arte, y una regla de arte TOLERARIA DE CARACTERES
# ROTOS (lineas 578, 20, 338, 515 de crates/) no es una regla de prosa. Se
# distinguen por composicion -- una linea cuya version sana seria solo guiones y
# espacios -- y se reportan aparte, como AVISO, nunca como veredicto. Tratarlas
# como contaminacion habria hecho que el guard exigiera redibujar arte, que es
# escribir para el guard en vez de para quien lee.
BOX_DRAWING = re.compile(r"[─━=\-_*#|~•·\.]")

# Una linea de arte de comentario puede llevar TEXTO en medio:
#   `// ── S-3: Pareto drops dominated ──────────`
# asi que la regla no puede exigir que la linea sea SOLO guiones. Lo que decide
# es si el caracter contaminado esta pegado a la BARRA, o si esta en medio de
# una palabra: `runtime[CJK]es` esta dentro de `converges`, y `───[FFFD]───`
# esta en la linea de arte. Una diferencia de composicion, no de criterio.
WORD_ADJACENT = re.compile(r"[A-Za-zÁÉÍÓÚÜÑáéíóúüñ0-9]")

# Preexistente fuera de `docs/`, uno a uno. Misma clave `ruta:linea`.
KNOWN_OUTSIDE_DOCS: dict[str, str] = {
    "crates/sddk-engine/src/operator.rs": "verbo ('converges') en un doc-comment; el resto de la frase esta intacta",
    "skills/deep-research/sub/deep-domain-modeler/SKILL.md": "corte de linea a media palabra",
    "skills/deep-research/sub/deep-research-orchestrator/SKILL.md": "vinieta; el texto sigue integro",
    "skills/deep-research/sub/deep-software-research/SKILL.md": "vinieta; el texto sigue integro. DOS lineas (56 y 57), declaradas juntas",
    "specs/E14-uat-guided-pipeline/E14.4-TEST-DISCOVERY-AGENT.md": "verbo ('reemplaza')",
    "tests/cycle-artifacts/p-63676b11dc0ef88f/session48-c3i-ctx-uat-002-003/RECEIPT.md": "el receipt NOMBRA la contaminacion que corrige; reescribirlo borraria la evidencia",
    "tests/cycle-artifacts/p-63676b11dc0ef88f/session58-c3l6-x07-second-binary/SCOPE-CONTRACT.md": "verbo ('proceso')",
    "CHANGELOG.md": "cita intencionada: esta entrada del changelog NOMBRA la contaminacion de operator.rs que el commit corrige; escribirla sin el CJK seria describir el defecto con una forma que el defecto no tiene",
}
# `docs/roadmap/SESSION-JOURNAL.md` esta en EXCLUDED_FILES y por eso NO lleva
# entradas aqui, aunque contenga citas intencionadas de la contaminacion
# medida. Se intento y la Regla 1 --allowlist obsoleta-- lo rechazo, que es lo
# correcto: si el fichero no se barre, una entrada que lo apunta esta
# obsoleta por definicion. **La exclusion ya ES la declaracion**, y por eso
# apunta a la POLITICA (append-only, inmutable) y no a un recuento de lineas.
# Una allowlist por linea sobre un fichero excluido solo podria desincronizarse.

# Preexistente, uno a uno. La clave es `ruta:linea` porque la corrupcion es de
# una frase, no de un fichero: cuando se corrija una linea, se quita su entrada
# y el resto sigue valiendo.
#
# Motivo generico: "prosa en espanol con el termino sustituido por otro sistema
# de escritura; la palabra original no es reconstruible (INC-DEBT-057)".
KNOWN: dict[str, str] = {
    "docs/adr/ADR-0072-secretary-budgets.md": "adjetivo ('ambos')",
    "docs/adr/ADR-0068-bounded-execution.md": "verbo ('hace')",
    "docs/adr/ADR-0002-atomic-gate-receipt-seq-allocation.md": "verbo ('abre')",
    "docs/debt/INC-DEBT-040-PREPUSH-BUMP-PREDICATE-UNSATISFIABLE-FOR-DECLARED-RELEASE.md": "verbo ('hizo')",
    "docs/debt/INC-DEBT-054-DOCTOR-STRICT-MEASURES-NOTHING.md": "adjetivo, no reconstruible",
    "docs/debt/INC-DEBT-056-BUNDLE-STAGING-DERIVED-FROM-SURFACE-LIST-NOT-THE-MANIFEST.md": "verbo ('probo'), cirilico",
    "docs/debt/INC-DEBT-020.md": "verbo, no reconstruible",
    "docs/architecture/adrs/ADR-0139-STATIC-ENHANCED-COVERAGE-CONTRACT.md": "interrogacion ('que'), dos U+FFFD",
}


def excluded(rel: str) -> bool:
    return rel.startswith(EXCLUDED_PREFIXES) or rel in EXCLUDED_FILES


def iter_scanned():
    """Yields (path, rel) de TODA la superficie, no solo de `docs/`.

    Una copia de este recorrido por superficie, en vez de una comprehension con
    la lista metida dentro, es lo que permite que `scan()` sea la UNICA
    extraccion: el guard de nombres sufrio exactamente este fallo cuando
    `exige()` y la comprobacion de vocabularioaco cada una su propia version
    del tokenizado, y las dos divergieron sin que nadie lo notara.
    """
    for surface, pattern in SCAN:
        if pattern is None:
            # Un fichero suelto, no un arbol (ver CHANGELOG.md en SCAN).
            path = ROOT / surface
            if path.is_file():
                rel = path.relative_to(ROOT).as_posix()
                if not excluded(rel):
                    yield path, rel
            continue
        base = ROOT / surface
        if not base.is_dir():
            continue
        ext = ".rs" if surface == "crates" else ".md"
        for path in sorted(base.rglob(f"*{ext}")):
            rel = path.relative_to(ROOT).as_posix()
            if excluded(rel):
                continue
            # El propio guard contiene los rangos de caracteres que busca.
            if rel == "tests/test_docs_script_contamination.py":
                continue
            yield path, rel


def is_box_drawing(line: str) -> bool:
    """¿Es una linea de ARTE de comentario en vez de prosa?

    Se decide por el CARACTER, y se pudo decidir porque se midio: en las cuatro
    lineas de arte el simbolo es `U+FFFD` pegado a la barra, y en toda la prosa
    contaminada es CJK o cirilico real. `U+FFFD` es un caracter de reemplazo --
    un simbolo que se perdio al codificar -- y una barra de comentario que ha
    perdido simbolos sigue siendo una barra; una palabra que ha perdido
    simbolos ya no es la palabra que alguien escribio, que es exactamente el
    defecto que este guard existe para declarar.

    Medido, no supuesto: arte = U+FFFD + barra pegada; prosa = CJK/cirilico
    dentro o entre palabra. Y hay que LAS DOS condiciones. Con solo "no esta
    entre dos letras", un `U+FFFD` al final de una URL se clasificaba como arte
    y se dejaban pasar seis entradas reales, una de ellas un enlace a un ADR.
    Con solo "la linea tiene barra", `/// The runtime[CJK]es when…` tiene barra
    y se clasificaba como arte, y ahi la palabra es `converges`.
    """
    if not CONTAMINATION.search(line):
        return False
    if BOX_DRAWING.search(line) is None:
        return False
    # La RACHA de simbolos perdidos tiene que estar pegada a la barra, por
    # cualquiera de los dos lados. Asi `───[FFFD][FFFD]───` cuenta como arte, y
    # `── Pin 5: … ──[FFFD][FFFD]──` tambien: la racha toca la barra aunque la
    # linea lleve texto descriptivo en medio, que es el caso de las cuatro
    # lineas reales que motivateson esto.
    #
    # Ni "todo lo que no sea barra es prosa" (entonces un titulo de seccion
    # con barra seria prosa y habria que redibujarlo) ni "basta con que tenga
    # barra" (entonces `/// The runtime[CJK]es when…` seria arte, y ahi la
    # palabra es `converges`). La racha pegada a la barra separa las dos.
    for m in CONTAMINATION.finditer(line):
        if m.group()[0] != "�":
            return False
    for m in re.finditer("�+", line):
        before = line[m.start() - 1] if m.start() > 0 else ""
        after = line[m.end()] if m.end() < len(line) else ""
        if not (BOX_DRAWING.match(before or " ") or BOX_DRAWING.match(after or " ")):
            return False
    return True


# Cuantas lineas contaminadas declara cada fichero. Solo hace falta ponerlo
# cuando es mas de una: la regla 1b compara contra 1 por defecto, asi que un
# fichero con dos necesita decir DOS, y uno con tres lo dira al declararse.
ALLOWLIST_LINES: dict[str, int] = {
    "skills/deep-research/sub/deep-software-research/SKILL.md": 2,
}


def scan() -> tuple[dict[str, str], dict[str, int], list[str]]:
    """(ocurrencias por `ruta`, recuento por fichero, lineas de arte).

    LA CLAVE ES `ruta`, NO `ruta:linea` — y el cambio se hizo porque la clave por
    linea resulto ser una fragilidad, no una precision. Medido: al anadir una
    entrada al principio de `CHANGELOG.md`, las dos entradas de ese fichero
    desplazaron su numero de linea y el guard reporto a la vez «contaminacion
    NUEVA en CHANGELOG.md:93» y «allowlist obsoleta: CHANGELOG.md:91», es decir
    **el mismo defecto contado dos veces y en dos direcciones**, con el numero de
    linea como unico sujeto del veredicto. Un guard cuyo veredicto depende de
    cuantas lineas hay encima del defecto **cambia de veredicto al insertar un
    parrafo**, y eso obliga a cada entrada nueva a venir acompanada de un
    parche de numeros que nadie lee.

    Con la clave por `ruta` la allowlist dice «este FICHERO tiene esta
    contaminacion declarada, por este motivo», que es la verdad que interesa: la
    corrupcion es de una frase, no de una linea, y el numero de linea es un
    detalle de como esta escrito hoy. El recuento por fichero se mantiene, asi
    que **si un fichero declarado gana una segunda ocurrencia, el recuento la
    delata** y no se puede esconder anadiendo mas texto.
    """
    found: dict[str, str] = {}
    per_file: dict[str, int] = {}
    arte: list[str] = []
    for path, rel in iter_scanned():
        text = path.read_text(encoding="utf-8", errors="replace")
        for n, line in enumerate(text.splitlines(), start=1):
            if not CONTAMINATION.search(line):
                continue
            if is_box_drawing(line):
                arte.append(f"{rel}:{n}")
                continue
            per_file[rel] = per_file.get(rel, 0) + 1
            # Se guarda la primera ocurrencia como representativa del fichero.
            found.setdefault(rel, line.strip()[:70])
    return found, per_file, arte


def main() -> int:
    found, per_file, arte = scan()
    problems: list[str] = []

    # Una sola allowlist. Dos listas --una para `docs/` y otra para el resto--
    # serian dos sitios donde la verdad se puede quedar obsoleta en silencio,
    # que es el defecto que esta ampliacion vino a cerrar.
    allowlist = {**KNOWN, **KNOWN_OUTSIDE_DOCS}

    nuevas = {k: v for k, v in found.items() if k not in allowlist}
    for key, snippet in sorted(nuevas.items()):
        n = per_file[key]
        problems.append(
            f"contaminacion NUEVA en {key} ({n} linea(s)): {snippet!r}"
        )

    # Regla 1: una entrada de la allowlist cuyo fichero ya no esta contaminado.
    for key in sorted(set(allowlist) - set(found)):
        problems.append(
            f"allowlist obsoleta: {key} ya no tiene contaminacion "
            f"(se corrigio, o paso a ser arte). Quitala de KNOWN o revisa el cambio."
        )

    # Regla 1b: un fichero declarado con MAS ocurrencias de las que la
    # allowlist anuncia. Es la contraparte de usar la ruta como clave: sin esta,
    # `found[rel] = primera` ocultaria una segunda contaminacion en el mismo
    # fichero detrás de la primera.
    for rel, n in sorted(per_file.items()):
        if rel in allowlist and n > ALLOWLIST_LINES.get(rel, 1):
            problems.append(
                f"fichero declarado que ha ganado contaminacion: {rel} tiene {n} "
                f"lineas y la allowlist declara "
                f"{ALLOWLIST_LINES.get(rel, 1)}. Anade el numero o corrige."
            )

    # Regla 2: una entrada cuyo fichero ya no existe del todo.
    for key in sorted(allowlist):
        rel = key.rsplit(":", 1)[0]
        if not (ROOT / rel).exists():
            problems.append(f"allowlist apunta a un fichero que no existe: {rel}")

    # Regla 3: el recuento sale del contenido, no de una constante.
    for rel, n in sorted(per_file.items()):
        if rel not in {k.rsplit(":", 1)[0] for k in allowlist}:
            problems.append(f"fichero contaminado sin ninguna entrada en KNOWN: {rel} ({n})")

    surfaces = []
    for surface, pattern in SCAN:
        if pattern is None:
            n = 1 if (ROOT / surface).is_file() else 0
            if n:
                surfaces.append(f"{surface}={n}")
            continue
        n = sum(1 for _p, rel in iter_scanned() if rel.startswith(surface + "/"))
        if n:
            surfaces.append(f"{surface}/={n}")

    print("== guard: contaminacion de sistema de escritura en la prosa del repo ==")
    print(f"  superficies barridas:            {', '.join(surfaces)}")
    print(f"  ficheros barridos:               {sum(1 for _ in iter_scanned())}")
    print(f"  ocurrencias en ficheros de la allowlist: {len(found) - len(nuevas)}")
    print(f"  entradas en la allowlist:        {len(allowlist)} "
          f"({len(KNOWN)} en docs/ + {len(KNOWN_OUTSIDE_DOCS)} fuera)")
    print(f"  contaminacion nueva:             {len(nuevas)}")
    print(f"  lineas de ARTE con caracter roto (aviso, no veredicto): {len(arte)}")
    for rel, n in sorted(per_file.items()):
        print(f"    {rel}: {n}")

    if problems:
        print(f"\nFAIL: {len(problems)} problema(s)")
        for p in problems:
            print(f"  [FAIL] {p}")
        print(
            "\nUn termino en otro sistema de escritura no es un detalle de estilo:\n"
            "si es texto en espanol, la palabra original no se puede reconstruir con\n"
            "fiabilidad, y sustituirla seria fabricar. Pregunta a quien lo escribio."
        )
        return 1

    print("\nRESULT: PASS — ninguna contaminacion nueva en ninguna superficie; la "
          "allowlist cuadra con el contenido.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
