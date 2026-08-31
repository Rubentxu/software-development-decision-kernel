# SDDK Decision Model v2

Single source of truth for SDDK routing decisions.

## Priority Order

The orchestrator runs these gates in order; each can short-circuit.

| # | Gate | Action if missing/blocked |
|---|------|---------------------------|
| 1 | Workspace + execution mode resolved | Ask or assume `auto` |
| 2 | Init context loaded from project XDG data / current cycle | Run `sddk-init` first |
| 3 | Triage: classify context quality C0-C3 | Drives path selection (see below) |
| 4 | Jurisprudence lookup (`mem_search` goal_pattern) | If hit → bias path toward prior successful pattern |
| 5 | Path selection (B-direct / A-min / A-lite / A-full) | Drives phase sequence |
| 6 | Lens selection (only if path ≥ A-min) | Use `lens-registry.md` |
| 7 | Lateral thinking config (F3 always on; F1/F4 opt-in) | Default: F3 only |
| 8 | Launch plan produced and validated | Required before each delegation |
| 9 | Pre-flight gates (artifact exists + approved + schema valid) | Block if any fail |
| 10 | Delivery gates (testing capability, review budget) | advisory — block only on safety-brake classes (see `cli-usage-contract.md#matrix.safety-brake`) |

## Context Quality

| Level | Signal | Recommended effort |
|-------|--------|--------------------|
| C0 | Vague request, no paths, no current behavior, no constraints | `deepen` (one blocking question) |
| C1 | Intent clear, but affected areas, invariants, ownership, or risks missing | `deepen` + selected lenses |
| C2 | Problem, state, areas, constraints, and risks clear | `verify` only (reuses context) |
| C3 | Exploration, specs, ADRs, tests, paths, and invariants explicit | `skip` (lightweight validation) |

## Path Selection (drives phase sequence + coherence gates)

```
B-direct  if: (C3 + jurisprudence_hit) OR user says "just do it" / "fix it"
A-min     if: C2 + scope simple (single apply phase, no architectural fork)
A-lite    if: C1 (default for bounded work)
A-full    if: C0 OR architectural change OR new domain
```

| Path | Phase sequence | Coherence gates | Closing HTML | Tag | Managed closure |
|------|----------------|-----------------|-------------|-----|-----------------|
| B-direct | skill → execute → light verify → release → archive | 0 | archive | patch | vault |
| A-min | spec → tasks → apply → verify → debt-verify → release → archive | 0 (unless spec complex) | archive | yes | vault |
| A-lite | propose → spec → tasks → apply → verify → debt-verify → release → archive | 1 (apply→verify) | archive | yes | vault |
| A-full | explore → propose → spec\|\|design → tasks → apply → verify → debt-verify → release → archive | 4 (propose→spec, spec+design→tasks, apply→verify, debt→release) | archive | yes | vault |

**Managed closure column**: `vault` means the path supports `DeliveryKind = ManagedClosureDelivery` via the `archive.vault.complete` route (ADR-0075). When a cycle declares `ManagedClosureDelivery`, it bypasses `release.complete` and enters `archive.vault.complete` directly from `BLOCKED` status, producing `vault-receipt.json` instead of `release-receipt.json`.

Jurisprudence may reduce planning depth when the prior cycle ended in PASS with
`first_pass_success=true`. It never removes verify, mandatory A-* debt-verify,
release, archive, or their evidence gates.

## Knowledge Layers

| Layer | Purpose | Authority |
|-------|---------|-----------|
| SDDK workflow state | Coordinate the change lifecycle in `{cycle-artifacts-dir}` | Procedural, current-run only |
| Durable project knowledge | Nodes under CLI-resolved `{vault}` | Canonical when fresh |
| Engram memory | Episodic observations, learnings, jurisprudence | Recoverable, never canonical alone |

## Source Hierarchy

`verified code/tests > vault requirements/ADRs > current cycle artifacts > read-only product docs > Engram memory > chat claims`

Rules:
1. Fresher contradictory evidence wins only if provenance is explicit.
2. Memory can suggest a path; only durable artifacts or verified code/runtime evidence can bind a decision.

## Knowledge States

| State | Allowed Usage |
|-------|---------------|
| `proposed` | Exploration only |
| `trusted` | Routing and implementation input |
| `stale` | Advisory only (must recheck) |
| `superseded` | Historical context only |
| `contradicted` | Escalation trigger |

Never delete knowledge. Supersede or mark stale/contradicted instead.

## Jurisprudence Schema

When a cycle closes with PASS + first_pass_success=true + a reusable decision (ADR, lens, atajo), persist as Engram observation:

```
topic_key: jurisprudence/{category}
title: "{goal_pattern} — {path_that_worked}"
type: jurisprudence
content:
  goal_pattern: "{normalized goal}"
  stack_match: [lang, framework, ...]
  context_quality_typical: C0|C1|C2|C3
  path_that_worked: B-direct|A-min|A-lite|A-full
  lenses_that_mattered: [lens_id, ...]
  typical_duration_hours: float
  typical_cost_usd: float
  correction_cycles_typical: int
  key_learnings: "1-3 sentences"
  reusable: bool
```

At cycle start: `mem_search` goal_pattern → if hit, bias toward `path_that_worked` and `lenses_that_mattered`.

## Authority Matrix (compact)

| Question | Primary Source | Fallback | Never |
|----------|----------------|----------|-------|
| What problem? | vault milestone, approved proposal | recent cycle node | chat alone |
| What's in scope? | proposal and cycle spec | launch plan | memory alone |
| What must remain true? | vault requirements/ADRs and tests | verified learnings | agent guess |
| Why this design? | vault ADRs | archive report, read-only product docs | implementation alone |
| What happened before? | archive reports, Engram | session summaries | user recollection |

## ADR Threshold

Write an ADR only when all three are true: hard to reverse, surprising without context, real trade-off. Otherwise keep in spec or comment.

## Retrieval Preflight (before routing a meaningful phase)

1. Project operational init from `$SDDK_DATA_DIR/projects/{project_id}/`
2. Current `{cycle-artifacts-dir}`
3. Relevant nodes under `{vault}`
4. Existing product docs as read-only evidence
5. Optional Engram jurisprudence and prior failures when profile-enabled

If a missing class blocks confidence, record the gap. Do not compensate with bigger prompts.

## Product Documentation Discipline

Existing product documentation, including `CONTEXT.md`, architecture docs,
roadmaps, and ADRs, may inform a phase as read-only evidence. SDDK never creates
or updates those workspace files. Generated or normalized knowledge is written
to `{vault}` with provenance. If code, vault knowledge, and product evidence
disagree, surface the contradiction.

## Anti-Patterns

- Treating Engram memory as canonical truth
- Re-explaining whole project in each prompt instead of retrieving
- Editing accepted decisions in place (supersede instead)
- Verify findings die in a report without updating knowledge state
- Inferring ownership from code shape when explicit records exist
- Running full SDDK for a C3 bug fix (use B-direct)
- Coherence check at every transition regardless of context quality
- Treating a release tag as proof that archive completed

## XDG paths for SDDK ledger

The canonical SDDK ledger lives at XDG `state_home`, not `data_home`. On Linux:

- **Canonical (read/write)**: `~/.local/state/sddk/projects/<id>/ledger.sqlite`
- **Orphan stub (legacy installs)**: `~/.local/share/sddk/projects/<id>/ledger.sqlite`

The `data_home` location may contain a 0-byte stub from older installs that never migrated. **Do NOT** use the `data_home` path for verification or persistence assertions — always go through the engine CLI which resolves to `state_home` per ADR-0006.
