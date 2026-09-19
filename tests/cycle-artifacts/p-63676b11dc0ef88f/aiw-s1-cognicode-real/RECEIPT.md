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

## Hallazgos del ciclo
1. subject mismatch detectado y corregido: el CLI emite además una
   observación Unit agregada (`unit:symbol:<name>`) con el count de
   usages, para que la claim pueda nombrar el subject directamente.
2. `context_fitness` gate: módulo root-level nuevo exige ADR →
   addendum en ADR-0137 (no baseline hack).
3. Incidencias registradas por separado (fuera de S1, por decisión del
   operador): bug upstream paginación tools/list (~54k entradas
   repetidas); endpoint HTTP del editor sin listener en 127.0.0.1:9847.

## Limitaciones conservadas (no resueltas en S1)
- impacted_files vacío con grafo lightweight ≠ ausencia de impacto.
- S1 NO declara STATIC_ENHANCED: solo demuestra la cadena vertical.
- proveedor arranca por stdio únicamente; sin daemon (AIS-004).

## Estado
- Commits: S0 `69f68a7`, S1 discovery `50c9238`, S1 código (este).
- Push: BLOQUEADO por hook (artifacts fuera de allowlist, sin bump).
  Publicación con el próximo release legítimo (v1.169.90).
