# ADR-0024 — Evidence Redaction Rules

**Status:** accepted
**Date:** 2026-08-19
**Trigger:** Roadmap SDDK 2.0 Phase 3 MUST: *"Define redaction rules for evidence"*

---

## Context

Phase 3 MUST item: *"Define redaction rules for evidence"*. Currently, evidence
(payloads stored with events) may contain sensitive data:

```
evidence payload example (current):
{
  "agent_id": "agent-1",
  "model": "claude-opus-4",
  "prompt_tokens": 1234,
  "output_tokens": 567,
  "raw_response": "The implementation uses sha256:abc123..."  ← could contain secrets
}
```

Redaction rules define which fields are:
- **Public** — safe to include in receipts, exports, and logs
- **Redacted** — replaced with `[REDACTED]` in exports, receipts, and UI
- **Omitted** — removed entirely from non-audit contexts

---

## Decision

### 1. Redaction Levels

```rust
// sddk-domain/src/evidence.rs
pub enum RedactionLevel {
    /// Field appears in all contexts unchanged.
    Public,
    /// Field replaced with [REDACTED] in receipts and UI.
    Restricted,
    /// Field removed from receipts and UI; available only in raw audit log.
    Confidential,
}
```

### 2. Redaction Registry

A static registry maps evidence field paths to redaction levels:

```rust
// sddk-domain/src/evidence.rs
pub struct RedactionRule {
    pub field_path: &'static str,  // e.g., "raw_response", "tool_calls[].output"
    pub level: RedactionLevel,
    pub reason: &'static str,
}

pub static REDACTION_RULES: &[RedactionRule] = &[
    RedactionRule {
        field_path: "raw_response",
        level: RedactionLevel::Restricted,
        reason: "May contain model-generated content with embedded secrets",
    },
    RedactionRule {
        field_path: "prompt",
        level: RedactionLevel::Restricted,
        reason: "May contain project names or internal context",
    },
    RedactionRule {
        field_path: "tool_calls[].output",
        level: RedactionLevel::Restricted,
        reason: "Tool outputs may contain sensitive system data",
    },
    // ... more rules
];
```

### 3. RedactedEvidence Wrapper

```rust
pub struct RedactedEvidence {
    pub original: serde_json::Value,
    pub redacted: serde_json::Value,
    pub fields_redacted: Vec<String>,
}

impl Evidence {
    /// Returns a redacted view of this evidence for receipts and UI.
    pub fn redacted(&self) -> RedactedEvidence { ... }
}
```

### 4. Scope: Evidence Payloads Only

Redaction applies **only** to evidence payloads in `EventEnvelopeV1.payload`.
The `content_hash`, `chain_hash`, `event_type`, and structural fields are NEVER
redacted (they are cryptographic integrity anchors).

### 5. Audit Log Access

A separate `evidence raw` command (restricted to operators) provides the full
unredacted payload for incident investigation:

```
$ sddk evidence raw <event_id> --require-mfa
[unredacted payload]
```

---

## Redaction Rules Initial Registry

| Field | Level | Reason |
|-------|-------|--------|
| `raw_response` | Restricted | May contain model-generated secrets |
| `prompt` | Restricted | May contain project context |
| `tool_calls[].output` | Restricted | Tool outputs may leak data |
| `system_prompt` | Confidential | Internal instructions |
| `api_key` | Confidential | Never in evidence but guard-rail |
| `password` | Confidential | Never in evidence but guard-rail |

Rules are additive. New rules can be added by PR with ADR amendment.

---

## Consequences

- **Positive:** Receipts, exports, and UI can display evidence summaries without
  leaking sensitive fields.
- **Positive:** Clear policy separates "audit everything" (operators) from
  "receipt for the record" (public evidence).
- **Negative:** Redaction is applied at serialization time — evidence stored in
  SQLite is unredacted. If the DB is compromised, redaction doesn't protect it.
  (For that, use encrypted evidence storage — deferred to Phase N).
- **Negative:** Developers must remember to use `evidence.redacted()` instead of
  `evidence.payload` directly.

---

## Implementation Plan (P3-RD-001)

| Step | Description | File | Issue |
|------|-------------|------|-------|
| 1 | Add `RedactionLevel` enum + `REDACTION_RULES` registry | `sddk-domain/src/evidence.rs` | P3-RD-001 |
| 2 | Add `Evidence::redacted()` method | `sddk-domain/src/event_envelope.rs` | P3-RD-002 |
| 3 | Apply redaction in `EventReceipt` serialization | `sddk-engine/src/lib.rs` | P3-RD-003 |
| 4 | Add `sddk evidence raw` command (operator-only) | `sddk-cli/src/evidence.rs` | P3-RD-004 |
| 5 | Unit tests for redaction logic | `sddk-domain/tests/` | P3-RD-005 |
| 6 | Verify receipts in E2E tests use redacted evidence | `tests/e2e/` | P3-RD-006 |

---

## Exit Criteria

- [ ] `EventReceipt` evidence field shows `[REDACTED]` for `raw_response`
- [ ] `sddk evidence raw <event_id>` (operator) shows full unredacted payload
- [ ] `sddk ledger events --jsonl` exports redacted evidence (not full)
- [ ] `sddk ledger verify-chain` / receipts unaffected by redaction (hash integrity preserved)
- [ ] Unit test: redaction rule matches `tool_calls[].output` nested field

---

## References

- Phase 3 MUST: *"Define redaction rules for evidence"*
- `EventEnvelopeV1::payload` field in `sddk-domain/src/event_envelope.rs`
- Receipt serialization in `sddk-engine/src/lib.rs` (EventReceipt)
