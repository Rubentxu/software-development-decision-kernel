# HANDOFF — A4-0b Observation input closed (v1.169.41)

> **A4-0b is CLOSED.** A4-1 has **not** started.

## Certified state

| | |
|---|---|
| **Slice** | A4-0b — declaration-supplied observations |
| **Cycle** | `p-63676b11dc0ef88f/a4-0b-observation-input` — **CLOSED** (sequence 16) |
| **Closes** | `FU-A4S0-1` (P1) |
| **Tag** | `v1.169.41` |
| **Release / bundle** | binary `1.169.41`, bundle `1.169.41`, framework `current → 1.169.41` |
| **HEAD == origin/main** | see push below |
| **Tests** | **205 blocks / 4317 passed / 0 failed** (+3) |
| **Baselines** | A3 green (`98b7fc7`/`v1.169.39`) · A4-0 green (`v1.169.40`) |

## What it closes

A4-0 delivered the substrate and pinned it at engine level, but `why_cmd` passed
`observations: None`. So **no user-facing run could populate the channel**, every
CLI answer reported `Insufficient`, and `Insufficient` was indistinguishable from
"the substrate does not work". That was the structural gap A4-0's handoff recorded;
this closes it.

The declaration — ADR-0120's *input*, not authority — may now carry:

```yaml
observations:
  - from: comp:dup
    to: comp:log
    kind: depends_on        # resolved from the closed CoreRelationKind vocabulary
    stance: affirms         # affirms | denies
    origin: static_provider  # deterministic_local | static_provider | runtime_provider
                             # | human_declared | inferred
    evidence: "static:dep-scan"
    producer: local-dep-scan
```

`convert_observations` is **fail-closed on every field**: unknown relation kind, an
endpoint naming no declared unit, unknown stance, unknown origin, blank evidence.
It sorts by id, so file order cannot reach identity. The relation kind is resolved
against `CoreRelationKind::ALL` **derived rather than hand-listed**, so a new core
kind becomes declarable the moment it exists.

`ArchitectureContext` builds the `ObservationSet` once in the shared seam;
`why_cmd` passes it. With an observation the `evidence → software relation` leg
**closes end to end**; without one it stays unresolved and the answer says so.

## Honest limitation (documented, not hidden)

The declaration supplies no knowledge basis, so the observation basis is the empty
basis at the declaration's own origin. `declared_at` does **not** enter identity, so
this is not a hidden zero — observation identity stays clock-stable, which is the
A4-0 invariant.

## Deliberately not in this cycle

**`FU-A3-CO-2`** (the relation-vocabulary disposition). Applying it moves
`CoreRelationKind::ALL` 16 → 14 and touches the historical anchor and post-AC1
baseline pins: an AC1 semantics change, not a behaviour-preserving cleanup. It needs
its own decision and remains **the gate before A4-1**, exactly as A4-0 recorded.

## Remaining follow-ups

| id | P | What |
|---|---|---|
| `FU-A3-CO-2` | **P2** | emit or remove `ContractedBy` / core `SpecifiedBy` — **gates A4-1** |
| `FU-A3-CO-1` | P2 | canonical relation encoding for `KnowledgePayload::Relation` |
| `FU-A3-CO-3` | P3 | disambiguate the two `SpecifiedBy` names |
| `FU-A3-S15-3` | P3 | `VerifiedBy` targets a spec node rather than evidence |
| `FU-A3-S15-4` | P3 | pin generated text against its condition |
| `ASC-MA-1` | P3 | root help describes `architecture` as only the receipt verb |

## Next

`FU-A3-CO-2`, then `A4-1` generic Verify. The A4-0 handoff's recommendation stands
and is now one item shorter: the evidence channel is **reachable** from a real run,
so the remaining blocker before A4-1 is the vocabulary decision, not the substrate.
