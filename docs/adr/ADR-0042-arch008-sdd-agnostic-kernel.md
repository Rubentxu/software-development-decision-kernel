# ADR-0042 — ARCH008 SDD-Agnostic Kernel Runtime + WV-0026 Waiver

**Status:** accepted
**Date:** 2026-08-19
**Trigger:** kernel-workflow-ir-contracts (v1.29.0) — enforcing clean kernel boundaries

---

## Context

ARCH008 requires that workflow runtime modules (`workflow_ir.rs`, `workflow_run.rs`) must not depend on the SDD phase taxonomy. Coupling the kernel to `Phase::`, `CyclePath::`, or phase-qualified SDD names creates a hard dependency between the kernel and the SDD owner — violating the architecture's clean separation principle.

However, the legacy `workflow.rs` and `event_bus.rs` intentionally emit `workflow.phase.entered/exited` events and reference `Phase`/`CyclePath` enums. Removing these references immediately would break the existing cycle CLI workflow.

---

## Decision

**Evaluator:** `evaluate_arch008()` uses a 4-pattern `RegexSet` over scoped `.rs` files:

```rust
static ARCH008_PATTERNS: Lazy<RegexSet> = Lazy::new(|| RegexSet::new([
    r"\bPhase::",                                                  // type-qualified
    r"\bCyclePath::",                                              // type-qualified
    r"\b(Explore|Specify|Design|Tasks|Apply|Verify|Archive)\s*::", // variant-qualified
    r"match\s+phase\s*\{",                                         // match-on-string
]).expect("static regex"));
```

**Scope:** `**/sddk-domain/src/workflow_ir.rs`, `**/sddk-domain/src/workflow_run.rs`, `**/sddk-engine/src/lib.rs`

**Waiver WV-0026:** Legacy `workflow.rs` and `event_bus.rs:96-117` are exempt under waiver `WV-0026-ARCH008-legacy-compat-seam` for v1.29.0 and v1.30.0. Waiver expires at v1.31.0 when the `kernel-dynamic-operators` cycle removes the legacy phase-emitting code.

---

## Consequences

- **Positive:** New IR modules (`workflow_ir.rs`, `workflow_run.rs`) are provably SDD-agnostic
- **Positive:** Incremental enforcement — no giant refactor required now
- **Positive:** `WV-0026` creates a visible cleanup contract with an expiry date
- **Negative:** Kernel violations become hard errors after v1.31.0 — the waiver is not renewable
- **Negative:** Regex-based evaluation may have false negatives (missing patterns) — mitigated by heuristic nature and unit test coverage

---

## References

- `evaluate_arch008()` in `crates/sddk-engine/src/rules/evaluators.rs`
- `ARCH008` rule definition in `docs/sddk-2.0-architecture-consolidation/data/architecture-rules.yaml`
- `WV-0026` waiver in same file
- ARCH006–015 evaluators stubbed as `NotApplicable` (substance deferred to cycle 3)
