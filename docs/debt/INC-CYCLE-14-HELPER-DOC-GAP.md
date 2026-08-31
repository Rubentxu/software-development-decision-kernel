---
id: INC-CYCLE-14-HELPER-DOC-GAP
title: "Correlation/causation helpers are pub fn with 0 production callers; rustdoc lacks deferred-wiring note"
status: open
severity: low
priority: P3
fingerprint: "a8954ad16336955b"
fingerprint_aliases: []
cluster_id: CL-DOC-QUALITY
created: 2026-08-22
created_by: sddk-debt-verify
owner: orchestrator
---

# INC-CYCLE-14-HELPER-DOC-GAP — pub correlation/causation helpers lack deferred-wiring rustdoc note

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

Cycle-14 (`p-52b95ef55999f9de/kernel-cycle-14-m2-event-foundation`,
A-min path) added three new `pub fn` helpers in
`crates/sddk-engine/src/event_bus.rs` via commit `4c6a0bd`:

- `with_correlation_from_context` (line 416)
- `with_causation` (line 426)
- `trace_causation_chain` (line 440)

After the REQ-M14-004 amendment of 2026-08-22, production wiring of
these helpers was explicitly **deferred to M6 SPEC-028** because
calling them from the existing `emit_*` builders would require threading
an `EventContext` parameter through the builders, which would change the
public signatures — violating the anti-acceptance-criteria.

Current state at `1bf93b1`:
- `grep -rn 'with_correlation_from_context\|with_causation\|trace_causation_chain' --include='*.rs'`
  returns **21 hits, ALL in `event_bus.rs`** (3 definitions + 18 test-file references).
- **Zero production callers** outside the helper definitions themselves.

The rustdoc on each helper explains the technical behaviour
("idempotent; no-op when field is preset") but does **not** mention:

1. That production wiring is intentionally deferred to M6 SPEC-028.
2. That the helpers currently have zero production callers by design.
3. A pointer to `spec.md` REQ-M14-004 amendment / `verify-report.md`
   Amendment-Deviation Record for the curious reader.

This is a **discoverability / rustdoc accuracy gap**, not a correctness
defect. The behaviour described ("idempotent no-op when preset") IS
correct; the gap is that it omits the WHY-NO-PRODUCTION-CALLER context
that an engineer reading the public API would naturally seek.

## Rationale

- **Severity = low**: this is a documentation-accuracy issue, not a
  behavioural defect. The helpers work as documented; the `verify-report.md`
  proves byte-equivalence preserved when fields are preset. No production
  code is broken by the absence of a deferred-wiring note.

- **Priority = P3**: opportunistic; one-cycle deferred-wiring rustdoc
  amendment. Recommended fix (cycle-15 or first post-M6-wiring commit):
  add a `///` note to each of the three helpers pointing to SPEC-028
  and the REQ-M14-004 amendment. Example wording:

  ```rust
  /// # Production wiring
  ///
  /// Helpers ship as public tested API only. Production wiring (calling
  /// these from the `emit_*` builders) is deferred to M6 SPEC-028,
  /// when the dispatcher primitive becomes the first real consumer.
  /// See `spec.md` REQ-M14-004 amendment 2026-08-22.
  ```

  Three-line patch; no behavioural change.

- **Cluster = `CL-DOC-QUALITY`** (same family as
  `INC-CYCLE-13-DURABILITY-COMMENT-ACCURACY` from cycle-13; same
  principle: rustdoc/comments under-document WHY a feature ships in
  its current shape).

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-08-22 | sddk-debt-verify | created | helpers added in `4c6a0bd`; 0 production callers confirmed via `grep` (21 hits, all in `event_bus.rs`) |

## References

- `crates/sddk-engine/src/event_bus.rs:411-420` — `with_correlation_from_context` rustdoc lacks deferred-wiring note
- `crates/sddk-engine/src/event_bus.rs:422-430` — `with_causation` rustdoc lacks deferred-wiring note
- `crates/sddk-engine/src/event_bus.rs:432-475` — `trace_causation_chain` rustdoc lacks deferred-wiring note
- `crates/sddk-engine/src/event_bus.rs:484-528` — 3 named tests exercise the helpers
- `$SDDK_DATA_DIR/.../kernel-cycle-14-m2-event-foundation/spec.md:63-78` — REQ-M14-004 amendment body
- `$SDDK_DATA_DIR/.../kernel-cycle-14-m2-event-foundation/verify-report.md:53-95` — Amendment-Deviation Record
