#!/usr/bin/env python3
"""Verifica que el repo SATISFACE ADR-0155 (C3m.3, provenance provider-neutral).

Que este script afirme el estado POST y no el estado PRE es una decision, no un
detalle: la primera version afirmaba que el defecto seguia presente
(`ProviderKind::CogniCode` como variante, el guard viejo llamandose
`t_ar_5c_no_cognicode_type`, el doc diciendo que los valores de produccion se
anadirian). Esos checks dieron ROJO en cuanto el defecto se corrigio — un
verificador que se pone rojo cuando arreglas el bug no verifica el
requerimiento, verifica que el defecto siga ahi.

Lo que se midio ANTES vive en el SCOPE como historia; lo que se verifica aqui es
lo que el repo tiene que CUMPLIR ahora.

Controles del propio instrumento, con dos casos conocidos, antes de publicar
ningun veredicto:
  - el puerto NO debe nombrar ningun proveedor como variante  (esperado: limpio)
  - el puerto DEBE poder expresar la identidad como dato     (esperado: presente)
Si alguno sale al reves, el instrumento esta DESCALIBRADO.
"""
import pathlib
import re
import sys

_AQUI = pathlib.Path(__file__).resolve()
_RAIZ = next((p for p in _AQUI.parents if (p / "crates" / "sddk-engine").is_dir()),
             None)
if _RAIZ is None:
    sys.exit("no encuentro crates/sddk-engine subiendo desde " + str(_AQUI.parent))

ENGINE = _RAIZ / "crates/sddk-engine/src"
PORT = ENGINE / "code_intelligence_port.rs"
FAKE = ENGINE / "code_intelligence_port_fake.rs"
MCP = ENGINE / "code_intelligence_port_mcp.rs"
CB = ENGINE / "circuit_breaker.rs"
LINT = _RAIZ / "crates/sddk-cli/tests/context_fitness.rs"
GUARD_F = _RAIZ / "crates/sddk-engine/tests/a6_cc_s1_static_graph_completeness.rs"

fallos = []


def comprobar(nombre, cond, detalle=""):
    ok = bool(cond)
    print(f"  {'OK   ' if ok else 'FALLA'} {nombre}" + (f"  ({detalle})" if detalle else ""))
    if not ok:
        fallos.append(nombre)
    return ok


def sin_comentarios(fuente):
    """El fuente SIN comentarios de linea ni de bloque.

    Existe por una razon concreta, y es la tercera vez en esta sesion que el
    mismo caso muerde: al reescribir el guard, su comentario EXPLICA que el
    guard viejo se llamaba `t_ar_5c_no_cognicode_type_in_sddk_engine` y hacia
    `assert!(src.contains("ProviderKind::CogniCode"))`. Un check que busca esas
    cadenas en el fichero crudo las encuentra en la explicacion de por que se
    quitaron, y falla — o, peor, para de comprobar si se relaja. **citar lo
    retirado es correcto; usarlo es el defecto**, y distinguitrlo exige quitar
    los comentarios en vez de buscar el token entero.
    """
    fuente = re.sub(r"/\*.*?\*/", " ", fuente, flags=re.S)
    return "\n".join(re.sub(r"//.*$", "", l) for l in fuente.splitlines())


def nombres_de_test(fuente):
    """Los NOMBRES declarados de los tests, no las menciones a ellos."""
    return set(re.findall(r"fn (t_ar_5c\w*)\s*\(", fuente))


port = PORT.read_text(encoding="utf-8")
fake = FAKE.read_text(encoding="utf-8")
mcp = MCP.read_text(encoding="utf-8")

# ── CONTROLES PRIMERO ─────────────────────────────────────────────────────────
print("Controles del instrumento (si fallan, ningun veredicto vale):")
c1 = re.search(r'pub enum ProviderKind \{(.*?)\n\}', port, re.S) is not None
c2 = "pub provider_id: String" in port
print(f"  control  el puerto define ProviderKind            : {c1}  "
      f"{'OK' if c1 else 'DESCALIBRADO'}")
print(f"  control  el puerto expresa la identidad como dato: {c2}  "
      f"{'OK' if c2 else 'DESCALIBRADO'}")
if not (c1 and c2):
    sys.exit("INSTRUMENTO DESCALIBRADO: el puerto no tiene la forma que se va a verificar.")
print()

# ── A. El enum ya no nombra proveedores ──────────────────────────────────────
print("A. El enum expresa estado de capacidad, no identidad de producto")
m = re.search(r'pub enum ProviderKind \{(.*?)\n\}', port, re.S)
comprobar("ProviderKind sigue siendo pub en el puerto", m is not None)
cuerpo = m.group(1) if m else ""
variantes = re.findall(r"^\s{4}(\w+),", cuerpo, re.M)
comprobar("sus variantes son Null/Fake/External",
          variantes == ["Null", "Fake", "External"], str(variantes))
comprobar("ninguna variante es un nombre de proveedor",
          not any(v in ("CogniCode", "Chronos", "CodeIntelligence") for v in variantes))
comprobar("ya no hay #[allow(dead_code)] en el enum",
          "allow(dead_code)" not in cuerpo)
plano = re.sub(r"\s+", " ", re.sub(r"//[/!]?", " ", port))
comprobar("el doc ya NO promete anadir valores de produccion",
          "will be added in CC-S1+" not in plano)
comprobar("el doc declara que la identidad viaja como dato",
          "capability state, not product identity" in plano
          or "estado de capacidad" in plano)
comprobar("el doc cita ADR-0155", "ADR-0155" in plano)

# ── B. La identidad es dato, y la declara el adaptador ───────────────────────
print("\nB. La identidad del proveedor es DATO, declarado por el adaptador")
comprobar("ObservationSet tiene provider_id: String",
          "pub provider_id: String" in port)
comprobar("el adaptador MCP declara PROVIDER_ID", "pub const PROVIDER_ID" in mcp)
comprobar("y su valor es el id del proveedor real",
          '"cognicode-mcp"' in mcp)
comprobar("el fake declara su propio PROVIDER_ID", "pub const PROVIDER_ID" in fake)
comprobar("el id del fake es explicito, no vacio (para que no se confunda con Null)",
          '"fake-inprocess"' in fake)
comprobar("el adaptador MCP reporta External, no Fake",
          mcp.count("provider_kind: ProviderKind::External,") == 2
          and "ProviderKind::Fake" not in mcp,
          f"External x{mcp.count('provider_kind: ProviderKind::External,')}")
comprobar("Null no declara id en ningun constructor",
          "ProviderKind::Null,\n" in fake
          and not re.search(r"provider_kind: ProviderKind::Null,\s*\n\s*provider_id: PROVIDER_ID",
                            fake))

# ── C. El otro ProviderKind no se toco ──────────────────────────────────────
print("\nC. El homonimo de circuit_breaker sigue intacto y neutral")
cb = CB.read_text(encoding="utf-8")
mcb = re.search(r'pub enum ProviderKind \{(.*?)\n\}', cb, re.S)
comprobar("circuit_breaker::ProviderKind sigue existiendo", mcb is not None)
if mcb:
    v2 = re.findall(r"^\s{4}(\w+),", mcb.group(1), re.M)
    comprobar("y sus variantes siguen siendo categorias",
              v2 == ["Llm", "Tool", "Mock", "Deterministic"], str(v2))
comprobar("ProviderIdentity sigue siendo el tipo de identidad de ruta",
          re.search(r"pub struct ProviderIdentity \{", cb) is not None)

# ── D. El lint no se extends ni se cita mal ──────────────────────────────────
print("\nD. El lint sigue midiendo lo que dice medir")
txt_lint = LINT.read_text(encoding="utf-8")
ml = re.search(r"fn no_knowledge_to_provider_sdk\(\) \{(.*?)\n\}", txt_lint, re.S)
comprobar("el lint existe", ml is not None)
if ml:
    modulos = re.findall(r'"([a-z_]+\.rs)"', ml.group(1))
    comprobar("sigue escaneando los mismos 5 modulos de knowledge",
              modulos == ["semantic_graph.rs", "semantic_node.rs",
                          "semantic_kind.rs", "vault_boundary.rs",
                          "why_queries.rs"], str(modulos))
    comprobar("y sigue SIN mirar el puerto (el puerto debe hablar con el proveedor)",
              "code_intelligence_port.rs" not in ml.group(1))

# ── E. El guard reescrito ────────────────────────────────────────────────────
print("\nE. El guard se llama por la propiedad y la comprueba")
g = GUARD_F.read_text(encoding="utf-8")
g_codigo = sin_comentarios(g)
declarados = nombres_de_test(g)
comprobar("existe t_ar_5c_no_provider_name_in_the_core_port",
          "t_ar_5c_no_provider_name_in_the_core_port" in declarados, str(sorted(declarados)))
comprobar("el nombre viejo no_cognicode_type ya no DECLARA un test",
          "t_ar_5c_no_cognicode_type_in_sddk_engine" not in declarados,
          "puede seguir citandolo en el comentario, que es lo correcto")
comprobar("el guard ya no AFIRMA la presencia del nombre (codigo, no comentario)",
          'assert!(src.contains("ProviderKind::CogniCode"))' not in g_codigo)
comprobar("vigila los tres nombres de proveedor",
          all(f'"{n}"' in g_codigo for n in ("CogniCode", "Chronos", "CodeIntelligence")))
comprobar("parsea VARIANTES en vez de buscar el token en el cuerpo crudo",
          "filter_map" in g_codigo and "variantes.contains(&provider)" in g_codigo)
comprobar("afirma que ha encontrado algo que comprobar",
          "no se han podido extraer las variantes" in g_codigo)
comprobar("exige que la identidad siga siendo expresable",
          "pub provider_id: String" in g_codigo and "ProviderKind::External" in g_codigo)
comprobar("vigila que Null no declare un id",
          "ProviderKind::Null," in g_codigo and "String::new()" in g_codigo)
comprobar("ya no cita el lint como si lo aplicara 'precisamente'",
          "enforces this precisely" not in g)

# ── F. El otro ObservationSet no fue tocado ──────────────────────────────────
print("\nF. El ObservationSet de observation::types (payload v1) no fue tocado")
obs = ENGINE / "observation/types.rs"
comprobar("el fichero existe", obs.exists())
if obs.exists():
    t = obs.read_text(encoding="utf-8")
    m2 = re.search(r'#\[derive\(([^)]*)\)\]\s*\npub struct ObservationSet', t, re.S)
    comprobar("sigue derivando Serialize y Deserialize",
              m2 is not None and "Serialize" in m2.group(1) and "Deserialize" in m2.group(1),
              m2.group(1).strip() if m2 else "sin derive")
    comprobar("y NO ha ganado un campo provider_id (contrato v1 congelado)",
              not re.search(r'pub struct ObservationSet \{(.*?)\n\}', t, re.S)
              or "provider_id" not in re.search(r'pub struct ObservationSet \{(.*?)\n\}',
                                                 t, re.S).group(1))

print()
if fallos:
    print(f"FALLOS ({len(fallos)}): {fallos}")
    print("El repo NO satisface ADR-0155: no se publica el cierre.")
    sys.exit(1)
print(f"ADR-0155 verificado en el repo: todas las condiciones se sostienen ({len(fallos)} fallos)")
