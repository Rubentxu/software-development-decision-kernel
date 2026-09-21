# AIW-ADR-01 — El resultado verificable pertenece al productor, no al relato del agente

**Estado: Proposed · Ámbito:** gateway, input normalization, Verification, Knowledge, agentes. **Relacionado:** SEC-1, IPB-001..012, ADR-0100, ADR-0137, arch-spec-042/043.

## Contexto

Los agentes ejecutan herramientas cuyos resultados suelen acabar como texto de fase; `runner::RunOutcome` y receipts existen, pero no se ha demostrado captura normalizada end-to-end para Verification. Un LLM puede confundir error de infraestructura, salida truncada y verificación superada. Persistir logs íntegros indiscriminadamente aumenta riesgo de secretos y coste.

## Decisión propuesta

1. Operaciones con resultado necesario para gate, claim o aceptación pasan por gateway/runner existente o puerto de proveedor, con Authority previa y registro de resultado **desde código**. La CLI del agente es entrada, no autoridad.
2. Fijar antes del comando el consumidor, base, scope, env y formato esperado. Normalizar **solo familias concretas** tras capturar status, timeout, completeness y artefacto. `stdout`/`stderr` breves pueden ser diagnósticos, nunca prueba universal de PASS.
3. Evidencia persiste únicamente vía writer canónico con CAS/ref/relation, checks de redacción e identidad completa. Resultado bruto sin normalizador puede ser artefacto no interpretado; `incomplete` no se degrada a `no findings`.
4. Attempt físico, operation-idempotency y resultado-content-digest no son la misma identidad. Un comando repetido produce una nueva observación de ejecución salvo contrato explícito de reutilización segura.
5. Preferir CLI/puertos directos para hechos; LLM decide scope/experimento o produce interpretación como Contribution con refs, separada del resultado factual.

## Alternativas rechazadas

- Parser de Markdown narrado por agente como fuente canónica: no permite probar exit code/timeout/alcance.
- Nuevo «execution recorder» con su propio subprocess, secrets store y ledger: rompe SRP y seguridad; adaptar runner/gateway.
- Registrar cada comando trivial y stdout completo: ruido, secretos, latencia y retención.

## Consecuencias y falsificación

Puede requerir un comando CLI nuevo si el actual no ofrece ruta gobernada; se justifica por segundo consumidor host/CLI. UAT: fallo exit!=0 descrito como «green» por agente sigue FAIL/UNKNOWN; timeout, cancelación y salida truncada distintos; secreto canary no figura en logs/error/CAS/host view; segundo intento conserva identidad distinta, misma input basis cuando proceda. Ver [uat/UAT-MATRIX.md](../uat/UAT-MATRIX.md).
