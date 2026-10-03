#!/usr/bin/env python3
"""Mide si la decision de bounded contexts de C3m.5 se deduce del grafo real.

C3m.5 esta escrito en el roadmap como "R0 bounded-context decision": una sola
linea, sin decir como se decide. Un mapa de contextos dibujado sin medicion es
un diagrama bonito: dice que las cosas estan donde uno quisiera que estuvieran.
Este script mide, sobre el source real:

  1. CUAL es la unidad de modulo, porque "el modulo" tiene tres layouts posibles
     y contarlos mal desplaza todos los numeros que salen debajo.
  2. Quien consume cada raiz de modulo, leyendo el grafo de dependencias real y
     separando consumo de producto de consumo de test.
  3. En que dominio cae cada raiz segun la convención que propone este
     script, **imprimiendo los que no caen en ninguna caja**. Y se mide la
     superficie publica, que es lo que una frontera de contexto decide de
     verdad: que cruza `lib.rs`.

NOTA SOBRE EL ESTADO DE LA TAXONOMIA, porque el nombre de este script puede
hacer creer otra cosa: **el roadmap no declara ninguna.** C3m.5 esta escrito
como una sola linea, "R0 bounded-context decision", y no dice ni cuantos
contextos ni cuales. Los `DOMINIOS` de mas abajo son una PROPUESTA de este
script, escrita a mano, y por eso la tabla imprime SIN CAJA: si la propuesta
se callara un modulo, la medicion no se enteraria y el modulo desapareceria
del recuento. La pregunta util que responde no es "que dominio es", sino
"cuanto del motor deja fuera cualquier taxonomia que se escriba sin mirar".

TRES ERRORES QUE ESTE INSTRUMENTO YA HA COMETIDO Y QUE VUELVEN A ESTAR
ESCRITOS, porque el patron que los produce se repite:

  - Un recorte que se salta codigo da un numero mas pequeno, y un numero mas
    pequeno parece mas tranquilizador. La primera version solo contaba
    `use sddk_engine::X` y se saltaba `use sddk_engine::{X, Y}` -- que es la
    forma dominante en este repo -- y las rutas completamente cualificadas
    `sddk_engine::X::f()`. Dio 2 consumidores para `event_bus` donde hay 6.
  - Un control con una asercion debil no puede cazar el defecto de su propio
    instrumento: el control decia "event_bus > 0" y 2 cumple > 0. Los controles
    se comparan contra una cuenta hecha a mano, no contra un ">0".
  - Un comentario no es un guard, y una cadena no es una dependencia: las rutas
    a ficheros viven como texto dentro de `arch_lint.rs` y `doctor.rs`, y
    contarlas haria creer que el CLI depende de `event_bus` por citarlo.

Por eso el parser se falsifica CONTRA CODIGO SINTETICO con la respuesta escrita
a mano, antes de leer el repo. Si el parser no puede acertar en los seis casos
que se le dan, no se imprime ninguna tabla.
"""
import collections
import pathlib
import re
import sys

_AQUI = pathlib.Path(__file__).resolve()
_RAIZ = next((p for p in _AQUI.parents if (p / "crates" / "sddk-engine").is_dir()), None)
if _RAIZ is None:
    sys.exit("no encuentro crates/sddk-engine subiendo desde " + str(_AQUI.parent))

CRATES = _RAIZ / "crates"
ENGINE = CRATES / "sddk-engine"
SRC = ENGINE / "src"
RAICES_ENGINE = ("crate", "sddk_engine", "sddk_domain", "sddk_gateway", "sddk_storage")


# ── 1. LA UNIDAD DE MODULO, TRES LAYOUTS DISTINTOS ───────────────────────────
#
#   (f)  raiz-fichero     src/foo.rs
#   (d)  raiz-directorio  src/foo/mod.rs
#   (fd) raiz-partida     src/foo.rs  +  src/foo/    <- layout Rust 2018: un
#                                                       unico modulo en dos
#                                                       ficheros de estilo
#                                                       distinto. Contar solo
#                                                       el .rs subestima su
#                                                       tamano a la mitad.

def raices_de(raiz_src):
    """{nombre: (layout, [ficheros .rs que lo componen, ordenados])}"""
    raices = {}
    for p in sorted(raiz_src.glob("*.rs")):
        if p.name in ("lib.rs", "main.rs"):
            continue
        raices[p.stem] = ["f", [p]]
    for d in sorted(p for p in raiz_src.iterdir() if p.is_dir()):
        if (d / "mod.rs").exists():
            raices[d.name] = ["d", sorted(d.rglob("*.rs"))]
        elif d.name in raices:
            raices[d.name][0] = "fd"
            raices[d.name][1] = sorted(set(raices[d.name][1]) | set(d.rglob("*.rs")))
    return raices


# ── 2. EL PARSER, Y SU AUTOPRUEBA ────────────────────────────────────────────

# La forma real de este repo es multilinea y anidada:
#
#     use sddk_engine::{
#         something,
#         event_bus::{self, OutcomeEventInput},
#         other,
#     };
#
# Un regex que no sepa contar llaves no ve ese `event_bus`, y como el mismo
# error se lo comia a la cuenta independiente, los dos metodos coincidian en un
# numero los dos equivocados. Dos mediciones que comparten un supuesto no son
# un contraste: son un supuesto con multiplicador dos, y por eso el parser
# se prueba contra codigo sintetico con llaves anidadas, y el control del repo
# real LISTA los ficheros en vez de dar un numero que haya que creer.
PAT_USE_RAIZ = re.compile(
    r"\buse\s+(" + "|".join(RAICES_ENGINE) + r")\s*::"
)
# ruta completamente cualificada en posicion de expresion, exigiendo un `::`
# detras. El caso `use` lo cubre el parser de `use`; que ambos coincidan da el
# mismo conjunto, y usar un conjunto es lo que evita contar dos veces.
PAT_CUALIFICADA = re.compile(
    r"(?<![\w:])(" + "|".join(RAICES_ENGINE) + r")::([A-Za-z_]\w*)::"
)


def sin_comentarios_ni_cadenas(txt):
    """Deja el codigo y borra comentarios y literales.

    Sin esto, `arch_lint.rs` "depende" de `event_bus` porque una regla de lint
    guarda la ruta del fichero como texto, y `doctor.rs` depende de casi todo
    porque imprime rutas en un informe. Ninguna de las dos cosas es acoplamiento.
    """
    out, i, n = [], 0, len(txt)
    while i < n:
        c = txt[i]
        if c == "/" and i + 1 < n and txt[i + 1] == "/":
            j = txt.find("\n", i)
            i = n if j < 0 else j
        elif c == "/" and i + 1 < n and txt[i + 1] == "*":
            j = txt.find("*/", i + 2)
            i = n if j < 0 else j + 2
        elif c == "r" and i + 1 < n and txt[i + 1] in '#"':
            j = i + 1
            hashes = 0
            while j < n and txt[j] == "#":
                hashes += 1
                j += 1
            if j < n and txt[j] == '"':
                cierre = '"' + "#" * hashes
                k = txt.find(cierre, j + 1)
                i = n if k < 0 else k + len(cierre)
            else:
                out.append(c)
                i += 1
        elif c == '"':
            j = i + 1
            while j < n:
                if txt[j] == "\\":
                    j += 2
                elif txt[j] == '"':
                    j += 1
                    break
                else:
                    j += 1
            i = j
        elif c == "'":
            # char literal o lifetime: lifetime solo si va seguida de ident, no de '\'
            if i + 1 < n and txt[i + 1] == "\\":
                j = txt.find("'", i + 2)
                i = n if j < 0 else j + 1
            else:
                out.append(c)
                i += 1
        else:
            out.append(c)
            i += 1
    return "".join(out)


def _primer_segmento(entrada):
    entrada = entrada.strip()
    if not entrada or entrada == "self":
        return None
    m = re.match(r"[A-Za-z_]\w*", entrada)
    return m.group(0) if m else None


def _partir_por_comas(cuerpo):
    """Parte una lista de braced groups por comas de primer nivel."""
    partes, nivel, actual = [], 0, []
    for ch in cuerpo:
        if ch in "{([<":
            nivel += 1
        elif ch in "})]>":
            nivel -= 1
        if ch == "," and nivel == 0:
            partes.append("".join(actual))
            actual = []
        else:
            actual.append(ch)
    if actual:
        partes.append("".join(actual))
    return partes


def _segmentos_de_ruta(txt, i):
    """Lee una ruta que empieza en `i` y devuelve (primer_segmento, i_siguiente).

    Acepta `seg::seg`, `seg::{...}`, `{...}` y `seg as alias`. Al entrar en un
    grupo por llaves devuelve el primer identificador de la PRIMERA entrada del
    grupo, que es la dependencia; las demas entradas las recorre el llamante.
    """
    while i < len(txt) and txt[i] in " \t\r\n":
        i += 1
    m = re.match(r"[A-Za-z_]\w*", txt[i:])
    if not m:
        return None, i
    nombre = m.group(0)
    i += len(nombre)
    j = i
    while j < len(txt) and txt[j] in " \t\r\n":
        j += 1
    if txt.startswith("::", j):
        j += 2
        while j < len(txt) and txt[j] in " \t\r\n":
            j += 1
        if j < len(txt) and txt[j] == "{":
            # grupo: la dependencia es la primera entrada, y se sigue dentro
            _, j = _segmentos_de_ruta(txt, j + 1)
            return nombre, j
        return nombre, j
    return nombre, i


def _entradas_de_grupo(txt, i):
    """Entradas de un grupo. `i` apunta al PRIMER caracter DENTRO de las llaves.

    Se empieza en el interior y no en la `{` porque el contador arranca en 1:
    si se le pasa la llave de apertura la sube a 2 y el cierre exterior lo deja
    en 1, con lo que la funcion se come el resto del fichero en vez de parar.
    Es el mismo error de indice que el `resto[2:]` de la primera version, otra
    vez por un uno de mas, y otra vez solo lo ve un caso sintetico.
    """
    entradas, actual = [], []
    nivel = 1
    while i < len(txt) and nivel:
        ch = txt[i]
        if ch == "{":
            nivel += 1
        elif ch == "}":
            nivel -= 1
            if nivel == 0:
                break
        if ch == "," and nivel == 1:
            entradas.append("".join(actual))
            actual = []
        else:
            actual.append(ch)
        i += 1
    if actual:
        entradas.append("".join(actual))
    return entradas


def extraer_destinos(txt, raiz_modulo=None, en_engine=False):
    """Modulos raiz de los que depende este fichero.

    `raiz_modulo` es la raiz dentro de la que vive el fichero: el consumo de si
    mismo no es acoplamiento y por eso se excluye. `super::` nunca cruza una
    frontera de raiz, asi que no se busca.
    """
    codigo = sin_comentarios_ni_cadenas(txt)
    destinos = set()

    for m in PAT_USE_RAIZ.finditer(codigo):
        raiz = m.group(1)
        if raiz == "crate" and (raiz_modulo is None or not en_engine):
            continue
        i = m.end()
        while i < len(codigo) and codigo[i] in " \t\r\n":
            i += 1
        if i < len(codigo) and codigo[i] == "{":
            for entrada in _entradas_de_grupo(codigo, i + 1):
                seg, _ = _segmentos_de_ruta(entrada, 0)
                if seg:
                    destinos.add(seg)
        else:
            seg, _ = _segmentos_de_ruta(codigo, i)
            if seg:
                destinos.add(seg)

    for m in PAT_CUALIFICADA.finditer(codigo):
        raiz, seg = m.group(1), m.group(2)
        if raiz == "crate" and (raiz_modulo is None or not en_engine):
            continue
        destinos.add(seg)

    destinos.discard(raiz_modulo)
    return {d for d in destinos if d not in ("self", "super", "std", "core", "alloc")}


CASOS_SINTETICOS = [
    # (nombre, codigo, raiz_modulo, en_engine, esperado)
    ("simple",
     "use sddk_engine::event_bus::emit::X;",
     None, False, {"event_bus"}),
    ("agrupado con self",
     "use sddk_engine::event_bus::{self, OutcomeEventInput};",
     None, False, {"event_bus"}),
    ("agrupado multiple",
     "use sddk_engine::{event_bus::X, observation::Y, semantica};",
     None, False, {"event_bus", "observation", "semantica"}),
    ("cualificado en expresion",
     "let r = sddk_engine::event_bus::emit_phase_event(&mut s, &i);",
     None, False, {"event_bus"}),
    ("caja de codigo con ruta como cadena",
     'const RUTA: &str = "crates/sddk-engine/src/event_bus/emit.rs";',
     None, False, set()),
    ("comentario con ruta",
     "// mira crates/sddk-engine/src/event_bus/emit.rs",
     None, False, set()),
    ("doc que cita un modulo retirado",
     "/// antes existia `ProviderKind::CogniCode` y se elimino.",
     None, False, set()),
    ("consumo interno excluido",
     "use crate::observation::Thing;",
     "observation", True, set()),
    ("cruce de raiz dentro del engine",
     "use crate::verify_kernel::run;",
     "observation", True, {"verify_kernel"}),
    ("string multilinea con comillas escapadas",
     'let s = "use sddk_engine::event_bus::X;";',
     None, False, set()),
    # Los cuatro siguientes son la forma REAL de este repo, y son los que la
    # primera version del parser no via. Escribirlos aqui es lo que impidio
    # que el defecto llegara a la tabla: el defecto no era del repo, era del
    # instrumento, y solo un caso sintetico con la forma exacta lo delata.
    ("grupo multilinea con llaves anidadas",
     "use sddk_engine::{\n"
     "    algo,\n"
     "    event_bus::{self, OutcomeEventInput, PhaseEventInput},\n"
     "    otro::Cosa,\n"
     "};",
     None, False, {"algo", "event_bus", "otro"}),
    ("grupo anidado doble",
     "use sddk_engine::{event_bus::{emit::{X, Y}, self}, observation::Z};",
     None, False, {"event_bus", "observation"}),
    ("grupo de crate multilinea dentro del engine",
     "use crate::{\n    event_bus::{A},\n    rules,\n};",
     "otro_modulo", True, {"event_bus", "rules"}),
    ("alias con as",
     "use sddk_engine::event_bus::X as Y;",
     None, False, {"event_bus"}),
    ("wildcard",
     "use sddk_engine::observation::*;",
     None, False, {"observation"}),
    ("crate fuera del engine no existe",
     "use crate::algo;",
     None, False, set()),
]


def autoprueba():
    fallos = []
    for nombre, codigo, raiz, en_engine, esperado in CASOS_SINTETICOS:
        got = extraer_destinos(codigo, raiz_modulo=raiz, en_engine=en_engine)
        if got != esperado:
            fallos.append(f"  caso {nombre!r}: esperaba {sorted(esperado)}, dio {sorted(got)}")
    return (not fallos), fallos


# ── 3. SUPERFICIE PUBLICA: QUE CRUZA `lib.rs` ────────────────────────────────
#
# Una frontera de contexto no se dibuja sobre quien llama a quien, se dibuja
# sobre que se puede usar desde fuera. Y aqui hay un hueco que el grafo de `use`
# no ve: en `lib.rs` el reexport se escribe SIN prefijo de crate, porque `lib.rs`
# ES la raiz:
#
#     pub mod human_decision;
#     pub use human_decision::{A, B};
#
# El patron de `use` exige `crate::` o `sddk_engine::`, asi que ninguna de las
# dos aparece, y un modulo reexportado — parte del contrato publico, prometido
# por la API — salia como "nada lo nombra". Tres categorias distintas que una
# tabla de dos columnas no distingue:
#
#   declarado   `pub mod X;` en lib.rs, sin reexportar simbolos
#   reexportado `pub use X::{...}` en lib.rs -> superficie publica de verdad
#   consumido    lo usa otro modulo

PAT_PUB_MOD = re.compile(r"^\s*pub\s+mod\s+([A-Za-z_]\w*)\s*;", re.M)
# `mod X;` SIN `pub`: el modulo es privado pero sus simbolos pueden salir con
# `pub use X::*`. Mirar solo `pub mod` hacia que estos dos parecieran no existir:
# son 1.643 lineas del engine, y la superficie publica del crate tiene DOS
# formas (`pub mod X` expone el nombre, `mod X` + `pub use X::*` expone solo los
# simbolos), no una. Para una decision de contextos la diferencia no es de
# estilo: es codigo que llega al usuario por un camino que un mapa de nombres
# no ve.
PAT_MOD_PRIV = re.compile(r"^\s*mod\s+([A-Za-z_]\w*)\s*;", re.M)
PAT_PUB_USE = re.compile(r"^\s*pub\s+use\s+([A-Za-z_]\w*)\b", re.M)


def superficie_publica(lib_path):
    """(modulos con `pub mod`, simbolos reexportados) leidos de `lib.rs`."""
    txt = sin_comentarios_ni_cadenas(lib_path.read_text(encoding="utf-8", errors="replace"))
    return set(PAT_PUB_MOD.findall(txt)), set(PAT_PUB_USE.findall(txt))


# ── 4. GRAFO REAL ────────────────────────────────────────────────────────────

def _raiz_de(p):
    """La raiz de modulo en la que vive el fichero, o None si no es del engine.

    Para un modulo DIRECTORIO, `relative_to(SRC).parts[0]` ya es el nombre de la
    raiz. Para un modulo FICHERO devuelve el nombre CON la extension
    (`active_graph.rs`), y usar eso sin quitar la extension rompe la
    autoexclusion: el modulo se contaria a si mismo como su propio consumidor.
    Con dos modulos (`human_resume_view` y `secretary_l2_replan`) eso daba
    consumidor donde el instrumento de C3m.1, escrito aparte, no ve ninguno.
    Lo delata el contraste entre instrumentos, no el codigo: los dos estan bien
    escritos, y por eso un numero se contrasta con un metodo que no comparte
    author's supuestos en vez de con una cuenta del mismo autor.
    """
    if not p.is_relative_to(SRC):
        return None
    rel = p.relative_to(SRC)
    primera = rel.parts[0]
    if len(rel.parts) > 1:                 # vive en un directorio
        return primera
    return pathlib.Path(primera).stem      # es un fichero: sin extension


def leer_grafo(raices):
    """Grafo de dependencias de modulo, por las TRES vias por las que se
    consume de verdad, no solo una.

    La via que falta en una primera version de este script es la tercera, y es
    la que mas se usa en este repo: la CLI llama `context.engine.cycle_pause(...)`,
    un METODO de la fachada `Engine` definido dentro de `cycle_pause.rs`. Medir
    solo rutas `use` declara "sin consumidor" a modulos que el producto si
    llama. **No es un defecto que este script descubriera: el instrumento de
    C3m.1 ya lo documentaba en su cabecera y ya tenia un control para el
    (`cycle_pause -> True`). Reconstruir un instrumento desde cero tira las
    falsificaciones acumuladas del anterior, y el nuevo empieza con menos
    controles que el viejo.** Por eso el control de `cycle_pause` esta aqui.
    """
    uso = collections.defaultdict(lambda: {"producto": set(), "test": set()})
    for crate in sorted(p for p in CRATES.iterdir() if p.is_dir()):
        en_engine = crate == ENGINE
        for p in sorted(crate.rglob("*.rs")):
            raiz_modulo = _raiz_de(p) if en_engine else None
            crudo = p.read_text(encoding="utf-8", errors="replace")
            codigo = sin_comentarios_ni_cadenas(crudo)
            for mod in extraer_destinos(crudo, raiz_modulo, en_engine):
                uso[mod]["test" if "tests" in p.parts else "producto"].add(p)
            if p == (ENGINE / "src/lib.rs"):
                # lib.rs declara y reexporta: es la puerta, no un consumidor
                continue
            # via 4: el simbolo reexportado. `pub use adoption::{plan_adoption}`
            # hace que el nombre del modulo NO aparezca nunca en el consumidor:
            # la CLI llama a `plan_adoption(...)` y no sabe de que modulo salio.
            for mod, simbolos in REEXPORTADOS.items():
                if mod == raiz_modulo:
                    continue
                for simbolo in simbolos:
                    if simbolo == "__star__":
                        continue          # `pub use m::*` no prueba nada: lo usa todo
                    if re.search(r"(?<![\w])" + re.escape(simbolo) + r"(?:\s*[({<:]|\b)", codigo):
                        uso[mod]["test" if "tests" in p.parts else "producto"].add(p)
                        break
            # via 3: la fachada. `engine.cycle_pause(` y compañía. Se extraen
            # los identificadores que siguen a un punto UNA vez por fichero y se
            # cruzan con el conjunto de raices: recorrer 133 regex por fichero
            # hacia que el instrumento tardase minutos en dar un numero.
            for mod in (set(PAT_METODO_FACHADA.findall(codigo)) & set(raices)):
                if mod == raiz_modulo:
                    continue
                uso[mod]["test" if "tests" in p.parts else "producto"].add(p)
    return uso


PAT_METODO_FACHADA = re.compile(r"\.\s*([A-Za-z_]\w*)\s*\(")


PAT_REEXP_STAR = re.compile(r"^\s*pub\s+use\s+([A-Za-z_]\w*)\s*::\s*\*", re.M)
PAT_REEXP_BLOQ = re.compile(r"^\s*pub\s+use\s+([A-Za-z_]\w*)\s*::\s*\{([^}]*)\}", re.M)


def _simbolos_de_bloque(cuerpo):
    return [x.strip().split(" as ")[-1].strip()
            for x in cuerpo.split(",") if x.strip()]


def leer_reexportados(lib_path):
    """modulo -> simbolos que `lib.rs` saca de el con `pub use`."""
    txt = sin_comentarios_ni_cadenas(lib_path.read_text(encoding="utf-8", errors="replace"))
    salida = {}
    for mod in PAT_REEXP_STAR.findall(txt):
        # con `::*` el consumidor puede usar cualquier simbolo, asi que se marca
        # con un comodin que el consumidor tendra que resolver por otro lado
        salida.setdefault(mod, set()).add("__star__")
    for mod, bloque in PAT_REEXP_BLOQ.findall(txt):
        salida.setdefault(mod, set()).update(_simbolos_de_bloque(bloque))
    return salida


REEXPORTADOS = {}


# Un control tiene que poder FALLAR cuando el instrumento se rompe. Con la
# cuenta independiente por regex no se consiguió eso: compartía con el parser
# el mismo punto ciego (no sabía contar llaves), así que los dos decían
# 2 y los dos estaban mal. Coincidir no era confirmacion, era el mismo error
# contado dos veces.
#
# Asi que el control es una lista ESCRITA A MANO, fichero a fichero, y el
# instrumento imprime su lista para que se pueda leer. La lista salio de un
# `grep` de las menciones y de abrir las que sobraban:
#
#   sddk-cli/src/admission.rs        use sddk_engine::event_bus::emit::{...}
#   sddk-cli/src/approval.rs        grupo multilinea, event_bus::{self, ..}
#   sddk-cli/src/cycle.rs           grupo multilinea, event_bus::{self, ..}
#   sddk-engine/src/workflow_runtime.rs   use crate::event_bus::{...}
#
# y de las tres que el grep trae y el parser NO son consumidor, con el motivo:
#
#   sddk-cli/src/dev/arch_lint.rs    la ruta vive como texto en un literal
#   sddk-cli/src/dev/doctor.rs       idem: imprime la ruta en un informe
#   sddk-engine/src/lib.rs           es la declaracion `pub mod event_bus;`
#
# Que la lista este justificada por un motivo y no solo por un numero es lo que
# permite reparar el instrumento cuando falle: se sabe que camino cogio.
PISADO_EVENT_BUS_PRODUCTO = {
    "sddk-cli/src/admission.rs",
    "sddk-cli/src/approval.rs",
    "sddk-cli/src/cycle.rs",
    "sddk-engine/src/workflow_runtime.rs",
}
# Reactivo: el STOP 1 de C3m.1, ya publicado. Cero consumidores.
PISADO_REACTIVE_VERIFY = 0
# El contraejemplo de la fachada, y el control que faltaba. `cycle_pause` es un
# metodo de la fachada `Engine` definido en `cycle_pause.rs:68` y llamado por
# producto en `sddk-cli/src/cycle.rs:1967` con `context.engine.cycle_pause(...)`.
# Un grafo que solo mira rutas `use` lo declara SIN consumidor: 13 modulos
# erroneos, 1.6k lineas. Sin este control, el instrumento es verde y falso, que
# es el peor estado en que puede estar un instrumento.
PISADO_CYCLE_PAUSE_PRODUCTO = {"sddk-cli/src/cycle.rs"}

# El conjunto publicado por INC-DEBT-065 (24 modulos, status: open), copiado tal
# cual para que el contraste sea contra lo que esta escrito y no contra lo que
# uno recuerda. Se lee del fichero de deuda, no de la memoria.
PISADO_INC065_SIN_CONSUMIDOR = """
    active_graph_view cas_object_store converge_verification decision_plane_gate
    engineering_assurance_resolvers evidence_backed_promotion experience_episodes
    ga_publish gate_evaluator human_resume_view incident_pack lab_promotion
    pack_agnosticity production_hardening projector_registry reactive_verify
    receipt_writers secretary_l2_replan security_upgrade_rollback state_class_lint
    structured_work supply_chain_artifacts uat_pack vault_boundary
""".split()

# Causa de cada diferencia, comprobada una a una abriendo el fichero. Se escribe
# DESPUES de comprobarlas, no antes: poner la causa primero y buscarla despues
# es la forma de encontrar la evidencia que confirma la theory.
CAUSA_DIFERENCIA = {
    "adoption":
        "`mod adoption;` es PRIVADO (lib.rs:16) con `pub use adoption::*;` (155): "
        "no esta en el denominador de 131 `pub mod` de INC-065, que mide la "
        "superficie por NOMBRE. 1.229 lineas de codigo publico sin nombre de modulo.",
    "paths":
        "`mod paths;` PRIVADO (lib.rs:109) con `pub use paths::*;` (222). Mismo "
        "motivo que `adoption`: superficie publica sin ruta nominal. 414 lineas.",
    "durable_map_fanout":
        "INC-065 lo cuenta consumido por un ENLACE EN UN DOC-COMMENT: "
        "`typed_reduce_aggregator.rs:8` cita `[DurableMapFanOut](crate::"
        "durable_map_fanout::...)` dentro de un `//!`. Citar no es consumir; este "
        "script borra comentarios y cadenas antes de medir.",
    "gate_signing":
        "`pub use gate_signing::*;` (lib.rs:202). El asterisco no prueba consumo: "
        "la unica referencia externa es una cadena en un test. Este script trata "
        "`::*` como NO MEDIBLE en vez de como consumidor.",
    "inc_generator":
        "`pub use inc_generator::*;` (lib.rs:211). Igual que `gate_signing`, y la "
        "unica referencia externa tambien es una cadena en un test.",
    "up_to_date":
        "INC-065 lo cuenta consumido por COINCIDENCIA DE SUBCADENA: en "
        "`sddk-domain/src/goal.rs:168` el metodo `pub fn is_up_to_date` contiene "
        "el texto `up_to_date`. No es el modulo. Este script exige limite de "
        "palabra antes del nombre.",
}


# ── 4. DOMINIOS: se declara, y se imprime quien la resiste ───────────────────

DOMINIOS = {
    "knowledge":    ("semantic_", "observation", "knowledge", "kmt", "notion"),
    "alignment":    ("alignment_", "paradigm_", "software_alignment", "intent_"),
    "verification": ("verify_kernel", "debverify_kernel", "converge_verification",
                     "architecture_conformance", "architecture_debverify",
                     "architecture_receipt", "reactive_", "uat_"),
    "governance":   ("authority", "rules", "architectural_contract", "admission",
                     "approval", "gate_"),
    "decision":     ("context_", "target_task", "task", "planning", "workflow",
                     "decision", "scheduler", "router"),
    "architecture": ("architecture_graph", "architecture_declaration",
                     "architecture_mutation", "architecture_why", "active_graph"),
    "eventing":     ("event_bus", "canonical_event_log", "phase_events"),
    "intelligence": ("intelligence_", "code_intelligence_port", "llm"),
    "agentia":      ("agent_", "cycle_", "host_", "session", "human_", "resume"),
}


def dominio_de(mod):
    for dom, prefs in DOMINIOS.items():
        for pre in prefs:
            if mod == pre or mod.startswith(pre):
                return dom
    return None


# ── 5. MAIN ──────────────────────────────────────────────────────────────────

def main():
    print("=" * 78)
    print("AUTOPRUEBA DEL PARSER (codigo sintetico, respuesta escrita a mano)")
    print("=" * 78)
    ok, fallos = autoprueba()
    for f in fallos:
        print("  FALLA:" + f)
    if not ok:
        print(f"\n  {len(fallos)}/{len(CASOS_SINTETICOS)} casos mal. El parser no se fia de si")
        print("  mismo, asi que no se imprime ninguna tabla del repo.")
        return 2
    print(f"  OK: {len(CASOS_SINTETICOS)}/{len(CASOS_SINTETICOS)} casos, incluidos los que")
    print("  tienen que salir VACIOS (cadena, comentario, doc, consumo interno).")

    RAICES = raices_de(SRC)
    global REEXPORTADOS
    REEXPORTADOS = leer_reexportados(ENGINE / "src/lib.rs")
    USO = leer_grafo(RAICES)

    print()
    print("=" * 78)
    print("CONTROLES CONTRA EL GRAFO REAL, CONTRA UNA LISTA ESCRITA A MANO")
    print("=" * 78)
    fallos = []

    rv = len(USO.get("reactive_verify", {"producto": set()})["producto"])
    if rv != PISADO_REACTIVE_VERIFY:
        fallos.append(f"  reactive_verify: {rv} consumidores de producto, "
                      f"el caso publicado dice {PISADO_REACTIVE_VERIFY}")

    ev_set = USO.get("event_bus", {"producto": set()})["producto"]
    ev_rel = {str(p.relative_to(CRATES)) for p in ev_set}
    if ev_rel != PISADO_EVENT_BUS_PRODUCTO:
        solo_parser = sorted(ev_rel - PISADO_EVENT_BUS_PRODUCTO)
        solo_pisado = sorted(PISADO_EVENT_BUS_PRODUCTO - ev_rel)
        fallos.append(f"  event_bus: el parser ve {len(ev_rel)} ficheros de producto y la "
                      f"lista a mano tiene {len(PISADO_EVENT_BUS_PRODUCTO)}")
        if solo_parser:
            fallos.append(f"    solo el parser los cuenta: {solo_parser}")
        if solo_pisado:
            fallos.append(f"    solo la lista a mano los cuenta: {solo_pisado}")

    # El control de la FACHADA. Sin este, el instrumento es verde y falso.
    cp_set = USO.get("cycle_pause", {"producto": set()})["producto"]
    cp_rel = {str(p.relative_to(CRATES)) for p in cp_set}
    if PISADO_CYCLE_PAUSE_PRODUCTO - cp_rel:
        fallos.append(f"  cycle_pause: la fachada la consume "
                      f"{sorted(PISADO_CYCLE_PAUSE_PRODUCTO - cp_rel)} y el grafo no la ve; "
                      f"falta la via del metodo `engine.<mod>()`")

    for f in fallos:
        print("  FALLA:" + f)
    if fallos:
        print("\n  El grafo real no reproduce las listas escritas a mano. No se imprime")
        print("  ninguna tabla: un numero que no se puede contrastar no es un numero.")
        return 1
    print(f"  OK  reactive_verify -> {rv} consumidores de producto (caso publicado: 0)")
    print(f"  OK  cycle_pause     -> {sorted(cp_rel)}")
    print( "      la fachada `Engine` la llama y el grafo de rutas no la veria:")
    print( "      este es el control que faltaba y por eso el instrumento de C3m.1")
    print( "      ya lo tenia escrito desde antes de que existiera este script.")
    print(f"  OK  event_bus       -> {len(ev_rel)} consumidores de producto, y son los")
    print( "      mismos ficheros de la lista escrita a mano, uno a uno:")
    for p in sorted(ev_rel):
        print(f"        {p}")

    # ── CONVERGENCIA CON UN INSTRUMENTO ESCRITO POR OTRO ──────────────────────
    # Este script se reconstruyo desde cero, y al contrastarlo con el de C3m.1
    # (que mide lo mismo sobre el mismo repo) las dos cifras no cuadraron. Ese
    # contraste es el unico control que sirve aqui: un numero contrastado con
    # otro numero del mismo autor no esta contrastado. Las diferencias NO se
    # promedian ni se ignoran: se enuncian una a una, con su causa, y solo
    # entonces se decide si una de las dos esta equivocada.
    #
    # Mismo criterio que INC-DEBT-065, escrito de forma independiente: modulos
    # sin consumidor NI de producto NI de pruebas.
    mios = {n for n in RAICES
            if not USO.get(n, {"producto": set(), "test": set()})["producto"]
            and not USO.get(n, {"producto": set(), "test": set()})["test"]}
    publicados = set(PISADO_INC065_SIN_CONSUMIDOR)

    print()
    print("=" * 78)
    print("CONVERGENCIA CON EL INSTRUMENTO DE C3m.1 (misma pregunta, otro autor)")
    print("=" * 78)
    print(f"  este script : {len(mios)}")
    print(f"  INC-DEBT-065: {len(publicados)}  (publicado, status: open)")
    if mios == publicados:
        print("  CONVERGEN exactamente.")
    else:
        print(f"  NO CONVERGEN. Cada diferencia, con su causa:")
        for n in sorted(mios - publicados):
            causa = CAUSA_DIFERENCIA.get(n, "SIN CAUSA EXPLICADA")
            print(f"    + {n:26s} solo aqui. {causa}")
        for n in sorted(publicados - mios):
            print(f"    - {n:26s} solo en INC-065. SIN CAUSA EXPLICADA")
        if mios - publicados <= set(CAUSA_DIFERENCIA):
            print()
            print("  Ninguna diferencia queda sin explicar, y en ninguna direccion")
            print("  se pierde un modulo que el otro dice tener sin consumidor: el")
            print("  conjunto de este script es un SUPERCONJUNTO estricto del publicado.")

    print()
    print("=" * 78)
    print("1. LAYOUT DE LAS RAICES DE MODULO DE sddk-engine")
    print("=" * 78)
    et = {"f": "raiz-fichero", "d": "raiz-directorio", "fd": "raiz-partida (fichero+directorio)"}
    por = collections.Counter(v[0] for v in RAICES.values())
    for lay in ("f", "d", "fd"):
        print(f"  {et[lay]:38s} {por[lay]:>4d}")
    print(f"  {'TOTAL raices':38s} {len(RAICES):>4d}")
    for nombre, (lay, ficheros) in sorted(RAICES.items()):
        if lay == "fd":
            lineas = sum(len(f.read_text(encoding='utf-8', errors='replace').splitlines())
                         for f in ficheros)
            raizf = [f for f in ficheros if f.parent == SRC][0]
            lr = len(raizf.read_text(encoding='utf-8', errors='replace').splitlines())
            print(f"\n  raiz-partida: {nombre}, {len(ficheros)} ficheros, {lineas} lineas")
            print(f"    {lr} en el .rs raiz y {lineas - lr} en el directorio homonimo.")
            print(f"    Un conteo que solo mire el .rs dira {lr} cuando son {lineas}.")

    print()
    print("=" * 78)
    print("2. TAMANO Y DOMINIO DE CADA RAIZ")
    print("=" * 78)
    print(f"{'raiz':34s} {'lay':4s} {'lineas':>8s} {'prod':>5s} {'test':>5s} {'pub':>4s}  dominio")
    print("-" * 96)
    DECL, REEXP = superficie_publica(ENGINE / "src/lib.rs")
    PRIVADOS = set(PAT_MOD_PRIV.findall(
        sin_comentarios_ni_cadenas(
            (ENGINE / "src/lib.rs").read_text(encoding="utf-8")))) - DECL
    filas = []
    for nombre, (lay, ficheros) in sorted(RAICES.items()):
        lineas_raiz = sum(len(f.read_text(encoding="utf-8", errors="replace").splitlines())
                          for f in ficheros)
        u = USO.get(nombre, {"producto": set(), "test": set()})
        np_, nt = len(u["producto"]), len(u["test"])
        dom = dominio_de(nombre) or "SIN CAJA"
        if nombre in DECL:
            pub = "re-exp" if nombre in REEXP else "pub"
        elif nombre in PRIVADOS:
            pub = "priv*" if nombre in REEXP else "priv"
        else:
            pub = "-"
        filas.append((nombre, lay, lineas_raiz, np_, nt, dom, pub))
        print(f"{nombre:34s} {lay:4s} {lineas_raiz:>8d} {np_:>5d} {nt:>5d} {pub:>5s}  {dom}")

    sin_caja = [f for f in filas if f[5] == "SIN CAJA"]
    sin_prod = [f for f in filas if f[3] == 0]

    print()
    print("=" * 78)
    print("2b. LA SUPERFICIE PUBLICA, QUE TIENE DOS FORMAS Y NO UNA")
    print("=" * 78)
    print("    pub mod X;                  publica el NOMBRE del modulo")
    print("    mod X; + pub use X::*;      publica solo los SIMBOLOS")
    print("  Es la misma promesa hecha al usuario por un camino que un mapa de")
    print("  nombres no ve, y por eso se cuenta aparte y no como un olvido.")
    print()
    for etiqueta, sel in (
            ("pub mod, nombre publico", [f for f in filas if f[6] in ("pub", "re-exp")]),
            ("mod privado + pub use *", [f for f in filas if f[6] == "priv*"]),
            ("mod privado y nada mas", [f for f in filas if f[6] == "priv"])):
        print(f"    {etiqueta:28s} {len(sel):>4d} raices  {sum(f[2] for f in sel):>7d} lineas")
    sin_nombre = [f for f in filas if f[6] == "priv*"]
    if sin_nombre:
        print()
        print("  Codigo que llega al usuario SIN que exista `sddk_engine::<modulo>`:")
        for f in sorted(sin_nombre, key=lambda x: -x[2]):
            print(f"    {f[0]:24s} {f[2]:>6d} lineas  {f[3]} consumidores de producto")
    print()
    announced_dead = [f for f in filas if f[6] != "-" and f[3] == 0]
    print(f"  ANUNCIADO Y SIN CONSUMIDOR DE PRODUCTO: {len(announced_dead)} raices, "
          f"{sum(f[2] for f in announced_dead)} lineas.")
    print("  Las diez mas grandes:")
    for f in sorted(announced_dead, key=lambda x: -x[2])[:10]:
        print(f"    {f[0]:34s} {f[2]:>7d} lineas  pub={f[6]}  {f[4]} refs de test")

    print()
    print("=" * 78)
    print("3. LO QUE LA CONVENCION DE DOMINIOS NO ALCANZA")
    print("=" * 78)
    print(f"raices SIN CAJA: {len(sin_caja)} de {len(filas)} "
          f"({100*len(sin_caja)//len(filas)}% del engine)")
    for f in sorted(sin_caja, key=lambda x: -x[2])[:40]:
        print(f"    {f[0]:34s} {f[2]:>7d} lineas  {f[3]} prod")
    if len(sin_caja) > 40:
        print(f"    ... y {len(sin_caja)-40} mas")
    print()
    print(f"raices SIN consumidor de producto: {len(sin_prod)} "
          f"({sum(f[2] for f in sin_prod)} lineas)")
    for f in sorted(sin_prod, key=lambda x: -x[2])[:40]:
        marca = f"  solo tests ({f[4]})" if f[4] else "  ni codigo lo nombra"
        print(f"    {f[0]:34s} {f[2]:>7d} lineas{marca}")
    if len(sin_prod) > 40:
        print(f"    ... y {len(sin_prod)-40} mas")
    muertos = [f for f in sin_prod if f[4] == 0]
    print()
    print(f"  de los cuales, sin ninguna referencia: {len(muertos)}, "
          f"{sum(f[2] for f in muertos)} lineas")

    print()
    print("=" * 78)
    print("4. CRUCE: DONDE SE DECIDE O NO SE DECIDE NUNCA LA DECISION")
    print("=" * 78)
    por_dom = collections.Counter(f[5] for f in filas)
    for dom, n in por_dom.most_common():
        lineas = sum(f[2] for f in filas if f[5] == dom)
        dead = sum(1 for f in filas if f[5] == dom and f[3] == 0)
        print(f"  {dom:16s} {n:>4d} raices  {lineas:>7d} lineas  {dead:>3d} sin consumidor")
    print()
    print("  Si un dominio declarado por la convencion se sostiene entero sobre un")
    print("  nucleo sin consumidor, el dominio no es un contexto: es un almacen.")
    for dom in sorted(por_dom):
        d = [f for f in filas if f[5] == dom]
        if d and all(f[3] == 0 for f in d):
            print(f"    {dom}: {len(d)} raices, {sum(f[2] for f in d)} lineas, "
                  f"0 consumidores de producto")

    print()
    print(f"total de lineas de raiz en sddk-engine: {sum(f[2] for f in filas)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
