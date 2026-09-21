# AIW-SPEC-01 — Input → evidencia → consumidor

**Estado: Proposed.** Los códigos siguientes tienen mapeo en `uat/UAT-MATRIX.md`.

## Contrato por productor

- **AIW-REQ-I01 MUST** identificar consumidor/claim/WorkItem/operación antes de persistir una nueva familia de input. Si no existe consumidor y cambio de comportamiento observable, limitar a artefacto explícitamente solicitado.
- **I02 MUST** conservar source/workspace/revision+dirty, scope, config, versión de productor, env relevante y completitud; cuando una dimensión no aplique, tipar razón. Un resultado sin base suficiente MAY guardarse como artefacto diagnóstico, MUST NOT convertirse en evidencia verificadora.
- **I03 MUST** separar observación, declaración, inferencia, contradicción, resultado operativo y decisión. Ausencia de findings en scope incompleto MUST NOT implicar negación ni PASS.
- **I04 MUST** aplicar Authority antes de operaciones observacionales con permisos/efectos, y al comprometer efecto; producer/projection MUST NOT conceder permisos.
- **I05 MUST** usar writer canónico, CAS/ref/relation y mecanismos de concurrency/idempotencia existentes. Sin SQL ad hoc ni proyecciones escritas por el productor.
- **I06 MUST** usar digest real especificado; `DigestSha256` FNV del spike MUST NOT actuar como SHA-256 ni identidad durable. Misma input basis+producer+config con resultado reproducible SHOULD permitir igualdad de digest canónico, distinta basis MUST invalidar reutilización cuando material.

## Herramientas delegadas

- **I07 MUST** registrar status/exit, timeout/cancel, attempt identity, output completeness y referencias desde runner/gateway para resultados necesarios de gate/claim. Narración LLM MUST NOT sustituir el resultado.
- **I08 MUST** redactor antes de exponer al host, logs, errores o almacenar material sensible; el plan de captura MUST limitar tamaño/retención/acceso. Proceso `exit=0` MUST NOT ser claim PASS sin contrato/scope/normalización pertinente.
- **I09 SHOULD** aprovechar `gateway::test_runner::dispatch` y runner en una familia ejecutable real, con normalizador acotado solo si se necesita dato por testcase. Un runner que solo da `RunOutcome` **no** cuenta como parser de reportes JUnit.
- **I10 MUST** distinguir attempt físico de petición idempotente; reintentos con ejecución real no se colapsan silenciosamente a un único receipt.

## A6 y persistencia observacional

- **I11 MUST** consumir desde proveedor REAL una relación negociada, mapear resultado al `observation::SoftwareObservation` SDDK-owned, y alimentar la ruta `VerifyKernel` que actualmente construye `ObservationSet::new()`. El `ObservationSet` del spike es homónimo y textual: NO usarlo como canónico sin normalización.
- **I12 MUST** mantener Null/timeout/partial/incompatible como ausencia o falta explícita de evidencia en enhanced; Base UAT sin proveedor MUST seguir verde. No simular salida de CogniCode mediante fake para AC10.
- **I13 CONDITIONAL MUST** ante un UAT demostrado de contradicción durable que no cabe en writer actual, aprobar ADR sucesora antes de migración. En caso contrario NO crear tabla nueva. Relación fail-closed, base y ObservationId sobreviven reboot.

## Proyecciones

- **I14 MUST** producir JSON/texto de un mismo resultado semántico y refs visibles. Read-model rebuildable en la persistencia de checkpoints existente solo si se justifica. `as-of` del consumidor debe corresponder a una base compatible; stale no se ofrece como fresh.
- **I15 SHOULD** ejecutar scans/diffs/proyecciones solo por consumidor/trigger/presupuesto; **MUST NOT** depender de daemon o Git hook para corrección. Hook ausente dispara fallback on-demand en ruta sensible.
