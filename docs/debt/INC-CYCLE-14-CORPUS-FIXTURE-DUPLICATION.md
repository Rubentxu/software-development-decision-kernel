---
id: INC-CYCLE-14-CORPUS-FIXTURE-DUPLICATION
title: "corpus_replay_through_validator inlines EventEnvelopeV1 construction duplicating valid_envelope fixture"
status: resolved
severity: low
priority: P3
fingerprint: "126823680682cdbc"
fingerprint_aliases: []
cluster_id: CL-DUPLICATION
created: 2026-08-22
created_by: sddk-debt-verify
owner: orchestrator
resolved_by: p-63676b11dc0ef88f/cycle-50-housekeeping-p3
resolved_at: 2026-09-01
---

# INC-CYCLE-14-CORPUS-FIXTURE-DUPLICATION — corpus test duplicates envelope-builder fixture

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

Cycle-14 (`p-52b95ef55999f9de/kernel-cycle-14-m2-event-foundation`,
A-min path) added `corpus_replay_through_validator` in commit `b6fc6d0`
as the round-1 remediation for missing corpus coverage of all 18
registered event types.

The test inlines a full `EventEnvelopeV1` construction (28 lines) at
`crates/sddk-domain/src/event_registry.rs:808-836` rather than reusing
the existing `valid_envelope(event_type, payload)` fixture defined
earlier in the same file at lines 533-563. A third fixture,
`minimal_env()`, lives in `crates/sddk-engine/src/event_bus.rs:613-641`
and follows the same pattern.

Duplication evidence (`grep` / diff):

| Fixture | Lines | Byte shape | Used by |
|---|---|---|---|
| `event_registry.rs:valid_envelope(event_type, payload)` | 533-563 | full envelope + recompute_content_hash | 5 other tests in `event_registry::tests` |
| `event_bus.rs:minimal_env()` | 613-641 | full envelope (preset `phase=build`) | 3 helper tests + `envelope_with_causation` builder |
| `event_registry.rs:corpus inline` | 808-836 | full envelope (per-corpus event_id, sequence, project_id, actor) | `corpus_replay_through_validator` only |

The corpus test cannot use `valid_envelope` directly because it needs
per-row event_id, sequence, project_id, and actor.id — these differ
per corpus entry. However, the structural construction (all the
None-defaultable fields and `content_hash = ""` then
`env.content_hash = env.compute_content_hash()`) is identical and
would benefit from extracting a `mk_event_envelope(corpus_row)`
helper or a `[corpus]; 18 fixture` table.

## Rationale

- **Severity = low**: test-only duplication. No production impact. The
  duplicated code IS correct (verified by `corpus_replay_through_validator`
  passing under `cargo test --workspace --locked`). Maintenance cost
  is: if `EventEnvelopeV1` gains/loses a field, three sites must be
  updated in lockstep rather than one fixture.

- **Priority = P3**: opportunistic; acceptable to remain until the
  next `EventEnvelopeV1` field change or until a second corpus test
  (e.g. partial-malformed-corpus) is added. Recommended fix is a
  ~15-LOC refactor:

  ```rust
  fn build_corpus_envelope(seq: usize, event_type: &str, payload: serde_json::Value) -> EventEnvelopeV1 {
      let mut env = EventEnvelopeV1 { /* defaults from valid_envelope */ };
      env.event_id   = format!("evt-corpus-{}", seq);
      env.event_type = event_type.to_string();
      env.sequence   = seq as u64;
      env.project_id = "p-corpus".to_string();
      env.actor.id   = "sddk-cli".to_string();
      env.payload    = payload;
      env.content_hash = env.compute_content_hash();
      env
  }
  ```

  Net ~28 LOC reduction in the corpus test body; single source of
  truth for envelope construction defaults.

- **Cluster = `CL-DUPLICATION`** (new cluster id; the existing
  `CL-LOC-OVERAGE` family is about implementation LOC budget, not
  fixture duplication; `CL-DOC-QUALITY` is for rustdoc/comments).

  Note: the launch packet restricts the debt-verify scope to the
  `coupling + overeng` clusters for A-min smoke. This finding is the
  first sddk-debt-verify observation under the `CL-DUPLICATION`
  cluster id; the cluster is being established by this INC.

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-08-22 | sddk-debt-verify | created | inline construction at `event_registry.rs:808-836` (added in `b6fc6d0`) duplicates `valid_envelope` at `event_registry.rs:533-563` |

## Closure Evidence

Closed by `p-63676b11dc0ef88f/cycle-50-housekeeping-p3` (v1.66.4).

- **Resolution:** Extracted `build_corpus_envelope(seq, event_type, payload)` helper inside `#[cfg(test)] mod tests` at `validator.rs:160-194`. 28-line inline construction removed from corpus test loop. Anti-double-increment verified: helper applies `+1` internally; call site passes 0-based `seq` unmodified. `corpus_replay_through_validator` passes with byte-equivalent JSON replay (evt-corpus-1..18, sequence 1..18).
- **Closing commit:** `4bd47fc` — refactor(domain): extract build_corpus_envelope helper (cycle-50 commit #4)
- **Release tag:** [v1.66.4](https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.66.4)

## References

- `crates/sddk-domain/src/event_registry.rs:533-563` — `valid_envelope(event_type, payload)` (existing fixture, 5 callers)
- `crates/sddk-domain/src/event_registry.rs:702-889` — `corpus_replay_through_validator` (corpus test, inline construction at 808-836)
- `crates/sddk-engine/src/event_bus.rs:613-641` — `minimal_env()` (sibling fixture in another file)
- `crates/sddk-engine/src/event_bus.rs:643-652` — `envelope_with_causation` (composition fixture built atop `minimal_env`)
