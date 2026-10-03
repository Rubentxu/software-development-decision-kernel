#!/usr/bin/env python3
"""¿Que modulos del engine NO tiene nadie consumiendo de producto?

VERSION CORREGIDA. Las dos anteriores_failed por un fallo del criterio, y las
dos veces el numero salio falso:

  v1. Buscaba solo en sddk-engine/src -> todo lo que consume la CLI salia
      como "sin entrada". 52 modulos. FALSO.
  v2. Busco en todo crates/, pero el criterio era "el primer `pub struct` del
      modulo aparece en otro fichero". Eso mide MENCION, no CONSUMO: la CLI
      llama `engine.cycle_pause(...)` sin construir jamas `CyclePauseInput`, y
      ese modulo tiene un consumidor de producto real. 41 modulos. FALSO.

  v3. Contaba "quien USA EL MODULO", pero miraba solo `src/<mod>.rs` como
      fichero base y miraba `lib.rs` como reexportacion. 38 modulos. FALSO por
      DOS motivos a la vez: (a) la CLI consume por el camino corto y el nombre
      del modulo no aparece en su fichero; (b) los simbolos que `lib.rs`
      reexporta con `pub use <mod>::*` llegan igual.  v4 (esta). v3 + la via del METODO (`engine.cycle_pause(...)`, que no lleva el
      nombre del modulo) + los simbolos reexportados + una categoria propia para
      "lo consumen solo tests". Y ademas `base_de()`: los modulos que son
      DIRECTORIO con `mod.rs` ya no se saltan. v3 se saltaba 23 de 131, y entre
      ellos estaba `architecture_receipt`, que es el consumidor real de la
      salida de `reactive_verify` segun el SCOPE. Un instrumento que se salta
      al consumidor de lo que investiga no puede declarar "sin consumidor".

CONTROLES, con casos que SI se conocen, verificados a mano y NO por este script:
  - `cycle_pause` tiene consumidor de producto (sddk-cli/src/cycle.rs:1967).
  - `reactive_verify` NO lo tiene (medido en C3m.1, STOP 1 negativo).
  - `event_bus` SI, y es un modulo DIRECTORIO (sddk-cli/src/cycle.rs:21 lo
    importa y llama `event_bus::emit_outcome_event(...)` en :1443). Primer
    control del camino `base_de()` + `es_propio()` para directorios.
  - `architecture_receipt` SI, y tambien DIRECTORIO
    (sddk-cli/src/architecture_cmd.rs:28). Falsifica el otro error posible:
    que un modulo se cuente a si mismo por sus propios submodulos.
Si alguno no sale como se espera, el instrumento es INCORRECTO y su numero no
vale, se diga lo que se diga.
"""
import os
import pathlib
import re
import sys

# El repo se deduce de la ubicacion de este fichero
# (docs/roadmap/receipts/<ciclo>/medir-modulos-consumidor.py -> raiz del repo),
# no de una ruta absoluta: un instrumento con la ruta escrita dentro no se puede
# re-ejecutar en otra maquina, y un numero que no se puede reproducir no es
# evidencia, es una afirmacion de sesion. SDDK_REPO lo sobreescribe si hace
# falta (worktree, copia, etc).
_AQUI = pathlib.Path(__file__).resolve()
_RAIZ = next((p for p in _AQUI.parents if (p / "crates" / "sddk-engine").is_dir()),
             None)
if _RAIZ is None:
    sys.exit("no encuentro crates/sddk-engine subiendo desde "
             f"{_AQUI.parent}; ejecuta esto desde dentro del repo")
REPO = pathlib.Path(os.environ.get("SDDK_REPO") or _RAIZ)
CRATES = REPO / "crates"
ENGINE = CRATES / "sddk-engine"
LIB = ENGINE / "src/lib.rs"

modulos = re.findall(r'^pub mod (\w+);', LIB.read_text(encoding="utf-8"), re.M)
ficheros = list(CRATES.rglob("*.rs"))
contenido = {p: p.read_text(encoding="utf-8", errors="replace") for p in ficheros}


def base_de(mod):
    """El fichero base de un modulo: `src/<m>.rs` O `src/<m>/mod.rs`.

    v3 asumia que todo modulo era un fichero plano y hacia `continue` sobre los
    que no lo eran. Eso se salto 23 de 131, y entre ellos `architecture_receipt`
    — el consumidor real de la salida de `reactive_verify`, segun el SCOPE.
    Un instrumento que se salta al consumidor de lo que investiga no puede
    luego decir "sin consumidor": se dice de si, no de lo que no miro.
    """
    plano = ENGINE / "src" / f"{mod}.rs"
    if plano.exists():
        return plano
    en_dir = ENGINE / "src" / mod / "mod.rs"
    if en_dir.exists():
        return en_dir
    return None


def es_test(p):
    return "tests" in p.parts or p.name.endswith("_tests.rs") or p.name == "tests.rs"


def es_propio(mod, p):
    """¿El fichero `p` es parte del propio modulo `mod`?

    Para un modulo plano (`src/<m>.rs`) es solo ese fichero. Para uno que es
    DIRECTORIO (`src/<m>/mod.rs`) es todo el subarbol: excluir solo `mod.rs`
    dejaria que los submodulos del propio modulo se contaran como consumidores
    de si mismos, que es la forma mas tonta de inflar un numero.

    OJO: el chequeo de subarbol solo aplica al caso directorio. En el plano,
    `base.parent` es `src/`, que esta en `p.parents` de practicamente todo el
    engine — aplicarlo ahi excluiria el motor entero y daria "sin consumidor"
    para todo, que es un falso nuevo y mas grave que el que se corrige.
    """
    base = base_de(mod)
    if base is None:
        return False
    if p == base:
        return True
    if base.name == "mod.rs":
        return (ENGINE / "src" / mod) in p.parents
    return False


# Simbolos que lib.rs reexporta con `pub use <mod>::*` o `pub use <mod>::{...}`.
# Se calculan leyendo lib.rs, no a mano.
REEXPORTADOS = {}
for _line in LIB.read_text(encoding="utf-8").splitlines():
    _m = re.match(r'pub use (\w+)::(\*|\{)', _line)
    if not _m:
        continue
    _mod, _rest = _m.group(1), _m.group(2)
    _txt = LIB.read_text(encoding="utf-8")
    _blq = re.search(r'pub use ' + re.escape(_mod) + r'::\{([^}]*)\}', _txt)
    if _rest == '*':
        _base = ENGINE / 'src' / f'{_mod}.rs'
        if _base.exists():
            _sims = re.findall(r'^pub (?:struct|enum|trait|fn|type) (\w+)',
                              _base.read_text(encoding="utf-8"), re.M)
            REEXPORTADOS.setdefault(_mod, set()).update(_sims)
    elif _blq:
        _sims = [x.strip().split(' as ')[-1].strip() for x in _blq.group(1).split(',') if x.strip()]
        REEXPORTADOS.setdefault(_mod, set()).update(_sims)


def usa_modulo(p, fichero_txt, mod):
    """El fichero CONSUME el modulo: lo importa o llama a algo suyo.

    NO cuenta la declaracion `pub mod <nombre>;` de lib.rs, que es donde el
    modulo se REGISTRA, no donde se usa. Contarla haria que todo modulo
    pareciera tener consumidor, que es exactamente el falso que la v3 publicaba.
    """
    if p == LIB:
        # lib.rs solo declara y reexporta: eso NO es consumo, es la puerta.
        return False
    # Si lib.rs hace `pub use <mod>::*`, el simbolo llega a la CLI por el camino
    # corto (`use sddk_engine::algo`) y el nombre del modulo no aparece en el
    # fichero consumidor. Hay que mirar tambien los simbolos que el modulo
    # reexporta, no solo su nombre.
    if mod in REEXPORTADOS:
        for simbolo in REEXPORTADOS[mod]:
            if re.search(r'\b' + re.escape(simbolo) + r'\b', fichero_txt):
                return True
    # Tres vias de consumo, y la tercera es la que mas se usa: la CLI llama
    # `engine.cycle_pause(...)`, un METODO, no un path `mod::fn`. Medir solo el
    # path daba "sin consumidor" para modulos que si los consume la CLI.
    return bool(
        re.search(r'\b' + re.escape(mod) + r'\s*::', fichero_txt)
        or re.search(r'\buse\s+(crate|sddk_engine)::' + re.escape(mod) + r'\b', fichero_txt)
        or re.search(r'\.\s*' + re.escape(mod) + r'\s*\(', fichero_txt)
    )


print(f"modulos publicos: {len(modulos)}\n")

# ── CONTROLES PRIMERO ────────────────────────────────────────────────────────
controles = []
for mod, esperado in (("cycle_pause", True), ("reactive_verify", False),
                      ("event_bus", True), ("architecture_receipt", True)):
    base = base_de(mod)
    consumidores = set()
    for p, txt in contenido.items():
        if es_propio(mod, p) or es_test(p):
            continue
        if usa_modulo(p, txt, mod):
            consumidores.add(p)
    obtenido = bool(consumidores)
    estado = "OK" if obtenido == esperado else "DESCALIBRADO"
    controles.append(estado == "OK")
    print(f"  control {mod:22s} consumidor={obtenido} esperado={esperado}  {estado}")

if not all(controles):
    print("\nINSTRUMENTO DESCALIBRADO: no se publica ningun numero.")
    sys.exit(2)

print("  -> el instrumento responde como se espera en los 4 casos conocidos\n")

# ── LA MEDICION ──────────────────────────────────────────────────────────────
sin_consumidor = []
solo_tests = []
con_consumidor = 0
sin_fichero = []
for mod in modulos:
    base = base_de(mod)
    if base is None:
        # Declarado en lib.rs y sin `src/<m>.rs` ni `src/<m>/mod.rs`. No se
        # clasifica: contarlo como "sin consumidor" seria inventar una categoria.
        sin_fichero.append(mod)
        continue
    de_producto = [
        p for p, txt in contenido.items()
        if not es_propio(mod, p) and not es_test(p) and usa_modulo(p, txt, mod)
    ]
    de_tests = [
        p for p, txt in contenido.items()
        if not es_propio(mod, p) and es_test(p) and usa_modulo(p, txt, mod)
    ]
    if de_producto:
        con_consumidor += 1
    elif de_tests:
        # DISTINTO de "sin consumidor": se usa, pero solo desde pruebas. Un
        # modulo que solo sus tests consumen es otra cosa, y reportarlo igual
        # que "nadie lo mira" seria el mismo error de medir mal.
        solo_tests.append((mod, base.name, len(de_tests)))
    else:
        sin_consumidor.append((mod, base.name))

medidos = con_consumidor + len(solo_tests) + len(sin_consumidor)
print(f"declarados en lib.rs        : {len(modulos)}")
print(f"medidos                     : {medidos}")
print(f"declarados SIN fichero base : {len(sin_fichero)}  {sorted(sin_fichero)}")
print()
print(f"CON consumidor de producto  : {con_consumidor}")
print(f"SOLO desde tests            : {len(solo_tests)}")
print(f"SIN consumidor, ni tests    : {len(sin_consumidor)}")
print("\n--- consumidos SOLO por pruebas (ni un comando los llama) ---")
for mod, f, n in sorted(solo_tests):
    print(f"  {mod:34s} {f}  ({n} ficheros de test)")
print("\n--- modulos sin consumidor de producto ---")
for mod, f in sorted(sin_consumidor):
    print(f"  {mod:34s} {f}")
