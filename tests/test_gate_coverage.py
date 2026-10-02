#!/usr/bin/env python3
"""test_gate_coverage.py — ningún test puede existir sin runner o sin motivo.

POR QUÉ EXISTE (session-65j)
----------------------------
El hallazgo fue un guard que llevaba **rojo desde session-65b** y que nadie
ejecutaba: `scripts/check_debt_index_coherence.sh` estaba referenciado solo
por su propio test de fixtures. Sus 10 casos PASABAN. Ese verde leía como
cobertura y no lo era.

La causa no era el guard: era que **la superficie de gates es una lista escrita
a mano** (`scripts/release.sh`, `ci.yml`) y un test nuevo no entra en ella
solo. Ese mismo día, antes de cablear, el barrido encontró **13 de 36 tests
sin runner**, incluidos cuatro `.py` que fijan contratos escritos en
sessions 65g/65h y que se ejecutaban a mano en cada slice.

LA PROPIEDAD
------------
Para todo `tests/test_*.sh` y `tests/test_*.py`:
  - o algún runner lo ejecuta, o
  - está en `EXCEPTIONS` con un motivo escrito.

Y dos reglas que impiden que la lista de excepciones se pudra:

1. Una excepción que **apunta a un test que ya no existe** es un FAIL: la
   excepción quedó obsoleta y hay que borrarla.
2. Una excepción para un test que **ya tiene runner** es un FAIL: la razón
   caducó, y dejarla convertiría la lista de excepciones en un cajón de sastre
   donde cualquier test puede acabar sin correr.

POR QUÉ SE DESCARTAN LOS COMENTARIOS
------------------------------------
Un test nombrado en un comentario **no está gated**. La nota de exclusión de
`release.sh` nombra dos tests precisamente porque NO se ejecutan, y un
`in` a pelo los puntuaba como cubiertos: la medición dio 7 sin runner donde
había 13. Contar prosa como cobertura es el mismo error que contar una
declaración como obediencia.

Salida: exit 0 si la propiedad se sostiene; 1 con el detalle si no.
"""

from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
TESTS_DIR = ROOT / "tests"

# Todo fichero que puede ejecutar un test. Los guards de `scripts/` se
# incluyen porque un test puede ser invocado por un guard en vez de por un
# `for t in`.
RUNNERS = [
    ROOT / "scripts" / "release.sh",
    *sorted((ROOT / "scripts").glob("*.sh")),
    *sorted((ROOT / ".github" / "workflows").glob("*.yml")),
]

# Tests que NO deben correr en un gate automático, cada uno con su motivo.
# Añadir aquí una entrada es una decisión, no un descuido: por eso el motivo es
# parte de la entrada y las reglas 1 y 2 la mantienen honesta.
EXCEPTIONS: dict[str, str] = {
    "test_h05_isolation.sh": (
        "pasa sin medir: sin el rlib release imprime `skip:` y aun asi reporta "
        "PASS=1 FAIL=0. Cablearlo devolveria un verde vacio (misma forma que "
        "INC-DEBT-054). Requiere `cargo build --release -p sddk-engine` antes."
    ),
    "test_release_routes_parity.sh": (
        "necesita `act` + `podman` y monta contenedores; la ruta cloud no se "
        "puede reproducir en local. Comprobacion opt-in."
    ),
    "test_release_bundle_parity.sh": (
        "necesita `act` + `podman` para reproducir la ruta cloud del bundle."
    ),
    "test_release_public_gate.sh": (
        "verifica el release PUBLICO contra la API de GitHub (curl/gh). "
        "Requiere red y un tag publicado; en un release aun no lo hay."
    ),
    "test_release_bump_derivation.sh": (
        "deriva la version consultando el remoto; requiere red."
    ),
}


def code_only(text: str) -> str:
    """Drop full-line comments and inline `# ...` tails.

    Prose is not obedience. See the module docstring.
    """
    kept: list[str] = []
    for line in text.splitlines():
        if line.strip().startswith("#"):
            continue
        kept.append(line.split(" #")[0])
    return "\n".join(kept)


def main() -> int:
    runner_code = "\n".join(
        code_only(p.read_text(encoding="utf-8")) for p in RUNNERS if p.exists()
    )

    tests = sorted(TESTS_DIR.glob("test_*.sh")) + sorted(TESTS_DIR.glob("test_*.py"))
    names = [t.name for t in tests]

    failures: list[str] = []
    notes: list[str] = []

    # Contabilidad explicita: covered / excepted / uncovered son conjuntos
    # disjuntos. La primera version los derivaba por resta y reportaba
    # "excepcionados: 0" con seis excepciones vivas — un guard que miente
    # sobre sus propias cifras no puede usarse para justificar por que el
    # resto pasa.
    covered = {n for n in names if n in runner_code}
    excepted = {n for n in names if n not in runner_code and n in EXCEPTIONS}
    uncovered = {n for n in names if n not in runner_code and n not in EXCEPTIONS}

    for name in sorted(uncovered):
        failures.append(
            f"{name}: ningun runner lo ejecuta y no esta en EXCEPTIONS con un motivo"
        )

    print(f"  tests en tests/test_*:        {len(names)}")
    print(f"  con runner:                   {len(covered)}")
    print(f"  excepcionados con motivo:     {len(excepted)}")
    print(f"  SIN runner y SIN motivo:      {len(uncovered)}")

    # Regla 1: excepcion que apunta a un test inexistente.
    for name in sorted(set(EXCEPTIONS) - set(names)):
        failures.append(
            f"{name}: figura en EXCEPTIONS pero el fichero no existe; "
            "la excepcion quedo obsoleta y hay que borrarla"
        )

    # Regla 2: excepcion para un test que ya tiene runner.
    for name in sorted(set(EXCEPTIONS) & covered):
        failures.append(
            f"{name}: figura en EXCEPTIONS pero ya tiene runner; "
            "la razon caducó y dejarla convertiria EXCEPTIONS en un cajon "
            "de sastre"
        )

    for name in uncovered:
        notes.append(f"  [sin runner] {name}")

    for note in notes:
        print(note)

    if failures:
        print()
        for f in failures:
            print(f"  [FAIL] {f}")
        print()
        print(
            "RESULT: FAIL — la superficie de gates es una lista escrita a mano y "
            "se ha quedado corta."
        )
        print(
            "         Cablear el test en scripts/release.sh (o .github/workflows) "
            "si es hermetico,"
        )
        print(
            "         o anadirlo a EXCEPTIONS con un motivo escrito si necesita "
            "red, contenedores o un artefacto ausente."
        )
        return 1

    print()
    print("RESULT: PASS — todo test tiene runner, o una excepcion con motivo.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
