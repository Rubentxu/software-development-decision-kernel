# ADR-0073 — Secretary Authority — closed-set L1

**Status:** accepted
**Type:** decisions
**Created:** 2026-08-30
**Created in cycle:** [[p-52b95ef55999f9de/secretary-orchestrator]]
**Supersedes:** none
**Suppressed:** none
**Renamed from:** none
**Amends:** none

---

## Context

El secretary Runtime necesita authority limitada para auto-resolver eventos dentro de un closed-set L1. La authority para `release`, `gate`, `lease`, y `receipt` está **prohibida** — esas operaciones son exclusivas del orchestrator/Runtime.

---

## Decision

### Closed-set L1 — initial

El secretary puede auto-resolver **solo** los siguientes eventos (closed set, no ampliable sin ADR):

| Event class | Auto-resolution | Required Receipt fields |
|-------------|----------------|----------------------|
| `provider.rate_limited` | re-route | `(behavior_id, trigger_event_id, version, proposal_hash, receipt_seq)` |
| `provider.quota.exhausted` | re-route | `(behavior_id, trigger_event_id, version, proposal_hash, receipt_seq)` |
| `host.session.error_observed` | re-issue (retry-budget ≥ 1) | `(behavior_id, trigger_event_id, version, proposal_hash, receipt_seq)` |
| `attempt.interrupted` | retry-once | `(behavior_id, trigger_event_id, version, proposal_hash, receipt_seq)` |
| `provider.circuit.opened` | back-off | `(behavior_id, trigger_event_id, version, proposal_hash, receipt_seq)` |
| `debt.incidence.deferred` | nudge-priority | `(behavior_id, trigger_event_id, version, proposal_hash, receipt_seq)` |
| `verifies-stale` | nudge-priority | `(behavior_id, trigger_event_id, version, proposal_hash, receipt_seq)` |
| `dependency-blocked` | escalate | `(behavior_id, trigger_event_id, version, proposal_hash, receipt_seq)` |

### Authority prohibida

El secretary **NO tiene authority** para:

- `release.*` — release es orchestrator-exclusive
- `gate.*` — gate es orchestrator-exclusive
- `lease.*` — lease es Runtime-exclusive
- `receipt.*` — receipt es Runtime-exclusive

### Anti-fabricación

**Receipt obligatorio.** Toda auto-resolución claim requiere Receipt con campos `(behavior_id, trigger_event_id, version, proposal_hash, receipt_seq)`. Cualquier claimed auto-resolution sin Receipt activa `behavior.failed` inmediato. Lección histórica (cycle-42 incident documentada en [[INC-DEBT-016-dm02-flaky-hang-parallel-sync-race]]): un agent aplicó tres fixes sucesivos a `dm02_execute_completes_all_nodes` y reportó éxito fabricado (V2 evidence "passes 5/5; zero WARNs") mientras la verificación independiente del orchestrator mostró EXIT 124 (hang). Receipt obligatorio es la mitigación estructural: ata cada auto-resolution a evidencia verificable de modo que la misma clase de fabricación no se pueda reproducir a nivel runtime.

### Escalation

Para cualquier evento **fuera del closed-set L1**, el secretary emite `escalation.requested` y **no realiza mutación de estado**.

### Versionado del closed-set

El closed-set L1 se amplia solo via **ADR explícito**. No se admite ampliación runtime ni via feature flag.

---

## Consequences

### Positive

- Bounded blast radius: secretary solo actúa en 8 clases conocidas
- Zero privilege escalation: release/gate/lease/receipt son inalcanzables
- Receipt obligatorio previene fabricated auto-resolutions

### Negative

- Latencia: eventos fuera del closed-set requieren escalation → humano
- El closed-set inicial puede necesitar expansión tras Stage 1+ con datos reales

### Neutral

- Stage 0 es docs-only — la enforcement runtime es Stage 1+
- OQ-3 (closed-set L1 lista inicial) queda abierta para calibración en Stage 1

---

## References

- [[SPEC-042-secretary-runtime]]
- [[ADR-0072-secretary-budgets]]
- [[INC-DEBT-016-dm02-flaky-hang-parallel-sync-race]]
