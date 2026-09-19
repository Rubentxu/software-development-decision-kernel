# Disposición de documentos y propuestas anteriores

**Regla:** reemplazar su **función como guía prospectiva** tras aceptación explícita, NO borrar hallazgos históricos, ADRs aceptadas, tests ni receipts. Son seis documentos de 19/09; el de auditoría contiene revisión 18:43 y extensión 18:55.

| Documento origen | Conservar como evidencia histórica | Nueva fuente prospectiva / cambio |
|---|---|---|
| `2026-09-19-jcode-anti-corruption-adapter-PROPOSAL.md` (b442635) | Riesgo de raw stdout/stderr y args denegados, pruebas SEC-1, propuesta J2 sin abrir | [ADR-05](../adrs/AIW-ADR-05-EXTERNAL-TOOLS-AND-CLI.md) y [SPEC-04](../specs/AIW-SPEC-04-PROVIDERS-AND-HOSTS.md): contrastar primero `arch-spec-030`; no copiar su trait por anticipado. |
| `2026-09-19-a6-cognicode-cc-s0-protocol-spike-PROPOSAL.md` (aa6855e) | Línea temporal/spike y batería T1..T6; ADR-0137 aceptada subsiste | [Estado](../01-CURRENT-STATE.md), [S1](../roadmap/MILESTONES.md): CC-S0 está en código; no reabrirlo; A6 exige proveedor real y AC10. |
| `2026-09-19-kernel-adaptativo-productores-encaje.md` (df60a8a) | Productor real vs stub, `ObservationSet::new()`, operators incompletos, FNV provisional | [Arquitectura](../architecture/01-OWNERSHIP-AND-FLOWS.md), [Specs](../specs/README.md), [UAT](../uat/UAT-MATRIX.md). No duplicar roadmap E0–E4. |
| `2026-09-19-input-persistencia-proyeccion-encaje.md` (6251af7) | Dos puentes, writer universal, CAS+checkpoints, relación contradicts bloqueada | [ADR-03](../adrs/AIW-ADR-03-DURABLE-EVIDENCE.md): sucesor **condicional**, normalizador concreto primero, no migración automática. |
| `2026-09-19-inputs-origen-y-adopcion-auditoria.md` (c717aab) | Inventario y conteo del bundle instalado auditado, 12 top-level sin menciones | [Estado](../01-CURRENT-STATE.md), [Specs](../specs/README.md): zero prompt mention ≠ zero uso externo; adopción exige consumidor end-to-end. |
| `2026-09-19-inputs-auto-vs-delegado.md` (da37510) | AIS-004, captura gobernada, CLI especializada seleccionada por LLM, scan/hook/checkpoint | [Inputs](../architecture/02-INPUTS-AND-EXECUTION.md), [ADR-01](../adrs/AIW-ADR-01-INPUT-OWNERSHIP.md): ejecución selectiva, mismo Gateway, seguridad, attempt-id correcto. |

## Ideas del diálogo anterior expresamente reformuladas

- «Agenda Runtime», «WorkHandoff», «Uncertainty Ledger», «Decision Graph DB» y «BT engine» pasan a ser **vistas, operaciones o spikes**, no modelos persistentes.
- `Decision CI` queda como experimento de calidad condicionado a corpus de casos reales, no prerequisito A6.
- Entropía matemática sobre salidas de Jev **no equivale** a exactitud de evidencia, y EIG requiere modelo de observación medido; no se crea Bayes posterior global.
- La regla «siempre que el mismo estado, mismo receipt id» se sustituye por distinguir operación idempotente, attempt físico y evidencia reproducible.
- `knowledge scan` puede activarse selectivamente; `knowledge import` NO se convierte automáticamente en autoridad.
- «Dividir SDDK en varios CLI» solo se evalúa en R11; un binario nuevo no es beneficio por sí mismo.
