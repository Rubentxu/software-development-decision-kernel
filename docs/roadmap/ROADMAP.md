# Roadmap único de continuación — SDDK production ready

**Edición:** 2026-09-21 · **Baseline inspeccionado:** `main@2ffff3127e7179b5f3c3104c471c8ad2c7d920ff`, workspace `1.169.127`; última release pública comprobada durante la auditoría: `v1.169.122`. **Estos valores son una fotografía, no un estado en tiempo real.** Consultar [CURRENT.md](CURRENT.md) y Git antes de actuar. Este plan NO declara que esa release sea actual ni que los gates estén verdes hoy.

## 0. Intención, autoridad y continuidad temporal

**Objetivo:** conservar el núcleo semántico ya entregado, cerrar desviaciones verificadas, certificar perfiles operativos reales y solo entonces abrir ampliaciones host/proveedor/arquitectura con valor demostrado. Mantener Knowledge (qué sabemos) → Alignment (interpretación/advisory) → Verification (qué se comprobó) → Governance/Authority (qué está permitido) como fronteras con una sola autoridad por concepto. Local-first, sin segunda fact log, segundo grafo canónico, segundo scheduler ni doble aprobación.

**No partir de cero.** Baseline certificado: M0–M9/C0–C7 históricos; A0–A5 Base certificado en `v1.169.88` **condicionado** a aceptación de G11 no verificado en su momento; revisión R14 posterior a nivel de código, no auditoría adversarial. Hitos AC0–AC14, J0–J6 y AIW-S0–S8 tienen cierres **de alcance concreto** en los recibos de 19–21/09. Las verticales reales CogniCode/Chronos NO equivalen a perfil enhanced completo; JCode semántico NO equivale a adapter productivo + AG2. Ver [certificación Base](../architecture/a5/A5-C-BASE-PRODUCTION-READY-CERTIFICATION.md), [estado AIW](../proposals/2026-09-19-adaptive-inputs-workflows/STATE-OF-AIW.md) y [último estudio de gaps](../research/2026-09-21-roadmap-gaps-deep-research.md).

**Estados admisibles de un WorkItem:** `PROPOSED | READY | IN_PROGRESS | BLOCKED | IMPLEMENTED | VERIFIED | CERTIFIED | DEFERRED | SUPERSEDED`. `IMPLEMENTED` exige commit y scope; `VERIFIED` requiere prueba OBSERVED vinculada a ese SHA; `CERTIFIED` requiere todos los gates del perfil en [CERTIFICATIONS.md](CERTIFICATIONS.md). Solo mover a `DEFERRED` con razón, dependencia y disparador; no contarlo como PASS. No porcentajes inventados: avance = requisitos verificados / requisitos aplicables congelados por hito; `NOT_RUN`, `BLOCKED` y `N/A` se contabilizan por separado.

## 1. Dependency DAG: continuidad sin abrir trabajo antes de tiempo

```text
C0 Reconciliar HEAD/release y congelar baseline + contratos
  └── C1 Endurecer contratos/identidades/boundaries (H01,H02,H05,H06)
        ├── C2 Certificar integración real: CogniCode, Chronos, JCode
        └── C3 Seguridad, Authority y Storage adversarial/recovery
               └── C4 Certificación del perfil de producto y publicación
                     └── C5 Opcionales evidencia-driven (X08,J7,J8,J9,R11)
```

C2 y C3 pueden realizarse en paralelo **solo después** de C1 y con el mismo contrato de seguridad; C4 exige el conjunto de gates aplicables de ambos. Se puede publicar Base sin exigir enhanced/Agentic; no etiquetar Full/GA por coexistencia de módulos o cuentas de tests.

## 2. Hitos, entregables y salida falsable

### C0 — Reconciliación y baseline reproducible (P0, primero)

**Entradas:** SHA, Cargo.toml, tag/release remota, A5-C, mini-roadmap 14/09, recibos A6/A7/A8/J/AIW, AGENTS, incidencias abiertas. Confirmar si existe una release posterior antes de copiar la fotografía de 21/09.

**Acciones:** inventariar `IMPLEMENTED/VERIFIED/CERTIFIED/DEFERRED` por feature; enlazar commit, test, evidencia real/fake, aceptación y límites; detectar drifts; identificar versión real del binario/bundle/config/schema; fijar los contratos de política/capacidades relevantes; preparar o reconciliar mecanismo de release `chore(release): bump version` sin publicar de manera automática.

**Salida:** `CURRENT.md` / `STATE.yaml` consistentes, matriz [UAT](UAT-MATRIX.md) congelada y recibo de baseline con referencias y riesgos aceptados. **No** pasar a C1 con repo dirty o SHA ambiguo. No modificar el histórico para forzar coherencia.

### C1 — Falsación de contratos y hardening acotado (P1)

**H01:** en `structured_work::shape_matches`, descriptor no soportado no puede dar `true`; versión/esquema explícito, campos requeridos y adicionales según contrato. **H02:** `request_id` duplicado no sustituye silenciosamente una petición no equivalente; errores de schema nunca interpolan valores sensibles. **H05:** test-seam `set_process_service_for_tests` con semántica real y aislamiento de producción. **H06:** defensa gateway de argumentos/bytes, límites agregados y redacción completa de las salidas.

**Método:** caracterización RED→GREEN por cambio mínimo, UAT T01–T07, revisión de compatibilidad/migración, verificar regresión acotada en `apply` y global en `verify`. No convertir el validador en nuevo motor de schemas sin justificación ni duplicar autorización en host.

**Salida:** código + tests negativos + recibo por slice con escenarios reales/fake distinguidos; resultados válidos/inválidos estables; cero exposición de canarios en corpus definido. Riesgos de cambios breaking documentados.

### C2 — Integraciones reales y declaraciones por perfil (P1, tras C1)

**C2a CogniCode:** preservar S1 real observado (v0.97.1) como evidencia histórica, verificar versión actual, ciclo de vida/capability negotiation, completitud/frescura del grafo, falsos negativos, contradicciones, restart y degradación Base sin proveedor. No elevar `find_usages` sobre grafo lightweight a evidencia de cobertura global. Ejecutar EXT con binario real y documentar limitaciones.

**C2b Chronos:** preservar S5 real histórico (v0.1.0); revisar adaptador, escenarios multi-programa/probes, timeout/cancelación, capture vacía, restart, recursos, compatibilidad, ausencia de exit-status inferido y no bypass de Authority. Ejecutar EXT real.

**C2c JCode:** distinguir J2–J6 semánticos de adaptación real. Verificar si el adapter público vive en otro repo antes de implementar uno duplicado; demostrar boundary `jcode-sdk` / SDDK SDK (o ADR de equivalencia), session≠run, transcript host-owned, event coalescing, context delta, structured return, redacción en gateway, negociación de capacidades y ejecución companion + orchestrated. No declarar JCODE_CORE_GA sin AG0–AG2 y recibo específico.

**Salida:** recibos verificables e independientes por proveedor/host, entorno, hashes, versiones, casos UAT T08–T18; `NOT_EVALUATED` si falta binario o consumidor real; ausencia de proveedor nunca es PASS.

### C3 — Resiliencia, seguridad y fiabilidad (P1, paralelo a C2 tras C1)

**Authority:** delimitar seq de proceso vs event log, digest/política y fencing; test de interleavings, políticas concurrentes, reinicios y no side effects al denegar. Ninguna ampliación multi-proceso antes de probar su contrato. **Storage:** inventariar 8 sitios IMMEDIATE diferidos, reproducir contención y priorizar únicamente rutas activas; idempotencia, CAS, fallo parcial y crash/reopen. **Seguridad:** pruebas adversariales con canarios en args/env/stdout/stderr/error/receipt/CAS, trust-boundary para host y provider, secret-screen y dependencias/supply chain. **Rendimiento:** presupuesto medible de p95 y recursos en tres escenarios fijados (Base, static, runtime); baseline antes de optimizar.

**Salida:** UAT T19–T27 y recibos con incidentes restantes clasificados; pruebas de contención/recuperación observadas, sin promesas de seguridad universal.

### C4 — Release y certificación de producto (P0 para cada declaración)

Evaluar [CERTIFICATIONS.md](CERTIFICATIONS.md), ejecutar perfil completo local sin `--skip-tests`, construir y verificar binario/bundle/manifest/SBOM/hashes, clean-machine UAT, migración/replay, publicar mediante `bash scripts/release.sh` **solo con autorización del operador**, verificar assets públicos y registrar SHA/tag/env/resultado. Un fallo de gate bloquea **esa certificación**; Base puede permanecer certificada aunque Enhanced no lo esté si no se ha roto Base. Certificación historical != current HEAD.

**Salida:** `CERTIFICATION-RECEIPT` reproducible para cada perfil válido, cero bloqueadores sin disposición, riesgos aceptados explícitos con caducidad/revisit trigger y versión pública exactamente correlacionada con recibos. UAT T28–T33.

### C5 — Evolución condicionada a pruebas de valor (P2/P3, no bloquea C4-Base)

**X08:** Jev benchmark solo con corpus etiquetado, baseline y métrica definidos. **J7:** MCP pull solo si un consumidor demuestra necesidad que push/SDK no resuelve. **J8:** operaciones host avanzadas tras negociación de capabilities y UAT propios. **J9:** segundo host real antes de declarar `AGENTIC_API_STABLE/1.0`. **R11:** split de crates solo con métricas sostenidas de dependencia, frecuencia de cambios y compilación; conservar puertos/ownership, no hacer un split cosmético. Future async Parallel es **nueva feature**, no reapertura de R12 ya cerrado.

**Salida:** cada idea es `DEFERRED` hasta que el disparador y el SCOPE existan; no convertirlas en backlog ejecutable por inercia.

## 3. Contrato de cambio para cualquier slice

1. `SCOPE-CONTRACT`: baseline SHA, objetivo falsable, invariantes y no-objetivos, riesgo, superficie de código, tests, dependencias, stop conditions y presupuesto.
2. Test de caracterización pre-fix cuando proceda; `apply` ejecuta solo test scope derivado del SUT; en `verify` ejecutar perfil completo.
3. `UAT-EVIDENCE`: entrada real/fake, comando, SHA, entorno, resultados, prueba negativa, output digests y limitaciones. `NOT_RUN` nunca se convierte en PASS por lectura de código.
4. `RECEIPT`: commit, pruebas, decisiones, riesgos, semántica de compatibilidad y release/tag cuando exista; subir documentación en el mismo cambio de concernencia.
5. Actualizar CURRENT + STATE + SESSION-JOURNAL **después** de comprobar el resultado. No alterar en silencio la baseline para acomodar resultados.

## 4. Indicadores operativos

Registrar, sin universal score: requisitos aplicables vs verificados por perfil, pruebas flakey/ignored, tiempo de ciclo verify, bloqueos por proveedor, defects escaped, duration p50/p95, tiempos de recuperación, cambios cruzados de crates y número de riesgos aceptados pendientes. Cualquier porcentaje informa denominador, SHA, suite y alcance. La ausencia de medida se presenta como `NOT_MEASURED`.

## 5. Fuentes históricas conservadas

[Arquitectura actual](../architecture/README.md) · [14/09 mini-roadmap histórico](../SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md) · [A5 certificación](../architecture/a5/A5-C-BASE-PRODUCTION-READY-CERTIFICATION.md) · [AIW state 21/09](../proposals/2026-09-19-adaptive-inputs-workflows/STATE-OF-AIW.md) · [Roadmap gaps 21/09](../research/2026-09-21-roadmap-gaps-deep-research.md) · [Archivo](../history/README.md).
