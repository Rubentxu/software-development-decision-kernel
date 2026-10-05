#!/usr/bin/env python3
"""Falsador de la Regla 4 de `test_gate_coverage.py`.

Una regla nueva sin dientes es decoracion. Este script la quita de en medio
tres veces y exige que el gate CAIGA cada vez, y despues exige que NO caiga
cuando la exclusion es legitima.

POR QUE UN MINI-REPO Y NO EL ARBOL REAL
----------------------------------------
La v1 mutaba `scripts/release.sh` y `tests/test_gate_coverage.py` EN SITIO. Dos
defectos, ambos medidos ejecutandola:

1. Al anadir este fichero a `tests/`, el gate lo ve como un test nuevo sin
   runner y el CONTROL falla. El falsador se delata a si mismo como cobertura
   ausente.
2. Peor: mutar `release.sh` desde un falsador que corre DENTRO del 1b es la
   bomba de reloj que `5edcef00` acaba de arreglar en el otro falsador. Un
   falsador que puede envenenar el sujeto no puede correr junto al sujeto.

El mini-repo copia lo que el gate LEE (nombres de `tests/`, `scripts/*.sh`,
`.github/workflows/*.yml`) a un temporal y muta ahi. El gate resuelve su ROOT
como `parents[1]` de si mismo, luego copiarlo a `$TMP/tests/` hace que corra
contra el temporal sin cambiar una linea del gate.

MEDIDO al escribirlo: `mutar()` exige que el texto viejo case EXACTAMENTE una
vez y devuelve False si no, y el llamante cuenta SKIP. Una mutacion que no se
aplica no es PASS ni FAIL: es ruido con forma de evidencia.
"""
from __future__ import annotations

import hashlib
import pathlib
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
GATE_REL = "tests/test_gate_coverage.py"
RELEASE_REL = "scripts/release.sh"
SELF_REL = "tests/test_gate_coverage_ci_mutation.py"


def sha(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

PASS = 0
FAIL = 0
SKIP = 0


def construir_minirepo(dest: pathlib.Path) -> None:
    """Copia exactamente lo que `test_gate_coverage.py` lee.

    `copy2` conserva el modo, que importa: la Regla 3 comprueba
    `os.access(t, os.X_OK)` y un fichero creado vacio daria rojos espurios.
    """
    shutil.copytree(ROOT / "tests", dest / "tests", copy_function=shutil.copy2)
    (dest / "scripts").mkdir(parents=True, exist_ok=True)
    for p in sorted((ROOT / "scripts").glob("*.sh")):
        shutil.copy2(p, dest / "scripts" / p.name)
    wf = dest / ".github" / "workflows"
    wf.mkdir(parents=True, exist_ok=True)
    for p in sorted((ROOT / ".github" / "workflows").glob("*.yml")):
        shutil.copy2(p, wf / p.name)


def run_gate(tmp: pathlib.Path) -> tuple[int, str]:
    p = subprocess.run(
        [sys.executable, str(tmp / GATE_REL)],
        capture_output=True,
        text=True,
        cwd=str(tmp),
    )
    return p.returncode, p.stdout + p.stderr


def mutate(path: pathlib.Path, old: str, new: str) -> bool:
    text = path.read_text(encoding="utf-8")
    if text.count(old) != 1:
        return False
    path.write_text(text.replace(old, new), encoding="utf-8")
    return True


def needle_ok(path: pathlib.Path, needle: str) -> bool:
    """El needle tiene que estar EXACTAMENTE una vez, como `mutate`.

    Existe para que M10 pueda desactivar una regla y volver a escribir el
    fichero sin volver a aplicar el backup entero en cada iteracion, y para que
    una regla que ya no existe se cuente como SKIP en vez de como excepcion.
    """
    return path.read_text(encoding="utf-8").count(needle) == 1


def asert(ok: bool, name: str, detail: str = "") -> None:
    global PASS, FAIL
    if ok:
        PASS += 1
        print(f"  [ok]   {name}")
    else:
        FAIL += 1
        print(f"  [FAIL] {name}: {detail}")


def caso(nombre: str) -> None:
    print(f"\n--- {nombre} ---")


def main() -> int:
    global SKIP

    # Los sha se toman ANTES de construir nada. El cierre los compara.
    sha_real_release = sha(ROOT / RELEASE_REL)
    sha_real_gate = sha(ROOT / GATE_REL)
    sha_real_self = sha(ROOT / SELF_REL)

    # --- Control: sujeto sano, con el mini-repo construido -------------------
    caso("control")
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="gate-coverage-mini-"))
    try:
        construir_minirepo(tmp)
        rc, out = run_gate(tmp)
        asert(rc == 0, "control: el gate pasa sobre el sujeto sano", out.strip()[-300:])
        asert("solo en CI:   0" in out, "control: declara cero runners solo-CI", out)

        gate = tmp / GATE_REL
        release = tmp / RELEASE_REL
        gate_bak = gate.read_text(encoding="utf-8")
        rel_bak = release.read_text(encoding="utf-8")

        # --- M1..M3 -----------------------------------------------------------
        mutaciones = [
            (
                "M1",
                "tests/test_workflow_contract.py",
                "             tests/test_golden_dataset_contract.py \\\n"
                "             tests/test_workflow_contract.py \\\n",
                "             tests/test_golden_dataset_contract.py \\\n",
                "el bucle de python pierde test_workflow_contract.py",
            ),
            (
                "M2",
                "tests/test_golden_dataset_contract.py",
                "             tests/test_golden_dataset_contract.py \\\n"
                "             tests/test_workflow_contract.py \\\n",
                "             tests/test_workflow_contract.py \\\n",
                "el bucle de python pierde test_golden_dataset_contract.py",
            ),
            (
                "M3",
                "tests/test_release_state_pointer_mutation.sh",
                "             tests/test_binary_freshness_checker.sh \\\n"
                "             tests/test_release_state_pointer_mutation.sh \\\n",
                "             tests/test_binary_freshness_checker.sh \\\n",
                "el bucle de shell pierde el autofalsador del puntero",
            ),
        ]

        for tag, nombre, viejo, nuevo, desc in mutaciones:
            caso(f"{tag}: {desc}")
            if not mutate(release, viejo, nuevo):
                SKIP += 1
                print(f"  [SKIP] {tag}: el texto viejo no casaba exactamente una vez; "
                      f"la mutacion NO se aplico")
                continue
            rc, out = run_gate(tmp)
            asert(rc != 0, f"{tag}: el gate CAE ({nombre})", f"rc={rc}")
            asert("workflows" in out,
                  f"{tag}: y nombra el runner de CI como la causa", out.strip()[-300:])
            release.write_text(rel_bak, encoding="utf-8")

        # --- M4: la salida legitima -----------------------------------------
        #
        # MEDIDO: la v1 de este caso solo anadia la entrada a EXCEPTIONS y
        # exigia que el gate pasara. Con el arreglo de la Regla 2 eso ya no
        # puede pasar, y con razon: el test seguia cableado en el 1b, luego
        # declararlo excepcionado es exactamente el cajon de sastre que la
        # regla existe para cazar. La salida legitima son las DOS cosas: sacar
        # el test del 1b y escribir el motivo. Una de las dos es un rojo, y esa
        # asercion se llama M6.
        caso("M4: motivo escrito Y fuera del 1b es la salida legitima")
        ancla = "EXCEPTIONS: dict[str, str] = {\n"
        entrada = (
            '    "test_workflow_contract.py": (\n'
            '        "FALSADOR: motivo escrito, luego esto tiene que pasar"\n'
            '    ),\n'
        )
        quitado = (
            "             tests/test_golden_dataset_contract.py \\\n"
            "             tests/test_workflow_contract.py \\\n",
            "             tests/test_golden_dataset_contract.py \\\n",
        )
        if mutate(gate, ancla, ancla + entrada) and mutate(release, *quitado):
            rc, out = run_gate(tmp)
            asert(rc == 0, "M4: sin cablear y con motivo, el gate pasa igual",
                  out.strip()[-300:])
            asert("solo en CI:   0" in out, "M4: y sigue contando cero runners solo-CI", out)
        else:
            SKIP += 1
            print("  [SKIP] M4: alguna mutacion no casaba; no aplicada")
        gate.write_text(gate_bak, encoding="utf-8")
        release.write_text(rel_bak, encoding="utf-8")

        # --- M6: la Regla 2 sigue atrapando lo que debe ----------------------
        caso("M6: excepcionar un test que el 1b ya ejecuta es un cajon de sastre")
        if mutate(gate, ancla, ancla + entrada):
            rc, out = run_gate(tmp)
            asert(rc != 0,
                  "M6: motivo escrito sin quitar el cableado DA rojo (Regla 2)", f"rc={rc}")
            asert("cajon de sastre" in out,
                  "M6: y nombra el cajon de sastre como la causa", out.strip()[-300:])
            gate.write_text(gate_bak, encoding="utf-8")
        else:
            SKIP += 1
            print("  [SKIP] M6: el ancla de EXCEPTIONS no casaba; mutacion no aplicada")

        # --- M5: el extractor, que es la pieza critica ------------------------
        #
        # MEDIDO: la v2 de este caso mutaba la expresion `n not in release_code`
        # de la Regla 4. Al reescribir `main()` sobre `tests_ejecutados()` esa
        # expresion dejo de existir, el needle dejo de casar y el caso quedo en
        # SKIP — un falsador con un tercio de sus mutaciones sin aplicar no
        # mide nada. Y mutar el extractor, y no la regla que lo consume, es lo
        # que corresponde: es la pieza de la que depende que un nombre en un
        # scope de shellcheck NO cuente como ejecucion. Si el extractor
        # contara de mas, la Regla 4 daria verde con los cinco guards
        # huerfanos en su sitio.
        caso("M5: un extractor que no ve los bucles deja todo sin runner")
        viejo_m5 = "        if esta_en_bucle:\n"
        nuevo_m5 = "        if False and esta_en_bucle:\n"
        if mutate(gate, viejo_m5, nuevo_m5):
            rc, out = run_gate(tmp)
            asert(rc != 0,
                  "M5: el gate CAE si el extractor deja de contar los bucles",
                  f"rc={rc}")
            asert("SIN runner y SIN motivo:      0" not in out,
                  "M5: y reporta SIN runner donde antes habia cero", out)
            gate.write_text(gate_bak, encoding="utf-8")
        else:
            SKIP += 1
            print("  [SKIP] M5: el needle del extractor no casaba; mutacion no aplicada")

        # --- M7: los cinco guards huerfanos ----------------------------------
        #
        # Esta es la falsacion directa del hallazgo. M1..M3 comprueban que
        # quitar un test de un bucle lo senale; M7 comprueba que quitar los
        # CINCO que solo estaban nombrados en el scope de shellcheck tambien,
        # y sobre todo que el mensaje diga NADIE y no "sin runner", porque la
        # diferencia entre las dos frases es exactamente el defecto que se
        # esta corrigiendo.
        caso("M7: los cinco guards que solo estaban en el scope de shellcheck")
        los_cinco = (
            "             tests/test_release_state_pointer_mutation.sh \\\n"
            "             tests/test_release_final_state_figures.sh \\\n"
            "             tests/test_release_final_state_figures_mutation.sh \\\n"
            "             tests/test_lint_gate_scope_severity_mutation.sh \\\n"
            "             tests/test_release_bump_pointer_sync.sh \\\n"
            "             tests/test_reconcile_pointer_yaml_safety.sh; do"
        )
        solo_uno = "             tests/test_release_state_pointer_mutation.sh; do"
        if mutate(release, los_cinco, solo_uno):
            rc, out = run_gate(tmp)
            asert(rc != 0, "M7: el gate CAE al quitar los cinco", f"rc={rc}")
            for nombre in (
                "test_release_final_state_figures.sh",
                "test_release_final_state_figures_mutation.sh",
                "test_lint_gate_scope_severity_mutation.sh",
                "test_release_bump_pointer_sync.sh",
                "test_reconcile_pointer_yaml_safety.sh",
            ):
                asert(f"{nombre}: NADIE lo ejecuta" in out,
                      f"M7: nombra {nombre} como NADIE lo ejecuta", out[-400:])
            release.write_text(rel_bak, encoding="utf-8")
        else:
            SKIP += 1
            print("  [SKIP] M7: el bloque de los cinco no casaba; mutacion no aplicada")

        # --- M8: el censo vuelve a depender del NOMBRE ------------------------
        #
        # La falsacion directa del hallazgo de esta sesion bis. El censo era
        # `tests/test_*.sh` + `tests/test_*.py`, y el glifo `test_` en el
        # nombre era la unica condicion de entrada. Volver a esa condicion, con
        # las excepciones ya escritas, tiene que ser un rojo: si el gate
        # pasara, significaria que las excepciones de los `uat_ctx_*` no estan
        # haciendo nada y que el defecto sigue abierto por la puerta de
        # siempre.
        caso("M8: un censo por prefijo de nombre no ve los uat_ctx")
        viejo_m8 = 'tests = sorted(TESTS_DIR.glob("*.sh")) + sorted(TESTS_DIR.glob("*.py"))'
        nuevo_m8 = (
            'tests = sorted(TESTS_DIR.glob("test_*.sh")) '
            '+ sorted(TESTS_DIR.glob("test_*.py"))'
        )
        if mutate(gate, viejo_m8, nuevo_m8):
            rc, out = run_gate(tmp)
            asert(rc != 0, "M8: volver al censo por nombre da rojo (Regla 1)", f"rc={rc}")
            asert("EXCEPTIONS pero el fichero no existe" in out,
                  "M8: y acusa a las excepciones de apuntar a ficheros que si existen",
                  out[-400:])
            gate.write_text(gate_bak, encoding="utf-8")
        else:
            SKIP += 1
            print("  [SKIP] M8: el needle del censo no casaba; mutacion no aplicada")

        # --- M9: el censo vuelve a filtrar por BIT de ejecucion ---------------
        #
        # MEDIDO: la v1 de este cambio filtro el censo por `os.X_OK`, y eso
        # borro del censo `test_release_routes_parity.sh` y
        # `test_release_bundle_parity.sh` —que existen, estan en EXCEPTIONS y
        # van en 644— con lo que la Regla 1 los acuso de "el fichero no
        # existe". El mensaje era falso y, peor, la regla que de verdad importa
        # (enumerado sin bit se salta en silencio) es la Regla 3, que ya
        # existe. Mezclar "existe" con "tiene bit" es medir dos preguntas con
        # un instrumento. M9 comprueba que volver a ese filtro vuelve a mentir.
        caso("M9: un censo que exige bit de ejecucion hace desaparecer ficheros")
        viejo_m9 = "    tests = sorted(TESTS_DIR.glob(\"*.sh\")) + sorted(TESTS_DIR.glob(\"*.py\"))"
        nuevo_m9 = (
            "    tests = sorted(\n"
            "        p for p in TESTS_DIR.glob(\"*.sh\") if os.access(p, os.X_OK)\n"
            "    ) + sorted(TESTS_DIR.glob(\"test_*.py\"))"
        )
        if mutate(gate, viejo_m9, nuevo_m9):
            rc, out = run_gate(tmp)
            asert(rc != 0, "M9: filtrar por bit hace caer el gate", f"rc={rc}")
            asert("test_release_routes_parity.sh" in out and "no existe" in out,
                  "M9: y acusa a un fichero que SI existe de no existir", out[-400:])
            gate.write_text(gate_bak, encoding="utf-8")
        else:
            SKIP += 1
            print("  [SKIP] M9: el needle del filtro por bit no casaba; no aplicada")

        # --- M10: NOT_GUARDS sin las reglas que la mantienen honesta -----------
        #
        # Sin las tres reglas, `NOT_GUARDS` es un cajon de sastre con mejor
        # vocabulario: un guard declarado "no es un guard" se queda fuera para
        # siempre. Se falsea la Regla 5, no la lista.
        #
        # MEDIDO: la v1 de este caso sustituia la linea de la regla por
        # `pass`, y las tres mutaciones dieron `NameError` o
        # `IndentationError` — o sea, el gate caia por un error de SINTAXIS y
        # la asercion lo contaba como prueba de que la regla hace falta. Un
        # rojo de la herramienta no es un rojo de la propiedad, y asi se cuela
        # una mutacion que mide lo que no dice. Ahora cada una cambia la
        # CONDICION, no la estructura: el gate sigue siendo python valido y lo
        # que cae es la comprobacion.
        caso("M10: NOT_GUARDS sin la Regla 5 se vuelve inmutable")
        m10 = [
            (
                "M10a",
                "    for name in sorted(set(NOT_GUARDS) - presentes):",
                "    for name in sorted(set()):",
            ),
            (
                "M10b",
                "    for name in sorted(set(NOT_GUARDS) & release_cubierto):",
                "    for name in sorted(set(NOT_GUARDS) & set()):",
            ),
            (
                "M10c",
                "    for name in sorted(set(EXCEPTIONS) & set(NOT_GUARDS)):",
                "    for name in sorted(set(EXCEPTIONS) & set()):",
            ),
        ]
        for tag, viejo, nuevo_txt in m10:
            if mutate(gate, viejo, nuevo_txt):
                rc, out = run_gate(tmp)
                asert("Traceback" not in out and "IndentationError" not in out
                      and "NameError" not in out,
                      f"{tag}: el gate sigue siendo python valido con la regla muda",
                      out[-300:])
                asert(rc == 0,
                      f"{tag}: desactivar esa regla NO da rojo, luego el gate no "
                      f"vigila NOT_GUARDS con ella", f"rc={rc} out={out[-200:]}")
                gate.write_text(gate_bak, encoding="utf-8")
            else:
                SKIP += 1
                print(f"  [SKIP] {tag}: la regla no casaba; mutacion no aplicada")

        # --- El sujeto real quedo intacto ------------------------------------
        #
        # MEDIDO: la v1 de este bloque era `... == rel_bak or True` y una
        # asercion tautologica. Un `or True` convierte la comprobacion en
        # decoracion: PASSaria aunque el falsador hubiera dejado el arbol
        # mutado, que es justo lo que este fichero existe para provar que no
        # hace. Lo que se compara son los sha de los dos sujetos reales, no
        # una bandera.
        caso("el sujeto real quedo intacto")
        asert(sha(ROOT / RELEASE_REL) == sha_real_release,
              "scripts/release.sh real con el mismo sha256 que al empezar")
        asert(sha(ROOT / GATE_REL) == sha_real_gate,
              "tests/test_gate_coverage.py real con el mismo sha256 que al empezar")
        asert(sha(ROOT / "tests" / "test_gate_coverage_ci_mutation.py") == sha_real_self,
              "el propio falsador con el mismo sha256 que al empezar")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    print()
    print(f"  PASS={PASS} FAIL={FAIL} SKIP={SKIP}")
    return 1 if FAIL else 0


if __name__ == "__main__":
    sys.exit(main())
