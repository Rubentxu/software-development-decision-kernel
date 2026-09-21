# Handoff — A4-5P — Intelligence Loop Entry Gate (preflight)

> **Cycle:** `p-63676b11dc0ef88f/a4-5p-intelligence-loop-entry-gate`
> **Status:** PARTIAL — STOP on the BLOCKS_A4-5a path.
> **Date:** 2026-09-17
> **Released baseline inherited:** v1.169.61 / `64c29adb2a9570febd29424e9da912bfb792c630`
> **Development head at cycle start:** `93c8a7404533d95e4d2a389e8bc73abe1b60201c`
> **Final marker commit:** `7a5f91dd157ac05f66244be1d9ff382ce6b9b2b2`
> **Branch state:** `HEAD == origin/main == 7a5f91d`. Working tree clean.
> **No release tag** (preflight cycle — zero code, zero new types).

## What shipped (and what didn't)

A4-5P is a preflight / reconciliation-only cycle. It performed 10 sections
of work and produced a **STOP** outcome at §3: the namespace audit
discovered that the A4-3R reducer's `SingleAuthority ↔ Unit` and
`UniqueOwner ↔ Unit` match arms compare `as_str()` between distinct typed
namespaces (`SoftwareUnitRef` vs `ComponentRef` vs `EntityRef`), which
is an accidental string equivalence, not a typed equality rule.

This is a real bug in A4-3R's typed binding that would have been silently
wired into the Intelligence Loop by A4-5a. The preflight surfaced it
before that wiring happened.

## Sections completed

| § | Subject | Outcome |
|---|---------|---------|
| 1 | ROADMAP-SYNC | `docs/architecture/README.md` updated: A4-3R row flipped to "CLOSED — v1.169.61"; 5-stage sequence A4-5P → A4-5a → A4-5b → A4-5C → A4-CLOSEOUT → A5 substituted for the single A4-5 row |
| 2 | Follow-up reconciliation | `.sddk/followups/a4-followups.md` updated: `FU-A4-3-CONSTRAINT-BINDING` CLOSED; 7 historical FUs classified; new `FU-A4-3R-TARGET-NAMESPACE-BRIDGE` registered |
| 3 | A4-3R target namespace audit | Branch B (no normative identity) confirmed; STOP; FU registered with three remediation options |
| 4 | Randomized probe disposition | `OBSERVED_SESSION_EVIDENCE` recorded; 18 pins + corpus remain the durable suite; property-based regression is a candidate for A5 |
| 5 | arch-spec-047 entry proof | Ordering re-validated; three new forbidden-shortcut pins added to `arch-spec-047`: `Alignment → Authority`, `Alignment → Capability`, `Alignment → InstructionSource` |
| 6 | A4-5a input inventory | Documented: KnowledgeBasis (Projection), ObservationSet (Fact), VerificationResult (Ephemeral), ReconciliationSummary (Ephemeral), LensContribution[] (Ephemeral), AlignmentAssessment (Projection) |
| 7 | A4-5a boundary | "Composition only" pinned: no AdvisoryContext, no WHY, no Governance, no providers, no CogniCode/Chronos/JCode, no new types beyond `IntelligenceLoopResult` + `IntelligenceLoopReceipt` |
| 8 | No overall status pins | `VerificationStatus != ReconciliationSummary != AlignmentState` pinned; forbidden: `OverallQuality`, `OverallHealth`, `OverallStatus`, `score`, `confidence` |
| 9 | Cycle ledger discipline | A4-era pattern (scope-contract + release as authority) confirmed; no retroactive ledger entries; not using `sddk cycle` for A4-5P (no transition state) |
| 10 | Exit gate | PARTIAL — STOP on the BLOCKS_A4-5a path |

## The discovery: namespace-bridge false positive

The reducer's match arms at
`crates/sddk-engine/src/software_alignment/reducer.rs:134-137`:

```rust
(BindingTarget::SingleAuthority(c), ObservationSubject::Unit(u)) => {
    u.as_str() == c.as_str()
}
(BindingTarget::UniqueOwner(e), ObservationSubject::Unit(u)) => u.as_str() == e.as_str(),
```

These compare `as_str()` between `ComponentRef(c)` (or `EntityRef(e)`) and
`SoftwareUnitRef(u)`. But the model declares these as **three distinct
namespaces**:

- `SoftwareEntityRef::canonical_tag()` (`crates/sddk-engine/src/observation/types.rs:30-37`):
  explicit comment "Namespaced so `unit:x` can never collide with `component:x`".
- `arch-spec-042` line 125: "`SoftwareEntityRef` reuses `SoftwareUnitRef`,
  `ComponentRef`, `EntityRef` — no parallel refs."

So `ComponentRef("auth")` and `SoftwareUnitRef("auth")` are
**different subjects** by architectural model, even though their inner
strings coincide. The current reducer treats them as the same subject,
producing a measurable false positive: an observation about a `Unit`
named "auth" will produce a `ContradictsMust` violation against a
`SingleAuthority(ComponentRef("auth"))` contract, even though they
describe different things.

This is exactly the kind of false positive that the typed binding
chain was meant to prevent. The substring-match was eliminated, but a
different shape of false positive (namespace-mismatch) was introduced.

Three remediation options documented at
`docs/debt/FU-A4-3R-TARGET-NAMESPACE-BRIDGE.md`:

- **A. Centralise canonical adapter.** Declare (or refuse) the bridge via
  an explicit adapter method on `ComponentRef`/`EntityRef`. Cheapest if
  the bridge is intended.
- **B. Restrict `ObservationSubject::Unit` binding.** Add a new variant
  `ObservationSubject::ComponentSubject(ComponentRef)` and update the
  match arms to require it. Strongest typed separation.
- **C. Declare canonical equivalence.** Acknowledge at the architectural
  layer that `ComponentRef("x")` and `SoftwareUnitRef("x")` represent the
  same subject. Add a falsification pin asserting the equivalence.

**A4-5a will pick one of A/B/C** (or escalate to A4-5b if option choice
requires architectural debate beyond the composition cycle).

## Commit map (this cycle)

| SHA | Subject | File scope |
|---|---|---|
| `7a5f91d` | `chore(release): bump version` (folded per INC-M7-9 ceremonial marker; original subject was the preflight commit) | 4 files: `docs/architecture/README.md`, `docs/architecture/specs/arch-spec-047-a4-intelligence-loop.md`, `docs/debt/FU-A4-3R-TARGET-NAMESPACE-BRIDGE.md` (NEW), `.sddk/followups/a4-followups.md` |

## Files of interest (this cycle)

- `.sddk/cycles/p-63676b11dc0ef88f-a4-5p-intelligence-loop-entry-gate/spec.md` — preflight scope contract (stashed to `~/.sddk-knowledge/sddk-framework/cycles/...` per `.gitignore` on `.sddk/cycles/`)
- `.sddk/cycles/p-63676b11dc0ef88f-a4-5p-intelligence-loop-entry-gate/a4-5a-input-inventory-and-boundary-pins.md` — input inventory + boundary pins for A4-5a
- `docs/debt/FU-A4-3R-TARGET-NAMESPACE-BRIDGE.md` — new P1 follow-up with falsification pin + 3 remediation options
- `docs/architecture/README.md` — A4-3R row flipped to CLOSED + 5-stage A4-5 sequence
- `docs/architecture/specs/arch-spec-047-a4-intelligence-loop.md` — three new forbidden-shortcut pins added
- `.sddk/followups/a4-followups.md` — `FU-A4-3-CONSTRAINT-BINDING` CLOSED + 7 historical FUs classified + new FU registered
- `docs/handoff/HANDOFF-2026-09-17-a4-5p-intelligence-loop-entry-gate.md` — this file

## Test status

- `cargo test --workspace --offline` — last known state from A4-3R: 4551/4551 pass (no code changes in A4-5P, so unchanged)
- `cargo clippy --workspace --all-targets -- -D warnings` — last known clean (no code changes in A4-5P)
- `sddk dev doctor` — binary `1.169.61`, bundle `1.169.61`, all_present

A4-5P did not run the test gates (no code changes; per the project's
change-scoped-testing discipline, doc-only cycles skip the workspace
test run).

## Phase-discipline honesty (orchestrator disclosures)

- **Phase 4 (apply) skipped.** A4-5P is a preflight cycle; the change
  budget is ZERO. There is no code to apply.
- **Phase 5 (verify) skipped.** No code to verify.
- **Phase 6 (debt-verify) skipped.** No code to debt-verify.
- **Phase 7 (release) skipped.** No release tag; preflight cycle does
  not produce a release.

## Resume point for the next session

CWD: `/var/mnt/DiscoChino2-fast/Proyectos/agentesIA/sddk-framework`
Branch: `main` at `7a5f91d`
Working tree: clean
Binary: `sddk 1.169.61` installed at `/home/rubentxu/.local/bin/sddk`
Framework bundle: `~/.local/share/sddk/framework/1.169.61/`
Last release: `v1.169.61` → `64c29ad`
Latest marker commit: `7a5f91d`

Next cycle candidates (user decides):

- **A4-5a** (Intelligence Loop composition) — blocked by
  `FU-A4-3R-TARGET-NAMESPACE-BRIDGE` (P1). Pick option A/B/C from the
  FU document, implement the chosen remediation, then open A4-5a.
- **Remediation cycle (standalone)** — fix the namespace bridge as its
  own cycle before opening A4-5a. Lower risk, but adds a cycle.

The discovery at A4-5P §3 is the cycle's durable value. A4-3R's typed
binding is correct for `ForbiddenDependency { from, to } ↔ Relation`
(typed equality via `SoftwareEntityRef::Component(from.clone())`), but
the `SingleAuthority ↔ Unit` and `UniqueOwner ↔ Unit` arms slipped a
hidden string equivalence into the typed chain. Fixing this is the
single most important thing before A4-5a starts.
