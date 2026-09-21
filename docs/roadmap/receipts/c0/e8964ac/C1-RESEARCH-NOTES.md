# C1-RESEARCH-NOTES — Preguntas concretas para el SCOPE-CONTRACT

> **Slice id:** `p-63676b11dc0ef88f/c1-contracts-hardening` (research stage)
> **Date (UTC):** 2026-09-21T12:06:00Z
> **Status:** DRAFT — NO SE EJECUTA hasta que el operador publique la release v1.169.128.
> **ROADMAP ref:** [C1 §2](../ROADMAP.md) (`docs/roadmap/ROADMAP.md` lines 36-43)

## §0 Why this document exists now

C0 está cerrado con `PASS_OBSERVED_WITH_NOTES`. La release v1.169.128 está pendiente del operador (system-law `git.release` human-gate). El ROADMAP §3 ("No pasar a C1 con repo dirty o SHA ambiguo") y el §2 C0 ("No modificar el histórico para forzar coherencia") prohíben arrancar C1 con workspace ahead of public release.

Sin embargo, hay investigación preparatoria que **no requiere SHA post-release** y **no toca código**: identificar las preguntas concretas que el SCOPE-CONTRACT de C1 tendrá que responder, los archivos a tocar, y los escenarios de prueba a diseñar. Esto se hace antes para que, cuando el operador confirme la release, el SCOPE-CONTRACT se emita con criterio en lugar de improvisarse.

Esto NO es un SCOPE-CONTRACT. NO es una propuesta. Es un cuaderno de investigación para que C1 arranque con substance.

## §1 H01 — `structured_work::shape_matches` no debe devolver `true` para descriptor no soportado

**Pregunta de investigación:**
- ¿Cómo está implementado `shape_matches` hoy en `crates/sddk-domain/src/structured_work.rs`?
- ¿Qué hace cuando recibe un descriptor con `schema_version` desconocido o campos requeridos faltantes?
- ¿Cuál es la rama de retorno para "descriptor soportado" — por `==` con un set de descriptores conocidos, por hash, por nombre de tipo, por lista explícita?

**Archivos candidatos a tocar:**
- `crates/sddk-domain/src/structured_work.rs` (función `shape_matches`)
- `crates/sddk-domain/tests/structured_work_negative.rs` o similar (tests de soporte)
- ADRs 0011..0097 que documenten el contrato de schema

**Escenarios UAT a diseñar (T03–T07):**
1. Descriptor con `schema_version` futura (e.g. `2.0` cuando el max conocido es `1.x`) → debe rechazar con error tipado.
2. Descriptor con campos requeridos faltantes explícitos → rechazar.
3. Descriptor con campos adicionales no documentados → rechazar (no aceptar "extensibilidad silenciosa").
4. Descriptor con tipo conocido pero `schema_version` stale → rechazar (versionado estricto).
5. Descriptor válido contra el catálogo actual → aceptar.

**Test-seam potencial:**
- `set_process_service_for_tests` (mencionado en H05) si `shape_matches` está acoplado al service.

**Riesgos:**
- Cambiar el contrato de `shape_matches` puede romper a clientes que ya envían descriptores con campos extra "útiles" (compatibilidad con adapters JCode que añaden campos por su cuenta).
- Necesidad de versionar el contrato: ¿la regla es `strict` por default o `strict + allowed_keys per profile`?

## §2 H02 — `request_id` duplicado no sustituye silenciosamente peticiones no equivalentes

**Pregunta de investigación:**
- ¿Dónde se valida la equivalencia entre dos requests con mismo `request_id`?
- ¿Se hace hash del contenido + request_id, o solo del request_id?
- ¿Qué pasa cuando dos procesos emiten el mismo `request_id` con `payload` distinto?
- ¿Hay un store de `request_id → payload_digest` o se computa bajo demanda?

**Archivos candidatos:**
- `crates/sddk-engine/src/request_dedup.rs` o equivalente.
- `crates/sddk-engine/src/gateway/` para el path de entrada.
- Tests de idempotencia existentes.

**Escenarios:**
1. Mismo `request_id`, mismo `payload` → idempotente (devuelve el mismo resultado).
2. Mismo `request_id`, payload distinto → **NO sustituir**; debe rechazar con error `RequestIdCollision` o equivalente.
3. Mismo `request_id`, payload con campos sensibles distintos (e.g. `target` apunta a otro usuario) → rechazar sin filtrar el campo sensible.
4. `request_id` colisiona entre dos procesos concurrentes → serialización correcta.

**Schema de errores tipados:**
- Confirmar que los errores no interpolan valores sensibles. Ejemplo incorrecto: `Error: payload { "user_secret": "abc" } conflict`. Correcto: `Error: payload conflict on field 'user_secret' (redacted)`.

## §3 H05 — test-seam `set_process_service_for_tests`

**Pregunta de investigación:**
- ¿Existe tal función en el código actual? ¿Cómo está nombrada realmente? (`set_process_service_for_tests` puede ser nombre conceptual).
- ¿Está gated por `#[cfg(test)]` o por feature flag?
- ¿Se invoca desde código de producción? (si sí, es un agujero).

**Archivos candidatos:**
- `crates/sddk-engine/src/service.rs` (si existe ese módulo).
- Cualquier `pub fn set_*_for_tests` o similar.

**Escenarios:**
- Verificar que el seam existe, está aislado, y devuelve el mismo resultado que producción.
- Verificar que ningún código de tests puede invocarse desde el binario público.

## §4 H06 — defensa gateway de argumentos/bytes + redacción completa de salidas

**Pregunta de investigación:**
- ¿Dónde está el gateway que recibe bytes desde el host?
- ¿Cómo se truncan / limitan payloads?
- ¿Cómo se redactan secretos en logs / receipts?
- ¿Hay un patrón canónico de "redaction helper" o cada subsistema lo hace ad-hoc?

**Archivos candidatos:**
- `crates/sddk-gateway/src/` (entrypoints HTTP/MCP).
- `crates/sddk-engine/src/redaction.rs` o similar.
- Todos los lugares donde se serializa output a un sink público.

**Escenarios:**
1. Args de 10MB → rechazo antes de procesar.
2. Args con bytes nulos / no-UTF-8 → rechazo limpio.
3. Env vars con `SDDK_TOKEN=…` → redacción en logs.
4. Errores que propagan paths absolutos del host → redacción.
5. Receipts que contienen hashes de inputs sensibles → redacción de hashes si el input era secreto.

## §5 Trabajo de caracterización previo (RED→GREEN)

Para cada H0X:
1. **Caracterización RED**: test que reproduce el bug actual (probablemente pasará como "PASA por código incorrecto" o fallará con un mensaje distinto).
2. **Aplicación mínima**: el cambio más pequeño que cambia el comportamiento al esperado.
3. **Caracterización GREEN**: el mismo test pasa ahora con el comportamiento correcto.
4. **No-regresión**: suite existente sigue verde.

## §6 Compatibilidad y migración

**Pregunta clave:** ¿Los cambios de H01/H02 son breaking? Si sí:
- ¿Hay adapter legacy que los abráse el tiempo de una ventana de migración?
- ¿Los recibos certificados A5-* siguen siendo válidos después del cambio? (Probable: sí, porque el contrato externo no cambia — el bug está en cómo se evalúa el contrato).

## §7 Entregables esperados del ciclo C1 (no comprometidos aún)

- 4 commits de `fix(engine)` o similar (uno por H0X), cada uno con:
  - Test de caracterización (negativo) que falla en el commit anterior.
  - Aplicación mínima.
  - Test positivo + suite del crate verde.
- 1 commit `chore(release)` bump para el merge final.
- SCOPE-CONTRACT, UAT-EVIDENCE (T03–T07), C1-RECEIPT.
- Actualización de CURRENT + STATE + SESSION-JOURNAL.

## §8 Stop conditions explícitos

- Si `shape_matches` resulta estar en un crate diferente al asumido → STOP, re-investigar.
- Si `set_process_service_for_tests` no existe como tal → STOP, el H05 puede ser conceptual.
- Si la redacción de outputs requiere un crate nuevo → STOP, proponer ADR-XXXXX antes de implementar.
- Si los cambios rompen tests A5-* certificados → STOP, marcar R-WAVE antes de avanzar.

## §9 Fuera de scope (defendido)

- C2 (CogniCode, Chronos, JCode real): distinto DAG, distinto release.
- C3 (seguridad adversarial): paralelo a C2, después de C1.
- Performance budgets (C3 §5 del ROADMAP): sin baseline previo, no optimizable.
- Nuevos crates o refactors: no introducidos por C1.

## §10 Operador + orquestador: próximos pasos

**Operador (sigue pendiente, system-law):**
```bash
cd ~/Proyectos/agentesIA/sddk-framework
bash scripts/release.sh   # publica v1.169.128
```

**Orquestador (autónomo, post-release confirmado):**
1. Re-correr T01; si `cargo package version == gh release vX.Y.Z`, marcar C0 CLOSED en STATE.yaml.
2. Emitir `SCOPE-CONTRACT.md` para `c1-contracts-hardening` partiendo de este research-notes.
3. Aplicar H01, H02, H05, H06 con método RED→GREEN.
4. C1-RECEIPT + cerrar el ciclo.

**NO SE EMITE SCOPE-CONTRACT hasta que el release esté hecho.** Este research-notes es la nota de contexto para que el SCOPE-CONTRACT se redacte con substance.
