---
id: arch-spec-A3-S14-architecture-findings
title: Inspecting the DebVerify audit
status: accepted
cycle: p-63676b11dc0ef88f/a3-14-architecture-findings
based_on: arch-spec-A3-S11-architecture-read-surfaces + arch-spec-A3-S13-architecture-verify-entry-point
---

# arch-spec-A3-S14 — Inspecting the DebVerify audit

`12-CLI-AGENT-UX.md` groups `sddk deb-verify architecture` with
`sddk verify architecture` under **Verification**.

A3-S13 established that the verification entry point is `sddk architecture
receipt` (ADR-0120 owns that surface). This cycle closes the other half: the AC5
audit already runs on every receipt, and its output is not inspectable.

## Naming disposition (recorded, not reopened)

**No `deb-verify` verb, no alias.** The proposal's two commands are not the same
operation:

- `receipt` **composes and gates**: AC2/AC4/AC5/AC6/AC7 → verdict → exit 1 when
  blocked. AC5's findings already participate (mandatory ones drive `Blocked`).
- what is absent is **inspection** of AC5's output — the shape A3-S11's five read
  surfaces already have: read-only, verdict-free, score-free projections.

So the realisation is a sixth read surface, `sddk architecture findings`. A third
verb that runs the audit and prints it would duplicate `receipt` (already runs
it) and `findings` (would print it), against §2.7/§2.9; an alias is forbidden
(§2.0).

## The gap being closed

AC5's `DebVerifyFinding` carries `kind`, `severity`, `subjects`, `contract_ids`
and `message`. The receipt keeps:

- `audit_findings`: `{canonical_tag: count}` — **counts only**;
- `unresolved`: `{kind, subjects, mandatory, waiver_refs}` — only for **mandatory**
  findings, and with no severity, no contract ids, no message.

Measured on a declaration with a shadow authority on `comp:b` and one elapsed
compatibility window:

```
audit_findings: { "shadow_authority": 1, "stale_compatibility": 1 }
unresolved: [ {kind: shadow_authority, subjects: ["comp:b"], mandatory: true},
              {kind: stale_compatibility, subjects: ["c-stale"], mandatory: false} ]
```

Discarded: **which two contracts** conflict (the actionable fact), the
**severity** (already deterministic, 3 closed variants), the message, and any
detail for non-mandatory kinds.

## Scope

**In:** `sddk architecture findings`; `--kind <tag>` fail-closed; `--format
json|text`; registration in `command_spec` and the `architecture` enum.

**Out:** a `deb-verify` verb or alias; `--severity`; changing the receipt payload;
any verdict, score, write or persistence; a new root module.

## Constraints

- MUST NOT add a new root module (no ADR required).
- MUST NOT change the receipt payload or the `receipt` verb's behaviour.
- MUST NOT emit a verdict or a numeric score (AC-UAT-043).
- MUST NOT accept a `--kind` it does not filter by.
- MUST share the audit invocation path with `receipt`, so the two cannot drift.
- MUST surface `severity`, `contract_ids` and `message`, not just counts.

## Requirements (REQ-A3S14-NNN)

- **REQ-A3S14-001** — `architecture findings` lists every AC5 finding with kind,
  severity, subjects, contract ids and message. Pin: e2e
  `architecture_findings_lists_full_shape`.
- **REQ-A3S14-002** — `--kind <tag>` narrows to that kind; the accepted tags are
  the five `DebVerifyFindingKind` canonical tags. Pin: e2e
  `architecture_findings_kind_filter`.
- **REQ-A3S14-003** — An unknown `--kind` fails closed: exit 2, and the error
  names the tag and the accepted set. Pin: e2e
  `architecture_findings_unknown_kind_fails_closed`.
- **REQ-A3S14-004** — A declaration with no findings is exit 0 and an explicitly
  empty listing, not an error and not a silent success. Pin: e2e
  `architecture_findings_empty_is_ok`.
- **REQ-A3S14-005** — Severity is surfaced and is the kind's deterministic
  severity (`Critical`/`High`/`Medium`). Pin: e2e
  `architecture_findings_reports_severity`.
- **REQ-A3S14-006** — The listing's per-kind counts agree with the receipt's
  `audit_findings` for the same declaration and clock. Pin: e2e
  `architecture_findings_agree_with_receipt`.
- **REQ-A3S14-007** — `--now-ms` defaults to the wall clock so
  `stale_compatibility` is honest; a pinned clock is reproducible. Pin: e2e
  `architecture_findings_clock_is_honest`.
- **REQ-A3S14-008** — The surface is read-only: no verdict, no score, and it
  writes nothing. Pin: e2e `architecture_findings_is_inspection_not_a_gate`.
- **REQ-A3S14-009** — A missing or invalid declaration fails closed (exit 2). Pin:
  e2e `architecture_findings_fails_closed_on_bad_declaration`.
- **REQ-A3S14-010** — The subcommand is registered in the command registry, so
  `sddk agent-help` and the CLI docs see it. Pin: `command_spec` suite.

## Design sketch

`findings` joins the read-surface family: `ReadArgs` gives `--root`,
`--contracts`, `--now-ms`, `--format`; `load_declaration` and `declare_overlay`
are shared with `receipt` and the other read surfaces, so the audit is invoked
identically in both places (which is what makes REQ-006 a real invariant rather
than a hope).

A new `FindingsArgs` adds `--kind: Option<String>`. The tag is resolved against
`DebVerifyFindingKind::ALL` plus each variant's `canonical_tag()`, so the accepted
set is derived from the closed enum rather than hand-listed.

Rendering mirrors the existing row-per-item read surfaces: one flat, aligned line
per finding, counts summarised at the end, no verdict line.

## Acceptance tests (planned, 10)

CLI e2e (`tests/architecture_findings_cli_e2e.rs`):

1. `architecture_findings_lists_full_shape`
2. `architecture_findings_kind_filter`
3. `architecture_findings_unknown_kind_fails_closed`
4. `architecture_findings_empty_is_ok`
5. `architecture_findings_reports_severity`
6. `architecture_findings_agree_with_receipt`
7. `architecture_findings_clock_is_honest`
8. `architecture_findings_is_inspection_not_a_gate`
9. `architecture_findings_fails_closed_on_bad_declaration`
10. `architecture_findings_subcommand_registered` (via the `command_spec` suite)
