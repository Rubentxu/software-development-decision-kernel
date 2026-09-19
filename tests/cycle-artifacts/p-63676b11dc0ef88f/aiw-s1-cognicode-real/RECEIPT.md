# AIW-S1 Receipt — CogniCode real → VerifyKernel

## Capacidad A6 implementada
Cadena vertical completa: proveedor CogniCode real (stdio MCP) →
SoftwareObservation tipada con basis canónico → VerifyKernel →
veredicto verificable. El CLI `sddk verify-kernel --domain static_provider`
es el primer comando que consume un proveedor de inteligencia externo
y produce un veredicto del kernel con evidencia tipada.

## Evidencia OBSERVED (ejecución real, 2026-09-19)
- E2E: `sddk verify-kernel --domain static_provider --claim port-isolation
  --subject unit:symbol:CodeIntelligencePort --provider-bin <cognicode-mcp> --root .`
  → verdict: **Verified**, 10 observaciones, observation_set digest SHA-256
  (OBSET|0916e8c1...).
- EXT tests (COGNICODE_MCP_BIN): **5/5 passed** contra binario real
  `cognicode-mcp v0.97.1` (107s; incluye A02 relación positiva revisada 2x).
- Base (sin proveedor): A03 negativo + A06 digest → **2 passed**.
- Negativos CLI: sin `--provider-bin` → error; binario inexistente →
  `provider unavailable`. Exit codes correctos.
- Regresión: `cargo test -p sddk-cli` → **1251 passed, 0 failed**
  (incl. context_fitness 7/7 tras addendum ADR-0137).
- CC-S0 contract tests: **13 passed** (SHA-256 no rompe los 6 existentes).

## Semántica del observation set (10 observaciones)

### Observación agregada `Unit` (índice 0 del set)
- **Subject:** `ObservationSubject::Unit(SoftwareUnitRef("symbol:<name>"))`.
  Su canonical_tag es `unit:symbol:<name>` — exactamente el tag que la
  claim nombra en `--subject`. Existe para que la claim referencie un
  subject real del set (el bug de mismatch detectado en el primer E2E:
  las observaciones relation producían tags `relation:<id>` que ninguna
  claim puede anticipar, porque el id es derivado).
- **Stance:** `Affirms`.
- **Evidence locator:** `cognicode-mcp://find_usages/<name>#usages=<N>`,
  donde N es el número de usages devueltos por el proveedor. El locator
  es la única vía de recuperar N; la observación no copia las filas de
  uso (esas viven en las observaciones relation).
- **Origin/basis:** `StaticProvider` / `ObservationBasis::for_provider_result`
  (derivado del digest SHA-256 del material canónico del input, no del
  proveedor).
- **Semántica:** "el proveedor observó N usos del símbolo en el grafo
  actual". NO afirma nada sobre localización ni clasificación de esos
  usos; para eso están las relation.

### 9 observaciones relation (una por usage)
- **Subject:** `ObservationSubject::Relation(SoftwareRelation(
    from: unit("rust:<file>:<line>"), kind: DependsOn, to: unit("symbol:<name>")))`.
  Cada una localiza un uso concreto (fichero+línea del provider).
- **Stance:** `Affirms` — el uso observado afirma la dependencia.
- Mismo origin/basis que la Unit. Sus canonical_tags (`relation:<sha>`)
  NO son nombrables por claims; son evidencia de soporte, no subject
  del veredicto.

### Relación con la claim
`StaticProviderClaim { subject_tag: "unit:symbol:<name>", contract_id }`
se evalúa así: el kernel busca observaciones cuyo subject tag coincida;
encuentra la Unit agregada. Las 9 relation NO matchean el subject, pero
quedan en el set y sustentan el digest. Veredicto `Verified` = la Unit
agregada affirma con basis fresco y no hay ninguna observación con
stance `Denies` sobre ese subject en el set.

### Invariante falsable
Si el proveedor devolviera 0 usages, el CLI falla (no emite observación
Unit vacía): `provider produced no usable usage observations`. La claim
no puede salir `Verified` por defecto sin evidencia.

## Hallazgos del ciclo
1. subject mismatch detectado y corregido: el CLI emite además una
   observación Unit agregada (`unit:symbol:<name>`) con el count de
   usages, para que la claim pueda nombrar el subject directamente.
2. `context_fitness` gate: módulo root-level nuevo exige ADR →
   addendum en ADR-0137 (no baseline hack).
3. Incidencias registradas por separado (fuera de S1, por decisión del
   operador): bug upstream paginación tools/list (~54k entradas
   repetidas); endpoint HTTP del editor sin listener en 127.0.0.1:9847.

## Limitaciones conservadas (no resueltas en S1, de S0 + S1)
- impacted_files vacío con grafo lightweight ≠ ausencia de impacto
  (S0 DISCOVERY): el veredicto se basa en find_usages, no en
  impacted_files, pero el agregado N refleja solo lo que el grafo
  lightweight alcanzó a indexar.
- Grafo lightweight (41.607 símbolos, ~35s en S0): cobertura parcial
  del workspace; N puede ser menor que el número real de usos en el
  código. La claim "port-isolation" es, por tanto, "Verified según el
  grafo provisto por el proveedor", no un recuento absoluto.
- S1 NO declara STATIC_ENHANCED: solo demuestra la cadena vertical.
- proveedor arranca por stdio únicamente; sin daemon (AIS-004).
- paginación tools/list con bug upstream (~54k entradas repetidas):
  S1 solo consume find_usages/build_graph, no el catálogo completo.

## Estado
- Release: **v1.169.91** — tag y HEAD en `22e4446`, pusheado y verificado
  (9 assets, no-draft, gate público OK, install local coherente:
  binary 1.169.91, bundle 1.169.91, doctor all_present, prune OK).
- binary_sha256: `sha256:7ce3ad999cb02bc9c703dfe4ba9b52ccabb162bb690eeb1cafdfd33820bc1d8c`.
- Nota: un bump espurio a 1.169.90 (`959b9b7`, sin release) quedó en
  origin/main por un intento prematuro de release; el release legítimo
  es monotónico sobre él (1.169.90 → 1.169.91).

## Reconciliación A6: qué satisface S1 y qué queda pendiente
S1 satisface:
- Conexión vertical real proveedor→observación→kernel→veredicto (núcleo A6).
- Digests SHA-256 reales y basis canónico derivado del resultado.
- Pruebas negativas sin proveedor (Base green) y con proveedor caído.
- Primer dominio del kernel (static_provider) que consume ObservationSet.

A6 pendiente (fuera de S1 por decisión del operador):
- STATIC_ENHANCED: NO declarado; requiere cobertura de grafo completa,
  no lightweight.
- Persistencia de ObservationSets y consultas históricas (AIW-S2,
  captura de resultados de tests): sin implementar.
- Representación de contradicciones reales del proveedor (stance Denies
  con evidencia): modelada en el kernel pero sin caso real observado.
  Abrir AIW-S1b SOLO si una prueba real demuestra una carencia de
  persistencia o representación de contradicciones.
- Operators adicionales, agenda, segundo store, endpoint HTTP del
  editor: fuera de alcance.
