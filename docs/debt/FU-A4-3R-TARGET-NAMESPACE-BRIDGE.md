# FU-A4-3R-TARGET-NAMESPACE-BRIDGE

> **Status:** OPEN (registered by A4-5P preflight on 2026-09-17)
> **Severity:** P1
> **Origin:** A4-5P §3 namespace audit (cycle `p-63676b11dc0ef88f/a4-5p-intelligence-loop-entry-gate`)
> **Blocks:** A4-5a (Intelligence Loop composition)
> **Disposes:** nothing — this is a new follow-up

## 1. Problem statement

`crates/sddk-engine/src/software_alignment/reducer.rs:134-137`:

```rust
(BindingTarget::SingleAuthority(c), ObservationSubject::Unit(u)) => {
    u.as_str() == c.as_str()
}
(BindingTarget::UniqueOwner(e), ObservationSubject::Unit(u)) => u.as_str() == e.as_str(),
```

The reducer compares the inner string of `ComponentRef(c)` (or `EntityRef(e)`)
against the inner string of `SoftwareUnitRef(u)` using `as_str()`. This is
an **accidental string equivalence** between two typed namespaces, not a
typed equality rule.

## 2. Normative model evidence

`crates/sddk-engine/src/observation/types.rs:14-37`:

```rust
/// three already name the things a relation can join.
pub enum SoftwareEntityRef {
    Unit(SoftwareUnitRef),
    Component(ComponentRef),
    Entity(EntityRef),
}

impl SoftwareEntityRef {
    /// Canonical tag used in identity derivation. Namespaced so `unit:x` can never
    /// collide with `component:x`.
    pub fn canonical_tag(&self) -> String {
        match self {
            SoftwareEntityRef::Unit(u) => format!("unit:{}", u.as_str()),
            SoftwareEntityRef::Component(c) => format!("component:{}", c.as_str()),
            SoftwareEntityRef::Entity(e) => format!("entity:{}", e.as_str()),
        }
    }
}
```

The `canonical_tag` is explicitly **namespaced** ("Namespaced so `unit:x`
can never collide with `component:x`"). `SoftwareUnitRef("auth")`,
`ComponentRef("auth")`, and `EntityRef("auth")` represent three distinct
subjects by architectural model.

`docs/architecture/specs/arch-spec-042-evidence-observation-provenance.md:125`:

> `SoftwareEntityRef` reuses `SoftwareUnitRef`, `ComponentRef`, `EntityRef` — no
> parallel refs.

The model reuses these three typed refs because they are distinct namespaces
that happen to share a string body. The reuse is structural (a tagged enum),
not a string-equivalence collapse.

## 3. Contradiction in the current reducer

The reducer's match arm `SingleAuthority(c) ↔ Unit(u)` ignores the namespace
distinction. Given:

- a contract `SingleAuthority(ComponentRef("auth"))` declared by an
  architectural decision, and
- an observation whose subject is `ObservationSubject::Unit(SoftwareUnitRef("auth"))`,

the reducer will produce a `ContradictsMust` violation — even though
"auth" in the contract namespace means an architectural component, while
"auth" in the observation namespace means a software unit. They are
different subjects.

By the same argument, an observation about `SoftwareUnitRef("auth")` would
NOT bind a `SingleAuthority(ComponentRef("auth"))` if the bridge is
declared wrong — but currently it DOES bind, incorrectly.

## 4. False negative vs false positive

| Bridge state | False negative risk | False positive risk |
|---|---|---|
| Current (string equiv) | low | **HIGH** — observation about `Unit("auth")` produces a `ContractViolation` against `SingleAuthority(ComponentRef("auth"))` even though they are different subjects |
| Bridge declared (option A) | low if adapter is correct | low |
| Bridge declared (option B) | **HIGH** — observation about `Unit("auth")` does NOT bind a `Component("auth")` contract; a real subject can be silently lost | low |

The current state has a measurable **false positive** in the
`SingleAuthority ↔ Unit` arm (and `UniqueOwner ↔ Unit`).

## 5. Falsification case

```rust
use sddk_engine::architectural_contract::{
    ArchitecturalContract, ComponentRef, ContractId, DecisionRef, Revision, SpecRef,
};
use sddk_engine::knowledge::{EventTime, KnowledgeBasis};
use sddk_engine::observation::{
    ObservationBasis, ObservationOrigin, ObservationSet, ObservationStance, ObservationSubject,
    SoftwareObservation,
};
use sddk_engine::architecture_graph::SoftwareUnitRef;
use sddk_engine::evidence_ref::{EvidenceKind, EvidenceRef as UniEvidenceRef};
use sddk_engine::software_alignment::reducer::reduce_alignment;
use sddk_engine::software_alignment::types::{
    AlignmentScope, ArchitecturalIntentSnapshot, ConstraintId, ExplicitConstraint,
    MustDirection, ParadigmTag,
};

#[test]
fn namespace_bridge_unit_x_does_not_bind_component_x() {
    let c = ArchitecturalContract::declare_single_authority(
        ContractId::new("sa:auth".to_string()).unwrap(),
        ComponentRef::new("auth").unwrap(),
        DecisionRef::Decision("DEC".into()),
        SpecRef::Spec("SPEC-A4-3R".to_string()),
        Revision::new("rev-1".to_string()).unwrap(),
        EventTime(0),
    ).unwrap();
    let s = AlignmentScope::new("test".to_string());
    let con = ExplicitConstraint {
        id: ConstraintId::new("k1".to_string()),
        contract_ref: ContractId::new("sa:auth".to_string()).unwrap(),
        direction: MustDirection::Must,
        label: None,
    };
    let i = ArchitecturalIntentSnapshot {
        id: ArchitecturalIntentSnapshot::derive_id(&s, ParadigmTag::Hexagonal, &[con.clone()]),
        scope: s, paradigm_tag: ParadigmTag::Hexagonal, explicit_constraints: vec![con],
    };
    let basis = KnowledgeBasis::empty(EventTime(0));
    let obs_basis = ObservationBasis::new("rev-1", basis.basis_hash().clone(), "input");
    let ev = UniEvidenceRef::new(EvidenceKind::Adhoc, "ev".to_string());
    let mut obs = ObservationSet::new();
    obs.insert(SoftwareObservation::declare(
        ObservationSubject::Unit(SoftwareUnitRef::new("auth")),
        ObservationStance::Denies,
        ev,
        ObservationOrigin::DeterministicLocal,
        obs_basis,
        None,
        "producer",
    ));
    let r = reduce_alignment(&i, &basis, &obs, &[c], &[], EventTime(0)).unwrap();
    // EXPECTED (under the namespace model): no violation, because Unit("auth") != Component("auth").
    // ACTUAL (current reducer): violation, because as_str() == as_str().
    assert!(
        r.findings.iter().all(|f| !matches!(f.kind, sddk_engine::software_alignment::types::AlignmentFindingKind::ContractViolation)),
        "FAIL: Unit(SoftwareUnitRef(\"auth\")) bound SingleAuthority(ComponentRef(\"auth\")) via as_str() equivalence. This is the namespace-bridge bug."
    );
}
```

This test is expected to FAIL on the current reducer (`as_str()` equivalence
binds them). After this FU is closed, the test must PASS.

## 6. Remediation options

| Option | Description | Pros | Cons |
|---|---|---|---|
| **A. Centralise canonical adapter** | Declare at the model layer that `ComponentRef("x")` and `SoftwareUnitRef("x")` represent the same canonical subject IF AND ONLY IF declared so by an explicit adapter; default is no-identity. Add a typed adapter `ComponentRef::canonical_unit_subject()` / `SoftwareUnitRef::as_component_subject()` that requires the architecture-graph provenance. Update the reducer to use the adapter. | Minimal blast radius; preserves backward compatibility for legacy observations; explicit at the call site | Adapter must be threaded through the observation producer; every site that constructs an observation must opt-in |
| **B. Restrict ObservationSubject::Unit binding** | `BindingTarget::SingleAuthority` matches only against `ObservationSubject::ComponentSubject(ComponentRef)` (a new variant, not yet existing) or via a dedicated `ObservationSubject::Unit(ComponentRef)` carrying a typed `ComponentRef`. The current `ObservationSubject::Unit(SoftwareUnitRef)` would not bind SingleAuthority. | Strongest typed separation; matches the `canonical_tag` namespacing invariant | Requires introducing `ObservationSubject::ComponentSubject`; breaks observation producers that use `SoftwareUnitRef` to name architectural components; needs a producer migration |
| **C. Document the equivalence as canonical** | Declare at the architectural-decision level that `Component("x")` and `Unit("x")` are canonical-equivalent for the purposes of architectural-contract binding. Update the doc comments on `ComponentRef` / `SoftwareUnitRef` / `BindingTarget::SingleAuthority` to declare the equivalence. Add a falsification pin that asserts the equivalence is intentional. | Cheapest; preserves current behaviour; transparent to callers | Requires explicit architectural decision; the equivalence is a project-wide assumption that constrains future model evolution; risk of accidentally equating `Entity` and `Unit` later |

## 7. Recommendation

**Do NOT pick a remediation inside A4-5P** (this is a preflight cycle, change
budget is ZERO).

**Register this FU as a blocker of A4-5a.** A4-5a's composition would wire
`reduce_alignment` into the Intelligence Loop — if the reducer silently
binds `Unit("auth")` to `Component("auth")`, the loop will produce
`ContractViolation` findings that the loop then has to interpret. Without
resolving the bridge, the loop cannot distinguish a real architectural
violation from a namespace-mismatch false positive.

A4-5a must either (a) pick one of options A/B/C above and implement it, or
(b) escalate to A4-5b if the choice requires architectural debate beyond
the composition cycle.

## 8. Exit criteria

This FU is closed only when ALL of:

1. The reducer's `SingleAuthority ↔ Unit` and `UniqueOwner ↔ Unit` arms
   are typed-equality checks (no `as_str()` comparison between
   different namespaces) — or the equivalence is declared canonical
   under option C with a falsification pin.
2. The 18 pin tests in `a4_3r_typed_constraint_binding.rs` are
   augmented with a namespace-bridge pin (the one in §5 of this doc,
   asserting `Unit("auth") != Component("auth")` under options A/B
   or `Unit("auth") == Component("auth")` under option C).
3. The architectural model (likely arch-spec-042 or a new
   arch-spec) explicitly states whether the bridge exists and under
   what conditions.
4. `cargo test -p sddk-engine --workspace` is green.
5. `cargo clippy --workspace --all-targets -- -D warnings` is clean.

## 9. Cross-references

- A4-3R spec: `.sddk/cycles/p-63676b11dc0ef88f-a4-3r-typed-constraint-binding/spec.md`
- A4-3R handoff: `docs/handoff/HANDOFF-2026-09-17-a4-3r-typed-constraint-binding-v1.169.61.md`
- A4-5P spec: `.sddk/cycles/p-63676b11dc0ef88f-a4-5p-intelligence-loop-entry-gate/spec.md`
- arch-spec-042: `docs/architecture/specs/arch-spec-042-evidence-observation-provenance.md`
- Reducer code: `crates/sddk-engine/src/software_alignment/reducer.rs:125-140`
- Observation namespace: `crates/sddk-engine/src/observation/types.rs:14-37`
