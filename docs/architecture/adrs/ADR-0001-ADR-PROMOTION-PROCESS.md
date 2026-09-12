---
id: ADR-0001-ADR-PROMOTION-PROCESS
title: "ADR promotion process — proposed → accepted → released transitions"
status: accepted
created: 2026-09-12
created_by: orchestrator
decided: 2026-09-12
accepted_at: 2026-09-12
accepted_by_cycle: "p-63676b11dc0ef88f/adr-promotion-m0-meta-and-batch-1"
supersedes: []
superseded_by: []
related_adrs:
  - "ADR-0096-SDLC-LIFECYCLE-SEMANTICS"
  - "ADR-0100-UNIVERSAL-EVIDENCE"
  - "ADR-0101-AGENT-OUTCOME-CONTRIBUTION-SYNTHESIS"
related_specs:
  - "docs/architecture/specs/arch-spec-001-canonical-authority.md"
  - "docs/architecture/specs/arch-spec-002-lifecycle-model.md"
stale_after: 2027-09-12
---

# ADR-0001 — ADR promotion process

> **Meta-ADR.** Defines the lifecycle an ADR follows from
> `status: proposed` through `status: accepted` and optionally
> `status: released`. Concrete ADRs (ADR-0094..ADR-0110+) reference
> this process when transitioning states.

## Context

At v1.168.31, 17 ADRs in `docs/architecture/adrs/` carry
`status: proposed` (ADR-0094 through ADR-0110, all adopted from
`docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/`
during the `architecture-adoption-m0-supersession` cycle on 2026-09-09).
Zero ADRs in that range carry `status: accepted`. The absence of a
documented promotion process blocks:

1. **M9 enforcement gap:** the deprecated-patterns lint registry
   (`docs/architecture/lints/deprecated_patterns.toml`) explicitly
   gates four lints (`evidence_kind_v1`, `orchestration_synthesis_no_dissent`,
   `execution_outcome_as_synthesis`, `transition_outcome_used`) on
   the acceptance of ADR-0100/0101/0096. Without a promotion process,
   the lints stay `default: allow` indefinitely.
2. **Knowledge drift:** `docs/architecture/README.md` line 92/93
   claims M8/M9 are "delivered", but the ADRs that encode the
   underlying decisions remain `proposed`. This creates a doc/ledger
   coherence gap: the README's "delivered" claim has no canonical
   governance backing.
3. **Vault divergence precedent:** project-specific ADRs in
   `~/.sddk-knowledge/sddk-framework/adrs/` (e.g. ADR-0079, ADR-0080,
   ADR-0078) already follow an established `proposed → accepted →
   released` pattern with frontmatter `accepted_at` / `accepted_by_cycle`
   / `released_in_milestone` fields and a lifecycle table. The
   consolidation ADRs do not adopt this convention.

## Decision

The repository ADRs follow a **three-state lifecycle**:
`proposed → accepted → released`. Transitions are documented in the
ADR file itself, not in an external ledger.

### 3.1. State semantics

| State | Meaning | Frontmatter marker | Gate to next state |
|---|---|---|---|
| `proposed` | Decision documented but not yet bound; may have implementation evidence, may not. | `status: proposed` | Implementation + evidence + governance approval (per §3.2). |
| `accepted` | Decision bound: implementation evidence exists AND a governance cycle has approved the binding. | `status: accepted`, `accepted_at`, `accepted_by_cycle` | Release to users (per §3.3). |
| `released` | Decision published via a tagged `sddk` release; users can rely on it. | `status: released`, `released_in_milestone`, `released_at` | None (terminal until supersession). |

### 3.2. Promotion criteria: `proposed → accepted`

Three conditions MUST hold, all observed in the ADR file or a referenced
evidence document:

1. **Implementation evidence.** At least one of:
   - Code reference (file path + module + key types) showing the
     decision is in effect in the live workspace, OR
   - A `cycle:` reference to a release that shipped the implementation
     (e.g. `cycle: p-63676b11dc0ef88f/m8-7-cross-input-drift-detection`),
     OR
   - A test name + file path showing the decision is pinned by a
     regression test.
2. **Governance approval.** A cycle has explicitly approved the
   promotion. The cycle id MUST be recorded in `accepted_by_cycle`.
   The cycle's archive manifest MUST reference the ADR id.
3. **Risk-register consideration.** If the ADR supersedes or
   constrains an existing pattern, `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/03-ROADMAP/RISK-REGISTER.md`
   MUST be reviewed and the relevant row updated (either confirmed
   in scope or explicitly waived). If no risk applies, the cycle
   that approves the promotion records "no risk-register impact" in
   its archive manifest.

### 3.3. Release criteria: `accepted → released`

Two conditions:

1. **Release published.** A `bash scripts/release.sh` invocation has
   shipped a binary + bundle that includes the ADR-acceptance change.
   The release's GH Release asset MUST be referenced in
   `released_in_milestone`.
2. **Changelog disclosure.** The version's CHANGELOG.md entry MUST
   name the ADR (or the cycle that promoted it) explicitly. This is
   the user-visible disclosure that the decision is now binding.

### 3.4. Required frontmatter fields

An ADR in `accepted` or `released` state MUST carry these frontmatter
fields (in addition to whatever its source package requires):

- `accepted_at: <ISO-8601 date>`
- `accepted_by_cycle: <cycle id>`
- `released_at: <ISO-8601 date>` (only when state is `released`)
- `released_in_milestone: <milestone id>` (only when state is `released`)

An ADR that does not carry all required fields for its claimed state
is **fail-closed**: the state is invalid and a regression test catches
it (see §3.5).

### 3.5. Regression prevention

A pin test in `crates/sddk-engine/tests/` (or equivalent location) MUST
enforce that, for any ADR file in `docs/architecture/adrs/`:

- If `status: accepted` or `status: released` is set, the
  corresponding `accepted_at` / `accepted_by_cycle` /
  `released_at` / `released_in_milestone` fields MUST be present and
  non-empty.
- The test MUST NOT require a minimum count of accepted ADRs (that is
  the responsibility of the cycle that promotes them; the test only
  validates the format of any that claim `accepted`).

The pin test is added as part of the **first cycle** that uses this
process (i.e. `adr-promotion-m0-meta-and-batch-1`).

### 3.6. Vault mirror

When an ADR in `docs/architecture/adrs/` transitions state, a
**minimal mirror node** MUST also be written in
`~/.sddk-knowledge/sddk-framework/adrs/` with the same id and state,
so vault-side queries (e.g. "how many ADRs are accepted?") reflect the
repo truth. The mirror node MAY be a stub pointing to the canonical
file (per the pattern used by INC-* in `~/.sddk-knowledge/sddk-framework/incs/`).

### 3.7. Supersession

If an accepted/released ADR is later replaced, the new ADR carries
`supersedes: ["<old-id>"]` and the old ADR carries `superseded_by:
"<new-id>"`. The vault mirror reflects both. The state of the
superseded ADR stays `accepted` or `released` (it was true at the
time); only the supersession chain documents the change.

## Consequences

### Positive

- The 4 advisory lints gated on ADR-0100/0101/0096 have a documented
  path to `default: deny`. Each promotion unblocks its corresponding
  lint's per-lint audit.
- Doc/ledger coherence between `docs/architecture/README.md` (which
  claims M0-M9 are "delivered") and `docs/architecture/adrs/`
  (which currently shows all consolidation ADRs as `proposed`) is
  restored as promotions happen.
- Future ADRs (e.g. new decisions from AX-S4 2nd provider arrival,
  or post-M9 follow-ups) have a non-ambiguous lifecycle.

### Negative / costs

- A new pin test must be maintained. The format is bounded (4
  frontmatter fields), so maintenance cost is low.
- Promoting an ADR requires evidence + governance cycle, which is
  more ceremony than just shipping code. This is intentional: the
  ceremony is what makes `status: accepted` trustworthy.
- Vault mirrors add a small overhead per ADR transition. The format
  is stub-friendly (see §3.6).

### Tradeoffs documented

- **Why a three-state lifecycle, not two?** `accepted` and `released`
  are semantically distinct: `accepted` means "the repo binds to this
  decision"; `released` means "users can rely on it". Conflating them
  would let "binding" decisions slip into releases without explicit
  governance. The split is enforced by the pin test.
- **Why a per-ADR lifecycle entry, not a centralized ledger?** The
  vault-side precedent (ADR-0078, ADR-0079, ADR-0080) uses per-file
  lifecycle entries; centralizing would create a second source of
  truth that drifts. Per-file entries are self-contained.
- **Why frontmatter fields, not body sections?** Frontmatter is
  parseable by tools (and the pin test) without markdown rendering.
  Body sections would require a markdown parser.

## Adoption

This meta-ADR is itself **accepted** upon creation (cycle
`adr-promotion-m0-meta-and-batch-1`). Its lifecycle serves as the
example for the first batch of consolidation-ADR promotions
(ADR-0096, ADR-0101 in the same cycle; ADR-0100 deferred per
construction-blocked evidence — see cycle archive).

The first promotion cycle ships a regression test pinning the
frontmatter invariants of §3.4, ensuring every future ADR follows
the convention.
