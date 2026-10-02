#!/usr/bin/env python3
"""test_uat_authority_citations.py — una cita que no resuelve es un criterio que
no puede fallar honestamente.

POR QUÉ EXISTE (session-65k)
----------------------------
`docs/roadmap/UAT-MATRIX.md` es la tabla que un agente lee para saber qué está
aceptado. Cada fila declara un *exit criterion*, y muchos de esos criterios
remiten a una autoridad: un ADR, un REQ, una entrada de deuda. Hasta ahora
**ningún runner validaba que esas referencias resuelvan a algo existente**.

El caso que loAbortó: `AT-UAT-019` (C3m.2) dice

    revise con mismo contenido / distinto tiempo ⇒ Comportamiento coincide
    con **el ADR de identidad**

Ese ADR **no existe**. El criterio no puede pasar ni fallar contra él, y por
eso la fila quedó en PASS PARCIAL de forma permanente — con un motivo que
nadie podía refutar porque nadie podía resolver la referencia. Eso se
registró como INC-DEBT-048.

LO QUE ESTE GUARD HACE Y LO QUE NO
----------------------------------
 Hace, y es comprobable:
   1. Toda autoridad citada por ID (`ADR-NNNN`, `REQ-...`, `INC-DEBT-NNN`)
      resuelve a un documento real.
   2. Cada fila tiene tantas celdas como la cabecera de SU tabla.
   3. Los IDs de fila son únicos en toda la matriz.

 NO hace, y conviene decirlo:
   - No puede verificar una autoridad citada **en prosa** («el ADR de
     identidad», «la spec vigente»). No hay ID que resolver. Un guard que
     fingiera cubrirlo estaría midiendo algo que no mide, que es la clase de
     defecto que este repo lleva varias slices persiguiendo.
     La prosa se reporta aparte, como aviso, no como veredicto.

NOTA DE MÉTODO
--------------
La primera medicion de este trabajo dio «28 filas con 5 celdas y 44
con 4», y de ahí casi se reporta un defecto de columnas que no existe: el
fichero tiene **dos tablas**, y el script las mezclaba porque recortaba las
líneas por un índice fijo en vez de agrupar por cabecera. La propiedad 2
existe porque un parser que no agrupe por cabecera se equivoca — y este guard
no quiere ser ese parser.

Salida: exit 0 si las tres propiedades se sostienen; 1 con el detalle si no.
"""

from __future__ import annotations

import pathlib
import re
import sys
from collections import Counter

ROOT = pathlib.Path(__file__).resolve().parent.parent
MATRIX = ROOT / "docs" / "roadmap" / "UAT-MATRIX.md"

AUTHORITY_TOKENS = re.compile(
    r"\b(ADR-\d{4}|REQ-[A-Z0-9]+-[A-Z0-9-]+|INC-DEBT-\d+)\b"
)
# Prosa que nombra una autoridad sin ID. Se reporta como aviso: no es
# verificable, y un aviso que se presentara como veredicto seria mentir.
PROSE_AUTHORITY = re.compile(
    r"\b(?:el|la|los|las)\s+(?:ADR|spec|specification|requisito|documento)\b",
    re.IGNORECASE,
)

ADR_DIRS = [
    ROOT / "docs" / "adr",
    ROOT / "docs" / "architecture" / "adrs",
    *sorted((ROOT / "docs" / "history").glob("*/adrs")),
]
SPEC_DIR = ROOT / "docs" / "architecture" / "specs"
DEBT_DIR = ROOT / "docs" / "debt"


def parse_tables(lines: list[str]) -> list[tuple[list[str], list[list[str]]]]:
    """Group table lines by HEADER, never by a fixed slice index.

    The matrix holds two tables with different column counts. Any parser that
    assumes one global shape reads the wrong cell for half the rows — which is
    how the first measurement of this work reported 28 rows that "should" have
    had 4 cells and did not.
    """
    tables: list[tuple[list[str], list[list[str]]]] = []
    header: list[str] | None = None
    rows: list[list[str]] = []
    for raw in lines:
        cells = [c.strip() for c in raw.strip().strip("|").split("|")]
        if cells and cells[0] == "ID":
            if header is not None:
                tables.append((header, rows))
            header, rows = cells, []
            continue
        if header is None or set(cells) <= {"---"} or not any(cells):
            continue
        rows.append(cells)
    if header is not None:
        tables.append((header, rows))
    return tables


def resolves(token: str) -> bool:
    if token.startswith("ADR-"):
        needle = token
        for directory in ADR_DIRS:
            if not directory.is_dir():
                continue
            for candidate in directory.glob(f"{needle}*.md"):
                if candidate.is_file():
                    return True
        return False
    if token.startswith("INC-DEBT-"):
        return any(DEBT_DIR.glob(f"{token}*.md"))
    # REQ-...: vive dentro de una spec, no tiene fichero propio.
    if not SPEC_DIR.is_dir():
        return False
    return any(token in p.read_text(encoding="utf-8", errors="replace") for p in SPEC_DIR.glob("*.md"))


def main() -> int:
    if not MATRIX.is_file():
        print(f"  [FAIL] la matriz no existe: {MATRIX}")
        return 1

    text = MATRIX.read_text(encoding="utf-8")
    tables = parse_tables([l for l in text.splitlines() if l.startswith("|")])

    failures: list[str] = []
    warnings: list[str] = []

    # Propiedad 2: cada fila tiene las celdas que declara SU cabecera.
    for header, rows in tables:
        ncol = len(header)
        for cells in rows:
            if len(cells) != ncol:
                failures.append(
                    f"{cells[0]}: {len(cells)} celdas, la cabecera de su tabla "
                    f"declara {ncol} ({' | '.join(header)})"
                )

    # Propiedad 3: IDs unicos en toda la matriz.
    all_ids = [cells[0] for _h, rows in tables for cells in rows if cells]
    duplicates = [i for i, n in Counter(all_ids).items() if n > 1]
    for dup in sorted(duplicates):
        failures.append(f"ID de fila duplicado: {dup}")

    # Propiedad 1: toda autoridad citada por ID resuelve.
    tokens = sorted(set(AUTHORITY_TOKENS.findall(text)))
    unresolved = [t for t in tokens if not resolves(t)]
    for token in unresolved:
        failures.append(f"autoridad citada que NO resuelve: {token}")

    # Aviso (no veredicto): criterios que nombran una autoridad solo en prosa.
    prose_hits = 0
    for _header, rows in tables:
        for cells in rows:
            if cells and PROSE_AUTHORITY.search(cells[-1]):
                prose_hits += 1
                warnings.append(
                    f"{cells[0]}: el exit criterion nombra una autoridad en prosa, "
                    f"sin ID resoluble ({cells[-1][:70]}…)"
                )

    print(f"  tablas:                       {len(tables)}")
    print(f"  filas:                        {len(all_ids)}")
    print(f"  autoridades por ID citadas:    {len(tokens)} ({len(tokens) - len(unresolved)} resuelven)")
    print(f"  avisos de prosa sin ID:       {prose_hits}")

    for w in warnings:
        print(f"  [aviso] {w}")

    if failures:
        print()
        for f in failures:
            print(f"  [FAIL] {f}")
        print()
        print("RESULT: FAIL — la matriz cita autoridades que no resuelven, o su "
              "estructura no es la que declara.")
        print("         Un criterio que remite a un documento inexistente no "
              "puede pasar ni fallar honestamente:")
        print("         corrigelo o deja de citarlo.")
        return 1

    print()
    print("RESULT: PASS — toda autoridad citada por ID resuelve y la estructura "
          "es la que cada cabecera declara.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
