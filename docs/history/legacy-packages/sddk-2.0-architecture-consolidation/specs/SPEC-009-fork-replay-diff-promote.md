# SPEC-009 — Fork, Replay, Diff and Promote

**Status:** Proposed

## 1. Goal

Enable controlled counterfactual experimentation on SDDK runs, policies, packs and agent configurations.

## 2. Frame vs fork

- **Frame:** bounded causal context within one run/cycle; shared durable history.
- **Fork:** durable branch from a specific ledger event/sequence, independently inspectable and diffable.

Use a fork when independent history, comparison or promotion may matter later.

## 3. Fork model

A fork records:

- `fork_id`;
- `parent_stream_id`;
- inclusive `at_sequence` / event ID;
- shared prefix hash;
- label;
- overrides;
- creator actor;
- created-at;
- replay/cache policy.

The shared prefix SHOULD be reconstructed from snapshots/cache without re-invoking LLM/tool I/O.

## 4. Replay

Two replay modes:

- `reconstruct`: rebuild state without invoking nondeterministic behaviors;
- `strict`: re-execute deterministic behavior and compare expected output/event hashes; recorded LLM/tool responses are served from cache unless explicitly re-recording.

Divergence MUST be surfaced with the first causal mismatch.

## 5. Diff

Diff MUST include structural differences and MAY add semantic metrics:

- events only in parent/fork;
- changed graph nodes/relations;
- changed workflow/gate outcomes;
- changed evidence coverage;
- changed findings;
- cost/latency/token deltas;
- architecture/coupling metrics;
- human corrections.

## 6. Promote

Promotion applies an accepted fork delta through normal SDDK policy/capability paths. It MUST fail closed if the parent changed incompatibly after the fork point.

Promotion of Git changes is separate from graph-state promotion and requires Git capabilities/receipts.

## 7. Agent A/B evaluation

Forks SHOULD support comparing agent/model/prompt/policy versions against the same recorded prefix and golden fixtures. This is the basis for evidence-driven agent evolution.

## 8. CLI target

```text
sddk fork create --at <event>
sddk fork set <fork> key=value
sddk fork run <fork>
sddk fork diff <parent> <fork>
sddk fork promote <fork>
```
