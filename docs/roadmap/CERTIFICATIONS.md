# Contrato de certificación SDDK — evidencia antes que etiquetas

**Vigente para nuevas declaraciones a partir del 2026-09-21.** Complementa y NO debilita [PRODUCTION-READY-GATE](../SDDK-Production-Readiness-Alignment-2026-09-14/03-PRODUCTION-READY-GATE.md) ni [A5-C](../architecture/a5/A5-C-BASE-PRODUCTION-READY-CERTIFICATION.md). La certificación histórica Base v1.169.88 conserva su alcance y excepción G11; ningún documento nuevo la re-certifica retroactivamente.

## 1. Dos ejes independientes: nivel de evidencia y perfil

| Nivel | Contrato exigible |
| --- | --- |
| `IMPLEMENTED` | SHA + diff + contrato de feature, sin atribuir aceptación. |
| `VERIFIED_LOCAL` | Tests deterministas/contractuales ejecutados en SHA exacto, sobre entorno registrado; mocks declarados. |
| `UAT_OBSERVED` | Escenario real E2E, artefacto/binario publicado o instalado, inputs y salidas capturados, negativos y failure modes; limitaciones explícitas. |
| `CERTIFIED_<PROFILE>` | Todos los gates obligatorios del perfil en PASS/N/A justificadamente permitido, public release + clean-machine UAT + recibo íntegro y revisado. |
| `EXPIRED / REVOKED / SUPERSEDED` | Nueva versión/contrato o fallo relevante obliga revalidación; nunca arrastrar PASS a otro SHA por defecto. |

Los niveles **no** sustituyen a los perfiles. La prueba determinista de un provider no certifica el binario real. Un `CLOSED` de roadmap no es un nivel de certificación.

## 2. Perfiles y gates obligatorios

| Perfil | Gates mínimos | Afirmación permitida |
| --- | --- | --- |
| `BASE` | G0–G6 + G8, más seguridad de release/secretos G11 como PASS **o excepción formal con alcance, responsable y revisit trigger**; clean machine, migración, replay y release real | Núcleo local-first sin providers opcionales. |
| `STATIC_ENHANCED` | BASE + G7 CogniCode **real**: lifecycle, negotiate, completeness, stable ref/digest, timeout, restart, degrade, Verify/Alignment | Evidencia estática observada en versión/config concreta. |
| `RUNTIME_ENHANCED` | BASE + G7 Chronos **real**: capture, resource bounds, provider lifecycle, failure/restart, golden multi-program | Evidencia runtime en límites observados. |
| `FULLY_ENHANCED` | BASE + STATIC + RUNTIME + contradicción estática/runtime, preservación de provenance, degradación independiente, AC12–AC14 UAT | Perfil conjunto, no simple suma de tests aislados. |
| `AGENTIC_API_EXPERIMENTAL` | AG0/AG1 según gate original; portabilidad limitada honestamente | API experimental, no GA. |
| `JCODE_CORE_GA` | AG0–AG2: dos procesos/productos independientes, SDK público, adapter real, session/context/events/reactive/structured/capabilities, seguridad end-to-end | GA JCode acotado a host/version/SDK comprobados. |
| `AGENTIC_API_STABLE` | JCODE_CORE_GA + AG4 con segundo host real, contratos genéricos sin tipos JCode ni quiebras forzadas | Estabilidad 1.0 solo tras portabilidad probada. |

J8 avanzado es independiente de Core GA; J7 MCP es opcional. No declarar Full mientras falte un proveedor real. Un fallo Enhanced no reabre automáticamente una certificación Base válida; un cambio compartido exige evaluar impacto y recertificar lo afectado.

## 3. Estados de gates y política fail-closed

`PASS_OBSERVED | FAIL | NOT_RUN | BLOCKED | NOT_APPLICABLE | WAIVED_WITH_EXPLICIT_RISK`. `PASS_BY_CODE_READING` no existe. `NOT_APPLICABLE` solo para gate no exigido por el perfil, con explicación; ningún gate de seguridad aplicable se omite por ausencia de entorno. Una excepción requiere ID, autoridad humana, rationale, scope, evidencia compensatoria, fecha límite y disparador de revalidación; sin aceptación explícita = no certificación. Un caso `IGNORED` es `NOT_RUN`, incluso si pasó en un release anterior.

No ocultar pruebas fallidas bajo agregados, `--skip-tests`, `|| true` o subsets declarados globales. Los resultados de `apply` no son por sí solos el perfil completo de `verify`.

## 4. Matriz de pruebas por nivel

- T0 tipos/propiedades, T1 contrato de componentes, T2 integración real storage/CLI, T3 fallos y carrera/crash, T4 host/proveedor real, T5 clean-machine/migración/release, T6 adversarial y recursos; niveles se registran por caso en [UAT-MATRIX.md](UAT-MATRIX.md).
- Cada criterio debe incluir al menos un negativo falsable (entrada malformada, provider ausente, competencia, stale, deny, crash, schema incompatible según el caso).
- Pruebas de regresión sobre **la versión instalada** del mismo SHA/tag; no aceptar solo `cargo test` frente al fuente si se certifica distribución.

## 5. Recibo obligatorio, append-only y verificable

Cada declaración nueva genera `docs/roadmap/receipts/<profile>/<tag-or-sha>/CERTIFICATION-RECEIPT.md` **después de observar los gates** (directorio inexistente hasta el primer recibo; no generar PASS de plantilla). Campos:

```yaml
schema_version: 1
profile: BASE # o perfil concreto
status: PASS_OBSERVED # o FAIL/BLOCKED; no CERTIFIED si faltan gates
repository: Rubentxu/software-development-decision-kernel
source_sha: "<40 hex>"
tag: "<public tag or null>"
binary_sha256: "<digest or null>"
bundle_manifest_sha256: "<digest or null>"
schema_version: "<storage schema>"
policy_digest: "<digest>"
command_registry_digest: "<digest>"
spec_manifest_digest: "<digest>"
environment:
  os: "<exact>"
  rust: "<exact>"
  host: "<name/version or null>"
  provider_static: "<name/version/rev/protocol or null>"
  provider_runtime: "<name/version/rev/protocol or null>"
gates:
  G0: {status: PASS_OBSERVED, evidence: "<test/run path>"}
uat:
  T01: {status: PASS_OBSERVED, command: "<exact>", evidence: "<artifact digest>"}
accepted_risks: [] # or {id, scope, approver, expires, revisit_trigger}
limitations: []
operator_approval_ref: "<recorded ref>"
verified_at_utc: "<ISO 8601>"
```

Recibo con `status: PASS_OBSERVED` solo tras pruebas reales en el **mismo** source_sha. Un historial de release anterior se cita como antecedente (`HISTORICAL`), no se copia como test actual. Firma de recibos y atestación supply-chain pueden añadirse según riesgo, sin falsear garantías criptográficas no existentes.

## 6. Promoción, caducidad y revocación

- Promoción requiere `SCOPE → tests → UAT → RECEIPT → revisión riesgos → human approval si aplica → release.sh → comprobación pública`. `release.sh` solo lo ejecuta el operador autorizado, respetando AGENTS.
- Un nuevo commit que cambie binario, schema, política, bundle o dependencias invalida las declaraciones dependientes hasta ejecutar análisis de impacto y pruebas aplicables. Un commit solo documental puede conservar recibos antiguos, pero no crea una nueva certificación.
- Incidencia de seguridad o falsación abre estado `SUSPENDED` del perfil afectado en CURRENT/diario y un ciclo de remediación; no modificar el recibo antiguo: emitir revocación/revisión enlazada.
- Cada sesión verifica que `main HEAD`, tag, binary/bundle y puntero coincidan con sus respectivos recibos; no presupone que HEAD==última release.
