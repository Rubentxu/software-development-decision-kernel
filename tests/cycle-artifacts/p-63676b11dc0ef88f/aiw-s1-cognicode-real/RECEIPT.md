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
- Commits: S0 `69f68a7`, S1 discovery `50c9238`, S1 código (este).
- Push: BLOQUEADO por hook (artifacts fuera de allowlist, sin bump).
  Publicación con el próximo release legítimo (v1.169.90).
