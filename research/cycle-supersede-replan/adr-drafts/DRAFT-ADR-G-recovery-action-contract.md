# DRAFT-ADR-G — Recovery-action contract for CLI failures

> **Status**: DRAFT (not accepted). Awaiting cycle-52+ adoption.
> **Date**: 2026-08-31
> **Author**: deep-research-orchestrator (read-only research)
> **Cycle binding**: none yet; candidate cycle-52 (depends on ADR-B)
> **Amends**: none (additive)
> **Supersedes**: none
> **Authority target**: `crates/sddk-cli/src/lib.rs` + cross-cutting

---

## Pre-decision: context, decisions, alternatives, compatibility, authority, migration

### Context

Some CLI error paths return a recovery command (e.g.,
`crates/sddk-cli/src/cycle.rs:773` — "cycle {} has no lease; acquire one
with `sddk cycle lock acquire` before rebuild"). Most return bare
diagnostics with no action. There is no enforced contract that every
failure response includes a single executable recovery action.

The principle "recover forward para proceso" requires that
**every** failure points to **one** executable recovery.

### Decision (proposed)

Adopt RFC 9457 "Problem Details for HTTP APIs" shape as the
canonical CLI error response:

```json
{
  "type": "https://sddk.local/errors/cycle-lock-acquire-required",
  "title": "Cycle lock required",
  "status": 409,
  "detail": "cycle p-52b95.../cycle-44 has no lease",
  "instance": "sddk:cycle:rebuild:p-52b95.../cycle-44",
  "recover_action": "sddk cycle lock acquire --cycle p-52b95.../cycle-44 --owner <name>",
  "recover_hint": "the fencing_token returned by acquire is required for the next rebuild"
}
```

The contract:

1. Every CLI error response carries `recover_action` (the executable
   command) and `recover_hint` (a one-line explanation).
2. The `recover_action` MUST be syntactically valid (parsable by the
   CLI's own clap parser; this is testable).
3. Recovery actions are **deterministic** — no LLM-generated text.

### Alternatives considered

| # | Alternative | Why rejected |
|---|---|---|
| G1 | Free-text hints (current behavior) | Non-deterministic; LLM-injected; not testable |
| G2 | Wiki-link to docs | Offline-unfriendly; brittle |
| G3 | LLM-generated recovery actions | Shifting the Burden; non-deterministic; per cycle-42 anti-fabrication discipline |
| G4 | Adopt RFC 7807 (older problem-details spec) | RFC 9457 (2023) supersedes it |

### Compatibility with current ledger

- **No ledger event change** (errors are CLI concern, not ledger).
- **Existing error responses** that already carry recovery actions
  (`cycle.rs:773`, `vault_cmd.rs:128-139`) keep their text but adopt the
  RFC 9457 JSON shape on `--format json`.
- **Plain text output** unchanged (humans prefer short text).
- **Backward compatibility**: clients that parse JSON output gain new
  fields; clients that ignore unknown fields are unaffected.

### Authority limits

- **`recover_action` is closed-set** — no free-form strings. The set is
  enumerated in `crates/sddk-cli/src/recovery.rs` (new module).
- **`recover_hint` is one line** (≤ 200 chars).
- **Errors without a recovery action are FORBIDDEN** — every error must
  have one. This is enforced by a lint check.

### Migration path

1. **Phase 1 (this research)**: ADR-G drafted.
2. **Phase 2 (cycle-52 candidate, A-min)**: implement
   `crates/sddk-cli/src/recovery.rs` with the closed-set registry; update
   error sites to adopt RFC 9457 JSON shape. RED test first.
3. **Phase 3 (cycle-53+)**: integrate with `sddk run` facade verb —
   process gates emit `recover-forward <command>`.

### Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Lint check too strict (rejects legitimate errors) | medium | medium | Whitelist for "internal-only" errors (cycle lock acquire is internal, not user-facing) |
| Closed-set registry becomes stale | low | medium | New recovery actions require ADR or lint allow-list with comment |
| Recovery action itself fails | low | medium | Recovery actions are themselves idempotent; failing to apply is logged |

---

## Post-decision: formal sections (for adoption)

### Status

(pending acceptance)

### Date

(pending acceptance)

### Consequences

(pending)

### Implementation notes

- New module: `crates/sddk-cli/src/recovery.rs` (closed-set registry).
- New lint rule in `crates/sddk-cli/src/lint.rs`: every error site must
  carry a recovery action.
- JSON output adopts RFC 9457 shape on `--format json`.

### Compatibility / migration

See Phase 1–3 above.

### Revisit trigger

Revisit when:

- A new category of error emerges (e.g., network errors with retry
  policies).
- The closed-set registry grows beyond ~50 actions (sign of
  fragmentation).

### Implementation trace

- **cycle-52** (target): implements recovery-action contract. Refer to
  `research/cycle-supersede-replan/evidence-cards/ec-css-007-recovery-action-contract.yml`.

---

## References

- `crates/sddk-cli/src/cycle.rs:773` (recovery hint inline)
- `crates/sddk-cli/src/vault_cmd.rs:128-139` (capability denial with hint)
- RFC 9457 "Problem Details for HTTP APIs" (March 2023) — `error.kind`
  taxonomy + standard fields
- AGENTS.md §4 principle ("Fail closed para seguridad; recover forward
  para proceso")
- `docs/adr/ADR-0047-durable-debt-remediation.md` (override discipline)