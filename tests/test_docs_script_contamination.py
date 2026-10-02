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
   `git mv` o una reescrituraodrTurn dejarían la allowlist apuntando al vacío y
   la exclusión seguiría valiendo para un texto que ya no es el que se revisó.
2. Una entrada whose fichero ya no contiene el carácter es un FAIL. Sin esto,
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

# Preexistente, uno a uno. La clave es `ruta:linea` porque la corrupcion es de
# una frase, no de un fichero: cuando se corrija una linea, se quita su entrada
# y el resto sigue valiendo.
#
# Motivo generico: "prosa en espanol con el termino sustituido por otro sistema
# de escritura; la palabra original no es reconstruible (INC-DEBT-057)".
KNOWN: dict[str, str] = {
    "docs/adr/ADR-0072-secretary-budgets.md:47": "adjetivo ('ambos')",
    "docs/adr/ADR-0068-bounded-execution.md:195": "verbo ('hace')",
    "docs/adr/ADR-0002-atomic-gate-receipt-seq-allocation.md:51": "verbo ('abre')",
    "docs/debt/INC-DEBT-040-PREPUSH-BUMP-PREDICATE-UNSATISFIABLE-FOR-DECLARED-RELEASE.md:251": "verbo ('hizo')",
    "docs/debt/INC-DEBT-054-DOCTOR-STRICT-MEASURES-NOTHING.md:43": "adjetivo, no reconstruible",
    "docs/debt/INC-DEBT-056-BUNDLE-STAGING-DERIVED-FROM-SURFACE-LIST-NOT-THE-MANIFEST.md:181": "verbo ('probo'), cirilico",
    "docs/debt/INC-DEBT-020.md:87": "verbo, no reconstruible",
    "docs/architecture/adrs/ADR-0139-STATIC-ENHANCED-COVERAGE-CONTRACT.md:89": "interrogacion ('que'), dos U+FFFD",
}


def excluded(rel: str) -> bool:
    return rel.startswith(EXCLUDED_PREFIXES) or rel in EXCLUDED_FILES


def scan() -> tuple[dict[str, str], dict[str, int]]:
    """Devuelve (ocurrencias por `ruta:linea`, recuento por fichero)."""
    found: dict[str, str] = {}
    per_file: dict[str, int] = {}
    for path in sorted(DOCS.rglob("*.md")):
        rel = path.relative_to(ROOT).as_posix()
        if excluded(rel):
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        for n, line in enumerate(text.splitlines(), start=1):
            if CONTAMINATION.search(line):
                key = f"{rel}:{n}"
                found[key] = line.strip()[:70]
                per_file[rel] = per_file.get(rel, 0) + 1
    return found, per_file


def main() -> int:
    found, per_file = scan()
    problems: list[str] = []

    nuevas = {k: v for k, v in found.items() if k not in KNOWN}
    for key, snippet in sorted(nuevas.items()):
        problems.append(f"contaminacion NUEVA en {key}: {snippet!r}")

    # Regla 1: una entrada de la allowlist cuya linea ya no coincide.
    for key in sorted(set(KNOWN) - set(found)):
        problems.append(
            f"allowlist obsoleta: {key} ya no tiene contaminacion. "
            f"Quitala de KNOWN o revisa el cambio."
        )

    # Regla 2: una entrada cuyo fichero ya no existe del todo.
    for key in sorted(KNOWN):
        rel = key.rsplit(":", 1)[0]
        if not (ROOT / rel).exists():
            problems.append(f"allowlist apunta a un fichero que no existe: {rel}")

    # Regla 3: el recuento sale del contenido, no de una constante.
    for rel, n in sorted(per_file.items()):
        if rel not in {k.rsplit(":", 1)[0] for k in KNOWN}:
            problems.append(f"fichero contaminado sin ninguna entrada en KNOWN: {rel} ({n})")

    print("== guard: contaminacion de sistema de escritura en docs/ ==")
    print(f"  ficheros barridos (excluye history/ y SESSION-JOURNAL.md): "
          f"{sum(1 for p in DOCS.rglob('*.md') if not excluded(p.relative_to(ROOT).as_posix()))}")
    print(f"  ocurrencias en ficheros de la allowlist: {len(found) - len(nuevas)}")
    print(f"  entradas en KNOWN: {len(KNOWN)}")
    print(f"  contaminacion nueva: {len(nuevas)}")
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

    print("\nRESULT: PASS — ninguna contaminacion nueva; la allowlist cuadra con el contenido.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
