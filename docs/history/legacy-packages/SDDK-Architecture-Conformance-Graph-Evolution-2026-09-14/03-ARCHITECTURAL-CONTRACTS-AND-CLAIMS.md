# Architectural Contracts and Claims

## ArchitecturalContract

An accepted decision may publish one or more typed contracts. The human ADR explains *why*; the typed contract states *what must remain true*.

Illustrative ADT:

```rust
pub enum ArchitectureConstraint {
    SingleAuthority { concept: ConceptRef },
    UniqueOwner { concept: ConceptRef },
    ForbiddenDependency { from: ContextRef, to: ContextRef },
    ProjectionOnly { component: ComponentRef },
    DerivedFrom { projection: ComponentRef, authority: ConceptRef },
    RequiresAdmission { effect: EffectClass },
    CompatibilityBounded {
        component: ComponentRef,
        replacement: ComponentRef,
        removal_trigger: RemovalTrigger,
    },
    ProviderBoundary { port: PortRef, forbidden_inward_type_family: TypeFamily },
    InstructionBoundary { source: ComponentRef, allowed: bool },
    Custom { schema_id: String, payload: TypedValue },
}
```

This is an example shape, not a commitment to these exact Rust names.

## Claim lifecycle

A contract creates verifiable claims:

```text
DECLARED
  ↓ evidence collected
OBSERVED / INFERRED
  ↓ verification
VERIFIED | CONTRADICTED | UNKNOWN | STALE
```

A missing analyzer, provider or fixture SHALL yield `UNKNOWN`/`EvidenceGap`, never PASS.

## Decision authority

Markdown is not runtime authority. Near-term ingestion MAY read machine-readable frontmatter, but after validation the authoritative semantic object belongs to Decision Memory/revision substrate.

A later ADR/spec update may project contracts back into human documentation.

## Traceability chain

Each accepted contract SHOULD support:

```text
DecisionRef
  ↓ DECIDES
ArchitecturalContract
  ↓ CLAIMS
ArchitectureClaim
  ↓ VERIFIED_BY
Evidence/Test/UAT/Receipt
  ↓ OBSERVES
SoftwareUnit / relation / runtime behavior
```

## Contract families

Initial families with high value:

- authority uniqueness;
- ownership uniqueness;
- dependency direction;
- projection-only/read-model constraints;
- canonical write path;
- side-effect admission;
- compatibility boundedness;
- provider/host anti-corruption boundaries;
- instruction/advisory separation;
- state-class constraints;
- paradigm-specific intent constraints.

## Avoid overfitting

Not every ADR needs an executable contract. A contract is justified when:

1. regression is realistically possible;
2. evidence can be gathered deterministically or honestly marked unknown;
3. the claim affects architectural integrity, compatibility or production readiness.

## Why typed claims matter

They let SDDK move from document search to evidence planning:

```text
changed units
  → affected contracts
  → required probes
  → minimal Verify scope
  → ConformanceDelta
```
