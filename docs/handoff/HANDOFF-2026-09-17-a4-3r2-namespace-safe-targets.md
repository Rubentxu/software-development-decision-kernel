# A4-3R2 — Namespace-Safe Constraint Targets (handoff)

> **Cycle:** `p-63676b11dc0ef88f-a4-3r2-namespace-safe-targets`
> **Closed:** 2026-09-17
> **Release:** v1.169.62 (release tag `v1.169.62` → SHA
> `b9e928c6229e65dc141fb282c594931dc7ef7df3`)
> **FU closed:** `FU-A4-3R-TARGET-NAMESPACE-BRIDGE` (P1)
> **A4-5P:** PARTIAL → **CLOSED**
> **A4-5a:** structurally **unblocked**

## 0. Why this cycle existed

A4-5P §3 (the Intelligence Loop Entry Gate preflight, closed
2026-09-17 at commit `cbb39ac`) discovered that
`software_alignment::reducer::observation_matches_target` used
`as_str()` cross-namespace equality between distinct typed refs:

```rust
(BindingTarget::SingleAuthority(c), ObservationSubject::Unit(u)) => {
    u.as_str() == c.as_str()
}
(BindingTarget::UniqueOwner(e), ObservationSubject::Unit(u)) => u.as_str() == e.as_str(),
```

A4-3R had introduced typed binding, but only for `ForbiddenDependency`
(Component→Component relations). The unary subjects (`Unit`,
`Contract`, `Knowledge`) still relied on the legacy `as_str()` rule,
which silently equated different namespaces.

`ObservationSet::for_entity` had a related defect:

```rust
ObservationSubject::Unit(u) => {
    u.as_str() == entity.canonical_tag().trim_start_matches("unit:")
}
```

— the same kind of structural fragility dressed up as canonical-tag
arithmetic.

## 1. Direction (M0 evidence gate)

The user pre-committed to **B' — strict namespace separation** as the
default, conditional on M0 confirming that no productive producer
emits `Unit("x")` to mean `Component("x")` or `Entity("x")`.

### M0 inventory (local, against `cbb39acd`)

| Site | What it emits | Verdict |
|---|---|---|
| `paradigm_lens/translation.rs:48` | `ObservationSubject::Unit(SoftwareUnitRef)` for AC7 paradigm-lens observations derived from `anchor.locator()` (an AC7 path) | **Legitimate Unit producer** — AC7 paradigm membership is a unit-level concept; no migration needed. |
| `paradigm_lens/lenses.rs:116` | Same case | Same. |
| `architecture_declaration/validate.rs:91,95` | `SoftwareEntityRef::Unit(SoftwareUnitRef)` for declared observation endpoints | **Migration site**: the CLI inferred `Unit` for all endpoints regardless of context. A4-3R2 makes the CLI infer the typed endpoint (`Component` / `Entity` / `Unit`) per observation based on the contracts declared in the same declaration. |
| `architecture_why/traverse.rs:298-308` | `contract_entity(SingleAuthority(c)) → SoftwareEntityRef::Unit(SoftwareUnitRef(c.as_str()))` | **Migration site**: the legacy coercion that closed the audit "evidence → software relation" leg. Migrated to `SoftwareEntityRef::Component(c)` / `SoftwareEntityRef::Entity(e)`. |
| `debverify_kernel/strategy_architecture.rs:130-138` | `subject_for_audit_finding` for contract ids / subjects → `SoftwareEntityRef::Unit` | **Migration site**: migrated to `SoftwareEntityRef::Component` with a `<unknown>` sentinel fallback. |

No productive producer emits `Unit("x")` to mean `Component("x")` or
`Entity("x")`. **B' confirmed.** Option C (declare cross-namespace
equivalence universal) was rejected by the user as contradicting the
already-published namespaced invariant
(`SoftwareEntityRef::canonical_tag` is explicitly namespaced;
`arch-spec-042 §3.1` says "no parallel refs"). Option A (typed adapter)
was reserved for a future cycle if such a mapping becomes architecturally
necessary.

## 2. What changed

### 2.1 Type layer (single change budget: NAMESPACE-SAFE OBSERVATION TARGET BINDING)

`crates/sddk-engine/src/observation/types.rs`:

- `ObservationSubject` gains first-class `Component(ComponentRef)` and
  `Entity(EntityRef)` variants. The enum is six variants closed:
  `SoftwareRelation`, `Unit`, `Component`, `Entity`, `Contract`,
  `Knowledge`.
- `ObservationSubject::canonical_tag()` always emits a literal
  namespace prefix: `relation:<id>`, `unit:x`, `component:x`,
  `entity:x`, `contract:x`, `knowledge:x`. The prefix is part of
  identity and is never stripped.

`crates/sddk-engine/src/observation/posture.rs`:

- `ObservationTargetRef` gains the same two first-class variants.
  `kind_tag()` and `canonical_tag()` updated to match.

`crates/sddk-engine/src/observation/types.rs` — `ObservationSet`:

- `for_subject` adds `(T::Component(c), ObservationSubject::Component(obs_c)) => obs_c == c`
  and `(T::Entity(e), ObservationSubject::Entity(obs_e)) => obs_e == e`.
- `for_entity` rewritten: no more `trim_start_matches`. Every branch
  builds `SoftwareEntityRef::*` and compares `canonical_tag()` strings.
  The Unit, Component, and Entity branches all share the same
  namespaced equality semantics.

### 2.2 Reducer

`crates/sddk-engine/src/software_alignment/reducer.rs`:

- `BindingTarget::SingleAuthority(ComponentRef)` binds **only**
  `ObservationSubject::Component(ComponentRef)` by newtype equality.
- `BindingTarget::UniqueOwner(EntityRef)` binds **only**
  `ObservationSubject::Entity(EntityRef)` by newtype equality.
- `BindingTarget::ForbiddenDependency { from, to }` unchanged: still
  binds via `SoftwareRelation` whose endpoints are
  `SoftwareEntityRef::Component(...)`.
- `observation_matches_target` carries an explicit "same-namespace typed
  equality only" docstring.
- `subject_canonical_tag` no longer special-cases
  `SoftwareRelation::render()`. It always returns
  `o.subject.canonical_tag()`. **Side-effect:** any
  `AlignmentAssessment.derive_id` whose `ContradictionMarker.subjects`
  carries a `SoftwareRelation` will differ from the pre-A4-3R2 value.
  This is a correctness fix (the prior branch embedded rendered text in
  identity — a relic of the pre-A4-3R substring-collision rule).

### 2.3 Architecture-why

`crates/sddk-engine/src/architecture_why/traverse.rs`:

- `contract_entity(SingleAuthority(c))` returns
  `Some(SoftwareEntityRef::Component(c.clone()))`.
- `contract_entity(UniqueOwner(e))` returns
  `Some(SoftwareEntityRef::Entity(e.clone()))`.
- The dead `SoftwareUnitRef` import was removed.

### 2.4 DebVerify kernel

`crates/sddk-engine/src/debverify_kernel/strategy_architecture.rs`:

- `subject_for_audit_finding` emits
  `SoftwareEntityRef::Component(ComponentRef::new(raw))` with a
  `<unknown>` sentinel fallback for empty / whitespace-only strings.

### 2.5 CLI declaration validation

`crates/sddk-engine/src/architecture_declaration/validate.rs`:

- `convert_observations` inspects the declared contracts to infer the
  typed endpoint per observation: when a relation endpoint string is
  referenced by a declared `SingleAuthority(component)` or
  `UniqueOwner(entity)` contract, the matching observation endpoint is
  `SoftwareEntityRef::Component` / `SoftwareEntityRef::Entity`; in every
  other case (including AC7 paradigm membership) it remains
  `SoftwareEntityRef::Unit`.

## 3. Tests

### 3.1 New corpus

`crates/sddk-engine/tests/a4_3r2_namespace_safe_targets.rs` (new file):
**18 pins + 1 deterministic property test (200 scenarios)**.

| Pin | Subject |
|---|---|
| pin_a01 | `canonical_tag()` emits namespaced prefix per kind. |
| pin_a02 | `SingleAuthority + Unit(same string) → NO violation`. |
| pin_a03 | `SingleAuthority + Component(correct) + Denies → violation (ContradictsMust)`. |
| pin_a04 | `SingleAuthority + Component(other) → NO violation`. |
| pin_a05 | `UniqueOwner + Unit(same string) → NO violation`. |
| pin_a06 | `UniqueOwner + Entity(correct) + Affirms (MustNot) → violation (ContradictsMust)`. |
| pin_a07 | `UniqueOwner + Entity(other) → NO violation`. |
| pin_a08 | `ForbiddenDependency` behavior unchanged (Component→Component via `SoftwareRelation`). |
| pin_a09 | Observation insertion order irrelevant (`r1.id == r2.id`). |
| pin_a10 | Subject kind changes `ObservationId` (three distinct ids for Unit/Component/Entity with the same inner string). |
| pin_a11 | Subject kind changes target identity in `for_subject` (`unit:auth` / `component:auth` / `entity:auth` are three distinct canonical tags). |
| pin_a12 | `for_subject(Component(x))` never returns Unit(x) observations. |
| pin_a13 | `for_subject(Entity(x))` never returns Unit(x) observations. |
| pin_a14 | `for_entity(Unit(x))` matches Unit(x) only — never Component(x) or Entity(x). |
| pin_a15 | `for_entity(Component(x))` matches Component(x) only. |
| pin_a16 | `for_entity(Entity(x))` matches Entity(x) only. |
| pin_a17 | ProviderBoundary contract → typed `EvidenceGap { kind_tag: "provider_boundary" }`. |
| pin_a18 | Extension contract → typed `EvidenceGap { kind_tag: "custom_kind" }`. |

The property test (`property_namespace_separation_under_bounded_generator`)
runs 200 scenarios with a fixed seed `0xA4_3F_2E_5C_5A_FA_DE` and an
alphabet of 32 strings chosen so they collide across namespaces. It
asserts two properties:

1. For every identifier in the alphabet, `Unit(S) / Component(S) / Entity(S)`
   produce three distinct `canonical_tag`s.
2. For every (subject, target) pair generated, binding succeeds only when
   (subject.kind, target.kind) is the supported pair
   (`Component` → `SingleAuthority`, `Entity` → `UniqueOwner`) **and**
   `subject.identity == target.identity`.

This converts the A4-5P randomized probe (recorded as
`OBSERVED_SESSION_EVIDENCE`) into durable, reproducible evidence.
Seed, generator bounds, scenario count, and command are all persisted
in the test source.

### 3.2 A4-3R corpus migrated

`crates/sddk-engine/tests/a4_3r_typed_constraint_binding.rs`:

- **Pin 4**: `pin_r04_single_authority_matches_unit_denies` (which
  emitted `ObservationSubject::Unit(...)` for `SingleAuthority(...)` and
  relied on the pre-fix bug) replaced by
  `pin_r04_single_authority_binds_component_subject`, which emits
  `ObservationSubject::Component(...)`.
- **Pin 6**: `pin_r06_unique_owner_matches_entity` (same story)
  replaced by `pin_r06_unique_owner_binds_entity_subject`.
- Pins 1-3, 5, 7, 8-18 unchanged in semantics; helper
  `obs_component_deny` / `obs_entity_affirm` added for the migrated
  tests.

The A4-3R corpus stays at 18 pins total.

### 3.3 Other test migrations

`crates/sddk-engine/src/software_alignment/tests.rs` and
`crates/sddk-engine/src/architecture_why/tests.rs` — six pre-fix tests
migrated from `ObservationSubject::Unit(...)` to the correct typed
subjects for the contract being evaluated. The migration is mechanical
and verified by the corpus.

## 4. Documentation

- `docs/architecture/specs/arch-spec-042-evidence-observation-provenance.md`
  §3.1 gains the normative rule: "`SoftwareUnitRef`, `ComponentRef`,
  and `EntityRef` are three distinct identity namespaces. Equal inner
  strings across namespaces do NOT imply subject equivalence.
  Cross-namespace equivalence requires an explicit typed mapping (option
  A in `FU-A4-3R-TARGET-NAMESPACE-BRIDGE`), which is **not** part of
  A4-3R2."
- `docs/debt/FU-A4-3R-TARGET-NAMESPACE-BRIDGE.md` flipped to CLOSED,
  with §0 closure evidence pointing to this release.
- `.sddk/followups/a4-followups.md` — `FU-A4-3R-TARGET-NAMESPACE-BRIDGE`
  row updated to CLOSED by A4-3R2.
- `docs/architecture/README.md` — A4-5P row flipped to `CLOSED —
  v1.169.62`; new A4-3R2 row inserted; A4-5a blocker lifted.

## 5. Verification

| Gate | Result |
|---|---|
| `cargo build --release -p sddk-cli` | OK |
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | **4570 passed, 0 failed** |
| A4-3R corpus (18 pins) | green |
| A4-3R2 corpus (18 pins + 1 property) | green |
| A4-4bR / A4-4b / A4-4M / A4-4C corpus | green |
| PublicReleaseGate (step 9b) | PASS |
| `gh release view v1.169.62` | 9 assets present, draft=false, prerelease=false, tag SHA matches `b9e928c…` |
| Local install (`sddk dev doctor`, `sddk dev update --prune-only --keep 1`) | bundle coherent, framework/<v>/ installed at `~/.local/share/sddk/framework/1.169.62/` |

## 6. Cycles closed

| Cycle | State transition |
|---|---|
| `p-63676b11dc0ef88f-a4-3r2-namespace-safe-targets` | OPEN → **CLOSED** |
| `p-63676b11dc0ef88f-a4-5p-intelligence-loop-entry-gate` | PARTIAL/STOP → **CLOSED** (entry gate unblocked) |

## 7. Roadmap after A4-3R2

```
A4-5P        PARTIAL ✓ → CLOSED ✓
   ↓
A4-3R2       namespace-safe targets     ← CLOSED (this cycle)
   ↓
A4-5a        composition only           ← structurally unblocked
   ↓
A4-5b        advisory + WHY/WHY-NOT
   ↓
A4-5C        arch-spec-047 acceptance
   ↓
A4-CLOSEOUT
   ↓
A5
```

## 8. STOP rule applies

A4-5a does NOT auto-open. The user's pattern is "one cycle per
session, STOP after close, await next green-light".

If the next green-light targets A4-5a, it should be a composition-only
cycle that uses the new typed substrate (`ObservationSubject::Component`
and `ObservationSubject::Entity` are now first-class) without
re-introducing any cross-namespace equivalence.

## 9. Cross-references

- Cycle scope contract:
  `.sddk/cycles/p-63676b11dc0ef88f-a4-3r2-namespace-safe-targets/spec.md`
  (also at `~/.sddk-knowledge/sddk-framework/cycles/p-63676b11dc0ef88f-a4-3r2-namespace-safe-targets/spec.md`)
- FU closure evidence: `docs/debt/FU-A4-3R-TARGET-NAMESPACE-BRIDGE.md`
- A4-5P handoff: `docs/handoff/HANDOFF-2026-09-17-a4-5p-intelligence-loop-entry-gate.md`
- A4-3R handoff (predecessor): `docs/handoff/HANDOFF-2026-09-17-a4-3r-typed-constraint-binding-v1.169.61.md`
- Roadmap: `docs/architecture/README.md`
- Release receipt: `gh-release-receipt.json` (in the GH Release assets)
