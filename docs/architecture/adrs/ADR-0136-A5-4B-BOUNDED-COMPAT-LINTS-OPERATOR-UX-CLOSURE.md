---
id: ADR-0136-A5-4B-BOUNDED-COMPAT-LINTS-OPERATOR-UX-CLOSURE
status: accepted
supersedes_history: false
adopted_at: 2026-09-18
accepted_at: 2026-09-18
accepted_by_cycle: p-63676b11dc0ef88f/a5-4b-bounded-compat-lints-operator-ux
---

# ADR-0136 — A5-4b: Bounded Compatibility, Lints & Operator UX Closure

## Context

A5 plan (§3 of `A5-DEBT-DISPOSITION.md`) carried 8 inherited
`MUST_CLOSE_A5` items from A3/A4 followups. A5-4a retired the
`paradigm_lens::evaluate_lens()` facade (v1.169.82, ADR-0135) but
left four items open:

- `FU-A3-CO-1` — relation payload encoding convention
- `FU-A3-CO-3` — remaining rename / shape cleanup (post-CO-2)
- `FU-A3-S15-4` — fitness rule / CLI lint / doctor conversion
- `ASC-MA-1` — `sddk --help` UX pass

The cycle also needed to dispose three advisory lints that remained
at `default = "allow"`:

- `execution_outcome_as_synthesis` (0 hits)
- `transition_outcome_used` (24 hits — state_machine regression guard)
- `asset_unregistered_cli_example` (0 hits — regex unsafe-by-design)

And one item needed honest disclosure: `EvidenceAttachmentV1` + the
compat decoder in `crates/sddk-engine/src/.../evidence_ref.rs:192`
could not be folded into a single-risk A5-4b cycle because it
encroaches on `EvidencePosture` / `Verify` / `DebVerify` /
`Alignment` / `IntelligenceLoop` / `AdvisoryWhy` /
`UniversalConcern` semantics.

## Decision

A5-4b is a **bounded-compatibility** cycle. Its budget is one
risk class: dispose the 4 MUST_CLOSE items, pin the 3 advisory
lints as `KEEP_ALLOW_WITH_REASON` with machine-readable
registry-pinned explanations, and carry `EvidenceAttachmentV1`
forward to a separate slice. No semantic freeze changes; no
Authority / provider / M0–M9 restructuring.

### 1. Disposition taxonomy (per item)

| Item | Disposition | Rationale |
|---|---|---|
| `FU-A3-CO-1` | **KEEP_WITH_REASON** | `KnowledgePayload::Relation` exists with zero production writers; A4-4 closed the lens-shape concern independently. Recorded so a future cycle does not re-open it as orphaned work. |
| `FU-A3-CO-3` | **CLOSED_BY_PRIOR_WORK** | `SpecifiedBy` is the one canonical relation post-A4-S15R (v1.169.65); `VerifiedBy` was repointed to `EvidenceRef` in the same cycle. |
| `FU-A3-S15-4` | **CLOSED_BY_PRIOR_WORK** | TOML-driven lint at `docs/architecture/lints/deprecated_patterns.toml` + impl in `crates/sddk-cli/src/dev/lint/deprecated_patterns.rs`. Zero duplicate implementations across lint / doctor / release. |
| `ASC-MA-1` | **CLOSE_BY_HELP** | `crates/sddk-cli/src/lib.rs:189` about-line corrected; pin tests assert BOTH absence of legacy substring AND presence of `sddk agent-help` substring. |

### 2. Advisory lints

All three `allow` lints carry an `explanation = """ ... """` body of
> 10 non-whitespace tokens in the registry. None satisfies the
ADR-0001 §3.2 acceptance gates for promotion
(deterministic detector, false-positive audit, migration complete,
0 illegitimate hits), so they stay `allow`.

The non-promotion is **machine-pinned** by
`crates/sddk-cli/tests/a5_4b_lint_disposition_pin.rs`:

1. `a5_4b_allow_lints_have_default_allow_and_non_empty_explanation` —
   walks the registry, asserts each `allow` entry has
   `default = "allow"` plus an explanation body of > 10 tokens.
2. `a5_4b_no_new_allow_lints_added_silently` — enforces
   `allow_blocks_in_registry == ALLOW_LINTS.len()` (3 at v1.169.83).
3. `a5_4b_denied_lints_have_zero_illegitimate_hits_at_v1_169_83` —
   freezes the deny-lint corridor (6 lints at 0 hits each).

### 3. Carry-forward

`EvidenceAttachmentV1` + compat decoder → **STOP_NEEDS_SEPARATE_SLICE**.
The compat decoder is C2.5 risk class (storage migration of how
attachments are stored), and A5-4b's anti-encroachment list
prohibits `EvidencePosture` / `Verify` / `DebVerify` /
`Alignment` / `IntelligenceLoop` / `AdvisoryWhy` /
`UniversalConcern` semantics changes. Tracked under
`A5-DEFERRED-POST-BASE.md`.

### 4. About-line rewrite (`crates/sddk-cli/src/lib.rs:189`)

Old:

> Deterministic SDDK workflow tooling — First-class commands:
> status, plan, run, ship, recover, memory

New:

> Deterministic SDDK workflow tooling — uses `sddk agent-help`
> for the operator-facing surface

The five legacy facade commands remain available, but no longer
carry the "first-class" label. Two pin tests
(`first_class_commands::help_drops_first_class_substring` and
`cli_first_class_help::help_drops_first_class_substring`)
verify the assertion in both directions: BOTH that the legacy
substring is absent AND that the new `sddk agent-help` substring
is present. Two golden fixtures are regenerated:
`docs/architecture/tests/fixtures/cli_golden/1.168.8/sddk-help.txt`
(54 → 58 lines) and `crates/sddk-cli/tests/fixtures/cli/help-top-level.txt`
(4148-byte stderr snapshot).

## Consequences

- Operators are pointed at the canonical `sddk agent-help` surface.
- The 4 MUST_CLOSE items leave the debt plan with machine-readable
  evidence of disposition (no orphaned work).
- The 3 advisory lints remain `allow` but with registry-pinned
  explanations and test-pinned denial of silent growth.
- `EvidenceAttachmentV1` is explicitly named as out-of-scope for
  this cycle and carried forward.
- No semantic-freeze change. No M0–M9 restructuring. No
  Authority/provider changes.

## Alternatives considered

- **Promote all three `allow` lints.** Rejected: none satisfies the
  ADR-0001 §3.2 acceptance gates. Promotion by enthusiasm is
  explicitly forbidden (`PROMOTE_DENY` in
  `docs/debt/PRIORITY.md`).
- **Strip the legacy facade commands entirely.** Rejected: anti-
  encroachment scope (A5-4b is bounded-compatibility, not
  command retirement). Stripping would expand to C2.
- **Fold `EvidenceAttachmentV1` into A5-4b.** Rejected: C2.5 risk
  class; anti-encroachment on `EvidencePosture` semantics. Single-
  risk budget cannot absorb.
- **Re-open FU-A3-CO-1 to invent a new encoding.** Rejected: A4-4
  already closed the lens-shape concern independently, and no
  alternate encoding exists in the codebase to close against.
