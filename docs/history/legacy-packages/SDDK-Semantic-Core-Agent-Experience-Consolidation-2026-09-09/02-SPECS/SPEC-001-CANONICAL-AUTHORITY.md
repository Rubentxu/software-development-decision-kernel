# SPEC-001 — Canonical authority and persistence contracts

## Requirements

- CA-001: exactly one logical CanonicalEventLog is authoritative for ordered facts.
- CA-002: immutable payloads larger than bounded event metadata use CAS references.
- CA-003: every projection declares source stream(s), projector version and rebuild behavior.
- CA-004: every store/model declares Fact/Object/Projection/Ephemeral class.
- CA-005: no projection may be the sole evidence for an irreversible side effect.
- CA-006: event/object canonicalization is deterministic and schema-versioned.
- CA-007: compatibility mirrors are explicitly marked and have removal criteria.

## UAT

1. delete SemanticGraph/metrics/run read models; rebuild from canonical facts and verify semantic equality;
2. inject divergence into a compatibility mirror; canonical reads remain unaffected and doctor reports divergence;
3. architecture lint rejects a second component declaring authority for the same concept.
