---
title: "Propuestas de pensamiento lateral — Cycle supersede y recover forward"
author: deep-research-orchestrator
date: 2026-08-31
status: draft
related: research/cycle-supersede-replan/cycle-supersede-replan-research-report.md
---

# Lateral-thinking proposals — Cycle supersede y recover forward

> Complemento al informe principal. Cada propuesta lateral tiene un
> análisis coste/beneficio y verificación de compatibilidad con el estado
> publicado. NO se implementan aquí — son **ideas** que un ciclo futuro
> puede adoptar, rechazar o reformular.

---

## L1 — Supersession como defensa frente a archive gaps

### Idea

Cuando un ciclo tiene evidencia valiosa pero no llega a archivarse (por
un gate bloqueante, por alcance inválido, por external-obsolete),
**supersede** debería ser la ruta preferente: cierra el ciclo con razón,
preserva la evidencia, y enlaza al sucesor.

Archive es destructivo (escribe en vault). Supersede es aditivo (solo
ledger events + receipt).

### Por qué es lateral

La intuición estándar sería "extender archive para que sea más
permisivo". La idea lateral invierte la carga: archive sigue siendo
estricto (gate de seguridad), pero supersede abre una **puerta paralela**
que satisface el principio "recover forward para proceso".

### Coste

Bajo. Es exactamente lo que ADR-A propone; aquí se justifica el orden:
supersede antes que archive cuando hay evidencia pero no cierre limpio.

### Beneficio

- Cierra el gap "ciclo valioso + archive gate fallido = evidencia
  huérfana".
- Refuerza ADR-0047 §4 ("Los artefactos se conservan por defecto"):
  supersede es no-destructivo.

### Compatibilidad

- VAULT003 / RepairReceipt (v1.65.6) — supersede NO toca vault.
- release.sh (v1.65.0) — supersede añade un `supersede-receipt.json`
  aditivo, sin modificar los 13 pasos del pipeline.
- ADR-0022 — sin tocar.

---

## L2 — Self-recovering process gates (gates que se autoreparan)

### Idea

Hoy un gate fallido bloquea el ciclo. La idea lateral es: gates `process`
(con su recovery_action) ejecutan automáticamente la acción de recovery
**sin intervención humana**, hasta un bounded retry.

Ejemplo: `tests-pass` con `recovery_action=retry` ejecuta `cargo test
--workspace --locked` una vez, sin agente en el loop. Si pasa, el ciclo
continúa. Si falla 3 veces, escala.

### Por qué es lateral

Rompe el modelo "agente decide". Pero el "framework debe bloquear cambios
peligrosos, no bloquear el aprendizaje" lo soporta: tests-pass es un gate
de proceso, no de seguridad.

### Coste

Medio. Requiere una nueva categoría de gate ("auto-recoverable") y un
bounded-retry policy. La política debe ser conservadora (3 reintentos,
escalación automática).

### Beneficio

- Reduce el cycle-time para gates rutinarios (más velocidad).
- Elimina "wait for human" en gates deterministas.

### Compatibilidad

- ADR-B (gate classification) — implementación directa: el campo
  `recoverable=true` activa este flujo.
- ADR-G (recovery-action contract) — el contrato es exactamente esto.
- cycle-42 incidente (fabricación dm02) — el bounded retry debe ser
  **verificado por orchestrator**, no por agente.

### Anti-patrón a evitar

"Rule beating": un agente que descubre que `tests-pass` se reintenta
automáticamente puede usarlo para evadir la verificación. Mitigación: el
recovery action es siempre la **misma acción** que el agente invocaría
manualmente — no hay forma de "engañar" al gate.

---

## L3 — Goal-scoped writer (cierre directo del insight #5)

### Idea

El insight #5 dice "los agentes no deberían poder crear v2/vault/...
dentro del repo". La idea lateral va más allá: el agente **no debería
poder especificar la ruta**. El escritor toma `(goal_hash, scope,
artifact_kind)` y resuelve la ruta XDG canónica. El agente no tiene
ningún input de path.

```rust
pub trait GoalScopedWriter {
    fn write(
        &mut self,
        goal: &GoalHash,
        scope: &ScopeBinding,
        kind: ArtifactKind,
        contents: &[u8],
    ) -> Result<Receipt, XdgWriterError>;
}
```

### Por qué es lateral

La intuición estándar sería validar el path. La idea lateral **elimina el
path como input**: no se valida lo que no existe.

### Coste

Medio. Refactor del vault_cmd.rs:443 (vault export) para que tome
`--kind <artifact>` en lugar de `--output <path>`. Breaking change para
quien use `--output` directamente.

### Beneficio

- Cierra el insight #5 de raíz, no por validación.
- Compatible con el principio XDG: el writer siempre sabe adónde va.

### Compatibilidad

- ADR-D (writer XDG fail-closed) — implementación; el trait L3 es más
  fuerte (no-input).
- v1.65.6 vault commands — `--output` se mantiene para `vault export` (no
  rompe a usuarios existentes), pero el nuevo `--kind` es preferente.

---

## L4 — Supersede receipt como nodo vault (extensión natural)

### Idea

El `supersede-receipt.json` puede tener un mirror en el vault:
`vault://cycles/<project_id-cycle_id>-supersede.md` con frontmatter que
enlaza al grafo activo.

Esto reutiliza el patrón existente en
`crates/sddk-cli/src/knowledge_ingest.rs:64` (`Authority::Superseded`) y
lo aplica a ciclos.

### Por qué es lateral

Combina dos superficies (ledger JSON + vault Markdown) que hoy están
separadas. La idea lateral propone que supersede **es ambas cosas** a la
vez, una para máquinas (ledger), otra para humanos (vault).

### Coste

Bajo. El supersede-receipt.json ya existe como artefacto; crear el nodo
vault es una proyección opcional.

### Beneficio

- Discoverability: un humano buscando "qué pasó con el cycle-X" lo
  encuentra en el vault, no en JSON.
- Coherencia con Knowledge Authority::Superseded.

### Compatibilidad

- VAULT003 (v1.65.6) — el nodo vault puede ser parte de la allow-list.
- ADR-0047 §4 — añadir nodo es consistente con "los artefactos se
  conservan por defecto".

---

## L5 — Phase regression audit (detección del loop "fixes that fail")

### Idea

Hoy un ciclo puede retroceder phases (apply → tasks → apply → tasks) sin
que el framework lo note. La idea lateral: un audit periódico (un tick
del orchestrator) que verifica que ningún ciclo acumula más de N phase
regressions.

Si la regresión es estructural, el audit emite `cycle.regression.detected`
y el orchestrator puede sugerir `cycle.replan` (L6 + ADR-C).

### Por qué es lateral

Rompe la asunción "el ciclo es lineal". Pero el feedback loop "fixes that
fail" (R0 §2.3) es exactamente eso: un agente retrocede, aplica, falla,
retrocede de nuevo. El audit pone un **límite visible** al loop.

### Coste

Bajo. Es un observador que lee el ledger; no necesita escribir.

### Beneficio

- Visibilidad del loop.
- Trigger automático para replan (cuando se combina con ADR-C).

### Compatibilidad

- ADR-0068 (bounded execution) — similar pattern: detecta NoProgress,
  BudgetExceeded. Phase regression es un NoProgress de más alto nivel.

---

## L6 — Repair-receipt waiver para cycle-supersede

### Idea

Cuando un ciclo se supersede, sus RepairReceipts asociadas **no se
eliminan** (coherente con ADR-0047 §4) pero se archivan con waiver
explícito:

```yaml
waiver:
  reason: "cycle superseded"
  supersede_receipt_id: "sup-abc123"
  waiver_expiry_days: 365
```

Esto evita que un supersede "limpie" la cola de RepairReceipts sin
trazabilidad.

### Por qué es lateral

La intuición estándar sería "borrar RepairReceipts de ciclos superseded".
La idea lateral preserva la evidencia pero la marca explícitamente como
"sin efecto futuro".

### Coste

Bajo. Es un campo nuevo en RepairReceipt + una operación de waiver.

### Beneficio

- Trazabilidad: siempre se puede responder "¿por qué esta Receipt ya no
  down-clasifica VAULT003?".
- Coherencia con el principio "los artefactos se conservan por defecto".

### Compatibilidad

- VAULT003 / RepairReceipt (v1.65.6) — aditivo; el campo `waiver` es
  opcional.
- ADR-0047 §4 — refuerza la política.

---

## L7 — Complexity budget como métrica, no como regla

### Idea

El insight #7 dice "presupuestar complejidad contra valor". La idea
lateral es que el budget **se mide y se reporta** en cada ciclo, pero
**no bloquea** el ciclo.

Solo bloquea si la **tendencia** es creciente en 3 ciclos consecutivos
(un ciclo puede tener muchos gates; 3 ciclos seguidos con muchos gates
nuevos sí es señal).

### Por qué es lateral

Rompe la intuición "budget = cap". El budget como métrica es un patrón
de "trend detection", no de "absolute enforcement". Es el equivalente a
un EKG: lo importante es la tendencia, no el valor instantáneo.

### Coste

Bajo. Es reporting + trend detection; no requiere blocking logic.

### Beneficio

- Visibilidad sin fricción.
- Detecta "more gates = more safety" (system trap) antes de que se
  materialice.
- No bloquea trabajo legítimo.

### Compatibilidad

- ADR-B — el budget es un sub-campo de la gate classification.
- ADR-G — recovery-action contract ya exige "single executable action";
  el budget reporta cuántas acciones hay en total.

---

## Resumen de las 7 propuestas laterales

| ID | Idea | Anti-trap que mitiga | Coste |
|---|---|---|---|
| L1 | Supersede antes de archive cuando hay evidencia | Policy Resistance | bajo |
| L2 | Self-recovering process gates (bounded retry) | Drift to Low Performance | medio |
| L3 | Goal-scoped writer (sin path input) | Policy Resistance (insight #5) | medio |
| L4 | Supersede receipt como nodo vault | Discoverability gap | bajo |
| L5 | Phase regression audit | Fixes that Fail | bajo |
| L6 | Repair-receipt waiver | Eroding Goals | bajo |
| L7 | Complexity budget como trend metric | Seeking the Wrong Goal | bajo |

---

## Compatibilidad agregada con v1.65.7

Las 7 ideas son **aditivas**: ninguna modifica código publicado, ninguna
revierte decisiones, ninguna propone breaking changes para usuarios
externos. Cada idea lateral puede adoptarse independientemente (ciclo
separado).

---

## Riesgos agregados de las propuestas laterales

| Riesgo | Likelihood | Impact | Mitigación |
|---|---|---|---|
| L2 self-recovery se use para evadir verificación | medium | medium | Bounded retry + escalación; el orchestrator verifica |
| L3 goal-scoped writer rompa `vault export --output` para humanos | low | low | `--output` se mantiene como escape hatch explícito |
| L4 supersede-nodo-vault prolifere nodos | low | medium | Periodic prune (futuro ADR, no aquí) |
| L5 phase regression audit emita demasiados eventos | low | low | Audit-rate-limit (1× por ciclo) |
| L6 waiver no respete el 90-day window de RepairReceipt | low | medium | El waiver es independiente del window; documentado |
| L7 trend detection tenga falsos positivos | medium | low | Threshold conservador (3 ciclos consecutivos); revisable |

---

## Orden de adopción sugerido (si se adoptan todas)

1. **L7** (cycle-50): trend metric no rompe nada.
2. **L4** (cycle-50/51): vault node como mirror, aditivo.
3. **L5** (cycle-51): audit observacional, sin escritura.
4. **L1** (cycle-51/52): depende de ADR-A.
5. **L6** (cycle-52): depende de ADR-A.
6. **L3** (cycle-53): refactor de vault_cmd.rs:443.
7. **L2** (cycle-54+): self-recovery, mayor cambio.

---

## Veredicto

Las 7 propuestas son **complementarias** a los 5 ADRs principales. Si se
adoptan todas, el ciclo total estimado es 4–5 ciclos (A-min), repartidos
entre cycle-50 y cycle-54+. Esto es coherente con el Wave plan (cycle-49
es Wave 4 facade; cycle-50+ abre el ciclo de recover-forward).

**Recomendación**: priorizar L7 + L4 + L1 (las de menor coste y mayor
beneficio inmediato). Dejar L2 + L3 para ciclo posterior, cuando el
framework haya validado la mecánica de supersede en producción.