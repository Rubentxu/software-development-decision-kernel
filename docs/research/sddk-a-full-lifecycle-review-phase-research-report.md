# SDDK A-Full Lifecycle — Review Phase Discrepancy Research

**Date:** 2026-08-30
**Cycle:** `p-52b95ef55999f9de/roadmap-priority`
**Scope:** Read-only research, evidence triangulation, R0–R6
**Status of cycle:** INCONCLUSIVE / blocked at `phase=verify` (see §1.4)

---

## Executive Summary

The runtime (`workflow/workflow.yaml`, `crates/sddk-domain/src/cycle.rs`,
`crates/sddk-cli/tests/cli.rs::walk_a_full_cycle_to_release_pending`)
declares a canonical `phase: review` between `verify` and `release` for
A-full paths. The prompt layer (`prompts/sddk/mcw.md`,
`prompts/sddk/workflows/sddk-a-full.yaml`,
`prompts/sddk/arsenal.md`, `prompts/sddk/phase-contracts.md`,
`prompts/sddk/phases/*.md`) NEVER executes a `review` phase — its
canonical ordering is `tasks → apply → verify → debt-verify → release →
archive`, with no `sddk-review` agent and no `prompts/sddk/phases/review.md`.

The runtime `Phase::Review` is therefore an **orphan phase**: present in
the canonical phase list, gated by `review-report` + `review-approved`
transitions, exercised by the test fixture, but with no executor in the
agent/prompt layer. The current cycle `roadmap-priority` is held at
`phase=verify` because the verify → review transition requires debt
receipts (`debt-severity-assigned`, `debt-priority-assigned`) that the
prompt layer has not yet produced as gate evaluations.

**Recommendation: Option 1 (Remove runtime review phase).** Aligns the
runtime with the MCW, eliminates the orphan, unblocks the
`roadmap-priority` cycle, and prepares the substrate for the dynamic
workflow engine (Phase 4) without adding a 5th verification layer.

---

## R0 — System Definition (Meadows)

| Element | Definition |
|---|---|
| **Subject** | The `phase: review` slice of the SDDK A-full lifecycle |
| **Boundary** | Runtime (workflow.yaml, cycle.rs, tests) ↔ Prompt layer (mcw.md, phases/*.md, arsenal.md) ↔ Cycle state (XDG) ↔ Roadmap priorities (vault REQ + ADR-041/042) |
| **Goal** | Decide whether to (a) keep `review` as runtime phase, (b) inline it as a verify/debt-verify substep, or (c) formalize it with a real executor |
| **Lenses** | Donella Meadows (paradigms → rules → structure → parameters); system archetypes (Policy Resistance, Shifting the Burden) |
| **Anti-patterns to avoid** | "collecting data without a lens" (R0 skip); "Shifting the Burden to L3 sources"; "Seeking the Wrong Goal" (more phases ≠ better verification) |

---

## R1 — Research Agenda

| Q | Source to consult | Result |
|---|---|---|
| Q1. Does the runtime have `phase: review`? | `workflow/workflow.yaml`, `crates/sddk-domain/src/cycle.rs` | YES (see §1.1) |
| Q2. Is there an executor for review? | `prompts/sddk/phases/`, `prompts/sddk/arsenal.md`, `agents/` | NO (see §1.2) |
| Q3. Does MCW prescribe review? | `prompts/sddk/mcw.md`, `prompts/sddk/workflows/sddk-a-full.yaml` | NO (see §1.3) |
| Q4. What is current cycle's runtime state? | XDG cycle artifacts + verify-report.md | INCONCLUSIVE / blocked at `phase=verify` (NOT `OPEN/review`) |
| Q5. What is the roadmap priority order? | `REQ-Roadmap-Priority-Line.md` (vault) | Phase 4 → Phase C tail → Secretary (gated on SPEC-028) |
| Q6. Relationship to verify / debt-verify? | `phases/verify.md`, `phases/debt-verify.md`, `phase-contracts.md` | verify = functional review; debt-verify = technical-debt review; coherence = cross-phase handoff; ALL THREE are NOT runtime phases per their own contract |

---

## R2 — Discovered Sources

### Runtime layer

- **`workflow/workflow.yaml:7-29`** — canonical statuses + phases
  - Statuses include `UAT_WAITING`, `APPROVAL_PENDING`, etc.
  - Phases (10): `explore, specify, design, plan, build, verify, uat, review, release, archive`
- **`workflow/workflow.yaml:31-69`** — per-path phase lists
  - **A-full phases:** `explore → specify → design → plan → build → verify → review → release`  (review IS in A-full)
  - **A-min phases:** `explore → specify → build → verify → release`  (no review)
  - **A-lite phases:** `explore → specify → design → build → verify → release`  (no review)
  - **B-direct phases:** `build → verify → release`  (no review)
- **`workflow/workflow.yaml:255-279`** — `phase.verify.complete` for A-full
  - `from: OPEN/verify` → `to: OPEN/review`
  - requires: `verification-report` + `tests-pass` + `policy-compliant` + **`debt-severity-assigned`** + **`debt-priority-assigned`**
  - `paths: A-full`
- **`workflow/workflow.yaml:281-329`** — `phase.verify.complete.a-min` / `.a-lite`
  - `from: OPEN/verify` → `to: RELEASE_PENDING/release` (skip review)
  - same required gates
- **`workflow/workflow.yaml:410-425`** — `phase.review.complete`
  - `from: OPEN/review` → `to: RELEASE_PENDING/release`
  - requires: `review-report` artifact + `review-approved` gate
- **`workflow/workflow.yaml:543-578`** — `phase.review.approval.{requested,resolved}`
  - OPEN/review ↔ APPROVAL_PENDING/review

### Domain layer

- **`crates/sddk-domain/src/cycle.rs:57-94`** — `pub enum Phase` has
  `Phase::Explore, ::Specify, ::Design, ::Plan, ::Build, ::Verify, ::Uat,
  ::Review, ::Release, ::Archive`
- **`crates/sddk-domain/tests/workflow_yaml.rs:78, 130, 195`** — tests
  assert `Phase::Review` is present, `phase.review.complete` is
  registered, and `review-approved` gate exists

### CLI / test layer

- **`crates/sddk-cli/tests/cli.rs:4295-4544`** —
  `walk_a_full_cycle_to_release_pending` walks A-full through
  `explore → specify → design → plan → build → verify → review → RELEASE_PENDING`
  using `phase.review.complete` + `review-approved` + `review-report`

### Prompt layer

- **`prompts/sddk/mcw.md:86, 128, 156, 211`** — canonical A-full
  ordering is `tasks → apply → verify → debt-verify → release → archive`.
  **No review phase** in any path. **"review-budget"** at step 1.7 is an
  *advisory* sizing check (ADR-0070), NOT the runtime `phase: review`.
- **`prompts/sddk/workflows/sddk-a-full.yaml:144-149`** —
  `phase: review-budget` step 1.7 is `advisory: true`, "no BLOCK on forecast".
- **`prompts/sddk/phases/`** — directory listing (15 files): apply,
  apply-strict-tdd, archive, coherence, debt-verify, design, explore,
  init, propose, release, spec, tasks, strict-tdd-verify, verify.
  **NO `review.md`.**
- **`prompts/sddk/arsenal.md:43-62`** — model assignments list:
  `sddk-init`, `sddk-explore`, `sddk-propose`, `sddk-spec`,
  `sddk-design`, `sddk-tasks`, `sddk-apply`, `sddk-verify` (lens + synthesis),
  `sddk-debt-verify` (phase), `debt-*-cluster`, `sddk-archive`.
  **NO `sddk-review`.**
- **`prompts/sddk/phase-contracts.md:30-55`** — phase responsibilities
  table lists `sddk-tasks, sddk-apply, sddk-verify, sddk-debt-verify,
  sddk-release, sddk-archive`. **NO `sddk-review` row.**
- **`prompts/sddk/phase-contracts.md:170-186`** — debt-verify section:
  contract authority, no dedicated runtime phase or CLI transition.
- **`prompts/sddk/phases/verify.md:7`** —
  > "Do not run `sddk-debt-verify`: that later phase audits broader technical debt."
- **`prompts/sddk/phases/debt-verify.md:7-9`** —
  > "Debt-verify is a workflow **capability/gate** between functional verify and release. It is not a new value in the legacy runtime `Phase` enum. Runtime and CLI enforcement are intentionally outside this specification change."
- **`prompts/sddk/phases/debt-verify.md:51`** (Policy trade-offs row 6):
  > "Specification-only runtime handoff — Avoids claiming CLI enforcement that does not exist. The declarative gate can be bypassed by current runtime integrations. Track typed runtime enforcement as deferred roadmap work."

### Roadmap & ADRs

- **`docs/sddk-decision-kernel-architecture/CHANGESET-2026-08-19-DYNAMIC-WORKFLOWS.md`**
  - "A-full is retained as reference/baseline. No previous documents are deleted by this change."
  - "`Phase/CyclePath` migration is still gradual."
  - Pipeline: `Template → Compiler → IR → Validator → Runtime → Dynamic Graph Revisions`
  - SDD result: `SHAPE → BUILD ⇄ CONVERGE → INTEGRATE`
- **`docs/sddk-decision-kernel-architecture/03-adrs/ADR-041-WORKFLOW-RUNTIME-V2.md`**
  - Accepted. WorkflowRuntime: Pending → Running → Completed (Failed).
  - Operator trait, ports, event emission. Cycle-17 will add async + capability dispatch.
- **`docs/sddk-decision-kernel-architecture/03-adrs/ADR-042-TEST-TOOLING-BOUNDARY.md`**
  - Accepted (user 2026-08-28). **Phase A** (current cycle): audit + annotation only.
  - **Phase B** (next cycle): shellcheck + Python linter + ownership prefix.
  - **Phase C** (deferred, "Do NOT execute Phase C in this cycle or the next"): cleanup after parity evidence.
- **`docs/sddk-decision-kernel-architecture/04-specs/SPEC-042-secretary-runtime.md`**
  - Secretary Stage 1+ is HARD-GATED on `SPEC-028-promoted` (SPEC-028 must reach `Accepted`/`Implemented`/`Superseded`).
- **`/var/home/rubentxu/.sddk-knowledge/sddk-framework/specs/roadmap-priority/REQ-Roadmap-Priority-Line.md`**
  - Priority 1: **Phase 4 — Dynamic workflow engine** (`kernel-cycle-N-m4-dynamic-workflow-engine`).
  - Priority 2: **Phase C tail — Test-tooling boundary residual**.
  - Priority 3: **Secretary Stage 1+** (HARD GATE on SPEC-028 promoted).

### Current cycle state (XDG)

- **`/home/rubentxu/.local/share/sddk/projects/p-52b95ef55999f9de/cycle-artifacts/p-52b95ef55999f9de/roadmap-priority/verify-report.md`**
  - Cycle is docs/vault-only zero-intrusion cycle (`base == head == 7e64b6b78a08`, empty diff).
  - **Runtime verdict:** `INCONCLUSIVE` / `status: blocked` at `phase=verify`.
  - **Product verdict:** PASS for the cycle's contribution.
  - Line 40: "The runtime transition `phase.verify.complete` is **unavailable** in the current baseline for A-* paths because it requires debt receipts (`debt-severity-assigned` and `debt-priority-assigned`) that can only be produced by the later `sddk-debt-verify` capability."
  - Line 167: "Cycle is held at phase=verify pending `sddk-debt-verify` receipt convergence."
- **`/home/rubentxu/.local/share/sddk/projects/p-52b95ef55999f9de/cycle-artifacts/p-52b95ef55999f9de/roadmap-priority/debt-report.md`**
  - Verdict: **PASS**, 0 findings across all 5 deep clusters.
  - `runtime_handoff: specification_only`; `desired_gate: debt-approved`.
  - `subject_sha == base_commit == head_commit`; verify evidence SHA matches.

---

## R3 — Credibility Assessment

| Source | Level | Authority | Notes |
|---|---|---|---|
| `workflow/workflow.yaml` | L1 (canonical) | First-party | Generated by `fc93b3e feat(runtime): add deterministic workflow foundation` (2026-08-04) |
| `crates/sddk-domain/src/cycle.rs` | L1 (canonical) | First-party | Phase enum is Rust source-of-truth |
| `crates/sddk-cli/tests/cli.rs::walk_a_full_cycle_to_release_pending` | L1 (test contract) | First-party | Test fixture explicitly walks review phase |
| `prompts/sddk/mcw.md` | L1 (canonical prose) | First-party | Step index §Quick Reference has 17 steps, NO review |
| `prompts/sddk/workflows/sddk-a-full.yaml` | L1 (canonical workflow) | First-party | 9 patterns, no review executor |
| `prompts/sddk/phases/{verify,debt-verify}.md` | L1 (canonical contract) | First-party | Each phase declares its own boundary |
| `prompts/sddk/phase-contracts.md` | L1 (cross-phase authority) | First-party | Line 51-54: debt-verify explicitly NOT a runtime phase |
| `prompts/sddk/arsenal.md` | L1 (agent registry) | First-party | Model assignment table has no sddk-review |
| `docs/sddk-decision-kernel-architecture/03-adrs/ADR-041/042` | L1 (accepted ADR) | First-party | Roadmap direction; ADR-042 binds migration phases |
| `REQ-Roadmap-Priority-Line.md` (vault) | L1 (requirement node) | First-party | Single source of truth for priority ordering |
| `cycle-artifacts/.../verify-report.md` | L1 (cycle audit) | First-party | Machine evidence + human interpretation |
| `cycle-artifacts/.../debt-report.md` | L1 (cycle audit) | First-party | `sha256:0701e919…` machine authority |
| `crates/sddk-domain/tests/workflow_yaml.rs` | L2 (test assertion) | First-party | Asserts runtime Phase::Review exists |
| `docs/sddk-decision-kernel-architecture/CHANGESET-2026-08-19-DYNAMIC-WORKFLOWS.md` | L1 (changeset) | First-party | Binds dynamic-workflow migration |

---

## R4 — Triangulation

### Claim 1 — "Verify is functional review, debt-verify is technical-debt review"

**CONFIRMED.** Triple-source:

1. `prompts/sddk/phases/verify.md:7` — "Do not run `sddk-debt-verify`: that later phase audits broader technical debt."
2. `prompts/sddk/phases/debt-verify.md:7-9` — "Debt-verify is a workflow capability/gate between functional verify and release."
3. `prompts/sddk/phase-contracts.md:170-175` — Debt-verify = "mandatory A-* handoff between passing functional verification and release".

### Claim 2 — "workflow/workflow.yaml:410-425 has `phase.review.complete` requiring `review-report` + `review-approved`"

**CONFIRMED.** Lines 410-425 declare the transition; lines 543-578 add
`phase.review.approval.{requested,resolved}` for A-full; lines 695 and
759 declare the artifact and gate definitions; the transition was added
in commit **`fc93b3e feat(runtime): add deterministic workflow
foundation`** (2026-08-04).

### Claim 3 — "mcw.md and sddk-a-full.yaml prescribe `tasks → apply → verify → debt-verify → release → archive`"

**CONFIRMED.** Four independent declarations agree:
  - `mcw.md:86` — A-full row in Phase 1 §Plan table.
  - `mcw.md:128` — "The A-full pipeline ordering is mandatory: `tasks → apply → verify → debt-verify → release → archive`"
  - `mcw.md:156-211` — Phase 2 §Build: apply, verify, debt-verify.
  - `sddk-a-full.yaml:14` — "Includes all phases: explore → propose → spec||design → tasks → apply → verify → **debt-verify (deep, mandatory)** → **release (mandatory)** → archive."
  - `mcw.md` Quick Reference index (lines 376-403): 17 step rows, NO review row.
  - `sddk-a-full.yaml` phase list (lines 44-291): no `phase: review` row.

### Claim 4 — "No `prompts/sddk/phases/review.md` or registered `sddk-review` agent"

**CONFIRMED.** Directory `prompts/sddk/phases/` has 14 files; none is
`review.md`. Repository-wide grep for `sddk-review` returns zero hits.
`prompts/sddk/arsenal.md` model-assignment table has 11 phase-agent
rows; none is `sddk-review`. `prompts/sddk/phase-contracts.md`
phase-responsibility table has 6 rows; none is `sddk-review`.

### Claim 5 — "Current `p-52b95ef55999f9de/roadmap-priority` is OPEN/review"

**PARTIALLY INACCURATE — refinement needed.** Per the cycle's
`verify-report.md`:

- Runtime verdict: `INCONCLUSIVE` / `status: blocked` at `phase=verify`
  (NOT `OPEN/review`).
- The cycle never reached `OPEN/review` because `phase.verify.complete`
  (A-full variant) requires `debt-severity-assigned` and
  `debt-priority-assigned` receipts which the prompt contract returns
  as `blocked`/`INCONCLUSIVE` after persisting gate receipts **without
  invoking the transition**.
- `debt-report.md` produced `runtime_handoff: specification_only` and
  `desired_gate: debt-approved`; the corresponding
  `sddk cycle evaluate-gate --gate debt-severity-assigned` and
  `--gate debt-priority-assigned` were never invoked.
- Product verdict is **PASS** — only the runtime handoff is blocked.

**The gap the user describes is real, but the current cycle is stuck one
transition earlier than the user stated.** The `phase: review` orphan is
still the structural defect; if the current cycle were unblocked, it
would advance to `OPEN/review` and deadlock there because no
`review-report` producer and no `review-approved` evaluator exist.

### Claim 6 — Debt-verify is NOT a runtime phase

**CONFIRMED.** Two contract statements:

- `prompts/sddk/phases/debt-verify.md:7-9` — "It is not a new value in the legacy runtime `Phase` enum."
- `prompts/sddk/phase-contracts.md:51-54` — "This declarative contract does not claim a dedicated runtime phase or CLI transition."

**But `Phase::Review` IS in the runtime enum.** This is the core
inconsistency: review has a runtime phase without an executor; debt-verify
has an executor without a runtime phase. The two contracts diverge.

---

## R5 — Consolidated Corpus (System Map)

### As-is (current) — orphan-phase diagram

```text
workflow.yaml phase list    arsenal.md agent registry    phases/ directory
───────────────────────     ─────────────────────────    ──────────────────
explore                     sddk-explore                 explore.md
specify                     sddk-spec                    (inline in MCW)
design                      sddk-design                  design.md
plan                        sddk-tasks                   tasks.md
build                       sddk-apply                   apply.md
verify                      sddk-verify                  verify.md
uat                         (UAT-2026-08 milestone)      (inline in MCW)
review                      ──────────                   ──────────
                            [ORPHAN: no agent]           [ORPHAN: no prompt]
release                     sddk-release                 release.md
archive                     sddk-archive                 archive.md
[NOT in phase list]         sddk-debt-verify             debt-verify.md
                            debt-*-cluster (5)           (spec-only contract)
```

### A-full canonical ordering — two views

**Runtime (`workflow.yaml` A-full row + `phase.verify.complete` A-full
variant):**
```
explore → specify → design → plan → build → verify → review → release
```
Required between verify and review: tests-pass, policy-compliant,
**debt-severity-assigned**, **debt-priority-assigned**.
Required between review and release: **review-report** artifact +
**review-approved** gate.

**Prompt layer (`mcw.md` + `sddk-a-full.yaml` + `phase-contracts.md`):**
```
explore → propose → spec+design(parallel) → coherence → tasks
       → apply → verify → debt-verify → coherence → release → archive
```
**No review row.** Coherence is explicitly NOT a runtime phase
(`phase-contracts.md:130-134`).

### Decisions that bind the resolution

| Decision | Authority | Statement |
|---|---|---|
| `sddk-debt-verify` is **not** a runtime Phase | `phases/debt-verify.md:7-9` + `phase-contracts.md:51-54` | Capability/gate, not a state-machine value |
| `Phase::Review` IS a runtime Phase | `crates/sddk-domain/src/cycle.rs:92` | Orphan |
| A-full is retained as reference/baseline | `CHANGESET-2026-08-19-DYNAMIC-WORKFLOWS.md` Compatibility row | Removal is allowed; dynamic workflow will replace it |
| `Phase/CyclePath` migration is gradual | Same changeset | Removal of orphan is on-track, not breaking |
| Phase 4 (dynamic workflow) is Priority 1 | `REQ-Roadmap-Priority-Line.md` | Substrate must be clean before Phase 4 |
| Phase C tail is deferred | `ADR-042-TEST-TOOLING-BOUNDARY.md` | Cleanup after parity; not this cycle |
| Secretary Stage 1+ is HARD-GATED on SPEC-028 | `SPEC-042-secretary-runtime.md` §Promotion gate | Out of scope here |

---

## R6 — Recommendation

### Three options compared

| Option | What changes | Pros | Cons | Complexity | Aligns with roadmap |
|---|---|---|---|---|---|
| **1. Remove runtime review phase** | Drop `Phase::Review` (cycle.rs), `phase.review.complete` + `phase.review.approval.*` (workflow.yaml), update test fixture, regenerate `docs/generated/workflow.md`. `phase.verify.complete` A-full variant goes `OPEN/verify → RELEASE_PENDING/release` (like A-min/A-lite). | (a) Aligns runtime with MCW. (b) Eliminates orphan. (c) Unblocks `roadmap-priority` cycle. (d) Clean substrate for Phase 4 dynamic workflow. (e) Removes a state-machine value the kernel has to maintain. | (a) Loses the "human review before release" semantic — but release already has `phase.release.approval.*` for mid-phase approval; pre-release gating is already enforced by `release.complete` (merge-receipt + release-receipt + no-pending-effects + release-uat-approved). | **Lowest** | **Best** (Phase 4 wants compact templates) |
| **2. Make review an internal substep of verify/debt-verify** | Add an evaluator-optimizer pattern within `debt-verify` (or between debt-verify and release) that mirrors `coherence` semantics. Remove `Phase::Review` from enum. Document as "release review" inline check. | (a) Follows the `coherence` precedent. (b) Preserves a "human review" gate if wanted. | (a) Adds a NEW pattern not yet formalized. (b) Ambiguous: which substep owns it? (c) Doesn't address the orphan Phase enum value. (d) Risks duplicating `coherence` semantics. | Medium | Neutral |
| **3. Formalize external review** | Create `prompts/sddk/phases/review.md`, register `sddk-review` agent, add to arsenal.md model assignments, wire into phase-contracts.md. | (a) Preserves current behavior 1:1. | (a) Adds a 5th verification layer (functional, debt, coherence, judgment-day, review). (b) Increases agent count. (c) User explicitly wants minimum complexity. (d) Doesn't align with dynamic-workflow direction. (e) Doesn't unblock the current cycle any faster than Option 1. | **Highest** | Worst (contradicts substrate compaction) |

### Recommended: **Option 1 — Remove runtime review phase**

**Rationale:**

1. **System trap avoided (Policy Resistance / Shifting the Burden).**
   Adding a 5th verification layer when 4 already exist (`sddk-verify`
   for functional, `sddk-debt-verify` for technical-debt, `sddk-coherence`
   for cross-phase handoff, judgment-day for adversarial) is the
   classic "more reviews = safer" trap. The user's instinct — verify is
   the functional review, debt-verify is the technical-debt review —
   matches the contract design.

2. **Paradigm alignment.** `prompts/sddk/phases/debt-verify.md:7-9` and
   `phase-contracts.md:51-54` already establish the paradigm: cross-phase
   gates (debt, coherence) are NOT runtime state-machine values. The
   orphan `Phase::Review` is the paradigm exception that proves the
   rule. Removing it returns the system to its own design rule.

3. **Roadmap priority alignment.** Phase 4 (dynamic workflow engine) is
   Priority 1. The runtime substrate must be compact and consistent for
   the WorkflowCompiler to produce clean WorkflowIR (ADR-024). A
   runtime phase with no executor is exactly the friction the compiler
   will have to special-case.

4. **Phase 4 sequencing respected.** `CHANGESET-2026-08-19-DYNAMIC-WORKFLOWS.md`
   Compatibility row: "A-full is retained as reference/baseline… Phase/CyclePath migration is gradual." Removing an orphan phase from the
   runtime is on the gradual path; it does not delete any prompt-layer
   document.

5. **Approval semantic preserved.** Release already has
   `phase.release.approval.{requested,resolved}` (workflow.yaml:580-615)
   for mid-phase human approval, and `release.complete` requires
   `merge-receipt + release-receipt + no-pending-effects +
   release-uat-approved`. The "human review before release" semantic is
   not lost — it is already distributed across those gates.

6. **Minimum complexity.** Option 1 removes code; Options 2 and 3 add
   code. The user explicitly framed the question as "minimize
   complexity".

### Migration plan (Option 1)

#### Phase 0 — Pre-conditions (this cycle, `roadmap-priority`)

- **R0.** Read this report; user approves Option 1.
- **R1.** Add ADR-0074 (or follow numbering convention) —
  *Remove orphan `Phase::Review` runtime value; align runtime with MCW.
  Approval semantic preserved by `phase.release.approval.*`.*
- **R2.** Update `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md`
  to log the ADR under active milestones.

#### Phase 1 — Source changes (next cycle, code change required)

1. **`crates/sddk-domain/src/cycle.rs`** — keep `Phase::Review` for
   backward compatibility OR remove it (decision: keep variant but
   deprecate, follow semver; v1.60+ adopts removal in v2.0).
   - Alternative: remove variant now and require migrations of any
     persisted state containing `Phase::Review`.
2. **`workflow/workflow.yaml`** —
   - Delete `phase.review.complete` (lines 410-425).
   - Delete `phase.review.approval.requested` (lines 543-560).
   - Delete `phase.review.approval.resolved` (lines 562-578).
   - Update `phase.verify.complete` A-full variant (lines 255-279):
     change `to.phase: release` and `to.status: RELEASE_PENDING` (mirror
     A-min/A-lite variants at lines 281-329).
   - Remove `review` from `paths.A-full.phases` (line 61).
   - Remove `review` from canonical `phases:` list (line 27).
   - Delete `review-report` artifact block (line 695).
   - Delete `review-approved` gate block (line 759).
3. **`crates/sddk-domain/tests/workflow_yaml.rs`** — remove or update
   the three assertions on lines 78, 130, 195.
4. **`crates/sddk-cli/tests/cli.rs::walk_a_full_cycle_to_release_pending`**
   — remove the `phase.review.complete` block (lines 4526-4544); the
   helper now walks verify → release directly for A-full.
5. **`docs/generated/workflow.md`** — regenerate via `sddk dev docs` or
   equivalent. The state diagram at lines 147-174 loses the
   `OPEN_review` node and its adjacent transitions.
6. **`prompts/sddk/phase-contracts.md`** — add an explicit note in the
   Git Phase Interleaving table: "No `sddk-review` phase. Pre-release
   human approval is via `phase.release.approval.*`."
7. **`prompts/sddk/arsenal.md`** — confirm absence of `sddk-review`
   model assignment (already absent; no edit needed).
8. **`prompts/sddk/mcw.md`** — confirm Step 1.7 `review-budget`
   (advisory sizing check) is not confused with the removed runtime
   `phase: review`; add a clarifying footnote.
9. **`docs/sddk-decision-kernel-architecture/03-adrs/`** — add ADR
   documenting the removal with rationale, alternatives considered,
   and cross-references to `debt-verify.md` §"not a new value in the
   legacy runtime Phase enum" as the precedent.

#### Phase 2 — Test + verification

- `cargo test --workspace --all-targets --release --locked` — all green.
- `sddk cycle evaluate-gate --gate review-approved` returns
  `gate_unknown` (gate removed).
- `sddk cycle transition --transition phase.review.complete` returns
  `transition_unknown` (transition removed).
- Re-generate `docs/generated/workflow.md`; commit.

#### Phase 3 — Documentation regeneration

- `sddk dev doctor | grep bundle_coherence` — bin and bundle coherent.
- `sddk lint` — clean.
- Update AGENTS.md if needed (no; AGENTS.md doesn't reference review).

### Recovery plan for current cycle `p-52b95ef55999f9de/roadmap-priority`

The cycle's actual state is `INCONCLUSIVE / status=blocked / phase=verify`
— **not** `OPEN/review`. So Option 1 does not change the cycle's
current state; it changes what happens *after* the cycle unblocks.

#### Unblock the cycle (independent of Option 1)

1. Issue the missing gate evaluations that debt-verify has produced:
   ```bash
   sddk cycle evaluate-gate \
     --root . --scope . --cycle p-52b95ef55999f9de/roadmap-priority \
     --transition phase.verify.complete \
     --gate debt-severity-assigned \
     --evaluator sddk.cli --outcome passed \
     --evidence '{"debt-report.json":"sha256:0701e919…","verdict":"PASS"}' \
     --timestamp <ISO> --actor sddk-cli
   ```
2. Same for `--gate debt-priority-assigned`.
3. Re-run `phase.verify.complete` with the four gate receipts.
4. After Option 1 lands: the transition goes directly to
   `RELEASE_PENDING/release` (no review step).
5. `sddk release plan --dry-run`, then `sddk ship <cycle>` (per
   ROADMAP §4 sddk-ship gate at line 831).

#### Recovery if Option 1 is NOT chosen

- Cycle remains stuck at `INCONCLUSIVE / phase=verify` until receipts
  are issued.
- Once issued, cycle advances to `OPEN/review` and deadlocks (no
  `sddk-review` agent can produce `review-report`; no evaluator can
  produce `review-approved`).
- Recovery requires either manual gate injection
  (`sddk cycle evaluate-gate --gate review-approved --outcome passed`)
  or a synthetic transition `cycle.unblock`. Both are debt-incidence
  material.

---

## Risks and Open Questions

| Risk | Mitigation |
|---|---|
| `Phase::Review` is referenced in persisted cycle state somewhere outside `p-52b95ef55999f9de` | Audit via `sddk ledger verify --format json | grep Phase::Review`; if found, add a v1→v2 migration in `sddk-storage/src/migrations.rs` |
| External API consumers depend on `review-report` artifact or `review-approved` gate | Add deprecation note in CHANGELOG; treat as semver-major |
| `crates/sddk-cli/tests/cli.rs::walk_a_full_cycle_to_release_pending` is a public test contract | Update it explicitly; document the change in the migration commit body |
| The user said "OPEN/review" but the cycle is "INCONCLUSIVE / phase=verify" — minor framing difference | Confirmed via `verify-report.md` §1.4; report states the more accurate status |
| UAT-2026-08 milestone added UAT phase; if UAT is activated, `phase.uat.complete` goes to `OPEN/review`. After Option 1, where does it go? | Map `phase.uat.complete` to `RELEASE_PENDING/release` directly (mirror `phase.verify.complete` A-min/A-lite variant). This is consistent because UAT's role is to produce `uat-report` which is required by `release.complete` (line 442) |
| `release-uat-approved` gate (line 442) — needs UAT? | Per workflow.yaml, `release-uat-approved` is only required when UAT is activated (per `uat.toml [release_gate]` configuration). No change needed. |

---

## Disputed claims, gaps, decay warnings

- **Disputed claim** (1) — User's "OPEN/review" framing vs. actual
  "INCONCLUSIVE / phase=verify" state. Substantive gap is one
  transition upstream; structural gap (orphan `Phase::Review`) is
  real.
- **Gap** — No ADR currently exists for the `Phase::Review` decision
  (positive or negative). Recommend ADR-0074 (or follow numbering).
- **Decay warning** — `prompts/sddk/phases/debt-verify.md` `stale_after`
  not declared; `SPEC-042-secretary-runtime.md` `stale_after =
  2026-11-28`; `REQ-Roadmap-Priority-Line.md` `stale_after =
  2026-11-28`. Re-verify before next re-anchor.

---

## Cross-references

- Runtime: `workflow/workflow.yaml`, `crates/sddk-domain/src/cycle.rs`,
  `crates/sddk-cli/tests/cli.rs`
- Prompt layer: `prompts/sddk/mcw.md`,
  `prompts/sddk/workflows/sddk-a-full.yaml`,
  `prompts/sddk/phases/{verify,debt-verify}.md`,
  `prompts/sddk/phase-contracts.md`, `prompts/sddk/arsenal.md`
- Roadmap: `REQ-Roadmap-Priority-Line.md` (vault),
  `docs/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md`,
  `docs/sddk-decision-kernel-architecture/03-adrs/ADR-041-WORKFLOW-RUNTIME-V2.md`,
  `docs/sddk-decision-kernel-architecture/03-adrs/ADR-042-TEST-TOOLING-BOUNDARY.md`,
  `docs/sddk-decision-kernel-architecture/04-specs/SPEC-042-secretary-runtime.md`,
  `docs/sddk-decision-kernel-architecture/CHANGESET-2026-08-19-DYNAMIC-WORKFLOWS.md`
- Cycle state:
  `/home/rubentxu/.local/share/sddk/projects/p-52b95ef55999f9de/cycle-artifacts/p-52b95ef55999f9de/roadmap-priority/{verify-report,debt-report}.md`