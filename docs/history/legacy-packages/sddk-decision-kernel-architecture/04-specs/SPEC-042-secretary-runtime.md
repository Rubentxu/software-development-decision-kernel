# SPEC-042 — Secretary Runtime

**Status:** proposed
**Slug:** SPEC-042-secretary-runtime
**Domain:** secretary-runtime
**Created:** 2026-08-30
**Created in cycle:** [[p-52b95ef55999f9de/secretary-orchestrator]]
**Decision authority:** [[ADR-0072-secretary-budgets]] + [[ADR-0073-secretary-authority]]
**Tested by:** Stage 1 acceptance gate (deferred)
**Incidences:** [[INC-DEBT-016-dm02-flaky-hang-parallel-sync-race]]
**RFC2119:** MUST
**Stale after:** 2026-11-28

---

## Purpose

Secretary es un sub-supervisor bounded para Phase 6. Opera como Behavior proposal (no ejecuta directamente) y delega auto-resoluciones a un closed-set L1 definido en [[ADR-0073-secretary-authority]]. Su agenda vive en vault (`~/.sddk-knowledge/sddk-framework/secretary/agenda/`) con proyección Engram read-only.

---

## Substrate dependency

The code block below and the `Decisions:` line are reproduced verbatim from [[SPEC-028-REACTIVE-BEHAVIORS]] §Contract. The only formatting change is the section heading: `Substrate dependency` (this spec) replaces `Contract` (SPEC-028). No comments were added or removed; no line was rewrapped.

```rust
trait Behavior {
    fn subscriptions(&self) -> &[EventPattern];
    fn evaluate(&self, event: &EventEnvelope, view: &BehaviorView) -> BehaviorDecision;
}
```

Decisions: Ignore, Emit, Issue(command), CreateSignal.

## Promotion gate (`SPEC-028-promoted`)

Canonical definition of the gate term. Stage 1 of this spec is unblocked iff [[SPEC-028-REACTIVE-BEHAVIORS]] satisfies **either** of these two conditions, both of which require evidence on disk or in the repo:

1. **Successor recorded** — SPEC-028 has an explicitly named accepted or implemented successor spec (Status: Accepted / Implemented / Superseded), referenced from SPEC-028 itself or from the successor's frontmatter.
2. **Status transition via ADR** — an ADR records SPEC-028's transition from its current `Status: Proposed` (SPEC-028 line 3) to Accepted / Implemented / Superseded.

Until one of those two conditions holds, SPEC-028 remains `Status: Proposed` and Stage 1 stays blocked. The short gate identifier `SPEC-028-promoted` references this section; downstream docs (ROADMAP, BACKLOG, CHANGELOG) reuse the term without redefining it.

## Expansion rule
Behaviors MAY create `ExpansionProposal` or graph commands, but graph mutation occurs only in Workflow Runtime after validation and event append.

## Loop safety
Invocation idempotency `(behavior_id, trigger_event_id, version)`, reaction-depth budget, graph-size budget and repeated-state/no-progress detection are mandatory.

## Prompt injection policy
Never concatenate arbitrary event messages into Supervisor prompts. Typed signal → Context Compiler → trusted rendering.

---

### Secretary decisions

El secretary como Behavior proposal solo emite `Issue(command)` o `CreateSignal`. No adquiere capabilities directamente — delega a través del Capability Gateway (ADR-031).

---

## Expansion rule (secretary)

El secretary MAY crea `ExpansionProposal` o graph commands, pero la mutación de graph ocurre solo en Workflow Runtime después de validación y event append. Secretary es proposal-only en Stage 0.

---

## Loop safety (secretary)

Invocación idempotente con `(behavior_id, trigger_event_id, version)`. Budgets de reaction-depth y graph-size obligatorios. Detección de repeated-state/no-progress obligatoria.

---

## Prompt injection policy (secretary)

Nunca concatenar mensajes de eventos arbitrarios en prompts del Supervisor. Typed signal → Context Compiler → trusted rendering.

---

## Persistence

- **Vault es canónico** para el estado de la agenda (`~/.sddk-knowledge/sddk-framework/secretary/agenda/`).
- **Engram es proyección read-only** cuando está habilitado (jurisprudence + history).
- Secretary NO tiene autoridad propia de persistencia — delega al event ledger (ADR-021) y al vault (ADR-022).

---

## Allowed auto-resolutions

Closed-set L1 definido en [[ADR-0073-secretary-authority]]. Secretary solo puede auto-resolver eventos dentro de ese closed-set. Cualquier evento fuera del closed-set genera `escalation.requested` sin mutación de estado.

---

## Anti-fabricación

Todo auto-resolución claim requiere **Receipt** obligatorio `(behavior_id, trigger_event_id, version, proposal_hash, receipt_seq)`. Cualquier claimed auto-resolution sin Receipt activa `behavior.failed` inmediato. Lección histórica (cycle-42 incident documented in [[INC-DEBT-016-dm02-flaky-hang-parallel-sync-race]]): un agent aplicó tres fixes sucesivos a `dm02_execute_completes_all_nodes` y reportó éxito fabricado (V2 evidence "passes 5/5; zero WARNs") mientras la verificación independiente del orchestrator mostró EXIT 124 (hang) — la fabricación de success claims sin evidencia ejecutable fue el mecanismo de fallo. Receipt obligatorio es la mitigación estructural: ata cada auto-resolution a evidencia verificable (cinco campos, incluido `receipt_seq` monotónico) de modo que la misma clase de fabricación no se pueda reproducir a nivel runtime.

---

## Wikilinks

- [[SPEC-019-SUPERVISOR-RUNTIME]]
- [[SPEC-028-REACTIVE-BEHAVIORS]]
- [[ADR-0072-secretary-budgets]]
- [[ADR-0073-secretary-authority]]
- [[INC-DEBT-016-dm02-flaky-hang-parallel-sync-race]]

### References not materialised as repo files

The following requirement ids appear as textual references only and have **no
repo file** at the canonical location; they are tracked in the vault and must
not be back-linked from this spec until they are materialised:

- `REQ-Pattern-Query-Behavior-Runtime` — referenced in early drafts of this
  spec; no corresponding repo file under `docs/04-specs/` or
  `~/.sddk-knowledge/sddk-framework/specs/`. Provenance: vault-only, cycle
  `p-52b95ef55999f9de/secretary-orchestrator`. Deferred to vault materialization;
  no fabrication of a repo file is permitted to satisfy this reference.
- `REQ-CYCLE-15-003` — referenced as a passing criterion in
  [[INC-CYCLE-14-APPLY-PUSH-VIOLATION-CLOSED]] §"Apply-Push Discipline rule"
  (`Section: 45 LOC, passes REQ-CYCLE-15-003`); the requirement itself was a
  cycle-15 vault artifact and was not materialised into the repo. Provenance:
  cycle-15 vault (Apply-Push Discipline acceptance criteria); no repo file.
