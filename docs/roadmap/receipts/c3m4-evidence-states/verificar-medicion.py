#!/usr/bin/env python3
"""Verifica, una a una, las afirmaciones del SCOPE de C3m.4.

El SCOPE cita: rutas que existen, lineas que dicen lo que dice, y conteos.
Cada afirmacion se comprueba contra el disco, no contra la memoria. Si una
fallo, sale con codigo 1 y el nombre del fallo, y NO se publica el documento.

Un documento de medicion que no se autocomprueba tiene el mismo problema que
un guard que no se falsifica: afirma mas de lo que ha medido.
"""
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
GW = REPO / "crates/sddk-gateway/src/storage_snapshot_l1_consumer.rs"
GW_T = REPO / "crates/sddk-gateway/tests/aiw_s7b_snapshot_l1_consumer.rs"
SEL = REPO / "crates/sddk-domain/src/test_select.rs"
SPEC_CITADO = (REPO / "tests/cycle-artifacts/p-63676b11dc0ef88f/"
               "aiw-s7b-storage-snapshot-l1-consumer/SCOPE-CONTRACT.md")
SPEC_REAL = (REPO / "tests/cycle-artifacts/p-63676b11dc0ef88f/"
             "aiw-s7-secretary-attention/SCOPE-CONTRACT.md")
UAT = (REPO / "docs/history/proposals/all-proposals/"
       "2026-09-19-adaptive-inputs-workflows/uat/UAT-MATRIX.md")
RECEIPT = (REPO / "tests/cycle-artifacts/p-63676b11dc0ef88f/"
           "aiw-s7b-storage-snapshot-l1-consumer/RECEIPT.md")
EVID = (REPO / "tests/cycle-artifacts/p-63676b11dc0ef88f/"
        "aiw-s7b-storage-snapshot-l1-consumer/UAT-EVIDENCE.yaml")

fallos = []


def comprobar(nombre, condicion, detalle=""):
    ok = bool(condicion)
    print(f"  {'OK  ' if ok else 'FALLA'} {nombre}" + (f"  ({detalle})" if detalle else ""))
    if not ok:
        fallos.append(nombre)
    return ok


print("A. La cita rota del modulo")
comprobar("el SCOPE que cita :6 NO existe (es el hallazgo)",
          not SPEC_CITADO.exists(),
          "el directorio si existe: " + str(SPEC_CITADO.parent.is_dir()))
comprobar("el directorio del ciclo si existe, con RECEIPT y UAT-EVIDENCE",
          SPEC_CITADO.parent.is_dir() and RECEIPT.exists() and EVID.exists())
txt_gw = GW.read_text(encoding="utf-8")
comprobar("la linea 6 cita ese path", "aiw-s7b-storage-snapshot-l1-consumer/SCOPE-CONTRACT.md" in txt_gw)

print("\nB. G01 no menciona confidence en ninguna de sus fuentes reales")
txt_spec = SPEC_REAL.read_text(encoding="utf-8")
comprobar("el SCOPE real de G01 SI existe", SPEC_REAL.exists())
comprobar("el SCOPE real tiene 0 menciones de 'confidence'",
          "confidence" not in txt_spec.lower(),
          f"{len(re.findall('confidence', txt_spec, re.I))} menciones")
comprobar("el SCOPE real contiene el texto de G01 citado",
          "snapshot Planning reconciliado, A bloquea B" in txt_spec)

print("\nC. La fila canonica de G01 no menciona confidence")
filas = [l for l in UAT.read_text(encoding="utf-8").splitlines()
         if l.strip().startswith("| G01")]
comprobar("la fila canonica de G01 existe y es una", len(filas) == 1,
          f"{len(filas)} filas")
if filas:
    fila = filas[0]
    comprobar("la fila canonica NO contiene 'confidence'",
              "confidence" not in fila.lower())
    comprobar("la fila canonica dice lo que el SCOPE transcribe",
              "A bloquea B" in fila and "project_next" in fila)

print("\nD. La clausula de la confianza la escribieron el RECEIPT y la evidencia")
txt_r = RECEIPT.read_text(encoding="utf-8")
txt_e = EVID.read_text(encoding="utf-8")
# Se fija LA FILA de G01 del receipt, no una busqueda de subcadena en el
# fichero entero: la primera version buscaba "PASS" en el receipt y eso lo
# encontra en filas ajenas, asi que la mutacion M5 (G01 deja de decir PASS)
# sobrevivia. Un check que se puede volver verde por el motivo equivocado no
# esta vigilando lo que dice vigilar.
filas_r = [l for l in txt_r.splitlines() if l.strip().startswith("| G01")]
comprobar("el RECEIPT tiene una fila de G01", len(filas_r) == 1, f"{len(filas_r)} filas")
if filas_r:
    fila_r = filas_r[0]
    comprobar("esa fila marca G01 como PASS", "| G01 | PASS |" in fila_r, fila_r[:70])
    comprobar("esa fila es la que introduce la clausula de la confianza",
              "0.95/0.5" in fila_r)
comprobar("UAT-EVIDENCE.yaml declara 0.95 y 0.5",
          "0.95" in txt_e and "0.5" in txt_e)
comprobar("UAT-EVIDENCE.yaml declara que su scope es el OTRO SCOPE",
          "aiw-s7-secretary-attention/SCOPE-CONTRACT.md" in txt_e)

print("\nE. Lecturas de confidence en codigo de producto (excluyendo #[cfg(test)])")


def fuera_de_tests(lineas_src):
    """Indices de linea que NO estan dentro de un `#[cfg(test)] mod tests`.

    Un `#[cfg(test)]` puede vivir dentro de un fichero de `src/`, asi que
    excluir solo el directorio `tests/` no basta: la primera version de este
    check conto `secretary_l1.rs:362` (un assert) como si fuera produccion.

    Y la segunda version acotaba desde el `#[cfg(test)]` hasta el FIN del
    fichero, lo que hacia invisible cualquier codigo de produccion escrito
    DESPUES del modulo de test — que es donde la mutacion M7 coloco su noveno
    consumidor. Se cierra por CONTEO DE LLAVES hasta devolver la profundidad a
    cero, que es donde acaba el modulo.
    """
    vivos = set(range(len(lineas_src)))
    for i, l in enumerate(lineas_src):
        if not l.strip().startswith("#[cfg(test)]"):
            continue
        # avanzar hasta la linea donde el modulo se cierra
        depth = 0
        abierto = False
        for j in range(i, len(lineas_src)):
            depth += lineas_src[j].count("{") - lineas_src[j].count("}")
            if "{" in lineas_src[j]:
                abierto = True
            if abierto and depth <= 0:
                for k in range(i, j + 1):
                    vivos.discard(k)
                break
    return vivos


lectores = []
for p in (REPO / "crates").rglob("*.rs"):
    if "tests" in p.parts or p.name.endswith("_tests.rs"):
        continue
    lineas = p.read_text(encoding="utf-8", errors="replace").splitlines()
    vivos = fuera_de_tests(lineas)
    for i, l in enumerate(lineas):
        if i not in vivos:
            continue
        m = re.search(r'\.confidence\b', l)
        if not m:
            continue
        resto = l[m.end():]
        if re.match(r'\s*=(?!=)', resto):
            continue  # escritura, no lectura
        lectores.append((str(p.relative_to(REPO)), i + 1, l.strip()[:78]))

print(f"  lecturas de .confidence en produccion: {len(lectores)}")
for f, n, l in lectores:
    print(f"    {f}:{n}  {l}")

# Lo que se afirma en el SCOPE no es «no hay ninguna lectura», sino que la
# lectura de la confianza de un SecretaryProposal no GOBERNA nada. Se fija el
# conjunto conocido para que un consumidor nuevo lo haga caer.
comprobadas = {f"{f}:{n}" for f, n, _ in lectores}
# Se fija el conjunto EXACTO de lecturas en produccion. No para declarar que
# no hay ninguna —hay 9— sino para que una decima caiga y obligue a mirarla.
# Y se afirma lo que si se ha medido: ninguna de las 9 es de un
# `SecretaryProposal`. Son de ContinuationCandidate, del trigger de
# dynamic_expansion, de AgentContributionEnvelope, de UatOracleAssessment y de
# TestSelectionPlanV1: cuatro tipos mas, cada uno con su propio contrato.
#
# NOTA DE REVISION: el primer recuento dio 8. El corte por `#[cfg(test)]`
# acababa en el FIN del fichero, y por eso no veía `sddk-cli/src/uat.rs:3896`
# (el CLI mostrando la confianza de un assessment), que queda despues del
# modulo de test de ese fichero. El numero subio a 9 al arreglar el corte: un
# corte que se salta codigo de produccion hace el numero MAS pequeno, y un
# numero mas pequeno parece mas tranquilizador. Por eso la correccion va en el
# check y no a la baja.
ESPERADAS = {
    "crates/sddk-engine/src/continuation_candidate.rs:302",
    "crates/sddk-engine/src/continuation_candidate.rs:305",
    "crates/sddk-engine/src/dynamic_expansion.rs:127",
    "crates/sddk-engine/src/dynamic_expansion.rs:415",
    "crates/sddk-engine/src/agent_contribution_envelope.rs:451",
    "crates/sddk-domain/src/uat.rs:595",
    "crates/sddk-domain/src/test_model.rs:965",
    "crates/sddk-domain/src/test_model.rs:967",
    "crates/sddk-cli/src/uat.rs:3896",
}
comprobar("las lecturas de produccion son las 9 conocidas (una decima obliga a mirar)",
          comprobadas == ESPERADAS,
          f"nuevas: {sorted(comprobadas - ESPERADAS)}")
comprobar("ninguna lectura en produccion toca la confianza de un SecretaryProposal",
          not any("storage_snapshot_l1_consumer" in f for f, _, _ in lectores))
comprobar("la unica escritura literal de 0.95/0.5 en produccion es la linea 124",
          "let confidence = if snapshot.log_head > 0 { 0.95 } else { 0.5 };"
          in GW.read_text(encoding="utf-8"))

print("\nF. La rama de test_select.rs:709 es inalcanzable")
txt_s = SEL.read_text(encoding="utf-8")
lineas = txt_s.splitlines()
idx_709 = 708
comprobar("la linea 709 es la computacion de confidence",
          "has_unmapped { 0.0 } else { 1.0 }" in lineas[idx_709])
# el return temprano que la precede
temprano = any("if prop.has_unmapped {" in l for l in lineas[680:706])
retorno = any("return Err(AdapterError::InvalidInput {" in l for l in lineas[680:706])
comprobar("hay un return temprano por la MISMA condicion antes de la 709",
          temprano and retorno)
apariciones = [(i + 1, l) for i, l in enumerate(lineas) if "has_unmapped" in l]
es_lectura = [(n, l) for n, l in apariciones
              if "let has_unmapped" not in l and "has_unmapped," not in l
              and "has_unmapped: bool" not in l]
comprobar("has_unmapped solo se lee, nunca se muta tras construirlo",
          all(("if prop.has_unmapped" in l) or ("let confidence" in l)
              for _, l in es_lectura),
          f"lecturas en {[n for n, _ in es_lectura]}")

print()
if fallos:
    print(f"FALLOS ({len(fallos)}): {fallos}")
    print("El SCOPE afirma cosas que el repo no sostiene: no se publica.")
    sys.exit(1)
print(f"todas las afirmaciones del SCOPE verificadas ({len(fallos)} fallos)")
