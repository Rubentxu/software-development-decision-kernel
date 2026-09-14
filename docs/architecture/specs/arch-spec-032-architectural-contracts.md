---
id: arch-spec-032-architectural-contracts
status: proposed
proposed_at: 2026-09-14
source: docs/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/
supersedes_history: false
---

# arch-spec-032 — Architectural Contracts

## Intent

Accepted architecture decisions SHALL be able to expose typed, revisioned, verifiable claims without making Markdown or lints a second authority.

## Requirements

- AC-032-001: stable contract identity includes semantic payload and decision/spec basis.
- AC-032-002: contract kinds use typed ADTs for closed common constraints and a versioned custom extension seam.
- AC-032-003: each contract links to Decision/Spec provenance.
- AC-032-004: claims support VERIFIED/CONTRADICTED/UNKNOWN/STALE-style outcomes with evidence refs.
- AC-032-005: missing evidence/provider cannot become PASS.
- AC-032-006: accepted semantic contract object, not Markdown text, is machine authority after ingestion.
- AC-032-007: contracts cannot directly grant capabilities or side effects.

## Acceptance

AC-UAT-001, 003, 004, 005, 006, 007.
