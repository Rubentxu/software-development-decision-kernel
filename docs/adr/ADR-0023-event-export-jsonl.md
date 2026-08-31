# ADR-0023 — Event Export JSONL for Debugging and Tooling

**Status:** proposed
**Date:** 2026-08-19
**Trigger:** Roadmap SDDK 2.0 Phase 2 SHOULD: *"Add event export JSONL for debugging/tooling"*

---

## Context

Phase 2 SHOULD item remains incomplete. No JSONL export capability exists in the codebase.

Use cases:
1. **Debugging:** `sddk ledger events --jsonl > /tmp/trace.jsonl` to replay locally
2. **Tooling:** External tools (e.g., `jq`, `grep`, custom scripts) consume the event stream
3. **Migration:** Export all events to migrate to a different storage backend
4. **Audit:** One-file audit trail that is append-only and line-oriented

---

## Decision

### 1. `sddk ledger events --jsonl` command

Add `--jsonl` flag to existing `sddk ledger events` command that emits one `EventEnvelopeV1`
JSON object per line, ordered by increasing `sequence`.

```
$ sddk ledger events --stream project:p-1 --jsonl | jq '.event_type'
"approval.capability.requested"
"approval.capability.granted"
```

### 2. `sddk ledger export <stream> --output <path>` command

New subcommand for bulk export with progress:

```
$ sddk ledger export project:p-1 --output /tmp/ledger.jsonl
Exported 1,234 events to /tmp/ledger.jsonl  [00:02 < 00:05, 520 events/s]
```

### 3. JSONL Line Format

Each line is a JSON object:

```json
{"event_id":"evt-1","event_type":"cycle.created","stream_id":"project:p-1","sequence":1,"content_hash":"sha256:...","chain_hash":"sha256:...","occurred_at":"2026-08-19T10:00:00Z","payload":{...}}
```

All fields are from `EventEnvelopeV1` (v1 schema). The `content_hash` and `chain_hash`
fields are included so external tools can verify integrity independently.

### 4. Streaming Output

For large ledgers (>10k events), export streams to stdout/file in chunks to avoid
memory pressure. The `SqliteEventStore::load_stream` already supports pagination via
`from_sequence` + `limit` — the export command uses this internally.

### 5. No Import Command (Out of Scope)

Import/load from JSONL is deferred. It requires idempotency handling (skip existing
event_ids) that adds complexity. Export-only is the minimal viable product.

---

## CLI Interface

```
sddk ledger events [OPTIONS]
  --stream <ID>          Stream to list (default: project stream)
  --frame <ID>           Filter by command frame
  --limit <N>             Max events (default: 50)
  --jsonl                Emit JSONL instead of human-readable table
  --format <FORMAT>      text | json | jsonl (default: text)

sddk ledger export <STREAM> [OPTIONS]
  --output <PATH>        Output file (default: stdout)
  --from-seq <N>         Start from sequence N (default: 1)
  --progress             Show progress bar
```

---

## Consequences

- **Positive:** Enables `jq`/`grep`/`awk` tooling on the event stream without DB access.
- **Positive:** One-line audit trail format is industry standard.
- **Positive:** Export is read-only — no risk to ledger integrity.
- **Negative:** Large exports can fill disk; add `--max-size` flag if needed.
- **Negative:** No import command means JSONL is not a migration path yet.

---

## Implementation Plan (P2-JL-001)

| Step | Description | File | Issue |
|------|-------------|------|-------|
| 1 | Add `--jsonl` flag to `LedgerEventsArgs` | `sddk-cli/src/ledger.rs` | P2-JL-001 |
| 2 | Implement JSONL rendering for events | `ledger.rs` | P2-JL-002 |
| 3 | Add `sddk ledger export` subcommand | `ledger.rs` | P2-JL-003 |
| 4 | Streaming export with pagination | `ledger.rs` | P2-JL-004 |
| 5 | Add `export_stream_jsonl` to `SqliteEventStore` | `event_store.rs` | P2-JL-005 |
| 6 | E2E test for export command | `tests/e2e/` | P2-JL-006 |

---

## Exit Criteria

- [ ] `sddk ledger events --stream project:p-1 --jsonl` emits valid JSON lines
- [ ] Each JSON line parses to an `EventEnvelopeV1`
- [ ] `sddk ledger export project:p-1 --output /tmp/test.jsonl` works
- [ ] Export of 10k+ events shows progress and completes without OOM
- [ ] `jq '.event_type' < events.jsonl` works correctly

---

## References

- Phase 2 SHOULD: *"Add event export JSONL for debugging/tooling"*
- `EventEnvelopeV1` schema in `sddk-domain/src/event_envelope.rs`
- `SqliteEventStore::load_stream` pagination pattern
