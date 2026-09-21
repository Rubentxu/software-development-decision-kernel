# CLI and Agent UX

The command names below are product proposals. The Command Registry remains the eventual typed source of truth.

## Read surfaces

```text
sddk architecture contracts
sddk architecture authorities
sddk architecture ownership
sddk architecture compatibility
sddk architecture paradigms
sddk architecture behavior-map
sddk architecture graph [--scope ...]
```

## Verification

```text
sddk verify architecture [--changed] [--contract ID]
sddk deb-verify architecture
sddk why architecture CONTRACT_OR_FINDING
```

## Diff/time travel

```text
sddk diff architecture REV_A..REV_B
```

## Counterfactual

```text
sddk plan architecture --move UNIT --to CONTEXT
sddk plan architecture --dependency FROM:TO
```

These commands return an advisory `CounterfactualAssessment`; they do not mutate code.

## Agent Experience

An agent should receive bounded context such as:

```text
ArchitectureConformanceDelta
- affected contracts
- verified claims
- contradictions
- unknown evidence
- relevant compatibility paths
- suggested investigation
```

not the whole architecture graph.

## WHY / WHY-NOT

`why architecture` should traverse:

```text
finding
 → claim assessment
 → contract
 → decision/spec
 → evidence
 → software relation
```

`why-not` can explain why a requested claim is not VERIFIED, including missing provider/evidence/fixture.

## DSL authoring

A future typed architecture-contract DSL SHOULD compile to the same `ArchitecturalContract` ADT used by programmatic APIs. Example authoring syntax may be YAML/Kotlin-like/Rust-builder-like, but syntax never owns semantics.

## Generated developer UX

Behavior maps, contract catalogs and paradigm topology SHOULD be generated from semantic objects/graph rather than maintained as competing handwritten truth.
