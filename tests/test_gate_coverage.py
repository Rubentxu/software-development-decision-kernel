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
  - o el CAMINO DE RELEASE lo ejecuta, o
  - esta en `EXCEPTIONS` con un motivo escrito.

Y cuatro reglas que impiden que la lista de excepciones se pudra:

1. Una excepcion que **apunta a un test que ya no existe** es un FAIL: la
   excepcion quedo obsoleta y hay que borrarla.
2. Una excepcion para un test que **ya ejecuta el camino de release** es un
   FAIL: la razon caduco, y dejarla convertiria la lista en un cajon de
   sastre donde cualquier test puede acabar sin correr.
3. Un test **enumerado en el bucle gateado** (`for t in` con `[ -x ]`) **sin
   bit de ejecucion** es un FAIL: el paso lo saltaria en silencio.
4. Un test cuyo **unico runner esta en `.github/workflows/`** es un FAIL, y
   el mensaje lo dice. No es una agracia: es una consecuencia medida de una
   politica del repo.

POR QUE UN RUNNER DE CI NO CUENTA (Regla 4)
-------------------------------------------
AGENTS.md seccion 2.5, que es norma de este repo y no una preferencia:

    GitHub Actions cloud NO bloquea: sin required status checks, runs =
    evidencia asincrona.
    Prohibido esperar runs de la nube.

Es decir, un runner de CI **no bloquea nada en el camino de publicacion**. Un
test cuyo unico runner esta ahi se ha ejecutado, en el mejor caso, *despues*
de publicar, y nadie lo mira. Contarlo como cubierto es la misma clase de
error que la Regla 3: la cobertura que cuenta el nombre no es la cobertura
que ejecuta el bit.

MEDIDO al escribir esta regla: tres tests estaban solo en `ci.yml`, los tres
hermeticos, y uno de ellos --`test_release_state_pointer_mutation.sh`-- es el
autofalsador del guard que `af98f9af` cita como causa de 41 commits de deriva.
Corren 6,5 s en total. Y los tres PASABAN, luego el defecto no era de
correccion sino de cobertura: la release publicaba sin ejecutar nada que
maldiga.

El segundo defecto lo destapo el falsador de esta misma regla, y por eso
importa que exista: la Regla 2 miraba "tiene runner" en vez de "lo ejecuta el
release", con lo que declarar el motivo de un test que CI cubre era un FAIL.
Es decir, la Regla 4 se podia cumplir cableando, pero **no tenia salida
legitima** -- una regla sin salida obliga a la unica accion que no siempre es
la correcta.

POR QUÉ SE DESCARTAN LOS COMENTARIOS Y LOS SCOPES
--------------------------------------------------
Un test nombrado en un comentario **no está gated**, y uno nombrado en un
**scope de shellcheck** tampoco: `release.sh` mantiene una lista de ficheros
que shellcheck revisa, y esa lista se parecía a un runner sin serlo. La nota
de exclusión nombra dos tests precisamente porque NO se ejecutan, y un `in` a
pelo los puntuaba como cubiertos: la medición dio 7 sin runner donde había 13.

Contar prosa como cobertura es el mismo error que contar una declaración como
obediencia, y **contar una lista de lint como cobertura** es el mismo error con
un disfraz mas. Por eso la cobertura se mide sobre la SINTAXIS que ejecuta —
bucles `for ... in` e invocaciones `test_gate` / `bash` / `python3`— y no
sobre el nombre, que aparece en sitios donde no ocurre nada.

Salida: exit 0 si la propiedad se sostiene; 1 con el detalle si no.
"""

from __future__ import annotations

import os
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
TESTS_DIR = ROOT / "tests"

# Todo fichero que puede ejecutar un test, en DOS grupos y no en uno.
#
# El separation no es cosmetica. `RUNNERS` incluia `.github/workflows/*.yml`
# porque un test puede ser invocado por un guard en vez de por un `for t in`,
# y la inclusion era correcta para su contrato. Lo que no sevio es que
# AGENTS.md §2.5 declara que "GitHub Actions cloud NO bloquea: sin required
# status checks, runs = evidencia asincrona" y que "Prohibido esperar runs de
# la nube". Es decir: un runner de CI no BLOQUEA NADA en el camino de
# publicacion. Un test cuyo unico runner esta en CI se ha ejecutado, en el
# mejor caso, despues de publicar, y nadie lo mira.
#
# MEDIDO en esta sesion: tres tests estaban en esa situacion, los tres
# hermeticos, y uno de ellos es el autofalsador del guard del puntero de
# estado — el mismo guard que `af98f9af` cita como causa de 41 commits de
# deriva. Corren 6,5 s en total.
#
# Por eso los dos grupos se nombran: `RELEASE_RUNNERS` es lo que puede
# bloquear una publicacion, `CI_RUNNERS` es lo que no.
RELEASE_RUNNERS = [
    ROOT / "scripts" / "release.sh",
    *sorted((ROOT / "scripts").glob("*.sh")),
]
CI_RUNNERS = sorted((ROOT / ".github" / "workflows").glob("*.yml"))
RUNNERS = RELEASE_RUNNERS + CI_RUNNERS

# --- Que cuenta como GUARD, y que no -----------------------------------------
#
# MEDIDO: el censo era `tests/test_*.sh` mas `tests/test_*.py`, y el glifo
# `test_` en el nombre era la unica condicion de entrada. En `tests/` hay
# ejecutables que NO casan, y cinco de ellos son guards de verdad:
#
#   uat_ctx_001_adoption_convergence.sh
#   uat_ctx_003_durable_deltas.sh
#   uat_ctx_005_explicit_cycle_migration.sh
#   uat_ctx_006_skill_runtime_alignment.sh
#   uat_ctx_007_context_expand.sh
#
# MEDIDO: los cinco son hermeticos (0 red, 0 contenedores) y NADIE los
# ejecutaba — ni el 1b, ni `ci.yml`, ni ningun otro script. Y los cinco salen
# **2** cuando se lanzan hoy, porque exigen el binario release construido, que
# no existe: `error: binary not found or not executable`. O sea que el defecto
# no es que estuvieran rotos, es que no se ejecutaban, y su nombre era la
# unica razon por la que el gate no los habia visto nunca.
#
# Un censo por prefijo de nombre es un censo que hay que actualizar cuando
# alguien nombra un guard `uat_*` en vez de `test_*`, y ese recordatorio es
# justo lo que falla. Por eso el censo ahora es **todo ejecutable en
# `tests/`**, y lo que no es un guard se declara aqui con su motivo.
NOT_GUARDS: dict[str, str] = {
    "lib_public_release_gate.sh": (
        "no es un guard: es una LIBRERIA. Define `run_public_release_gate` y "
        "la sourcean `scripts/release-assets-contract.sh` y "
        "`tests/test_release_public_gate.sh`. Ejecutarla directamente no hace "
        "nada porque no tiene main. Censusarla como guard seria el mismo error "
        "al reves: llamar guard a una libreria."
    ),
    "ext_provider_gate.sh": (
        "no es un guard autonomous: es un LAUNCHER de frontera MCP_EXTERNAL. "
        "Sin argumentos sale 3 (not_run) y con un provider ausente sale 2 "
        "(blocked_external_dependency), que por contrato NUNCA es pass — "
        "cablearlo al 1b mataria toda release en una maquina sin `chronos-mcp`. "
        "Su otro mitad, el enum de estados, esta pineado por un test de "
        "produccion (`ext_outcome_states_match_the_launcher_contract`)."
    ),
    "falsify-ci-anchor-real.sh": (
        "falsificador del anchor de CI: requiere un tag publicado y red. "
        "Sufijo `_real` explicito, para que la regla de nombres de C3n (3f) "
        "pueda exigir que un nombre asi declare frontera."
    ),
}

# --- Que es EJECUTAR un test, y que es NOMBRARLO --------------------------------
#
# MEDIDO, y es la segunda mitad del mismo defecto. La primera version de la
# Regla 4 miraba `nombre in release_code`, con `release_code` siendo el TEXTO
# de `release.sh`. Y hay un sitio donde `release.sh` nombra un test sin
# ejecutarlo nunca: **el scope de shellcheck del 1b**, una lista de ficheros
# que shellcheck revisa. Cinco guards estaban ahi y en ningun otro sitio:
#
#   tests/test_release_final_state_figures.sh
#   tests/test_release_final_state_figures_mutation.sh
#   tests/test_lint_gate_scope_severity_mutation.sh
#   tests/test_release_bump_pointer_sync.sh
#   tests/test_reconcile_pointer_yaml_safety.sh
#
# MEDIDO contra el log de un release que paso el 1b entero: **0 apariciones
# de los cinco**, 0 en `ci.yml`, 0 en cualquier otro script. No los corre
# nadie. Y el changelog los publica como gates con su PASS:
# "Guard `test_release_final_state_figures.sh` `PASS=12 FAIL=0`".
#
# Es el mismo punto ciego que la Regla 3 ya tuvo y que su propio comentario
# describe: *"el alcance es el bucle gateado, NO todo el fichero"*. Repetirlo
# en la regla siguiente, en el mismo fichero, es la forma de que la leccion
# no se aprenda.
#
# Por eso aqui se separa EJECUTAR de NOMBRAR, y se hace por SINTAXIS y no por
# prosa: un test se ejecuta si esta en un bucle `for t in` / `for p in` o si
# aparece en una invocacion (`test_gate "X"`, `bash tests/X`,
# `python3 tests/X`). Estar en un scope de shellcheck, en un `ok`, o en un
# comentario es nombrarlo.
_TEST_PATH_RE = re.compile(r"tests/([A-Za-z0-9_.-]+\.(?:sh|py))")
_INVOKE_RE = re.compile(
    r"\b(?:bash|python3)\s+tests/[A-Za-z0-9_.-]+\.(?:sh|py)"
    r"|\btest_gate\s+\"([A-Za-z0-9_.-]+)\""
)


def tests_ejecutados(text: str) -> set[str]:
    """Nombres de los tests que `text` EJECUTA, no los que nombra.

    MEDIDO tres veces el mismo bug al escribir el extractor, y las tres por
    el dato y no por la lectura: (1) hacer `continue` en la linea que abre el
    bucle perdia el PRIMER elemento; (2) cerrar el bucle antes de recolectar
    perdia el ULTIMO, que es el unico sin continuacion; (3) olvidar poner
    `en_bucle = True` dejaba los bucles casi vacios. Un instrumento que se
    equivoca en el caso mas obvio no puede usarse para acusar a nadie, y por
    eso la extraccion se valida contra el log de un release real.
    """
    ejecuta: set[str] = set()
    en_bucle = False
    for raw in text.splitlines():
        s = raw.strip()
        if s.startswith("#"):
            continue
        code = raw.split(" #")[0]

        abriendo = bool(re.match(r"for\s+[a-z]\s+in\b", s))
        cerrando = s.endswith("; do")
        # `esta_en_bucle` describe la LINEA; `en_bucle` describe lo que viene
        # despues. Confundir las dos cosas es el bug (2) de la lista.
        esta_en_bucle = en_bucle or abriendo

        if cerrando:
            en_bucle = False
        elif abriendo:
            en_bucle = True

        if esta_en_bucle:
            ejecuta.update(_TEST_PATH_RE.findall(code))
            continue
        for m in _INVOKE_RE.finditer(code):
            if m.group(1):
                ejecuta.add(m.group(1))
            else:
                ejecuta.update(_TEST_PATH_RE.findall(m.group(0)))
    return ejecuta

# Tests que NO deben correr en un gate automático, cada uno con su motivo.
# Añadir aquí una entrada es una decisión, no un descuido: por eso el motivo es
# parte de la entrada y las reglas 1 y 2 la mantienen honesta.
EXCEPTIONS: dict[str, str] = {
    "test_h05_isolation.sh": (
        "pasa sin medir: sin el rlib release imprime `skip:` y aun asi reporta "
        "PASS=1 FAIL=0. Cablearlo devolveria un verde vacio (misma forma que "
        "INC-DEBT-054). Requiere `cargo build --release -p sddk-engine` antes."
    ),
    # Los cinco `uat_ctx_*`: integracion end-to-end que exige el BINARIO
    # release construido. Misma condicion que `test_h05_isolation.sh` de
    # arriba, y el motivo se escribe una vez y aqui se referencia.
    #
    # MEDIDO, y el motivo NO es "no son hermeticos" —que serian: 0 red, 0
    # contenedores—, sino ORDEN. Los cinco salen 2 hoy con
    # `sddk binary not found or not executable`, y el binario lo construye el
    # release en el paso **3**, mientras el 1b corre en el **1b**: no puede
    # estar en el 1b por una razon de secuencia, no por una limitacion de la
    # maquina. Con `--bin <ruta>` si accepts, luego no estan rotos: no se
    # ejecutaban, y su unica razon para no estar en el censo era no llevar el
    # glifo `test_` en el nombre.
    "uat_ctx_001_adoption_convergence.sh": (
        "integracion E2E: exige el binario release construido, que el release "
        "construye en el paso 3 y el 1b corre antes. Ver `uat_ctx_007` para el "
        "motivo comun a los cinco."
    ),
    "uat_ctx_003_durable_deltas.sh": (
        "integracion E2E: exige el binario release construido (paso 3, "
        "posterior al 1b). Ver `uat_ctx_007` para el motivo comun."
    ),
    "uat_ctx_005_explicit_cycle_migration.sh": (
        "integracion E2E: exige el binario release construido (paso 3, "
        "posterior al 1b). Ver `uat_ctx_007` para el motivo comun."
    ),
    "uat_ctx_006_skill_runtime_alignment.sh": (
        "integracion E2E: exige el binario release construido (paso 3, "
        "posterior al 1b). Ver `uat_ctx_007` para el motivo comun."
    ),
    "uat_ctx_007_context_expand.sh": (
        "integracion E2E: exige el binario release construido, que el release "
        "construye en el paso 3 y el 1b corre antes. No es una limitacion de la "
        "maquina: los cinco aceptan `--bin <ruta>` y con un binario presente "
        "ejercitan de verdad. Lo que NO puede ser es un gate del 1b, porque "
        "aun no existe el binario que necesitan. Su sitio natural es un paso "
        "posterior al 3, y cablearlos ahi es la decision pendiente, no un "
        "detalle de este commit."
    ),
    "clean_machine_uat.sh": (
        "levantador de una maquina limpia: 39 llamadas a `docker`/`podman` y "
        "4 de red. Monta contenedores de verdad, luego no es reproducible en "
        "el 1b local. Lo ejecuta `.github/workflows/clean-machine-uat.yml`, "
        "que ademas lo gatilla por tag; AGENTS.md 2.5 dice que el CI no "
        "bloquea, luego es evidencia asincrona por diseno, no un hueco."
    ),
    "uat_ctx_002_context_bootstrap.sh": (
        "integracion E2E: exige el binario release construido, igual que los "
        "otros cuatro `uat_ctx_*`. Ver `uat_ctx_007` para el motivo comun. Lo "
        "Ejecuta `ci.yml`, que segun AGENTS.md 2.5 no bloquea."
    ),
    "uat_ctx_004_cycle_inference.sh": (
        "integracion E2E: exige el binario release construido, igual que los "
        "cinco anteriores. Ver `uat_ctx_007` para el motivo comun. MEDIDO: sale "
        "2 con `sddk binary not found`, acepta `--bin <ruta>`, y es el sexto de "
        "la familia — se habria colado porque `ci.yml` lo nombra, que es "
        "justamente el patron que este cambio viene a cerrar."
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


def main() -> int:
    def ejecuta_de(paths: list[pathlib.Path]) -> set[str]:
        out: set[str] = set()
        for p in paths:
            if p.exists():
                out |= tests_ejecutados(p.read_text(encoding="utf-8"))
        return out

    release_ejecuta = ejecuta_de(RELEASE_RUNNERS)
    ci_ejecuta = ejecuta_de(CI_RUNNERS)

    # El censo es TODO lo ejecutable en `tests/`, no lo que lleva `test_` en el
    # nombre. Ver NOT_GUARDS para por que esa condicion se quedo corta.
    #
    # MEDIDO, y es un defecto de la v1 de este cambio: el censo filtraba por
    # `os.X_OK`, con lo que `test_release_routes_parity.sh` y
    # `test_release_bundle_parity.sh` —que existen y estan en EXCEPTIONS—
    # desaparecian del censo, y la Regla 1 los acusaba de "el fichero no
    # existe". El mensaje era falso y la propiedad que de verdad importa es la
    # de la Regla 3: enumerado sin bit se SALTA en silencio. Son dos
    # preguntas distintas y el filtro las mezclaba. El censo es **todo `.sh`
    # y `.py` de `tests/`**; el bit lo comprueba la Regla 3, que es donde vive
    # y donde ya vivia.
    tests = sorted(TESTS_DIR.glob("*.sh")) + sorted(TESTS_DIR.glob("*.py"))
    names = [t.name for t in tests if t.name not in NOT_GUARDS]

    failures: list[str] = []
    notes: list[str] = []

    # Contabilidad explicita y DISJUNTA. La primera version de este fichero
    # derivaba las cifras por resta y reportaba "excepcionados: 0" con seis
    # excepciones vivas — un guard que miente sobre sus propias cifras no
    # puede usarse para justificar por que el resto pasa.
    #
    # Todo se cuenta sobre EJECUCION, no sobre mencion. Un test que aparece
    # en el scope de shellcheck del 1b esta nombrado, no ejecutado, y esa
    # distincion es la que la Regla 4 mide.
    release_cubierto = {n for n in names if n in release_ejecuta}
    ci_cubierto = {n for n in names if n in ci_ejecuta}
    excepted = {n for n in names if n not in release_cubierto and n in EXCEPTIONS}
    uncovered = {n for n in names if n not in release_cubierto
                 and n not in ci_cubierto and n not in EXCEPTIONS}
    # Regla 4: lo ejecuta CI y no el camino de release. AGENTS.md 2.5 dice que
    # el CI cloud no bloquea, luego no bloquea nada antes de publicar.
    ci_only = {n for n in names if n in ci_cubierto
               and n not in release_cubierto and n not in EXCEPTIONS}

    for name in sorted(uncovered):
        failures.append(
            f"{name}: NADIE lo ejecuta y no esta en EXCEPTIONS con un motivo. "
            f"Estar nombrado en el scope de shellcheck de release.sh o en un "
            f"comentario NO es ejecucion"
        )

    for name in sorted(ci_only):
        failures.append(
            f"{name}: su UNICO runner es .github/workflows/, que no bloquea "
            "(AGENTS.md 2.5); se ejecuta, en el mejor caso, DESPUES de "
            "publicar. Cablealo en scripts/release.sh si es hermetico, o "
            "anadelo a EXCEPTIONS con el motivo escrito"
        )

    covered = release_cubierto | ci_cubierto
    print(f"  tests en tests/test_*:        {len(names)}")
    print(f"  con runner:                   {len(covered)}")
    print(f"    de los cuales, solo en CI:   {len(ci_only)}")
    print(f"  excepcionados con motivo:     {len(excepted)}")
    print(f"  SIN runner y SIN motivo:      {len(uncovered)}")

    # Regla 1: excepcion que apunta a un test inexistente.
    for name in sorted(set(EXCEPTIONS) - set(names)):
        failures.append(
            f"{name}: figura en EXCEPTIONS pero el fichero no existe; "
            "la excepcion quedo obsoleta y hay que borrarla"
        )

    # Regla 5: `NOT_GUARDS` con la misma exigencia que EXCEPTIONS.
    #
    # Sin esto, la lista que acabo de crear seria un cajon de sastre con mejor
    # vocabulario: un guard que se declara "no es un guard" se queda fuera del
    # censo para siempre y nadie vuelve a mirar por que. Las tres condiciones
    # que la mantienen honesta son las de EXCEPTIONS, y por la misma razon.
    presentes = {p.name for p in TESTS_DIR.glob("*.sh")}
    for name in sorted(set(NOT_GUARDS) - presentes):
        failures.append(
            f"{name}: figura en NOT_GUARDS pero el fichero no existe; "
            "la nota quedo obsoleta y hay que borrarla"
        )
    for name in sorted(set(NOT_GUARDS) & release_cubierto):
        failures.append(
            f"{name}: figura en NOT_GUARDS pero el camino de release lo "
            "ejecuta; la razon caducó y dejarla convertiria NOT_GUARDS en un "
            "cajon de sastre"
        )
    for name in sorted(set(EXCEPTIONS) & set(NOT_GUARDS)):
        failures.append(
            f"{name}: esta a la vez en EXCEPTIONS y en NOT_GUARDS; son dos "
            "salidas distintas para lo mismo y una de las dos esta caducada"
        )

    # Regla 2: excepcion para un test que ya se ejecuta en el camino de
    # release.
    #
    # MEDIDO: miraba `covered`, que incluye los workflows de CI, y con eso la
    # regla era indemostrable. Un test se mete en EXCEPTIONS precisamente
    # porque el 1b NO puede correrlo —necesita red, contenedores, o un
    # artefacto que no existe todavia— y que ademas lo corra CI es lo
    # deseado, no la senal de que la razon caducó. Con la regla antigua,
    # declarar el motivo de un test que CI cubre era un FAIL, o sea que
    # EXCEPTIONS no tenia salida para casi nada y la Regla 4 de este mismo
    # gate solo se podia cumplir cableando en el 1b.
    #
    # Lo que la regla sigue atrapando es lo que debe atrapar: una excepcion
    # para un test que ya corre en el 1b es un cajon de sastre, porque la
    # razon ya no aplica y nadie va a mirar la lista.
    release_covered = release_cubierto
    for name in sorted(set(EXCEPTIONS) & release_covered):
        failures.append(
            f"{name}: figura en EXCEPTIONS pero ya lo ejecuta el camino de "
            "release; la razon caducó y dejarla convertiria EXCEPTIONS en un "
            "cajon de sastre"
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
            "RESULT: FAIL — la superficie de gates del CAMINO DE RELEASE esta "
            "incompleta: o falta cablear, o el motivo escrito caducó."
        )
        print(
            "         Cablear el test en scripts/release.sh si es hermetico. "
            "Estar solo en .github/workflows NO cuenta:"
        )
        print(
            "         AGENTS.md 2.5 declara que el CI cloud no bloquea, luego un "
            "runner de CI se ejecuta, en el mejor caso, despues de publicar."
        )
        print(
            "         Anadirlo a EXCEPTIONS con el motivo escrito si necesita red, "
            "contenedores o un artefacto ausente."
        )
        return 1

    print()
    print(
        "RESULT: PASS — todo test lo ejecuta el camino de release, o tiene un "
        "motivo escrito para no hacerlo."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
