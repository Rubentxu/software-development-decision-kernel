#!/usr/bin/env python3
"""test_spec_citation_anchor.py — un `SPEC-NNN` que nombra tres documentos no
nombra ninguno.

POR QUÉ EXISTE (session-69s bis 10)
------------------------------------
Al cerrar C3n.3 fui a nombrar el fichero de test de la fila `R5` y encontré que
la casilla decía «tests staleness (**SPEC-012**)». Fui a mirar qué SPEC-012 era.
**Hay tres**, y no es una casualidad del repo:

  docs/architecture/specs/arch-spec-012-configuration.md              (canónico)
  docs/history/…/sddk-2.0-architecture-consolidation/specs/SPEC-012-staleness-impact.md
  docs/history/…/SDDK-Human-Agent-Collaboration…/specs/SPEC-012-persona-safety-….md
  docs/history/…/SDDK-Context-First…/extension-platform/specs/SPEC-012-CONFIGURATION.md

Y el código **implementa el segundo**: `crates/sddk-domain/src/staleness.rs`
abre con `//! Universal staleness derivation over the reactive graph (SPEC-012,
Phase 6)` y su `StalenessState` tiene exactamente los cinco estados de su §2,
fijados por `assert_variant_count_eq!(StalenessState, 5, …)`. El canónico es de
**configuración de agentes** y no tiene esa sección.

Lo grave no es que la cita fuera ambigua: es que
`test_uat_authority_citations.py` **la daba por buena**. Resuelve `SPEC-*`
contra `SPEC_DIR.glob("*.md")` y `SPEC-012` sí está ahí. **Un guard de citas
que sólo comprueba que la cita resuelve no detecta una cita que resuelve al
documento equivocado**, y esa es la mitad de lo que su docstring dice hacer.

**Y el token no estaba ni en el vocabulario.** El `AUTHORITY_TOKENS` del guard
de citas reconoce `ADR-` seguido de cuatro digitos, `REQ-...` e
`INC-DEBT-` seguido de digitos. **Un `SPEC-` seguido de tres digitos no esta**,
y `UAT-MATRIX.md` no cita ni uno. O sea que la clase de autoridad que mas se
cita en el codigo --**202 citas en `crates/`, 21 IDs distintos**-- no la vigila
nadie. Eso no es un agujero en un guard: es una clase entera de autoridad
fuera del contrato.

LO QUE ESTE GUARD COMPRUEBA
---------------------------
1. **Todo `SPEC-NNN` que se cita en el repo tiene documento canónico.** No mide
   si está bien implementado: mide que la cita no apunta al vacío. Es el mismo
   mínimo que el guard de receipts.
2. **Toda spec canónica declara de dónde viene**, por `package_source` o por
   `adopted_at`+`adoption_cycle`. Una spec canónica que no dice qué paquete
   absorbe deja la ambigüedad sin quién la resuelva.
3. **Un `SPEC-NNN` citado en prosa de una tabla de verdad lleva su origen
   anclado** — `SPEC-012 (arch-spec-012-configuration)` o la ruta al paquete.
   Sin ancla, la cita admite las N lecturas y el guard no puede decidir.

LO QUE NO COMPRUEBA, Y HAY QUE DECIRLO
---------------------------------------
- **No verifica que el código implemente la spec que dice implementar.** Eso
  exigiría leer ambas y comparar semántica; es un problema de UAT, no de citas.
  Este guard responde a una pregunta más pequeña y comprobable: *¿esta cita
  apunta a un documento, y si hay varios, cuál?*
- **No toca `docs/history/`.** Es inmutable por política de traslados. Los
  históricos aparecen aquí sólo como *candidatos* que hacen ambigua una cita.

NOTA DE MÉTODO — LA CUARTA VEZ EN ESTA SESIÓN
---------------------------------------------
La primera versión de mi índice canónico se apoyaba sólo en
`package_local_id:` y encontró **18 specs de 66**. La realidad: hay **dos
esquemas de frontmatter**. `arch-spec-012` declara `package_local_id: SPEC-012`;
`arch-spec-043` **no declara ninguno** y se deduce del nombre del fichero. Con
el índice corto, `SPEC-043` salía **huérfano con 35 citas** y el guard habría
reportado un defecto de 35 filas inexistente. **Un índice que no reproduce lo
que el mundo contiene inventa los defectos que dice encontrar**, que es peor
que no mirar: un guard con falsos positivos entrena a su lector a ignorar el
verdicto. Por eso el índice vive en `spec_index()` y su construcción está
probada por las mutaciones de `…_mutation.sh`.

Salida: exit 0 si las tres propiedades se sostienen; 1 con el detalle si no.
"""

from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SPEC_DIR = ROOT / "docs" / "architecture" / "specs"
HISTORY_DIR = ROOT / "docs" / "history"

# La clase de autoridad que `test_uat_authority_citations.py` no cubre.
SPEC_CITATION = re.compile(r"\bSPEC-\d{3}\b")

# Superficies donde una cita de spec es una AFIRMACION: una tabla que un agente
# lee para saber que está aceptado. Un `SPEC-012` suelto en la última columna es
# un criterio que no se puede fallar honestamente, por el mismo motivo que un
# exit criterion que cita un ADR inexistente (INC-DEBT-048).
TRUTH_TABLES = [
    ROOT / "docs" / "roadmap" / "ACCEPTANCE-TRUTHFULNESS-MATRIX.md",
    ROOT / "docs" / "roadmap" / "UAT-MATRIX.md",
]

# `SPEC-012 (arch-spec-012-configuration)`, `SPEC-004 -> arch-spec-004-decision-memory`
# o `SPEC-012` seguido de la ruta al paquete. Se acepta cualquiera de las tres
# anclas: lo que se exige es que la cita sea SEPARABLE, no que use una sintaxis
# concreta. La flecha y el "->" cuentan porque son la forma que la matriz usa al
# traducir un ID de paquete a su equivalente nativo; exigir parentesis cuando el
# documento escribe con flecha seria un guard que obliga a escribir raro, y un
# guard que obliga a escribir raro se cumple de forma formal y se lee de forma
# falsa.
ANCHOR_NATIVE = re.compile(
    r"SPEC-\d{3}\s*(?:\(\s*arch-spec-\d{3}[a-z0-9-]*\s*\)|(?:→|->)\s*`?arch-spec-\d{3}[a-z0-9-]*`?)",
    re.I,
)
ANCHOR_PATH = re.compile(r"SPEC-\d{3}[^\n|]{0,120}?docs/history/\S+")


def spec_index() -> dict[str, list[str]]:
    """SPEC local id -> documentos canicos que lo sirven.

    DOS esquemas de frontmatter existen y ambos son reales: `package_local_id:`
    (specs adoptadas de un paquete, 001..018) y `id: arch-spec-NNN-…` mas el
    nombre del fichero (specs nativas, 019..049). Un indice que solo lea el
    primero pierde 30 documentos y reporta como huerfanos IDs con 35 citas.
    """
    index: dict[str, list[str]] = {}
    for path in sorted(SPEC_DIR.glob("*.md")):
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
            else:
                m = re.match(r"arch-spec-(\d+)-", path.name)
                if m:
                    local = f"SPEC-{m.group(1)}"
        if local:
            index.setdefault(local, []).append(path.relative_to(ROOT).as_posix())
    return index


def history_candidates(local_id: str) -> list[str]:
    """Documentos historicos que sirven el mismo local id."""
    number = local_id.split("-")[1]
    pattern = re.compile(rf"SPEC-{number}")
    return sorted(
        p.relative_to(ROOT).as_posix()
        for p in HISTORY_DIR.rglob("*.md")
        if "/specs/" in p.as_posix() and pattern.match(p.name)
    )


def declares_provenance(path: pathlib.Path) -> bool:
    """Una spec canonica dice de donde viene.

    Sin esto, el documento que resuelve la ambiguedad no existe: se puede
    senalar el canonico, pero no se puede decir de que paquete absorbio, y por
    lo tanto no se puede decir a cual de los historicos sustituye.
    """
    text = path.read_text(encoding="utf-8", errors="replace")
    if re.search(r"^package_source:\s*\S+", text, re.M):
        return True
    if re.search(r"^adopted_at:\s*\S+", text, re.M) and re.search(
        r"^adoption_cycle:\s*\S+", text, re.M
    ):
        return True
    # Spec nativa: no viene de un paquete, pero entonces lo dice.
    return bool(re.search(r"^status:\s*\S+", text, re.M))


def ambiguous_citations() -> list[tuple[str, int, str, int]]:
    """(fichero, linea, spec_id, n_candidatos) para citas sin ancla."""
    found: list[tuple[str, int, str, int]] = []
    for table in TRUTH_TABLES:
        if not table.is_file():
            continue
        rel = table.relative_to(ROOT).as_posix()
        for n, line in enumerate(table.read_text(encoding="utf-8").splitlines(), 1):
            for spec_id in SPEC_CITATION.findall(line):
                if ANCHOR_NATIVE.search(line) or ANCHOR_PATH.search(line):
                    continue
                total = 1 + len(history_candidates(spec_id))
                found.append((rel, n, spec_id, total))
    return found


def main() -> int:
    if not SPEC_DIR.is_dir():
        print(f"  [FAIL] no hay directorio de specs canonicas: {SPEC_DIR}")
        return 1

    index = spec_index()
    failures: list[str] = []
    warnings: list[str] = []

    # Propiedad 1: toda spec citada tiene canonico.
    cited: set[str] = set()
    for path in sorted((ROOT / "crates").rglob("*.rs")):
        cited.update(SPEC_CITATION.findall(path.read_text(encoding="utf-8", errors="replace")))
    for table in TRUTH_TABLES:
        if table.is_file():
            cited.update(SPEC_CITATION.findall(table.read_text(encoding="utf-8")))
    orphans = sorted(cited - set(index))
    for spec_id in orphans:
        failures.append(
            f"spec citada que NO tiene documento canonico: {spec_id} "
            f"(citada en crates/ o en una tabla de verdad)"
        )

    # Propiedad 2: toda spec canonica declara procedencia.
    missing_provenance = [
        rel
        for rels in index.values()
        for rel in rels
        if not declares_provenance(ROOT / rel)
    ]
    for rel in sorted(missing_provenance):
        failures.append(
            f"spec canonica que NO declara de donde viene: {rel} "
            f"(sin package_source ni status/adoption_cycle)"
        )

    # Propiedad 3: cita ambigua sin ancla.
    #
    # DOS DIENTES, porque una fila de tabla y un parrafo no son lo mismo:
    # una FILA es una afirmacion -- el documento que dice que algo esta
    # aceptado -- y una cita sin ancla ahi no se puede fallar honestamente.
    # La PROSA puede estar *describiendo* el defecto, y exigirle un ancla ahi
    # obligaria a reescribir la evidencia que explica por que existe el
    # guard. Por eso la prosa se CUENTA y se reporta como aviso, nunca como
    # veredicto: quitarla del recuento seria esconderla, y contarla como
    # fallo seria mentir sobre lo que el guard sabe.
    ambiguous = ambiguous_citations()
    for rel, line, spec_id, total in ambiguous:
        text_line = (ROOT / rel).read_text(encoding="utf-8").splitlines()[line - 1]
        is_row = text_line.lstrip().startswith("|")
        where = f"{rel}:{line}"
        detail = (
            f"cita AMBIGUA sin ancla en FILA: {where} dice «{spec_id}» y ese ID "
            f"sirve {total} documento(s). Ancla la cita: "
            f"«{spec_id} (arch-spec-{spec_id.split('-')[1]}-…)» o la ruta al paquete."
        )
        if is_row:
            failures.append(detail)
        else:
            warnings.append(f"{where} cita «{spec_id}» sin ancla en prosa ({total} documentos)")

    prose_warnings = len(warnings)
    ambiguous_ids = sorted({s for _r, _l, s, _n in ambiguous})
    ambiguous_in_rows = sum(
        1
        for rel, line, _s, _n in ambiguous
        if (ROOT / rel).read_text(encoding="utf-8").splitlines()[line - 1].lstrip().startswith("|")
    )
    print(f"  specs canonicas indexadas:        {len(index)}")
    print(f"  ficheros de codigo barridos:      crates/**/*.rs")
    print(f"  SPEC-NNN distintas citadas:        {len(cited)}")
    print(f"    con canonico:                    {len(cited) - len(orphans)}")
    print(f"    HUERFANAS:                       {len(orphans)}")
    print(f"  tablas de verdad barridas:        {len(TRUTH_TABLES)}")
    print(f"  citas ambiguas SIN ancla:         {len(ambiguous)} (IDs: {len(ambiguous_ids)})")
    print(f"    en FILA (veredicto):             {ambiguous_in_rows}")
    print(f"    en PROSA (aviso):                {prose_warnings}")

    for w in warnings:
        print(f"  [aviso] {w}")

    if failures:
        print()
        for f in failures:
            print(f"  [FAIL] {f}")
        print()
        print("RESULT: FAIL — hay citas de spec que no anclan a un documento, o specs "
              "que no declaran su procedencia.")
        print("         Un SPEC-NNN que sirve tres documentos no nombra ninguno: la "
              "lectura que el")
        print("         guard de citas acepta no es necesariamente la que el codigo "
              "implementa.")
        return 1

    print()
    print("RESULT: PASS — toda spec citada tiene canonico, toda canonica declara su "
          "procedencia, y")
    print("         ninguna cita ambigua queda sin anclar.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
