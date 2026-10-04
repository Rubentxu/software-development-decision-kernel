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

import os
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
    # test_doctor_identity_states.sh YA NO esta excepcionado (session-69s bis 4).
    # Exigia dos binarios porque no tenia modo de uno solo, y el camino de
    # release solo tiene el concluyente. MEDIDO: pasando el concluyente en los
    # dos huecos, O2-O5 y O7 pasan y SOLO O6 falla -- O6 es el unico objetivo que
    # depende de la procedencia no concluyente. El guard gano un modo de un
    # binario que DECLARA O6 como NOT_RUN con su motivo y baja la cuenta de
    # veredictos de 5 a 4, que es lo que de verdad midio. Por eso ahora corre en
    # release.sh contra el binario publicado. El residuo --el estado de
    # procedencia no concluyente-- es cobertura de SESION, no de pipeline, y
    # queda escrito en release.sh y en INC-DEBT-064.
    #
    # test_release_bump_derivation.sh YA NO esta excepcionado (session-77).
    # La razon decia "deriva la version consultando el remoto; requiere red", y
    # era falsa en las dos direcciones: el script consultaba los tags LOCALES
    # (por eso no tocaba la red), y sus fixtures usan un remoto BARE en disco,
    # luego `git ls-remote` funciona sin red. Al cablearlo en el 1b, la Regla 2
    # —una excepcion para un test que ya tiene runner es un FAIL— cobro la razon
    # caducada. La excepcion sobrevivia porque otro guard la mencionaba en un
    # comentario, que es exactamente lo que este fichero no cuenta.
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

    # Regla 3: enumerado NO es ejecutado. El `for t in` del paso 1b de
    # release.sh esta gateado por `[ -x ]`, luego un test shell citado ahi sin
    # bit de ejecucion se SALTA con un `warn` y el paso imprime despues
    # "shell contract tests green". MEDIDO en session-75: cinco de los
    # enumerados llevaban 644 y se saltaron en v2.5.3, v2.5.4 y v2.5.5, y uno
    # de ellos (test_release_state_pointer.sh) arrastraba 41 commits de deriva
    # y `manifest.toml` dos versiones atras sin que nada lo delatara -- porque
    # el guard que lo detecta jamas habia corrido. La cobertura que cuenta el
    # nombre no es la cobertura que ejecuta el bit.
    #
    # El alcance es el bucle gateado, NO todo el fichero: cuatro tests mas
    # (changelog_coverage, doctor_identity_states y las dos de la politica de
    # nombres) aparecen citados en release.sh pero se ejecutan desde su propio
    # paso con `bash`, sin pasar por `[ -x ]`. Medido: una primera version de
    # esta regla los senalo y habria exigido un `chmod +x` inutil -- un guard
    # que acusa de rojo a algo que corre bien entrena a ignorar sus rojos.
    non_executable: list[str] = []
    release_runner_path = ROOT / "scripts" / "release.sh"
    if release_runner_path.exists():
        release_runner = release_runner_path.read_text(encoding="utf-8").splitlines()
        gated: set[str] = set()
        for i, line in enumerate(release_runner):
            if '[ -x "$t" ]' not in line:
                continue
            # Recoger hacia atras las continuaciones del `for t in` que abre el
            # bucle gateado, hasta la linea `for t in`.
            for back in range(i, -1, -1):
                prev = release_runner[back]
                gated.update(re.findall(r"tests/[A-Za-z0-9_.-]+\.sh", prev))
                if re.search(r"\bfor\s+\w+\s+in\b", prev):
                    break
        for rel in sorted(gated):
            bare = rel.rsplit("/", 1)[-1]
            t = ROOT / rel
            if bare in EXCEPTIONS or not t.exists():
                continue
            if not os.access(t, os.X_OK):
                non_executable.append(bare)
                failures.append(
                    f"{bare}: esta en el bucle gateado con `[ -x ]` de "
                    "scripts/release.sh pero no es ejecutable; el paso 1b lo "
                    f"saltaria en silencio. `chmod +x {rel}`"
                )

    print(f"  en el bucle `[ -x ]` sin bit de ejecucion:  {len(non_executable)}")

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
            "se ha quedado corta, o hay entradas enumeradas que el 1b no ejecuta."
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
